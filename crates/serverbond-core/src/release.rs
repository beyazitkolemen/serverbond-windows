//! Local production release ("Forge-like" deployment): git pull/branch
//! switch, Composer install, migrations, cache rebuild, extra Artisan lines
//! and job restarts, recorded as a JSON-lines history per project.

use crate::{
    jobs::artisan_file,
    model::{Project, ProjectRelease},
    process::{command, ManagedChild},
    Manager,
};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

const RELEASE_TIMEOUT: Duration = Duration::from_secs(600);
const HISTORY_LIMIT: usize = 20;
const MAX_OUTPUT_CHARS: usize = 200_000;
const MAX_EXTRA_ARTISAN: usize = 12;
const GIT_MISSING: &str = "git PATH üzerinde bulunamadı. Git for Windows kurun ve PATH'e ekleyin.";

pub fn release_revision(release: &ProjectRelease) -> Result<String> {
    use sha2::{Digest, Sha256};
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(release)?)
    ))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReleaseStep {
    pub name: String,
    pub args: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseRecord {
    pub started_at: String,
    pub duration_ms: u64,
    pub branch: String,
    pub sha: String,
    pub success: bool,
    pub output: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectGitStatus {
    pub present: bool,
    pub branch: String,
    pub sha: String,
}

pub fn git_program_from(path: Option<&OsStr>) -> Result<PathBuf> {
    let Some(path) = path.filter(|value| !value.is_empty()) else {
        bail!("{GIT_MISSING}");
    };
    for dir in std::env::split_paths(path) {
        for name in ["git", "git.exe"] {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    bail!("{GIT_MISSING}");
}

pub fn git_program() -> Result<PathBuf> {
    git_program_from(std::env::var_os("PATH").as_deref())
}

pub fn validate_git_branch(branch: &str) -> Result<()> {
    if branch.is_empty() {
        return Ok(());
    }
    if branch.len() > 128
        || branch.starts_with('-')
        || branch.starts_with('/')
        || branch.ends_with('/')
        || branch.contains("..")
        || !branch
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '/' | '.'))
    {
        bail!("Dal adı 1–128 karakter olmalı; harf, rakam, /, -, _ ve nokta kullanın.");
    }
    Ok(())
}

pub fn parse_extra_artisan(line: &str) -> Result<Vec<String>> {
    let line = line.trim();
    if line.is_empty() {
        bail!("Artisan satırı boş olamaz.");
    }
    if line.len() > 160 {
        bail!("Artisan satırı en fazla 160 karakter olabilir.");
    }
    let mut parts = line.split_whitespace();
    let command = parts.next().unwrap_or_default();
    if command.is_empty()
        || command.starts_with('-')
        || !command.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b':' | b'_' | b'-')
        })
    {
        bail!("Artisan komutu a-z, 0-9, :, _ ve - karakterlerinden oluşmalı.");
    }
    let mut args = vec!["artisan".into(), command.into()];
    for part in parts {
        let Some(rest) = part.strip_prefix("--") else {
            bail!("Ek Artisan satırında yalnızca --bayrak veya --bayrak=değer kullanın.");
        };
        let (name, value) = rest
            .split_once('=')
            .map(|(name, value)| (name, Some(value)))
            .unwrap_or((rest, None));
        if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
            bail!("Geçersiz Artisan bayrağı.");
        }
        if let Some(value) = value {
            if value.is_empty()
                || value.bytes().any(|b| {
                    matches!(
                        b,
                        b';' | b'|' | b'&' | b'$' | b'`' | b'\n' | b'\r' | b'(' | b')'
                    )
                })
            {
                bail!("Artisan bayrak değeri geçersiz karakter içeriyor.");
            }
        }
        args.push(part.to_string());
    }
    Ok(args)
}

pub fn migrate_args() -> Vec<String> {
    vec![
        "artisan".into(),
        "migrate".into(),
        "--force".into(),
        "--no-ansi".into(),
    ]
}

pub fn optimize_clear_args() -> Vec<String> {
    vec![
        "artisan".into(),
        "optimize:clear".into(),
        "--no-interaction".into(),
        "--no-ansi".into(),
    ]
}

pub fn composer_install_args(no_dev: bool) -> Vec<String> {
    let mut args = vec![
        "install".into(),
        "--no-interaction".into(),
        "--prefer-dist".into(),
        "--no-progress".into(),
        "--no-ansi".into(),
    ];
    if no_dev {
        args.push("--no-dev".into());
        args.push("--optimize-autoloader".into());
    }
    args
}

pub fn validate_project_release(release: &ProjectRelease) -> Result<()> {
    validate_git_branch(&release.branch)?;
    if release.extra_artisan.len() > MAX_EXTRA_ARTISAN {
        bail!("En fazla {MAX_EXTRA_ARTISAN} ek Artisan satırı eklenebilir.");
    }
    for line in &release.extra_artisan {
        parse_extra_artisan(line)?;
    }
    Ok(())
}

pub fn release_steps(release: &ProjectRelease) -> Result<Vec<ReleaseStep>> {
    validate_project_release(release)?;
    let mut steps = Vec::new();
    if release.git_pull {
        steps.push(ReleaseStep {
            name: "git".into(),
            args: vec!["pull".into(), "--ff-only".into(), "--no-edit".into()],
        });
    }
    if release.composer {
        steps.push(ReleaseStep {
            name: "composer".into(),
            args: composer_install_args(release.composer_no_dev),
        });
    }
    if release.build {
        steps.push(ReleaseStep {
            name: "build".into(),
            args: vec![],
        });
    }
    if release.migrate {
        steps.push(ReleaseStep {
            name: "migrate".into(),
            args: migrate_args(),
        });
    }
    if release.optimize_clear {
        steps.push(ReleaseStep {
            name: "optimize".into(),
            args: optimize_clear_args(),
        });
    }
    for line in &release.extra_artisan {
        let args = parse_extra_artisan(line)?;
        steps.push(ReleaseStep {
            name: args[1].clone(),
            args,
        });
    }
    if release.restart_jobs {
        steps.push(ReleaseStep {
            name: "jobs".into(),
            args: Vec::new(),
        });
    }
    if steps.is_empty() {
        bail!("Sürüm tarifinde en az bir adım seçin.");
    }
    Ok(steps)
}

fn history_path(home: &Path, project_id: &str) -> Result<PathBuf> {
    if uuid::Uuid::parse_str(project_id).is_err() {
        bail!("Geçersiz proje kimliği.");
    }
    Ok(home
        .join("logs")
        .join(format!("release-{project_id}.jsonl")))
}

fn trim_output(text: &str) -> String {
    if text.chars().count() <= MAX_OUTPUT_CHARS {
        return text.to_string();
    }
    text.chars().take(MAX_OUTPUT_CHARS).collect::<String>() + "\n… (çıktı kısaltıldı)"
}

fn combined(stdout: &[u8], stderr: &[u8]) -> String {
    let out = String::from_utf8_lossy(stdout).trim().to_string();
    let err = String::from_utf8_lossy(stderr).trim().to_string();
    match (out.is_empty(), err.is_empty()) {
        (true, true) => String::new(),
        (false, true) => out,
        (true, false) => err,
        (false, false) => format!("{out}\n{err}"),
    }
}

fn remaining(deadline: Instant) -> Result<Duration> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|left| *left > Duration::from_millis(250))
        .ok_or_else(|| anyhow::anyhow!("Sürüm 10 dakika sınırına ulaştı."))
}

fn run_command(cmd: Command, timeout: Duration) -> Result<String> {
    let output = ManagedChild::output(cmd, timeout)?;
    let text = combined(&output.stdout, &output.stderr);
    if !output.status.success() {
        bail!(
            "{}",
            if text.is_empty() {
                output.status.to_string()
            } else {
                text
            }
        );
    }
    Ok(text)
}

fn git_command_with_token(
    manager: &Manager,
    project: &Project,
    args: &[String],
    access_token: Option<&str>,
) -> Result<Command> {
    let git = git_program()?;
    let mut cmd = command(git);
    cmd.args(args).current_dir(&project.path);
    // Local status/branch operations and unrelated remotes must remain usable
    // when a saved GitHub credential has expired or cannot be decrypted.
    let network = matches!(args.first().map(String::as_str), Some("fetch" | "pull"));
    let github_remote = network
        && manager
            .git_output(project, &["remote".into(), "-v".into()])?
            .lines()
            .filter_map(|line| line.split_whitespace().nth(1))
            .any(crate::github::is_github_https_url);
    if github_remote {
        manager.apply_github_git_auth_with(&mut cmd, access_token)?;
    } else {
        crate::github::apply_github_git_auth(&mut cmd, None);
    }
    Ok(cmd)
}

impl Manager {
    pub fn save_project_release(&self, id: &str, release: ProjectRelease) -> Result<()> {
        self.save_project_release_checked(id, release, None)
    }

    pub fn save_project_release_checked(
        &self,
        id: &str,
        release: ProjectRelease,
        expected: Option<&str>,
    ) -> Result<()> {
        let _guard = self.gate()?;
        validate_project_release(&release)?;
        let mut config = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let project = config
            .projects
            .iter_mut()
            .find(|p| p.id == id)
            .context("Proje bulunamadı.")?;
        let name = project.name.clone();
        if let Some(expected) = expected {
            anyhow::ensure!(
                release_revision(&project.release)? == expected,
                "Sürüm tarifi değişti. Güncel bilgileri alın."
            );
        }
        project.release = release;
        self.save_config(&config)?;
        self.log(format!("Sürüm tarifi kaydedildi: {name}"));
        Ok(())
    }

    pub fn project_git_status(&self, id: &str) -> Result<ProjectGitStatus> {
        let project = self.project(id)?;
        if git_program().is_err() || !project.path.join(".git").exists() {
            return Ok(ProjectGitStatus {
                present: false,
                branch: String::new(),
                sha: String::new(),
            });
        }
        let branch = match self.git_output(&project, &["branch".into(), "--show-current".into()]) {
            Ok(value) => value,
            Err(_) => {
                return Ok(ProjectGitStatus {
                    present: false,
                    branch: String::new(),
                    sha: String::new(),
                })
            }
        };
        let sha = self
            .git_output(
                &project,
                &["rev-parse".into(), "--short".into(), "HEAD".into()],
            )
            .unwrap_or_default();
        if sha.is_empty() {
            return Ok(ProjectGitStatus {
                present: false,
                branch: String::new(),
                sha: String::new(),
            });
        }
        Ok(ProjectGitStatus {
            present: true,
            branch,
            sha,
        })
    }

    pub fn list_project_releases(&self, id: &str) -> Result<Vec<ReleaseRecord>> {
        let _project = self.project(id)?;
        let path = history_path(&self.home, id)?;
        if !path.exists() {
            return Ok(Vec::new());
        }
        let text = String::from_utf8_lossy(&crate::storage::read_limited(&path, 16 * 1024 * 1024)?)
            .into_owned();
        let mut records = Vec::new();
        for line in text.lines().rev() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(record) = serde_json::from_str::<ReleaseRecord>(line) {
                records.push(record);
            }
            if records.len() == HISTORY_LIMIT {
                break;
            }
        }
        Ok(records)
    }

    pub fn deploy_project(&self, id: &str) -> Result<ReleaseRecord> {
        self.deploy_project_checked(id, None)
    }

    pub fn deploy_project_checked(
        &self,
        id: &str,
        expected: Option<&str>,
    ) -> Result<ReleaseRecord> {
        self.deploy_project_checked_with_token(id, expected, None)
    }

    pub fn deploy_project_checked_with_token(
        &self,
        id: &str,
        expected: Option<&str>,
        access_token: Option<&str>,
    ) -> Result<ReleaseRecord> {
        let _guard = self.gate()?;
        let project = self.project(id)?;
        if let Some(expected) = expected {
            anyhow::ensure!(
                release_revision(&project.release)? == expected,
                "Sürüm tarifi değişti. Dağıtımı başlatmadan güncel bilgileri alın."
            );
        }
        let steps = release_steps(&project.release)?;
        if project.release.git_pull {
            git_program()?;
        }
        if project.release.composer {
            self.executable("composer")?;
        }
        if project.release.migrate
            || project.release.optimize_clear
            || !project.release.extra_artisan.is_empty()
            || project.release.restart_jobs
        {
            artisan_file(&project)?;
        }
        if project.release.build {
            self.tool_directory(crate::node::ID)?;
        }
        let started = Instant::now();
        let started_at = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let deadline = started + RELEASE_TIMEOUT;
        let mut chunks = Vec::new();
        let mut success = true;
        let mut error = None;
        for step in steps {
            self.cloud_stage(match step.name.as_str() {
                "git" => "clone",
                "composer" => "composer",
                "build" => "build",
                "migrate" => "database",
                _ => "configure",
            });
            chunks.push(format!("--- {} ---", step.name));
            match self.run_release_step_with_token(&project, &step, deadline, access_token) {
                Ok(output) => {
                    if !output.is_empty() {
                        // Cap per step: a chatty composer run must not hold
                        // tens of megabytes before the final trim.
                        chunks.push(trim_output(&output));
                    }
                }
                Err(err) => {
                    success = false;
                    let message = trim_output(&format!("{err:#}"));
                    chunks.push(message.clone());
                    error = Some(message);
                    break;
                }
            }
        }
        let git = self.project_git_status(id).unwrap_or(ProjectGitStatus {
            present: false,
            branch: project.release.branch.clone(),
            sha: String::new(),
        });
        let record = ReleaseRecord {
            started_at,
            duration_ms: started.elapsed().as_millis() as u64,
            branch: if git.branch.is_empty() {
                project.release.branch.clone()
            } else {
                git.branch
            },
            sha: git.sha,
            success,
            output: trim_output(&chunks.join("\n")),
        };
        if let Err(error) = self.write_release_record(id, &record) {
            // The release itself already happened; a history-file problem is
            // worth a log line, not a failed deployment.
            self.log(format!(
                "{} sürüm geçmişi yazılamadı: {error:#}",
                project.name
            ));
        }
        self.log(format!(
            "{} yerel sürüm {}: {} ms",
            project.name,
            if success {
                "tamamlandı"
            } else {
                "başarısız"
            },
            record.duration_ms
        ));
        if let Some(message) = error {
            bail!("{message}");
        }
        Ok(record)
    }

    fn run_release_step_with_token(
        &self,
        project: &Project,
        step: &ReleaseStep,
        deadline: Instant,
        access_token: Option<&str>,
    ) -> Result<String> {
        let timeout = remaining(deadline)?;
        match step.name.as_str() {
            "git" => {
                let branch = project.release.branch.as_str();
                if branch.is_empty() {
                    return run_command(
                        git_command_with_token(self, project, &step.args, access_token)?,
                        timeout,
                    );
                }
                // Single-branch clones only track the branch they were created
                // with, so fetch the requested branch explicitly before switching;
                // `switch` never confuses a branch name with a path the way
                // `checkout` can, and pulling with an explicit remote does not
                // depend on upstream configuration.
                let remote = self.git_remote(project)?;
                let mut parts = Vec::new();
                // A single-branch clone only fetches the branch it was created
                // with; widen the remote so the release branch becomes a real
                // remote-tracking branch that `--track` and future pulls accept.
                run_command(
                    git_command_with_token(
                        self,
                        project,
                        &[
                            "remote".into(),
                            "set-branches".into(),
                            "--add".into(),
                            remote.clone(),
                            branch.into(),
                        ],
                        access_token,
                    )?,
                    timeout,
                )?;
                let fetch = vec![
                    "fetch".to_string(),
                    "--no-tags".into(),
                    remote.clone(),
                    branch.into(),
                ];
                let output = run_command(
                    git_command_with_token(self, project, &fetch, access_token)?,
                    remaining(deadline)?,
                )?;
                if !output.is_empty() {
                    parts.push(output);
                }
                let local_exists = self
                    .git_output(
                        project,
                        &[
                            "rev-parse".into(),
                            "--verify".into(),
                            "--quiet".into(),
                            format!("refs/heads/{branch}"),
                        ],
                    )
                    .is_ok();
                let switch = if local_exists {
                    vec!["switch".to_string(), branch.into()]
                } else {
                    vec![
                        "switch".to_string(),
                        "-c".into(),
                        branch.into(),
                        "--track".into(),
                        format!("{remote}/{branch}"),
                    ]
                };
                for args in [
                    switch,
                    vec![
                        "pull".into(),
                        "--ff-only".into(),
                        "--no-edit".into(),
                        remote.clone(),
                        branch.into(),
                    ],
                ] {
                    let output = run_command(
                        git_command_with_token(self, project, &args, access_token)?,
                        remaining(deadline)?,
                    )?;
                    if !output.is_empty() {
                        parts.push(output);
                    }
                }
                Ok(parts.join("\n"))
            }
            "build" => {
                self.build_project_assets(project, timeout)?;
                Ok("Frontend derlemesi tamamlandı.".into())
            }
            "composer" => {
                let composer = self.executable("composer")?;
                let mut cmd = self.project_php_command(project)?;
                cmd.arg(&composer).args(&step.args);
                let php_dir = self.home.join("bin/php").join(&project.php_version);
                let mut paths = vec![php_dir];
                if let Some(existing) = std::env::var_os("PATH") {
                    paths.extend(std::env::split_paths(&existing));
                }
                cmd.env("PATH", std::env::join_paths(paths)?)
                    .env("COMPOSER_NO_INTERACTION", "1")
                    .env("COMPOSER_MEMORY_LIMIT", "-1");
                run_command(cmd, timeout)
            }
            "jobs" => self.restart_enabled_jobs(project),
            _ => {
                artisan_file(project)?;
                let mut cmd = self.project_php_command(project)?;
                cmd.args(&step.args);
                run_command(cmd, timeout)
            }
        }
    }

    fn git_output(&self, project: &Project, args: &[String]) -> Result<String> {
        run_command(git_command_with_token(self, project, args, None)?, Duration::from_secs(8))
    }

    /// `origin` when it exists, otherwise the first configured remote.
    fn git_remote(&self, project: &Project) -> Result<String> {
        let listed = self.git_output(project, &["remote".into()])?;
        let mut remotes = listed
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty());
        let first = remotes
            .next()
            .context("Depoda uzak sunucu (remote) tanımlı değil; git pull yapılamaz.")?;
        Ok(
            if first == "origin" || !listed.lines().any(|line| line.trim() == "origin") {
                first.to_string()
            } else {
                "origin".to_string()
            },
        )
    }

    fn write_release_record(&self, id: &str, record: &ReleaseRecord) -> Result<()> {
        let path = history_path(&self.home, id)?;
        let mut records = if path.exists() {
            String::from_utf8_lossy(&crate::storage::read_limited(&path, 16 * 1024 * 1024)?)
                .lines()
                .filter_map(|line| serde_json::from_str::<ReleaseRecord>(line).ok())
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        records.push(record.clone());
        if records.len() > HISTORY_LIMIT {
            let drop = records.len() - HISTORY_LIMIT;
            records.drain(0..drop);
        }
        let mut bytes = Vec::new();
        for item in records {
            writeln!(bytes, "{}", serde_json::to_string(&item)?)?;
        }
        crate::storage::atomic_write(&path, bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recipe() -> ProjectRelease {
        ProjectRelease {
            extra_artisan: vec!["config:cache".into(), "route:cache --no-ansi".into()],
            ..Default::default()
        }
    }

    #[test]
    fn extra_artisan_rejects_injection() {
        assert_eq!(
            parse_extra_artisan("config:cache").unwrap(),
            ["artisan", "config:cache"]
        );
        assert_eq!(
            parse_extra_artisan("route:cache --no-ansi").unwrap(),
            ["artisan", "route:cache", "--no-ansi"]
        );
        assert!(parse_extra_artisan("migrate; rm").is_err());
        assert!(parse_extra_artisan("view:cache --flag=$(id)").is_err());
        assert!(parse_extra_artisan("serve 80").is_err());
        assert!(parse_extra_artisan("Tinker").is_err());
        assert!(validate_git_branch("main").is_ok());
        assert!(validate_git_branch("release/1.2").is_ok());
        assert!(validate_git_branch("main;rm").is_err());
        assert!(validate_git_branch("-b").is_err());
        assert!(validate_git_branch("../etc").is_err());
    }

    #[test]
    fn release_steps_follow_forge_order() {
        let steps = release_steps(&recipe()).unwrap();
        assert_eq!(
            steps
                .iter()
                .map(|step| step.name.as_str())
                .collect::<Vec<_>>(),
            [
                "git",
                "composer",
                "migrate",
                "optimize",
                "config:cache",
                "route:cache",
                "jobs",
            ]
        );
        assert_eq!(steps[0].args, ["pull", "--ff-only", "--no-edit"]);
        assert!(steps[1].args.contains(&"--no-dev".into()));
        assert_eq!(steps[2].args, migrate_args());
        assert!(steps[2].args.contains(&"--force".into()));
        assert!(steps[2].args.contains(&"--no-ansi".into()));
        assert_eq!(steps[3].args, optimize_clear_args());
        let mut without_git = recipe();
        without_git.git_pull = false;
        without_git.branch = "main".into();
        without_git.composer_no_dev = false;
        let rest = release_steps(&without_git).unwrap();
        assert_eq!(rest[0].name, "composer");
        assert!(!rest[0].args.contains(&"--no-dev".into()));
    }

    #[test]
    fn git_missing_from_empty_path() {
        let error = git_program_from(Some(OsStr::new("")))
            .unwrap_err()
            .to_string();
        assert!(error.contains("git PATH"), "{error}");
        let missing = git_program_from(Some(OsStr::new("/tmp/serverbond-no-git-bin"))).unwrap_err();
        assert!(missing.to_string().contains("Git for Windows"), "{missing}");
    }

    #[test]
    fn frontend_build_precedes_database_changes_and_is_opt_in_for_old_recipes() {
        let mut release = recipe();
        let mut old = serde_json::to_value(&release).unwrap();
        old.as_object_mut().unwrap().remove("build");
        assert!(!serde_json::from_value::<ProjectRelease>(old).unwrap().build);
        release.build = true;
        let steps = release_steps(&release).unwrap();
        let position = |name| steps.iter().position(|step| step.name == name).unwrap();
        assert!(position("composer") < position("build"));
        assert!(position("build") < position("migrate"));
        assert!(position("build") < position("jobs"));
    }

    #[test]
    fn old_project_json_loads_default_release() {
        let project: Project = serde_json::from_str(
            r#"{
                "id": "11111111-1111-1111-1111-111111111111",
                "name": "magaza",
                "path": "C:/projects/magaza",
                "host": "magaza.localhost"
            }"#,
        )
        .unwrap();
        assert_eq!(project.release, ProjectRelease::default());
        assert!(project.release.git_pull);
        assert!(project.release.composer_no_dev);
        assert!(project.release.migrate);
        assert!(project.release.restart_jobs);
        assert!(project.workers.is_empty());
    }

    #[test]
    fn migrate_flags_are_literal() {
        assert_eq!(
            migrate_args(),
            ["artisan", "migrate", "--force", "--no-ansi"]
        );
        assert!(migrate_args()
            .iter()
            .all(|arg| !arg.contains(';') && !arg.contains('|')));
    }
}
