//! Git-based project import: URL validation (https/file only), GitHub token
//! storage and scoped `http.extraHeader` auth, cloning with cleanup on
//! failure, and the Laravel-root check after cloning.

use crate::{
    model::{slug_from_folder, validate_slug, Project},
    process::{command, ManagedChild},
    release::{git_program, validate_git_branch},
    secrets, storage, Manager,
};
use anyhow::{bail, Context, Result};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

mod connection;
pub(crate) use connection::GithubRuntime;
pub use connection::{GithubAuthFlow, GithubAuthPoll, GithubBranchPage, GithubRepoPage};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GithubRepository {
    pub owner: String,
    pub name: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubState {
    pub token_saved: bool,
    pub login: Option<String>,
    pub oauth_client_id: String,
    pub auth_method: Option<String>,
    pub expires_at: Option<u64>,
}

#[derive(Serialize, Deserialize)]
struct GithubAccount {
    login: String,
}

pub fn parse_github_repository(raw: &str) -> Result<GithubRepository> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        bail!("GitHub deposu gerekli. owner/repo veya https://github.com/owner/repo yazın.");
    }
    if trimmed.len() > 256 {
        bail!("GitHub deposu en fazla 256 karakter olabilir.");
    }
    let value = trimmed
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .replace('\\', "/");
    let path = if let Some(rest) = value.strip_prefix("git@github.com:") {
        rest.to_string()
    } else if let Some(rest) = value
        .strip_prefix("https://github.com/")
        .or_else(|| value.strip_prefix("http://github.com/"))
        .or_else(|| value.strip_prefix("github.com/"))
    {
        rest.to_string()
    } else if value.contains("://") || value.contains('@') || value.contains("github.com") {
        bail!("Yalnızca github.com depoları desteklenir.");
    } else {
        value
    };
    let mut parts = path.split('/');
    let owner = parts.next().unwrap_or_default();
    let name = parts.next().unwrap_or_default();
    if parts.next().is_some() {
        bail!("Depo adresi owner/repo olmalı; ek yol kabul edilmez.");
    }
    if !valid_github_name(owner) || !valid_github_name(name) {
        bail!("Geçersiz depo. owner/repo yalnızca harf, rakam, nokta, _ ve - içerebilir.");
    }
    Ok(GithubRepository {
        owner: owner.into(),
        name: name.into(),
    })
}

/// Accept only the transports the release flow can drive without prompts:
/// `https://` remotes and absolute `file://` repositories (used by tests and
/// for mirroring from another disk). SSH would need an agent, and `ext::`
/// or bare paths would let a remote string act as a command line.
pub fn validate_git_url(raw: &str) -> Result<String> {
    let url = raw.trim();
    if url.is_empty() {
        bail!("Git deposu adresi gerekli.");
    }
    if url.len() > 512 {
        bail!("Git deposu adresi en fazla 512 karakter olabilir.");
    }
    if url.starts_with('-') || url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        bail!("Git deposu adresi boşluk veya denetim karakteri içeremez.");
    }
    let rest = if let Some(rest) = url.strip_prefix("https://") {
        rest
    } else if let Some(rest) = url.strip_prefix("file://") {
        if rest.is_empty() {
            bail!("file:// adresi mutlak bir klasör yolu içermeli.");
        }
        return Ok(url.to_string());
    } else {
        bail!("Yalnızca https:// veya file:// git adresleri desteklenir.");
    };
    let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
    if host.is_empty()
        || host.contains('@')
        || !host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b':'))
    {
        bail!("Git deposu sunucu adı geçersiz.");
    }
    if path.is_empty() || path.split('/').any(|segment| segment == "..") {
        bail!("Git deposu yolu owner/repo biçiminde olmalı.");
    }
    Ok(url.to_string())
}

pub fn validate_github_token(raw: &str) -> Result<String> {
    let token: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    if !(20..=255).contains(&token.len()) {
        bail!("GitHub jetonu 20–255 karakter olmalı. GitHub → Settings → Developer settings → Personal access tokens.");
    }
    let known = token.starts_with("ghp_")
        || token.starts_with("github_pat_")
        || token.starts_with("gho_")
        || token.starts_with("ghu_")
        || token.starts_with("ghs_")
        || (token.len() == 40 && token.bytes().all(|b| b.is_ascii_hexdigit()));
    if !known
        || !token
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        bail!("GitHub kişisel erişim jetonunu olduğu gibi yapıştırın. Özel depolar için repo yetkisi gerekir.");
    }
    Ok(token)
}

fn valid_github_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 100
        && !value.starts_with('.')
        && !value.ends_with('.')
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

pub fn apply_github_git_auth(cmd: &mut Command, token: Option<&str>) {
    cmd.env("GIT_TERMINAL_PROMPT", "0")
        .env("GCM_INTERACTIVE", "never")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        // Submodule and redirect targets inherit this: never let a repository
        // pull in ssh/ext transports that could prompt or run commands.
        .env("GIT_ALLOW_PROTOCOL", "https:file");
    if let Some(token) = token {
        // URL-scoped: a release of a project whose origin is GitLab, Bitbucket
        // or a company server must never receive the GitHub token.
        cmd.env("GIT_CONFIG_COUNT", "2")
            .env("GIT_CONFIG_KEY_0", "credential.helper")
            .env("GIT_CONFIG_VALUE_0", "")
            .env("GIT_CONFIG_KEY_1", "http.https://github.com/.extraHeader")
            .env(
                "GIT_CONFIG_VALUE_1",
                // Git smart HTTP uses Basic token auth, matching actions/checkout.
                // REST API requests use Bearer separately in connection.rs.
                format!(
                    "AUTHORIZATION: basic {}",
                    base64::engine::general_purpose::STANDARD
                        .encode(format!("x-access-token:{token}"))
                ),
            );
    }
}

pub(crate) fn is_github_https_url(url: &str) -> bool {
    reqwest::Url::parse(url)
        .is_ok_and(|url| url.scheme() == "https" && url.host_str() == Some("github.com"))
}

impl Manager {
    pub(crate) fn github_state(&self) -> GithubState {
        let connection = self.github_connection().ok().flatten();
        GithubState {
            token_saved: self.github_has_token(),
            login: connection
                .as_ref()
                .map(|c| c.login.clone())
                .or_else(|| self.github_login()),
            oauth_client_id: self.github_client_id().unwrap_or_default(),
            auth_method: connection
                .as_ref()
                .map(|c| c.method.clone())
                .or_else(|| self.github_token_path().is_file().then(|| "token".into())),
            expires_at: connection.and_then(|c| c.expires_at),
        }
    }

    pub fn save_github_token(&self, token: &str) -> Result<()> {
        let _guard = self.gate()?;
        let token = validate_github_token(token)?;
        let login = self.github_login_for_token(&token)?;
        self.github_store_manual_token(&token, &login)?;
        self.log(format!(
            "GitHub hesabı kaydedildi: {login}. Jeton Windows hesabınıza bağlı olarak şifrelenir."
        ));
        Ok(())
    }

    pub fn clear_github_token(&self) -> Result<()> {
        let _guard = self.cleanup_gate()?;
        self.github_forget_connection()?;
        for path in [self.github_token_path(), self.github_account_path()] {
            if path.exists() {
                std::fs::remove_file(&path).context("GitHub jetonu silinemedi.")?;
            }
        }
        self.log("GitHub jetonu silindi. Genel depolar jeton olmadan klonlanabilir.");
        Ok(())
    }

    pub fn import_github_project(
        &self,
        repository: &str,
        name: String,
        branch: String,
    ) -> Result<Project> {
        self.import_github_project_with_token(repository, name, branch, None)
    }

    pub fn import_github_project_with_token(
        &self,
        repository: &str,
        name: String,
        branch: String,
        access_token: Option<&str>,
    ) -> Result<Project> {
        let repo = parse_github_repository(repository)?;
        let name = if name.trim().is_empty() {
            slug_from_folder(&repo.name)?
        } else {
            name
        };
        let url = format!("https://github.com/{}/{}.git", repo.owner, repo.name);
        self.import_git_project_with_token(&url, name, branch, access_token)
    }

    /// Clone any `https://` or `file://` repository into the projects folder and
    /// register it. The GitHub token, when saved, is attached to every remote
    /// call so private repositories work without credential prompts.
    pub fn import_git_project(&self, url: &str, name: String, branch: String) -> Result<Project> {
        self.import_git_project_with_token(url, name, branch, None)
    }

    pub fn import_git_project_with_token(
        &self,
        url: &str,
        name: String,
        branch: String,
        access_token: Option<&str>,
    ) -> Result<Project> {
        let _guard = self.gate()?;
        let url = validate_git_url(url)?;
        validate_git_branch(&branch)?;
        let name = if name.trim().is_empty() {
            let folder = url
                .trim_end_matches(['/', '\\'])
                .trim_end_matches(".git")
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or_default();
            slug_from_folder(folder)?
        } else {
            validate_slug(&name)?;
            name
        };
        if self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .projects
            .iter()
            .any(|project| project.name == name)
        {
            bail!("Bu proje adı zaten kayıtlı.");
        }
        let parent = self.github_clone_parent()?;
        let destination = parent.join(&name);
        if destination.exists() {
            bail!("Hedef klasör zaten var; mevcut dosyaların üzerine yazılmadı.");
        }
        std::fs::create_dir_all(&parent).context("Proje çalışma alanı oluşturulamadı.")?;
        crate::storage::require_space(&parent, 64 * 1024 * 1024)?;
        self.clone_git_repository_with_token(&url, &destination, &branch, access_token)?;
        if !destination.join("public/index.php").is_file() {
            bail!(
                "Depo klonlandı ama public/index.php yok. Laravel kökünü seçin; oluşan klasör korundu."
            );
        }
        let mut project = self
            .add_project_inner(name, destination.clone())
            .with_context(|| {
                format!(
                    "Depo {} klasörüne klonlandı ancak proje listesine eklenemedi",
                    destination.display()
                )
            })?;
        if !branch.is_empty() {
            self.set_project_release_branch(&project.id, &branch)?;
            project.release.branch = branch;
        }
        Ok(project)
    }

    pub(crate) fn apply_github_git_auth(&self, cmd: &mut Command) -> Result<()> {
        self.apply_github_git_auth_with(cmd, None)
    }

    pub(crate) fn apply_github_git_auth_with(
        &self,
        cmd: &mut Command,
        access_token: Option<&str>,
    ) -> Result<()> {
        let owned;
        let token = if let Some(token) = access_token {
            owned = Some(validate_github_token(token)?);
            owned.as_deref()
        } else if self.github_has_token() {
            owned = Some(self.github_token()?);
            owned.as_deref()
        } else {
            None
        };
        apply_github_git_auth(cmd, token);
        Ok(())
    }

    fn github_token(&self) -> Result<String> {
        self.github_access_token()
    }

    fn github_login(&self) -> Option<String> {
        let bytes = storage::read_limited(&self.github_account_path(), 4 * 1024).ok()?;
        serde_json::from_slice::<GithubAccount>(&bytes)
            .ok()
            .map(|account| account.login)
    }

    fn github_token_path(&self) -> PathBuf {
        self.home.join("config/github-token.dpapi")
    }

    fn github_account_path(&self) -> PathBuf {
        self.home.join("config/github-account.json")
    }

    fn github_clone_parent(&self) -> Result<PathBuf> {
        let projects_dir = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .settings
            .projects_dir
            .clone();
        if projects_dir.is_empty() {
            Ok(self.default_projects_dir())
        } else {
            Ok(PathBuf::from(projects_dir))
        }
    }

    fn github_login_for_token(&self, token: &str) -> Result<String> {
        self.github_runtime.http.login(token)
    }

    pub(crate) fn clone_git_repository(
        &self,
        url: &str,
        destination: &Path,
        branch: &str,
    ) -> Result<()> {
        self.clone_git_repository_with_token(url, destination, branch, None)
    }

    pub(crate) fn clone_git_repository_with_token(
        &self,
        url: &str,
        destination: &Path,
        branch: &str,
        access_token: Option<&str>,
    ) -> Result<()> {
        let git = git_program()?;
        let mut cmd = command(git);
        cmd.arg("clone").arg("--no-tags");
        if !branch.is_empty() {
            cmd.args(["--branch", branch, "--single-branch"]);
        }
        cmd.arg("--").arg(url).arg(destination);
        if is_github_https_url(url) {
            self.apply_github_git_auth_with(&mut cmd, access_token)?;
        } else {
            apply_github_git_auth(&mut cmd, None);
        }
        self.log(format!(
            "Git deposu klonlanıyor: {url}{}",
            if branch.is_empty() {
                String::new()
            } else {
                format!(" ({branch})")
            }
        ));
        let result = ManagedChild::spawn(cmd, &self.home.join("logs/github.log"))
            .and_then(|mut child| child.wait_timeout(Duration::from_secs(600)));
        if let Err(error) = result {
            // A half-written clone would block the next attempt with "folder
            // exists"; the folder did not exist before this call, so drop it.
            if destination.exists() {
                let _ = std::fs::remove_dir_all(destination);
            }
            let hint = if access_token.is_some() || self.github_has_token() {
                "Git deposu klonlanamadı. github günlüğünü kontrol edin."
            } else {
                "Git deposu klonlanamadı. Özel depolar için Hizmetler → GitHub ekranından jeton kaydedin."
            };
            return Err(error.context(hint));
        }
        Ok(())
    }

    fn set_project_release_branch(&self, id: &str, branch: &str) -> Result<()> {
        let mut config = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let project = config
            .projects
            .iter_mut()
            .find(|project| project.id == id)
            .context("Proje bulunamadı.")?;
        project.release.branch = branch.into();
        self.save_config(&config)
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_github_repository, validate_git_url, validate_github_token};

    #[test]
    fn git_urls_are_limited_to_https_and_file() {
        assert!(validate_git_url("https://github.com/owner/repo.git").is_ok());
        assert!(validate_git_url("https://gitlab.example.com:8443/group/repo").is_ok());
        assert!(validate_git_url("file:///srv/git/repo.git").is_ok());
        for raw in [
            "",
            "git@github.com:owner/repo.git",
            "ssh://git@github.com/owner/repo",
            "ext::sh -c id",
            "http://github.com/owner/repo",
            "https://",
            "https://github.com",
            "https://user@github.com/owner/repo",
            "https://github.com/../repo",
            "-c core.pager=id",
            "file://",
            "https://github.com/owner/repo with space",
        ] {
            assert!(validate_git_url(raw).is_err(), "{raw}");
        }
    }

    #[test]
    fn parses_owner_repo_and_common_urls() {
        for raw in [
            "beyazitkolemen/magaza",
            "https://github.com/beyazitkolemen/magaza",
            "https://github.com/beyazitkolemen/magaza.git",
            "git@github.com:beyazitkolemen/magaza.git",
            "github.com/beyazitkolemen/magaza/",
        ] {
            let repo = parse_github_repository(raw).unwrap();
            assert_eq!(repo.owner, "beyazitkolemen");
            assert_eq!(repo.name, "magaza");
        }
    }

    #[test]
    fn rejects_non_github_hosts_and_paths() {
        for raw in [
            "",
            "https://gitlab.com/owner/repo",
            "owner/repo/extra",
            "../etc",
            "owner/repo;rm",
        ] {
            assert!(parse_github_repository(raw).is_err(), "{raw}");
        }
    }

    #[test]
    fn accepts_known_personal_access_tokens() {
        assert!(validate_github_token(&format!("ghp_{}", "a".repeat(36))).is_ok());
        assert!(validate_github_token(&format!("github_pat_{}", "b".repeat(40))).is_ok());
        assert!(validate_github_token(&format!("ghs_{}", "c".repeat(36))).is_ok());
        assert!(validate_github_token(&"c".repeat(40)).is_ok());
        assert!(validate_github_token("too-short").is_err());
        assert!(validate_github_token("ghp_bad token spaces").is_err());
    }
}
