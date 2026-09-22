use serverbond_core::{env_revision, jobs_revision, release_revision, Manager};
use std::fs;

fn project_fixture() -> (tempfile::TempDir, Manager, serverbond_core::model::Project) {
    let home = tempfile::tempdir().unwrap();
    let manager = Manager::new(home.path().into()).unwrap();
    let path = home.path().join("www/demo");
    fs::create_dir_all(path.join("public")).unwrap();
    fs::write(path.join("public/index.php"), "<?php").unwrap();
    let project = manager.add_project("demo".into(), path).unwrap();
    (home, manager, project)
}

#[test]
fn desktop_env_save_preserves_external_creation_edits_and_deletion() {
    let (_home, manager, project) = project_fixture();
    let missing = manager.read_project_env(&project.id).unwrap();
    // Existence belongs to the revision: an empty file differs from no file.
    fs::write(project.path.join(".env"), "").unwrap();
    assert!(manager
        .save_project_env_checked(
            &project.id,
            "LOCAL=1\n".into(),
            Some(&env_revision(&missing))
        )
        .is_err());
    let original = manager.read_project_env(&project.id).unwrap();
    fs::write(project.path.join(".env"), "REMOTE=1\n").unwrap();
    assert!(manager
        .save_project_env_checked(
            &project.id,
            "LOCAL=1\n".into(),
            Some(&env_revision(&original))
        )
        .is_err());
    assert_eq!(
        fs::read_to_string(project.path.join(".env")).unwrap(),
        "REMOTE=1\n"
    );
    let fresh = manager.read_project_env(&project.id).unwrap();
    let saved = manager
        .save_project_env_checked(&project.id, "LOCAL=2\n".into(), Some(&env_revision(&fresh)))
        .unwrap();
    assert_eq!(saved.content, "LOCAL=2\n");
    fs::remove_file(project.path.join(".env")).unwrap();
    assert!(manager
        .save_project_env_checked(&project.id, "LOCAL=3\n".into(), Some(&env_revision(&saved)))
        .is_err());
    assert!(!project.path.join(".env").exists());
}

#[test]
fn desktop_jobs_save_rejects_stale_snapshot_without_changing_configuration() {
    let (home, manager, project) = project_fixture();
    let revision = jobs_revision(&project.workers, &project.schedule).unwrap();
    let schedule = serverbond_core::model::ProjectSchedule {
        enabled: true,
        auto_start: false,
    };
    manager
        .save_project_jobs(&project.id, vec![], schedule.clone())
        .unwrap();
    let before = fs::read(home.path().join("config.json")).unwrap();
    assert!(manager
        .save_project_jobs_checked(&project.id, vec![], project.schedule, Some(&revision))
        .is_err());
    assert_eq!(fs::read(home.path().join("config.json")).unwrap(), before);
    manager
        .save_project_jobs_checked(
            &project.id,
            vec![],
            Default::default(),
            Some(&jobs_revision(&[], &schedule).unwrap()),
        )
        .unwrap();
}

#[test]
fn desktop_release_save_and_run_both_reject_a_changed_recipe() {
    let (home, manager, project) = project_fixture();
    let revision = release_revision(&project.release).unwrap();
    let mut changed = project.release.clone();
    changed.branch = "changed-elsewhere".into();
    manager
        .save_project_release(&project.id, changed.clone())
        .unwrap();
    let before = fs::read(home.path().join("config.json")).unwrap();
    assert!(manager
        .save_project_release_checked(&project.id, project.release.clone(), Some(&revision))
        .is_err());
    let error = manager
        .deploy_project_checked(&project.id, Some(&revision))
        .unwrap_err()
        .to_string();
    assert!(error.contains("Sürüm tarifi değişti"));
    assert!(manager
        .list_project_releases(&project.id)
        .unwrap()
        .is_empty());
    assert_eq!(fs::read(home.path().join("config.json")).unwrap(), before);
    manager
        .save_project_release_checked(
            &project.id,
            project.release,
            Some(&release_revision(&changed).unwrap()),
        )
        .unwrap();
}

#[test]
fn desktop_api_save_protects_concurrent_api_changes() {
    let home = tempfile::tempdir().unwrap();
    let manager = Manager::new(home.path().into()).unwrap();
    let (settings, _) = manager.settings_with_revision().unwrap();
    let baseline = settings.api;
    let mut remote = baseline.clone();
    remote.mcp_enabled = !remote.mcp_enabled;
    manager.save_api_settings(remote.clone()).unwrap();
    let before = fs::read(home.path().join("config.json")).unwrap();
    assert!(manager
        .save_api_settings_checked(baseline.clone(), Some(&baseline))
        .is_err());
    assert_eq!(fs::read(home.path().join("config.json")).unwrap(), before);
    manager
        .save_api_settings_checked(baseline, Some(&remote))
        .unwrap();
}
