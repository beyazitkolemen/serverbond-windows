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
    let mut args = vec![
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
        "--no-ansi".into(),
    ];
    if worker.max_jobs > 0 {
        args.push(format!("--max-jobs={}", worker.max_jobs));
    }
    if worker.max_time > 0 {
        args.push(format!("--max-time={}", worker.max_time));
    }
    args
}

pub fn schedule_work_args() -> Vec<String> {
    vec![
        "artisan".into(),
        "schedule:work".into(),
        "--no-interaction".into(),
        "--no-ansi".into(),
    ]
}

pub fn schedule_list_args(next: bool) -> Vec<String> {
    let mut args = vec![
        "artisan".into(),
        "schedule:list".into(),
        "--no-interaction".into(),
        "--no-ansi".into(),
    ];
    if next {
        args.push("--next".into());
    }
    args
}

pub fn queue_failed_args() -> Vec<String> {
    vec![
        "artisan".into(),
        "queue:failed".into(),
        "--no-interaction".into(),
        "--no-ansi".into(),
    ]
}

pub fn queue_retry_args(job: &str) -> Vec<String> {
    vec![
        "artisan".into(),
        "queue:retry".into(),
        job.into(),
        "--no-interaction".into(),
        "--no-ansi".into(),
    ]
}

pub fn queue_flush_args() -> Vec<String> {
    vec![
        "artisan".into(),
        "queue:flush".into(),
        "--no-interaction".into(),
        "--no-ansi".into(),
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
    if worker.max_jobs > 1_000_000 {
        bail!("Azami iş sayısı en fazla 1000000 olabilir.");
    }
    if worker.max_time > 604_800 {
        bail!("Azami çalışma süresi en fazla 604800 saniye (7 gün) olabilir.");
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

pub fn should_respawn_worker(worker: &QueueWorker, was_running: bool) -> bool {
    was_running && worker.enabled
}

pub fn should_stop_schedule(enabled: bool, running: bool) -> bool {
    running && !enabled
}

pub fn should_start_saved_worker(
    previous: Option<&QueueWorker>,
    current: &QueueWorker,
    running: bool,
    env_active: bool,
) -> bool {
    current.enabled
        && current.auto_start
        && env_active
        && !running
        && match previous {
            None => true,
            Some(old) => !old.enabled || !old.auto_start,
        }
}

pub fn should_start_saved_schedule(
    previous: &ProjectSchedule,
    current: &ProjectSchedule,
    running: bool,
    env_active: bool,
) -> bool {
    current.enabled
        && current.auto_start
        && env_active
        && !running
        && (!previous.enabled || !previous.auto_start)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectLogKind {
    Php,
    Schedule,
    Worker(String),
}

pub fn parse_project_log_source(source: &str) -> Result<ProjectLogKind> {
    match source {
        "php" => Ok(ProjectLogKind::Php),
        "schedule" => Ok(ProjectLogKind::Schedule),
        other => {
            let worker = other
                .strip_prefix("worker:")
                .context("Geçersiz günlük kaynağı.")?;
            if uuid::Uuid::parse_str(worker).is_err() {
                bail!("Geçersiz kuyruk işçisi kimliği.");
            }
            Ok(ProjectLogKind::Worker(worker.into()))
        }
    }
}

pub fn failed_job_token(value: Option<&str>) -> Result<String> {
    let value = value.unwrap_or("all").trim();
    if value.eq_ignore_ascii_case("all") {
        return Ok("all".into());
    }
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        bail!("İş kimliği harf, rakam veya tire olmalı; tümü için all kullanın.");
    }
    Ok(value.into())
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

pub(crate) fn artisan_file(project: &Project) -> Result<&Path> {
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
        let name = project.name.clone();
        let previous = project.workers.clone();
        let previous_schedule = project.schedule.clone();
        project.workers = workers.clone();
        project.schedule = schedule.clone();
        self.save_config(&config)?;
        self.reconcile_project_jobs(id, &previous, &workers, &previous_schedule, &schedule)?;
        self.log(format!("Kuyruk ve zamanlayıcı ayarları kaydedildi: {name}"));
        Ok(())
    }

    pub fn start_project_worker(&self, id: &str, worker_id: &str) -> Result<()> {
        let _guard = self.gate()?;
        let (project, worker) = self.project_worker(id, worker_id)?;
        if !worker.enabled {
            bail!("Bu kuyruk işçisi kapalı. Etkinleştirip kaydedin.");
        }
        self.spawn_queue_worker(&project, &worker)
    }

    pub fn stop_project_worker(&self, id: &str, worker_id: &str) -> Result<()> {
        let _guard = self.cleanup_gate()?;
        let (project, worker) = self.project_worker(id, worker_id)?;
        self.stop_queue_worker(&project.id, &worker)
    }

    pub fn restart_project_worker(&self, id: &str, worker_id: &str) -> Result<()> {
        let _guard = self.gate()?;
        let (project, worker) = self.project_worker(id, worker_id)?;
        if !worker.enabled {
            bail!("Bu kuyruk işçisi kapalı. Etkinleştirip kaydedin.");
        }
        self.stop_queue_worker(&project.id, &worker)?;
        self.spawn_queue_worker(&project, &worker)
    }

    pub fn start_project_schedule(&self, id: &str) -> Result<()> {
        let _guard = self.gate()?;
        let project = self.project(id)?;
        if !project.schedule.enabled {
            bail!("Zamanlayıcı kapalı. Etkinleştirip kaydedin.");
        }
        self.spawn_schedule(&project)
    }

    pub fn stop_project_schedule(&self, id: &str) -> Result<()> {
        let _guard = self.cleanup_gate()?;
        self.stop_service(&schedule_service_id(id))
    }

    pub fn restart_project_schedule(&self, id: &str) -> Result<()> {
        let _guard = self.gate()?;
        let project = self.project(id)?;
        if !project.schedule.enabled {
            bail!("Zamanlayıcı kapalı. Etkinleştirip kaydedin.");
        }
        self.stop_service(&schedule_service_id(id))?;
        self.spawn_schedule(&project)
    }

    pub fn list_project_schedule(&self, id: &str) -> Result<String> {
        let project = self.project(id)?;
        let output = match self.artisan_output(&project, &schedule_list_args(true)) {
            Ok(output) => output,
            Err(error)
                if error.to_string().contains("--next")
                    || error.to_string().contains("nknown option") =>
            {
                self.artisan_output(&project, &schedule_list_args(false))?
            }
            Err(error) => {
                return Err(error).context("Zamanlanmış görevler listelenemedi");
            }
        };
        Ok(if output.is_empty() {
            "Kayıtlı zamanlanmış görev yok. app/Console veya routes/console.php içinde Schedule tanımlayın."
                .into()
        } else {
            output
        })
    }

    pub fn list_failed_jobs(&self, id: &str) -> Result<String> {
        let project = self.project(id)?;
        let output = self
            .artisan_output(&project, &queue_failed_args())
            .context("Başarısız kuyruk işleri listelenemedi")?;
        Ok(if output.is_empty() {
            "Başarısız kuyruk işi yok.".into()
        } else {
            output
        })
    }

    pub fn retry_failed_jobs(&self, id: &str, job: Option<&str>) -> Result<String> {
        let _guard = self.gate()?;
        let project = self.project(id)?;
        let job = failed_job_token(job)?;
        let output = self
            .artisan_output(&project, &queue_retry_args(&job))
            .context("Başarısız kuyruk işleri yeniden kuyruğa alınamadı")?;
        self.log(format!(
            "{} başarısız kuyruk işleri yeniden denenecek: {job}",
            project.name
        ));
        Ok(if output.is_empty() {
            "Başarısız işler yeniden kuyruğa alındı.".into()
        } else {
            output
        })
    }

    pub fn flush_failed_jobs(&self, id: &str) -> Result<String> {
        let _guard = self.gate()?;
        let project = self.project(id)?;
        let output = self
            .artisan_output(&project, &queue_flush_args())
            .context("Başarısız kuyruk işleri temizlenemedi")?;
        self.log(format!(
            "{} başarısız kuyruk işleri temizlendi.",
            project.name
        ));
        Ok(if output.is_empty() {
            "Başarısız kuyruk işleri temizlendi.".into()
        } else {
            output
        })
    }

    pub fn read_project_worker_log(&self, id: &str, worker_id: &str) -> Result<String> {
        let (project, worker) = self.project_worker(id, worker_id)?;
        let mut parts = Vec::new();
        let mut empty = true;
        for index in 0..worker.processes.max(1) {
            let sid = queue_service_id(&project.id, &worker.id, index);
            let text = self.read_log(&sid)?;
            if text != "Henüz günlük kaydı yok." {
                empty = false;
            }
            if worker.processes > 1 {
                parts.push(format!("— süreç {} —\n{text}", index + 1));
            } else {
                parts.push(text);
            }
        }
        if empty {
            return Ok("Henüz günlük kaydı yok.".into());
        }
        Ok(parts.join("\n\n"))
    }

    pub fn read_project_schedule_log(&self, id: &str) -> Result<String> {
        let _project = self.project(id)?;
        self.read_log(&schedule_service_id(id))
    }

    pub fn read_project_log(&self, id: &str, source: &str) -> Result<String> {
        match parse_project_log_source(source)? {
            ProjectLogKind::Php => {
                let project = self.project(id)?;
                self.read_log(&Self::project_service_id(&project.id))
            }
            ProjectLogKind::Schedule => self.read_project_schedule_log(id),
            ProjectLogKind::Worker(worker_id) => self.read_project_worker_log(id, &worker_id),
        }
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
        if [
            "f4box",
            "php",
            "mysql",
            "caddy",
            "composer",
            crate::tunnel::ID,
            crate::mail::ID,
            crate::postgres::ID,
            "github",
        ]
        .contains(&id)
        {
            return true;
        }
        if Self::is_project_service_id(id) || Self::is_job_service_id(id) {
            return true;
        }
        false
    }

    pub(crate) fn project(&self, id: &str) -> Result<Project> {
        self.config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .projects
            .iter()
            .find(|p| p.id == id)
            .cloned()
            .context("Proje bulunamadı.")
    }

    fn project_worker(&self, id: &str, worker_id: &str) -> Result<(Project, QueueWorker)> {
        let project = self.project(id)?;
        let worker = project
            .workers
            .iter()
            .find(|w| w.id == worker_id)
            .cloned()
            .context("Kuyruk işçisi bulunamadı.")?;
        Ok((project, worker))
    }

    pub(crate) fn artisan_output(&self, project: &Project, args: &[String]) -> Result<String> {
        artisan_file(project)?;
        let mut cmd = self.project_php_command(project)?;
        cmd.args(args);
        let output = ManagedChild::output(cmd, Duration::from_secs(20))?;
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if !output.status.success() {
            bail!("{}", if stderr.is_empty() { stdout } else { stderr });
        }
        Ok(stdout)
    }

    fn project_jobs_env_active(&self, project_id: &str) -> bool {
        let processes = self.processes.lock().unwrap_or_else(|e| e.into_inner());
        if processes.contains_key(&Self::project_service_id(project_id)) {
            return true;
        }
        processes.keys().any(|id| {
            parse_queue_service_id(id).is_some_and(|(p, _, _)| p == project_id)
                || id.as_str() == schedule_service_id(project_id)
        })
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

    fn queue_worker_running(&self, project_id: &str, worker: &QueueWorker) -> bool {
        (0..worker.processes.max(MAX_PROCESSES)).any(|index| {
            self.processes
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .contains_key(&queue_service_id(project_id, &worker.id, index))
        })
    }

    fn reconcile_project_jobs(
        &self,
        project_id: &str,
        previous: &[QueueWorker],
        current: &[QueueWorker],
        previous_schedule: &ProjectSchedule,
        current_schedule: &ProjectSchedule,
    ) -> Result<()> {
        for old in previous {
            let still = current.iter().find(|w| w.id == old.id);
            match still {
                None => self.stop_queue_worker(project_id, old)?,
                Some(new) if new != old => {
                    let running = self.queue_worker_running(project_id, old);
                    self.stop_queue_worker(project_id, old)?;
                    if should_respawn_worker(new, running) {
                        let project = self.project(project_id)?;
                        self.spawn_queue_worker(&project, new)?;
                    }
                }
                Some(_) => {}
            }
        }
        let env_active = self.project_jobs_env_active(project_id);
        for worker in current {
            let previous_worker = previous.iter().find(|item| item.id == worker.id);
            let running = self.queue_worker_running(project_id, worker);
            if should_start_saved_worker(previous_worker, worker, running, env_active) {
                let project = self.project(project_id)?;
                self.spawn_queue_worker(&project, worker)?;
            }
        }
        let schedule_id = schedule_service_id(project_id);
        let schedule_running = self
            .processes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains_key(&schedule_id);
        if should_stop_schedule(current_schedule.enabled, schedule_running) {
            self.stop_service(&schedule_id)?;
        } else if should_start_saved_schedule(
            previous_schedule,
            current_schedule,
            schedule_running,
            env_active,
        ) {
            let project = self.project(project_id)?;
            self.spawn_schedule(&project)?;
        }
        Ok(())
    }

    pub(crate) fn restart_enabled_jobs(&self, project: &Project) -> Result<String> {
        artisan_file(project)?;
        let mut restarted = Vec::new();
        for worker in &project.workers {
            if worker.enabled {
                self.stop_queue_worker(&project.id, worker)?;
                self.spawn_queue_worker(project, worker)?;
                restarted.push(format!("kuyruk: {}", worker.name));
            }
        }
        if project.schedule.enabled {
            self.stop_service(&schedule_service_id(&project.id))?;
            self.spawn_schedule(project)?;
            restarted.push("zamanlayıcı".into());
        }
        if restarted.is_empty() {
            Ok("Yeniden başlatılacak etkin işçi veya zamanlayıcı yok.".into())
        } else {
            Ok(format!("Yeniden başlatıldı: {}", restarted.join(", ")))
        }
    }

    pub(crate) fn project_php_command(&self, project: &Project) -> Result<Command> {
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
        assert!(args.contains(&"--no-ansi".into()));
        assert!(args.contains(&"--no-interaction".into()));
        assert!(!args.iter().any(|arg| arg.starts_with("--max-jobs=")));
        assert!(!args.iter().any(|arg| arg.starts_with("--max-time=")));
        worker.max_jobs = 250;
        worker.max_time = 3600;
        let limited = queue_work_args(&worker);
        assert!(limited.contains(&"--max-jobs=250".into()));
        assert!(limited.contains(&"--max-time=3600".into()));
        assert!(limited
            .iter()
            .all(|arg| !arg.contains('&') && !arg.contains('|')));
        assert_eq!(schedule_work_args()[1], "schedule:work");
        assert!(schedule_work_args().contains(&"--no-ansi".into()));
        assert!(schedule_list_args(true).contains(&"--next".into()));
        assert!(!schedule_list_args(false).contains(&"--next".into()));
        assert_eq!(queue_retry_args("all")[2], "all");
        assert_eq!(failed_job_token(None).unwrap(), "all");
        assert_eq!(failed_job_token(Some("42")).unwrap(), "42");
        assert!(failed_job_token(Some("rm;rf")).is_err());
        assert_eq!(
            parse_project_log_source("php").unwrap(),
            ProjectLogKind::Php
        );
        assert_eq!(
            parse_project_log_source("schedule").unwrap(),
            ProjectLogKind::Schedule
        );
        let worker = uuid::Uuid::new_v4().to_string();
        assert_eq!(
            parse_project_log_source(&format!("worker:{worker}")).unwrap(),
            ProjectLogKind::Worker(worker)
        );
        assert!(parse_project_log_source("worker:not-a-uuid").is_err());
        assert!(parse_project_log_source("../php").is_err());
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
        sample.memory = 128;
        sample.max_jobs = 1_000_001;
        assert!(validate_worker(&sample).is_err());
        sample.max_jobs = 10;
        sample.max_time = 604_801;
        assert!(validate_worker(&sample).is_err());
        sample.max_time = 60;
        validate_worker(&sample).unwrap();
        let many = (0..=MAX_WORKERS)
            .map(|index| {
                let mut extra = worker();
                extra.name = format!("w{index}");
                extra
            })
            .collect::<Vec<_>>();
        assert!(validate_project_jobs(&many, &ProjectSchedule::default()).is_err());
    }

    #[test]
    fn saving_a_disabled_worker_does_not_restart_it() {
        let mut worker = worker();
        worker.enabled = false;
        assert!(!should_respawn_worker(&worker, true));
        worker.enabled = true;
        assert!(should_respawn_worker(&worker, true));
        assert!(!should_respawn_worker(&worker, false));
        assert!(should_stop_schedule(false, true));
        assert!(!should_stop_schedule(true, true));
        assert!(!should_stop_schedule(false, false));
        let mut enabled = worker.clone();
        enabled.enabled = true;
        enabled.auto_start = true;
        assert!(should_start_saved_worker(None, &enabled, false, true));
        assert!(!should_start_saved_worker(None, &enabled, false, false));
        assert!(!should_start_saved_worker(None, &enabled, true, true));
        assert!(!should_start_saved_worker(
            Some(&enabled),
            &enabled,
            false,
            true
        ));
        let mut disabled = enabled.clone();
        disabled.enabled = false;
        assert!(should_start_saved_worker(
            Some(&disabled),
            &enabled,
            false,
            true
        ));
        let off = ProjectSchedule::default();
        let on = ProjectSchedule {
            enabled: true,
            auto_start: true,
        };
        assert!(should_start_saved_schedule(&off, &on, false, true));
        assert!(!should_start_saved_schedule(&on, &on, false, true));
        assert!(!should_start_saved_schedule(&off, &on, true, true));
        assert!(!should_start_saved_schedule(&off, &on, false, false));
    }

    #[test]
    fn saved_worker_json_defaults_new_limits() {
        let worker: QueueWorker = serde_json::from_str(
            r#"{
                "id": "11111111-1111-1111-1111-111111111111",
                "name": "default",
                "connection": "default",
                "queue": "default",
                "processes": 1,
                "timeout": 60,
                "sleep": 3,
                "maxTries": 1,
                "memory": 128,
                "backoff": 0,
                "enabled": true,
                "autoStart": true
            }"#,
        )
        .unwrap();
        assert_eq!(worker.max_jobs, 0);
        assert_eq!(worker.max_time, 0);
    }
}
