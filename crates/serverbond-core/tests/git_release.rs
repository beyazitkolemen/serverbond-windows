//! End-to-end simulation of the git side of a Laravel deployment without any
//! Windows runtime: a bare repository stands in for GitHub, the project is
//! cloned over `file://`, registered, and released twice (fast-forward pull,
//! then a branch switch). Runs wherever `git` is on PATH.

use serverbond_core::Manager;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "ServerBond Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
        .env("GIT_COMMITTER_NAME", "ServerBond Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {:?} failed: {}{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

/// A minimal Laravel-shaped working tree: only the files ServerBond inspects.
fn write_laravel_skeleton(root: &Path, version: &str) {
    fs::create_dir_all(root.join("public")).unwrap();
    fs::create_dir_all(root.join("bootstrap")).unwrap();
    fs::write(
        root.join("public/index.php"),
        "<?php require __DIR__.'/../vendor/autoload.php';\n",
    )
    .unwrap();
    fs::write(root.join("artisan"), "#!/usr/bin/env php\n<?php exit(0);\n").unwrap();
    fs::write(
        root.join("composer.json"),
        r#"{"name":"acme/magaza","require":{"laravel/framework":"^12.0"}}"#,
    )
    .unwrap();
    fs::write(
        root.join(".env.example"),
        "APP_NAME=Magaza\nAPP_ENV=production\n",
    )
    .unwrap();
    fs::write(root.join("VERSION"), version).unwrap();
}

struct Fixture {
    _home: tempfile::TempDir,
    _upstream: tempfile::TempDir,
    manager: Manager,
    bare: PathBuf,
    work: PathBuf,
}

fn fixture() -> Fixture {
    let home = tempfile::tempdir().unwrap();
    let upstream = tempfile::tempdir().unwrap();
    let bare = upstream.path().join("magaza.git");
    let work = upstream.path().join("work");
    git(
        upstream.path(),
        &["init", "--bare", "-b", "main", "magaza.git"],
    );
    fs::create_dir_all(&work).unwrap();
    git(&work, &["init", "-b", "main"]);
    write_laravel_skeleton(&work, "1.0.0");
    git(&work, &["add", "."]);
    git(&work, &["commit", "-q", "-m", "Initial Laravel skeleton"]);
    git(&work, &["remote", "add", "origin", bare.to_str().unwrap()]);
    git(&work, &["push", "-q", "-u", "origin", "main"]);
    let manager = Manager::new(home.path().to_path_buf()).unwrap();
    Fixture {
        manager,
        bare,
        work,
        _home: home,
        _upstream: upstream,
    }
}

fn file_url(path: &Path) -> String {
    format!("file://{}", path.display())
}

#[test]
fn clones_registers_and_releases_a_laravel_project_over_git() {
    if !git_available() {
        eprintln!("git not installed; skipping");
        return;
    }
    let fx = fixture();
    let project = fx
        .manager
        .import_git_project(&file_url(&fx.bare), String::new(), String::new())
        .unwrap();
    assert_eq!(project.name, "magaza");
    assert!(project.path.join("public/index.php").is_file());
    assert_eq!(
        fs::read_to_string(project.path.join("VERSION")).unwrap(),
        "1.0.0"
    );
    let status = fx.manager.project_git_status(&project.id).unwrap();
    assert!(status.present);
    assert_eq!(status.branch, "main");
    assert_eq!(status.sha.len(), 7, "{}", status.sha);

    // Upstream moves forward; a release must fast-forward the working tree.
    write_laravel_skeleton(&fx.work, "1.1.0");
    git(&fx.work, &["commit", "-q", "-am", "Release 1.1.0"]);
    git(&fx.work, &["push", "-q", "origin", "main"]);
    let upstream_sha = git(&fx.work, &["rev-parse", "--short", "HEAD"]);

    let mut release = project.release.clone();
    release.composer = false;
    release.migrate = false;
    release.optimize_clear = false;
    release.restart_jobs = false;
    fx.manager
        .save_project_release(&project.id, release)
        .unwrap();
    let record = fx.manager.deploy_project(&project.id).unwrap();
    assert!(record.success, "{}", record.output);
    assert_eq!(record.branch, "main");
    assert_eq!(record.sha, upstream_sha);
    assert!(
        record.output.starts_with("--- git ---"),
        "{}",
        record.output
    );
    assert_eq!(
        fs::read_to_string(project.path.join("VERSION")).unwrap(),
        "1.1.0"
    );
    let history = fx.manager.list_project_releases(&project.id).unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0], record);
}

#[test]
fn release_switches_to_the_configured_branch_even_after_single_branch_clone() {
    if !git_available() {
        eprintln!("git not installed; skipping");
        return;
    }
    let fx = fixture();
    // A staging branch that does not exist yet when the project is cloned.
    let project = fx
        .manager
        .import_git_project(&file_url(&fx.bare), "magaza".into(), "main".into())
        .unwrap();
    assert_eq!(project.release.branch, "main");
    git(&fx.work, &["switch", "-q", "-c", "staging"]);
    write_laravel_skeleton(&fx.work, "2.0.0-rc1");
    git(&fx.work, &["commit", "-q", "-am", "Staging build"]);
    git(&fx.work, &["push", "-q", "-u", "origin", "staging"]);

    let mut release = project.release.clone();
    release.branch = "staging".into();
    release.composer = false;
    release.migrate = false;
    release.optimize_clear = false;
    release.restart_jobs = false;
    fx.manager
        .save_project_release(&project.id, release)
        .unwrap();
    let record = fx.manager.deploy_project(&project.id).unwrap();
    assert!(record.success, "{}", record.output);
    assert_eq!(record.branch, "staging");
    assert_eq!(
        fs::read_to_string(project.path.join("VERSION")).unwrap(),
        "2.0.0-rc1"
    );
    assert_eq!(
        fx.manager.project_git_status(&project.id).unwrap().branch,
        "staging"
    );
}

#[test]
fn failed_release_is_recorded_and_reported() {
    if !git_available() {
        eprintln!("git not installed; skipping");
        return;
    }
    let fx = fixture();
    let project = fx
        .manager
        .import_git_project(&file_url(&fx.bare), String::new(), String::new())
        .unwrap();
    // Diverging local history makes --ff-only refuse the pull.
    fs::write(project.path.join("VERSION"), "local-hotfix").unwrap();
    git(&project.path, &["commit", "-q", "-am", "Local hotfix"]);
    write_laravel_skeleton(&fx.work, "1.0.1");
    git(&fx.work, &["commit", "-q", "-am", "Upstream fix"]);
    git(&fx.work, &["push", "-q", "origin", "main"]);

    let mut release = project.release.clone();
    release.composer = false;
    release.migrate = false;
    release.optimize_clear = false;
    release.restart_jobs = false;
    fx.manager
        .save_project_release(&project.id, release)
        .unwrap();
    let error = fx.manager.deploy_project(&project.id).unwrap_err();
    let text = format!("{error:#}").to_lowercase();
    assert!(
        text.contains("fast-forward") || text.contains("diverg") || text.contains("fatal"),
        "{text}"
    );
    let history = fx.manager.list_project_releases(&project.id).unwrap();
    assert_eq!(history.len(), 1);
    assert!(!history[0].success);
    assert_eq!(
        fs::read_to_string(project.path.join("VERSION")).unwrap(),
        "local-hotfix",
        "a refused pull must not touch the working tree"
    );
}

#[test]
fn clone_failures_leave_no_half_written_project_folder() {
    if !git_available() {
        eprintln!("git not installed; skipping");
        return;
    }
    let fx = fixture();
    let missing = fx.bare.with_file_name("does-not-exist.git");
    let error = fx
        .manager
        .import_git_project(&file_url(&missing), "ghost".into(), String::new())
        .unwrap_err();
    assert!(format!("{error:#}").contains("klonlanamadı"), "{error:#}");
    assert!(!fx.manager.default_projects_dir().join("ghost").exists());
    // The next attempt with a valid remote must not be blocked.
    fx.manager
        .import_git_project(&file_url(&fx.bare), "ghost".into(), String::new())
        .unwrap();
}

#[test]
fn repositories_without_a_laravel_root_are_rejected_after_cloning() {
    if !git_available() {
        eprintln!("git not installed; skipping");
        return;
    }
    let fx = fixture();
    fs::remove_file(fx.work.join("public/index.php")).unwrap();
    git(&fx.work, &["commit", "-q", "-am", "Not Laravel any more"]);
    git(&fx.work, &["push", "-q", "origin", "main"]);
    let error = fx
        .manager
        .import_git_project(&file_url(&fx.bare), String::new(), String::new())
        .unwrap_err();
    assert!(
        format!("{error:#}").contains("public/index.php"),
        "{error:#}"
    );
    assert!(fx.manager.snapshot().unwrap().projects.is_empty());
}

#[test]
fn ssh_and_ext_remotes_are_refused_before_git_runs() {
    let home = tempfile::tempdir().unwrap();
    let manager = Manager::new(home.path().to_path_buf()).unwrap();
    for url in [
        "git@github.com:owner/repo.git",
        "ext::sh -c id",
        "ssh://git@github.com/owner/repo",
    ] {
        let error = manager
            .import_git_project(url, String::new(), String::new())
            .unwrap_err();
        assert!(
            format!("{error:#}").to_lowercase().contains("adres"),
            "{url}: {error:#}"
        );
    }
    assert!(manager.snapshot().unwrap().projects.is_empty());
    assert!(!home.path().join("logs/github.log").exists());
}
