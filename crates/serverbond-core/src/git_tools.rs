//! Private, verified MinGit fallback. Existing system Git keeps precedence;
//! no user or machine PATH setting is changed.
use crate::Manager;
use anyhow::Result;
use std::{ffi::OsStr, path::PathBuf};

pub const ID: &str = crate::domain::ComponentId::Git.as_str();

impl Manager {
    fn git_program_with_path(&self, path: Option<&OsStr>) -> Result<PathBuf> {
        crate::release::git_program_from(path).or_else(|_| self.tool_executable(ID))
    }

    /// Observation only: preflight and status never download tools.
    pub fn git_program(&self) -> Result<PathBuf> {
        self.git_program_with_path(std::env::var_os("PATH").as_deref())
    }

    /// The caller already holds the Manager write gate.
    pub(crate) fn ensure_git_inner(&self) -> Result<PathBuf> {
        self.ensure_git_with_path(std::env::var_os("PATH").as_deref())
    }

    fn ensure_git_with_path(&self, path: Option<&OsStr>) -> Result<PathBuf> {
        if let Ok(git) = self.git_program_with_path(path) {
            return Ok(git);
        }
        self.log("Git bulunamadı; doğrulanmış MinGit uygulama klasörüne kuruluyor…");
        if self.tool_health(ID).repairable {
            self.check_install_requirements()?;
            self.repair_tool(ID)?;
        } else {
            self.install_tool(ID)?;
        }
        self.git_program_with_path(path)
    }

    pub fn prepare_git(&self) -> Result<()> {
        let _guard = self.gate()?;
        self.ensure_git_inner().map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(manager: &Manager) -> PathBuf {
        let package = crate::model::tool_package(ID).unwrap();
        let dir = manager.home.join("bin/git").join(&package.version);
        std::fs::create_dir_all(dir.join("cmd")).unwrap();
        std::fs::write(dir.join("cmd/git.exe"), b"fixture").unwrap();
        for file in crate::install::required_files(&package)
            .into_iter()
            .filter(|file| *file != "cmd/git.exe")
        {
            let path = dir.join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, b"fixture dependency").unwrap();
        }
        std::fs::write(
            dir.join("installed.json"),
            serde_json::to_vec(&package).unwrap(),
        )
        .unwrap();
        dir.join("cmd/git.exe")
    }

    #[test]
    fn private_git_is_used_only_when_system_git_is_missing() {
        let home = tempfile::tempdir().unwrap();
        let manager = Manager::new(home.path().into()).unwrap();
        let private = fixture(&manager);
        assert_eq!(manager.git_program_with_path(None).unwrap(), private);
        let system = home.path().join("system");
        std::fs::create_dir(&system).unwrap();
        std::fs::write(system.join("git.exe"), b"system").unwrap();
        assert_eq!(
            manager
                .git_program_with_path(Some(system.as_os_str()))
                .unwrap(),
            system.join("git.exe")
        );
    }

    #[test]
    fn read_only_lookup_preserves_incomplete_installation() {
        let home = tempfile::tempdir().unwrap();
        let manager = Manager::new(home.path().into()).unwrap();
        let git = fixture(&manager);
        std::fs::remove_file(
            git.parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("installed.json"),
        )
        .unwrap();
        assert!(manager.git_program_with_path(None).is_err());
        assert_eq!(std::fs::read(git).unwrap(), b"fixture");
        assert_eq!(
            std::fs::read_dir(home.path().join("cache"))
                .unwrap()
                .count(),
            0
        );
    }

    #[test]
    fn preparation_is_idempotent_when_git_is_available() {
        let home = tempfile::tempdir().unwrap();
        let manager = Manager::new(home.path().into()).unwrap();
        let git = fixture(&manager);
        manager.prepare_git().unwrap();
        manager.prepare_git().unwrap();
        assert_eq!(std::fs::read(git).unwrap(), b"fixture");
        assert!(!manager.is_busy());
        assert_eq!(
            std::fs::read_dir(home.path().join("cache"))
                .unwrap()
                .count(),
            0
        );
    }

    #[test]
    fn private_git_rejects_missing_or_empty_runtime_dependencies() {
        let package = crate::model::tool_package(ID).unwrap();
        for missing in crate::install::required_files(&package) {
            let home = tempfile::tempdir().unwrap();
            let manager = Manager::new(home.path().into()).unwrap();
            let git = fixture(&manager);
            let dependency = git.parent().unwrap().parent().unwrap().join(missing);
            std::fs::write(&dependency, b"").unwrap();
            assert!(
                manager.git_program_with_path(None).is_err(),
                "empty {missing}"
            );
            std::fs::remove_file(dependency).unwrap();
            assert!(
                manager.git_program_with_path(None).is_err(),
                "missing {missing}"
            );
            assert!(manager.tool_health(ID).repairable);
        }
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "Downloads and extracts the verified Windows MinGit package"]
    fn repairs_private_git_and_preserves_previous_files() {
        assert!(crate::requirements::vc_runtime(), "This test requires an existing Visual C++ runtime; it must not install host prerequisites.");
        let home = tempfile::tempdir_in("D:/Temp/F4").unwrap();
        let manager = Manager::new(home.path().into()).unwrap();
        let git = fixture(&manager);
        let directory = git.parent().unwrap().parent().unwrap();
        std::fs::remove_file(directory.join("installed.json")).unwrap();
        std::fs::write(directory.join("user-added.txt"), b"preserve").unwrap();
        let _gate = manager.gate().unwrap();
        let repaired = manager.ensure_git_with_path(None).unwrap();
        let output = crate::process::ManagedChild::output(
            {
                let mut command = crate::process::command(&repaired);
                command.arg("--version");
                command
            },
            std::time::Duration::from_secs(30),
        )
        .unwrap();
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("2.56.0.windows.2"));
        let backup = std::fs::read_dir(directory.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .find(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .contains("before-repair")
            })
            .unwrap()
            .path();
        assert_eq!(
            std::fs::read(backup.join("user-added.txt")).unwrap(),
            b"preserve"
        );
        assert_eq!(
            std::fs::read(backup.join("cmd/git.exe")).unwrap(),
            b"fixture"
        );
        assert!(manager.git_program_with_path(None).is_ok());
    }
}
