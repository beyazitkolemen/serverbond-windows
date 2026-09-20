use crate::{
    install,
    model::{php_package, Project, ProjectSchedule, QueueWorker},
    portable_path,
    process::{command, ManagedChild},
    Manager,
};
use anyhow::{bail, Context, Result};
use std::{
    path::Path,
    process::Command,
    time::{Duration, Instant},
};

pub const MAX_WORKERS: usize = 8;
pub const MAX_PROCESSES: u8 = 8;

pub fn queue_service_id(project_id: &str, worker_id: &str, index: u8) -> String {
    format!("queue-{project_id}-{worker_id}-{index}")
}

pub fn schedule_service_id(project_id: &str) -> String {
    format!("schedule-{project_id}")
}

pub fn is_queue_service_id(id: &str) -> bool {
    parse_queue_service_id(id).is_some()
}

pub fn is_schedule_service_id(id: &str) -> bool {
    id.strip_prefix("schedule-")
        .is_some_and(|id| uuid::Uuid::parse_str(id).is_ok())
}

pub fn parse_queue_service_id(id: &str) -> Option<(String, String, u8)> {
    let rest = id.strip_prefix("queue-")?;
    let (project, rest) = rest.split_at_checked(36)?;
    let rest = rest.strip_prefix('-')?;
    let (worker, rest) = rest.split_at_checked(36)?;
    let index = rest.strip_prefix('-')?.parse().ok()?;
    uuid::Uuid::parse_str(project).ok()?;
    uuid::Uuid::parse_str(worker).ok()?;
    Some((project.into(), worker.into(), index))
}

pub fn queue_work_args(worker: &QueueWorker) -> Vec<String> {
    vec![
        "artisan".into(),
        "queue:work".into(),
        worker.connection.clone(),
        format!("--queue={}", worker.queue),
        format!("--sleep={}", worker.sleep),
        format!("--tries={}", worker.max_tries),
        format!("--timeout={}", worker.timeout),
        format!("--memory={}", worker.memory),
        format!("--backoff={}", worker.backoff),
        "--no-interaction".into(),
    ]
}

pub fn schedule_work_args() -> Vec<String> {
    vec![
        "artisan".into(),
        "schedule:work".into(),
        "--no-interaction".into(),
    ]
}

fn token(value: &str, label: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 32
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        bail!("{label} 1–32 karakter olmalı; harf, rakam, alt çizgi ve tire kullanın.");
    }
    Ok(())
}

fn queue_names(value: &str) -> Result<()> {
    if value.is_empty() || value.len() > 128 {
        bail!("Kuyruk adı 1–128 karakter olmalı.");
    }
    for part in value.split(',') {
        if part.is_empty()
            || !part
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            bail!(
                "Kuyruk adlarını virgülle ayırın; her ad harf, rakam, alt çizgi veya tire olmalı."
            );
        }
    }
    Ok(())
}

pub fn validate_worker(worker: &QueueWorker) -> Result<()> {
    if uuid::Uuid::parse_str(&worker.id).is_err() {
        bail!("Geçersiz kuyruk işçisi kimliği.");
    }
    token(&worker.name, "İşçi adı")?;
    token(&worker.connection, "Kuyruk bağlantısı")?;
    queue_names(&worker.queue)?;
    if !(1..=MAX_PROCESSES).contains(&worker.processes) {
        bail!("İşçi süreç sayısı 1–{MAX_PROCESSES} arasında olmalı.");
    }
    if !(1..=86_400).contains(&worker.timeout) {
        bail!("Zaman aşımı 1–86400 saniye olmalı.");
    }
    if !(1..=60).contains(&worker.sleep) {
        bail!("Bekleme süresi 1–60 saniye olmalı.");
    }
    if worker.max_tries > 1000 {
        bail!("Deneme sayısı en fazla 1000 olabilir.");
    }
    if !(32..=2048).contains(&worker.memory) {
        bail!("Bellek limiti 32–2048 MB olmalı.");
    }
    if worker.backoff > 3600 {
        bail!("Geri offset en fazla 3600 saniye olabilir.");
    }
    Ok(())
}

pub fn validate_project_jobs(workers: &[QueueWorker], _schedule: &ProjectSchedule) -> Result<()> {
    if workers.len() > MAX_WORKERS {
        bail!("Bir projede en fazla {MAX_WORKERS} kuyruk işçisi olabilir.");
    }
    let mut ids = std::collections::HashSet::new();
    let mut names = std::collections::HashSet::new();
    for worker in workers {
        validate_worker(worker)?;
        if !ids.insert(worker.id.to_lowercase()) || !names.insert(worker.name.clone()) {
            bail!("Kuyruk işçisi adı veya kimliği yineleniyor.");
        }
    }
    Ok(())
}

pub fn assign_worker_ids(workers: Vec<QueueWorker>) -> Result<Vec<QueueWorker>> {
    let mut assigned = Vec::with_capacity(workers.len());
    for mut worker in workers {
        if worker.id.is_empty() {
            worker.id = uuid::Uuid::new_v4().to_string();
        }
        assigned.push(worker);
    }
    Ok(assigned)
}

fn artisan_file(project: &Project) -> Result<&Path> {
    let artisan = project.path.join("artisan");
    if !artisan.is_file() {
        bail!(
            "{} klasöründe artisan yok. Kuyruk ve zamanlayıcı yalnızca Laravel projelerinde çalışır.",
            project.name
        );
    }
    Ok(project.path.as_path())
}

impl Manager {
    pub fn save_project_jobs(
        &self,
        id: &str,
        workers: Vec<QueueWorker>,
        schedule: ProjectSchedule,
    ) -> Result<()> {
        let _guard = self.gate()?;
        let workers = assign_worker_ids(workers)?;
        validate_project_jobs(&workers, &schedule)?;
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
        let previous = project.workers.clone();
        project.workers = workers.clone();
        project.schedule = schedule;
        self.save_config(&config)?;
        self.reconcile_project_jobs(id, &previous, &workers)?;
        self.log(format!("Kuyruk ve zamanlayıcı ayarları kaydedildi: {}", id));
        Ok(())
    }

    pub fn start_project_worker(&self, id: &str, worker_id: &str) -> Result<()> {
        let _guard = self.gate()?;
        let project = self.project(id)?;
        let worker = project
            .workers
            .iter()
            .find(|w| w.id == worker_id)
            .cloned()
            .context("Kuyruk işçisi bulunamadı.")?;
        self.spawn_queue_worker(&project, &worker)
    }

    pub fn stop_project_worker(&self, id: &str, worker_id: &str) -> Result<()> {
        let _guard = self.cleanup_gate()?;
        let project = self.project(id)?;
        let worker = project
            .workers
            .iter()
            .find(|w| w.id == worker_id)
            .cloned()
            .context("Kuyruk işçisi bulunamadı.")?;
        self.stop_queue_worker(&project.id, &worker)
    }

    pub fn start_project_schedule(&self, id: &str) -> Result<()> {
        let _guard = self.gate()?;
        let project = self.project(id)?;
        self.spawn_schedule(&project)
    }

    pub fn stop_project_schedule(&self, id: &str) -> Result<()> {
        let _guard = self.cleanup_gate()?;
        self.stop_service(&schedule_service_id(id))
    }

    pub fn list_project_schedule(&self, id: &str) -> Result<String> {
        let project = self.project(id)?;
        artisan_file(&project)?;
        let mut cmd = self.project_php_command(&project)?;
        cmd.args(["artisan", "schedule:list", "--no-interaction"]);
        let output = ManagedChild::output(cmd, Duration::from_secs(20))?;
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if !output.status.success() {
            bail!(
                "Zamanlanmış görevler listelenemedi: {}",
                if stderr.is_empty() { stdout } else { stderr }
            );
        }
        Ok(if stdout.is_empty() {
            "Kayıtlı zamanlanmış görev yok. app/Console veya routes/console.php içinde Schedule tanımlayın."
                .into()
        } else {
            stdout
        })
    }

    pub(crate) fn start_autostart_jobs(&self) {
        let projects = self
            .config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .projects
            .clone();
        for project in projects {
            if let Err(error) = self.start_project_autostart(&project) {
                let message = format!(
                    "{} kuyruk/zamanlayıcı otomatik başlamadı: {error:#}",
                    project.name
                );
                self.log(&message);
                self.service_errors
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .insert(format!("jobs-{}", project.id), message);
            }
        }
    }

    pub(crate) fn stop_project_jobs(&self) -> Result<()> {
        let ids = self
            .processes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .keys()
            .filter(|id| is_queue_service_id(id) || is_schedule_service_id(id))
            .cloned()
            .collect::<Vec<_>>();
        let mut failures = Vec::new();
        for id in ids {
            if let Err(error) = self.stop_service(&id) {
                failures.push(format!("{id}: {error:#}"));
            }
        }
        if !failures.is_empty() {
            bail!("{}", failures.join("; "));
        }
        Ok(())
    }

    pub(crate) fn stop_jobs_for_project(&self, project_id: &str) -> Result<()> {
        let ids = self
            .processes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .keys()
            .filter(|id| {
                parse_queue_service_id(id).is_some_and(|(p, _, _)| p == project_id)
                    || id.as_str() == schedule_service_id(project_id)
            })
            .cloned()
            .collect::<Vec<_>>();
        for id in ids {
            self.stop_service(&id)?;
        }
        Ok(())
    }

    pub(crate) fn is_job_service_id(id: &str) -> bool {
        is_queue_service_id(id) || is_schedule_service_id(id)
    }

    pub(crate) fn is_managed_log(id: &str) -> bool {
        if ["f4box", "php", "mysql", "caddy", "composer"].contains(&id) {
            return true;
        }
        if Self::is_project_service_id(id) || Self::is_job_service_id(id) {
            return true;
        }
        false
    }

    fn project(&self, id: &str) -> Result<Project> {
        self.config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .projects
            .iter()
            .find(|p| p.id == id)
            .cloned()
            .context("Proje bulunamadı.")
    }

    fn start_project_autostart(&self, project: &Project) -> Result<()> {
        for worker in &project.workers {
            if worker.enabled && worker.auto_start {
                self.spawn_queue_worker(project, worker)?;
            }
        }
        if project.schedule.enabled && project.schedule.auto_start {
            self.spawn_schedule(project)?;
        }
        Ok(())
    }

    fn spawn_queue_worker(&self, project: &Project, worker: &QueueWorker) -> Result<()> {
        artisan_file(project)?;
        for index in 0..worker.processes {
            let id = queue_service_id(&project.id, &worker.id, index);
            let mut cmd = self.project_php_command(project)?;
            cmd.args(queue_work_args(worker));
            self.spawn_process(&id, cmd).with_context(|| {
                format!("{} işçisi ({}) başlatılamadı", project.name, worker.name)
            })?;
        }
        self.service_errors
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&format!("jobs-{}", project.id));
        self.log(format!(
            "{} kuyruk işçisi çalışıyor: {} × {}",
            project.name, worker.name, worker.processes
        ));
        Ok(())
    }

    fn stop_queue_worker(&self, project_id: &str, worker: &QueueWorker) -> Result<()> {
        for index in 0..worker.processes.max(MAX_PROCESSES) {
            self.stop_service(&queue_service_id(project_id, &worker.id, index))?;
        }
        Ok(())
    }

    fn spawn_schedule(&self, project: &Project) -> Result<()> {
        artisan_file(project)?;
        let id = schedule_service_id(&project.id);
        let mut cmd = self.project_php_command(project)?;
        cmd.args(schedule_work_args());
        self.spawn_process(&id, cmd)
            .with_context(|| format!("{} zamanlayıcısı başlatılamadı", project.name))?;
        self.service_errors
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&format!("jobs-{}", project.id));
        self.log(format!(
            "{} Laravel zamanlayıcısı çalışıyor (her dakika schedule:run).",
            project.name
        ));
        Ok(())
    }

    fn reconcile_project_jobs(
        &self,
        project_id: &str,
        previous: &[QueueWorker],
        current: &[QueueWorker],
    ) -> Result<()> {
        for old in previous {
            let still = current.iter().find(|w| w.id == old.id);
            match still {
                None => self.stop_queue_worker(project_id, old)?,
                Some(new) if new != old => {
                    let running = (0..old.processes).any(|index| {
                        self.processes
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .contains_key(&queue_service_id(project_id, &old.id, index))
                    });
                    self.stop_queue_worker(project_id, old)?;
                    if running {
                        if let Ok(project) = self.project(project_id) {
                            let _ = self.spawn_queue_worker(&project, new);
                        }
                    }
                }
                Some(_) => {}
            }
        }
        Ok(())
    }

    fn project_php_command(&self, project: &Project) -> Result<Command> {
        let package = php_package(&project.php_version)?;
        let directory = self.home.join("bin/php").join(&project.php_version);
        install::validate_installation(&directory, &package).with_context(|| {
            format!(
                "{} için PHP {} kurulu değil. Önce projenin PHP sürümünü indirin.",
                project.name, project.php_version
            )
        })?;
        let ini = self.write_php_config_for(&project.php_version)?;
        let mut cmd = command(directory.join("php.exe"));
        cmd.arg("-c")
            .arg(&ini)
            .arg("-d")
            .arg(format!(
                "extension_dir=\"{}\"",
                portable_path(&directory.join("ext"))
            ))
            .current_dir(&project.path)
            .env("PHPRC", &ini)
            .env("F4BOX_PHP_EXT", directory.join("ext"))
            .env("PHP_INI_SCAN_DIR", "");
        Ok(cmd)
    }
}

impl Manager {
    pub(crate) fn spawn_process(&self, id: &str, cmd: Command) -> Result<()> {
        if self
            .processes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get_mut(id)
            .is_some_and(|p| p.alive())
        {
            return Ok(());
        }
        let mut child =
            ManagedChild::spawn(cmd, &self.home.join("logs").join(format!("{id}.log")))?;
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            if let Some(status) = child.child.try_wait()? {
                bail!(
                    "{id} hemen çıktı ({status}). artisan, kuyruk bağlantısı ve günlükleri kontrol edin."
                );
            }
            if Instant::now() > deadline {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        self.processes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(id.into(), child);
        self.service_errors
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn worker() -> QueueWorker {
        QueueWorker {
            id: uuid::Uuid::new_v4().to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn queue_ids_round_trip() {
        let project = uuid::Uuid::new_v4().to_string();
        let worker = uuid::Uuid::new_v4().to_string();
        let id = queue_service_id(&project, &worker, 2);
        assert_eq!(
            parse_queue_service_id(&id),
            Some((project.clone(), worker, 2))
        );
        assert!(is_queue_service_id(&id));
        assert!(is_schedule_service_id(&schedule_service_id(&project)));
        assert!(!is_queue_service_id("php-project-not-a-uuid"));
        assert!(!is_queue_service_id(&format!("queue-{project}-x-0")));
    }

    #[test]
    fn artisan_arguments_are_literal_flags() {
        let mut worker = worker();
        worker.connection = "redis".into();
        worker.queue = "high,default".into();
        worker.timeout = 90;
        let args = queue_work_args(&worker);
        assert_eq!(args[0], "artisan");
        assert_eq!(args[1], "queue:work");
        assert_eq!(args[2], "redis");
        assert!(args.contains(&"--queue=high,default".into()));
        assert!(args.contains(&"--timeout=90".into()));
        assert!(args
            .iter()
            .all(|arg| !arg.contains('&') && !arg.contains('|')));
        assert_eq!(schedule_work_args()[1], "schedule:work");
    }

    #[test]
    fn worker_validation_rejects_injection_and_bounds() {
        let mut sample = worker();
        sample.name = "ok".into();
        validate_worker(&sample).unwrap();
        sample.connection = "redis;rm".into();
        assert!(validate_worker(&sample).is_err());
        sample.connection = "default".into();
        sample.queue = "a, ,b".into();
        assert!(validate_worker(&sample).is_err());
        sample.queue = "default".into();
        sample.processes = 0;
        assert!(validate_worker(&sample).is_err());
        sample.processes = 1;
        sample.memory = 8;
        assert!(validate_worker(&sample).is_err());
        let many = (0..=MAX_WORKERS)
            .map(|index| {
                let mut extra = worker();
                extra.name = format!("w{index}");
                extra
            })
            .collect::<Vec<_>>();
        assert!(validate_project_jobs(&many, &ProjectSchedule::default()).is_err());
    }
}
