use crate::{
    model::{ProjectSchedule, QueueWorker},
    Manager,
};
use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Save {
    pub id: String,
    pub workers: Vec<QueueWorker>,
    pub schedule: ProjectSchedule,
    pub expected_revision: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Control {
    pub id: String,
    pub worker_id: Option<String>,
    pub action: Action,
}
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum Action {
    Start,
    Stop,
    Restart,
}

pub(super) fn show(manager: &Manager, id: &str) -> Result<Value> {
    uuid::Uuid::parse_str(id)?;
    let state = manager
        .snapshot()?
        .projects
        .into_iter()
        .find(|p| p.project.id == id)
        .context("Proje bulunamadı.")?;
    let revision = crate::jobs::jobs_revision(&state.project.workers, &state.project.schedule)?;
    let states: Vec<Value> = state
        .worker_states
        .into_iter()
        .map(|s| json!({"id":s.id,"running":s.running,"hasIssue":s.issue.is_some()}))
        .collect();
    Ok(
        json!({"projectId":id,"name":state.project.name,"revision":revision,"workers":state.project.workers,"schedule":state.project.schedule,"states":states,"scheduleRunning":state.schedule_running,"scheduleHasIssue":state.schedule_issue.is_some()}),
    )
}
pub(super) fn save(manager: &Manager, input: Save) -> Result<Value> {
    uuid::Uuid::parse_str(&input.id)?;
    manager.save_project_jobs_checked(
        &input.id,
        input.workers,
        input.schedule,
        Some(&input.expected_revision),
    )?;
    show(manager, &input.id)
}
pub(super) fn control(manager: &Manager, input: Control, worker: bool) -> Result<Value> {
    uuid::Uuid::parse_str(&input.id)?;
    manager.project(&input.id)?;
    if worker {
        let id = input
            .worker_id
            .as_deref()
            .context("İşçi kimliği gerekli.")?;
        uuid::Uuid::parse_str(id)?;
        match input.action {
            Action::Start => manager.start_project_worker(&input.id, id)?,
            Action::Stop => manager.stop_project_worker(&input.id, id)?,
            Action::Restart => manager.restart_project_worker(&input.id, id)?,
        }
    } else {
        anyhow::ensure!(
            input.worker_id.is_none(),
            "Zamanlayıcı işçi kimliği kullanmaz."
        );
        match input.action {
            Action::Start => manager.start_project_schedule(&input.id)?,
            Action::Stop => manager.stop_project_schedule(&input.id)?,
            Action::Restart => manager.restart_project_schedule(&input.id)?,
        }
    }
    show(manager, &input.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_are_revision_checked_and_stopping_never_creates_processes() {
        let home = tempfile::tempdir().unwrap();
        let manager = Manager::new(home.path().into()).unwrap();
        let path = home.path().join("www/jobs");
        std::fs::create_dir_all(path.join("public")).unwrap();
        std::fs::write(path.join("public/index.php"), "<?php").unwrap();
        let project = manager.add_project("jobs".into(), path).unwrap();
        let before = show(&manager, &project.id).unwrap();
        let worker = QueueWorker {
            enabled: false,
            auto_start: false,
            ..Default::default()
        };
        let input = || Save {
            id: project.id.clone(),
            workers: vec![worker.clone()],
            schedule: Default::default(),
            expected_revision: before["revision"].as_str().unwrap().into(),
        };
        let after = save(&manager, input()).unwrap();
        assert_ne!(before["revision"], after["revision"]);
        assert!(save(&manager, input()).is_err());
        let worker_id = after["workers"][0]["id"].as_str().unwrap();
        let control_input = |action| Control {
            id: project.id.clone(),
            worker_id: Some(worker_id.into()),
            action,
        };
        assert!(control(&manager, control_input(Action::Start), true).is_err());
        let stopped = control(&manager, control_input(Action::Stop), true).unwrap();
        assert_eq!(stopped["states"][0]["running"], 0);
        assert_eq!(stopped["workers"].as_array().unwrap().len(), 1);
        assert!(!manager.snapshot().unwrap().any_running);
    }
}
