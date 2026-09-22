use crate::Manager;
use std::fs;

#[test]
fn discovery_excludes_dependency_folders_even_if_they_have_a_public_index() {
    let home = tempfile::tempdir().unwrap();
    let manager = Manager::new(home.path().into()).unwrap();
    for name in [
        "shop",
        "vendor",
        "node_modules",
        ".git",
        "storage",
        "acme/vendor",
    ] {
        let path = manager.default_projects_dir().join(name).join("public");
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("index.php"), "<?php").unwrap();
    }
    let found = manager.discover_projects().unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name, "shop");
}

#[test]
fn archive_rejects_windows_devices_aliases_and_collisions_before_writing() {
    use std::io::Write;
    let bad = [
        "NUL.txt",
        "bin/CON",
        "COM¹.log",
        "folder./file",
        "folder /file",
        " leading.txt",
        "file?.txt",
        "file|name",
        "bin/.. /outside",
    ];
    for name in bad {
        let home = tempfile::tempdir().unwrap();
        let archive = home.path().join("archive.zip");
        let mut zip = zip::ZipWriter::new(fs::File::create(&archive).unwrap());
        for path in ["valid.txt", name] {
            zip.start_file(path, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"data").unwrap();
        }
        zip.finish().unwrap();
        let target = home.path().join("output");
        assert!(
            crate::install::extract_zip(&archive, &target, "").is_err(),
            "{name}"
        );
        assert!(
            !target.exists(),
            "{name} was rejected only after writing files"
        );
    }
    for paths in [["bin/App.exe", "bin/app.exe"], ["bin", "bin/app.exe"]] {
        let home = tempfile::tempdir().unwrap();
        let archive = home.path().join("archive.zip");
        let mut zip = zip::ZipWriter::new(fs::File::create(&archive).unwrap());
        for path in paths {
            zip.start_file(path, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"data").unwrap();
        }
        zip.finish().unwrap();
        let target = home.path().join("output");
        assert!(crate::install::extract_zip(&archive, &target, "").is_err());
        assert!(!target.exists());
    }
}

#[test]
fn discovered_names_survive_partial_and_reversed_batch_imports() {
    for reversed in [false, true] {
        let home = tempfile::tempdir().unwrap();
        let manager = Manager::new(home.path().into()).unwrap();
        for name in ["shop", "acme/shop", "other/shop"] {
            let path = manager.default_projects_dir().join(name);
            fs::create_dir_all(path.join("public")).unwrap();
            fs::write(path.join("public/index.php"), "<?php").unwrap();
            fs::write(path.join(".env"), "APP_KEY=preserve").unwrap();
        }
        let mut found = manager.discover_projects().unwrap();
        if reversed {
            found.reverse();
        } else {
            found.retain(|p| p.name != "shop");
        }
        let imported = manager
            .import_projects(found.iter().map(|p| p.path.clone()).collect())
            .unwrap();
        for (expected, actual) in found.iter().zip(imported) {
            assert_eq!(actual.name, expected.name);
            assert_eq!(actual.host, expected.host);
            assert_eq!(
                fs::read_to_string(actual.path.join(".env")).unwrap(),
                "APP_KEY=preserve"
            );
        }
    }
}

#[cfg(windows)]
mod windows {
    use super::*;
    use crate::process::{command, ManagedChild};
    use crate::storage;
    use std::{
        io::Write,
        os::windows::fs::OpenOptionsExt,
        thread,
        time::{Duration, Instant},
    };

    #[test]
    fn atomic_save_recovers_from_a_short_windows_file_lock() {
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join("ayarlar-İstanbul.json");
        fs::write(&path, "original").unwrap();
        let locked = fs::OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&path)
            .unwrap();
        let release = thread::spawn(move || {
            thread::sleep(Duration::from_millis(120));
            drop(locked);
        });
        let saved = storage::atomic_write(&path, "replacement");
        release.join().unwrap();
        saved.unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "replacement");
        assert_eq!(fs::read_dir(home.path()).unwrap().count(), 1);
    }

    #[test]
    fn permanent_file_lock_preserves_original_and_removes_staging_file() {
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join("config.json");
        fs::write(&path, "original").unwrap();
        let locked = fs::OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&path)
            .unwrap();
        let started = Instant::now();
        assert!(storage::atomic_write(&path, "replacement").is_err());
        assert!(started.elapsed() < Duration::from_secs(3));
        assert_eq!(fs::read_to_string(&path).unwrap(), "original");
        assert_eq!(fs::read_dir(home.path()).unwrap().count(), 1);
        drop(locked);
    }

    #[test]
    fn timeout_releases_descendant_file_handles_before_managed_child_is_dropped() {
        use base64::Engine;
        let home = tempfile::Builder::new()
            .prefix("ServerBond süreç Türkçe ")
            .tempdir()
            .unwrap();
        let lock_path = home.path().join("child.lock");
        let ready = home.path().join("ready");
        let q = |p: &std::path::Path| crate::terminal::literal(&p.to_string_lossy());
        let script = format!("$f=[IO.File]::Open({},'Create','ReadWrite','None'); [IO.File]::WriteAllText({},'ready'); Start-Sleep -Seconds 30; $f.Dispose()", q(&lock_path), q(&ready));
        let encoded = base64::engine::general_purpose::STANDARD.encode(
            script
                .encode_utf16()
                .flat_map(u16::to_le_bytes)
                .collect::<Vec<_>>(),
        );
        let parent = format!("Start-Process -WindowStyle Hidden -FilePath {} -ArgumentList '-NoProfile','-NonInteractive','-EncodedCommand','{encoded}'; Start-Sleep -Seconds 30",q(&crate::terminal::powershell_path()));
        let mut cmd = command(crate::terminal::powershell_path());
        cmd.args(["-NoProfile", "-NonInteractive", "-Command", &parent]);
        let mut child = ManagedChild::spawn(cmd, &home.path().join("parent.log")).unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while !ready.exists() {
            assert!(Instant::now() < deadline, "descendant did not start");
            thread::sleep(Duration::from_millis(20));
        }
        assert!(fs::OpenOptions::new().write(true).open(&lock_path).is_err());
        assert!(child.wait_timeout(Duration::from_millis(50)).is_err());
        // Keep `child` alive: timeout itself must terminate its entire job.
        // A sharing violation may persist briefly while Windows completes
        // process teardown; an escaped descendant will exceed this deadline.
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            match fs::OpenOptions::new().write(true).open(&lock_path) {
                Ok(mut file) => {
                    file.write_all(b"released").unwrap();
                    break;
                }
                Err(error) if error.raw_os_error() == Some(32) && Instant::now() < deadline => {
                    thread::sleep(Duration::from_millis(20));
                }
                Err(error) => panic!("descendant file handle remained locked: {error}"),
            }
        }
        drop(child);
    }

    #[test]
    fn snapshot_defers_worker_restart_during_mutation_and_honors_stop() {
        let home = tempfile::tempdir().unwrap();
        let manager = Manager::new(home.path().into()).unwrap();
        let project_path = home.path().join("www/app");
        fs::create_dir_all(project_path.join("public")).unwrap();
        fs::write(project_path.join("public/index.php"), "<?php").unwrap();
        let project = manager.add_project("app".into(), project_path).unwrap();
        let worker = crate::model::QueueWorker {
            id: uuid::Uuid::new_v4().to_string(),
            enabled: true,
            ..Default::default()
        };
        manager.config.lock().unwrap().projects[0]
            .workers
            .push(worker.clone());
        let id = crate::jobs::queue_service_id(&project.id, &worker.id, 0);
        let mut cmd = command("cmd.exe");
        cmd.args(["/D", "/C", "exit /b 0"]);
        let mut child = ManagedChild::spawn(cmd, &home.path().join("logs/worker.log")).unwrap();
        child.wait_timeout(Duration::from_secs(5)).unwrap();
        child.spawned_at = Instant::now() - Duration::from_secs(120);
        manager.processes.lock().unwrap().insert(id.clone(), child);
        let operation = manager.gate().unwrap();
        let status = manager.snapshot().unwrap();
        assert!(!status.any_running);
        assert_eq!(status.projects[0].worker_states[0].running, 0);
        assert!(
            manager.service_errors.lock().unwrap().is_empty(),
            "snapshot must not restart a worker during a mutation"
        );
        assert!(manager.processes.lock().unwrap().contains_key(&id));
        manager.stop_service(&id).unwrap();
        drop(operation);
        assert!(!manager.snapshot().unwrap().any_running);
        assert!(manager.service_errors.lock().unwrap().is_empty());
    }
}
