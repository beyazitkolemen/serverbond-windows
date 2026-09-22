//! Panel state sections pushed to Cloud without waiting for read commands.

use crate::Manager;
use anyhow::Result;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const BODY_LIMIT: usize = 768 * 1024;
const MAX_BATCH_ITEMS: usize = 40;

#[derive(Clone, Debug)]
pub(super) struct Section {
    pub section: String,
    pub target_id: String,
    pub fingerprint: String,
    pub data: Value,
}

pub(super) struct Snapshot {
    pub sections: BTreeMap<String, Section>,
    pub protected: BTreeSet<String>,
}

impl Snapshot {
    pub fn fingerprints(&self, previous: &BTreeMap<String, String>) -> BTreeMap<String, String> {
        let mut fingerprints = fingerprint_map(&self.sections);
        for key in &self.protected {
            if let Some(value) = previous.get(key) {
                fingerprints.insert(key.clone(), value.clone());
            }
        }
        fingerprints
    }
}

pub(super) fn fingerprint_key(section: &str, target_id: &str) -> String {
    if target_id.is_empty() {
        section.to_string()
    } else {
        format!("{section}:{target_id}")
    }
}

pub(super) fn fingerprint(data: &Value) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(data)?)))
}

fn byte_limit(section: &str) -> usize {
    if section == "env" {
        768 * 1024
    } else if section == "projects" {
        700 * 1024
    } else {
        256 * 1024
    }
}

fn read(manager: &Manager, name: &str, parameters: Value) -> Result<Value> {
    super::operations::Operation::parse(name, &parameters)?.execute(manager)
}

fn insert(
    map: &mut BTreeMap<String, Section>,
    section: &str,
    target_id: &str,
    data: Value,
) -> bool {
    let Ok(encoded) = serde_json::to_vec(&data) else {
        return false;
    };
    if encoded.len() > byte_limit(section) {
        return false;
    }
    let Ok(fingerprint) = fingerprint(&data) else {
        return false;
    };
    map.insert(
        fingerprint_key(section, target_id),
        Section {
            section: section.into(),
            target_id: target_id.into(),
            fingerprint,
            data,
        },
    );
    true
}

fn capture(
    snapshot: &mut Snapshot,
    manager: &Manager,
    section: &str,
    target: &str,
    operation: &str,
    parameters: Value,
) {
    let key = fingerprint_key(section, target);
    if !read(manager, operation, parameters)
        .is_ok_and(|data| insert(&mut snapshot.sections, section, target, data))
    {
        snapshot.protected.insert(key);
    }
}

/// Build every Cloud panel section that this agent can report.
pub(super) fn sections(manager: &Manager, previous: &BTreeMap<String, String>) -> Snapshot {
    let mut snapshot = Snapshot {
        sections: BTreeMap::new(),
        protected: BTreeSet::new(),
    };
    capture(&mut snapshot, manager, "php", "", "php.list", json!({}));
    capture(
        &mut snapshot,
        manager,
        "settings",
        "",
        "settings.show",
        json!({}),
    );
    capture(
        &mut snapshot,
        manager,
        "tunnel",
        "",
        "tunnel.show",
        json!({}),
    );
    capture(
        &mut snapshot,
        manager,
        "diagnostics",
        "",
        "system.diagnostics",
        json!({}),
    );
    if manager.desktop_api().is_some() {
        capture(
            &mut snapshot,
            manager,
            "desktop",
            "",
            "desktop.show",
            json!({}),
        );
    }
    match manager.snapshot() {
        Ok(inventory) => {
            let projects = super::operations::project_inventory(&inventory.projects, 0, 1000);
            if !insert(&mut snapshot.sections, "projects", "", projects) {
                snapshot.protected.insert("projects".into());
            }
            for project in inventory.projects {
                let id = project.project.id;
                capture(
                    &mut snapshot,
                    manager,
                    "project",
                    &id,
                    "projects.show",
                    json!({"id": id}),
                );
                capture(
                    &mut snapshot,
                    manager,
                    "jobs",
                    &id,
                    "jobs.show",
                    json!({"id": id}),
                );
                capture(
                    &mut snapshot,
                    manager,
                    "database",
                    &id,
                    "database.show",
                    json!({"id": id}),
                );
                capture(
                    &mut snapshot,
                    manager,
                    "env",
                    &id,
                    "env.read",
                    json!({"id": id}),
                );
            }
        }
        Err(_) => {
            snapshot.protected.insert("projects".into());
            for key in previous.keys() {
                if ["project:", "jobs:", "database:", "env:"]
                    .iter()
                    .any(|prefix| key.starts_with(prefix))
                {
                    snapshot.protected.insert(key.clone());
                }
            }
        }
    }
    snapshot
}

pub(super) fn fingerprint_map(sections: &BTreeMap<String, Section>) -> BTreeMap<String, String> {
    sections
        .iter()
        .map(|(key, section)| (key.clone(), section.fingerprint.clone()))
        .collect()
}

pub(super) fn diff(
    current: &BTreeMap<String, Section>,
    previous: &BTreeMap<String, String>,
    protected: &BTreeSet<String>,
) -> (Vec<Section>, Vec<(String, String)>) {
    let mut changed = Vec::new();
    for (key, section) in current {
        if previous.get(key).map(String::as_str) != Some(section.fingerprint.as_str()) {
            changed.push(section.clone());
        }
    }
    let mut removed = Vec::new();
    for key in previous.keys() {
        if !current.contains_key(key) && !protected.contains(key) {
            if let Some((section, target)) = parse_key(key) {
                removed.push((section, target));
            }
        }
    }
    (changed, removed)
}

fn parse_key(key: &str) -> Option<(String, String)> {
    if let Some((section, target)) = key.split_once(':') {
        if !target.is_empty() {
            return Some((section.into(), target.into()));
        }
    }
    Some((key.into(), String::new()))
}

fn section_item(section: &Section) -> Value {
    let mut item = json!({
        "section": section.section,
        "fingerprint": section.fingerprint,
        "data": section.data,
    });
    if !section.target_id.is_empty() {
        item["targetId"] = section.target_id.clone().into();
    }
    item
}

fn removed_item(section: &str, target: &str) -> Value {
    let mut item = json!({"section": section});
    if !target.is_empty() {
        item["targetId"] = target.into();
    }
    item
}

fn fits(sections: &[Value], removed: &[Value]) -> Result<bool> {
    Ok(sections.len() <= MAX_BATCH_ITEMS
        && removed.len() <= MAX_BATCH_ITEMS
        && serde_json::to_vec(&json!({"sections": sections, "removed": removed}))?.len()
            <= BODY_LIMIT)
}

fn flush(out: &mut Vec<Value>, sections: &mut Vec<Value>, removed: &mut Vec<Value>) {
    if !sections.is_empty() || !removed.is_empty() {
        out.push(json!({"sections": std::mem::take(sections), "removed": std::mem::take(removed)}));
    }
}

/// Pack changed sections under both Cloud's byte and item limits.
pub(super) fn bodies(sections: &[Section], removed: &[(String, String)]) -> Result<Vec<Value>> {
    let mut out = Vec::new();
    let mut batch_sections = Vec::new();
    let mut batch_removed = Vec::new();
    for section in sections {
        batch_sections.push(section_item(section));
        if !fits(&batch_sections, &batch_removed)? {
            let item = batch_sections.pop().expect("section was just added");
            flush(&mut out, &mut batch_sections, &mut batch_removed);
            batch_sections.push(item);
            anyhow::ensure!(
                fits(&batch_sections, &batch_removed)?,
                "Durum bölümü çok büyük."
            );
        }
    }
    for (section, target) in removed {
        batch_removed.push(removed_item(section, target));
        if !fits(&batch_sections, &batch_removed)? {
            let item = batch_removed.pop().expect("removal was just added");
            flush(&mut out, &mut batch_sections, &mut batch_removed);
            batch_removed.push(item);
            anyhow::ensure!(
                fits(&batch_sections, &batch_removed)?,
                "Durum kaldırma listesi çok büyük."
            );
        }
    }
    flush(&mut out, &mut batch_sections, &mut batch_removed);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_is_stable_for_the_same_payload() {
        let data = json!({"selected":"8.4","anyRunning":false,"versions":[]});
        assert_eq!(fingerprint(&data).unwrap(), fingerprint(&data).unwrap());
        assert_eq!(fingerprint(&data).unwrap().len(), 64);
        assert_ne!(
            fingerprint(&data).unwrap(),
            fingerprint(&json!({"selected":"8.3","anyRunning":false,"versions":[]})).unwrap()
        );
    }

    #[test]
    fn sections_cover_device_wide_catalog_keys() {
        let home = tempfile::tempdir().unwrap();
        let manager = Manager::new(home.path().into()).unwrap();
        let snapshot = sections(&manager, &BTreeMap::new());
        let map = snapshot.sections;
        for key in ["projects", "php", "settings", "tunnel", "diagnostics"] {
            assert!(map.contains_key(key), "missing {key}");
            assert_eq!(map[key].section, key);
            assert!(map[key].target_id.is_empty());
            assert_eq!(map[key].fingerprint.len(), 64);
        }
        assert!(!map.contains_key("desktop"));
        assert_eq!(map["projects"].data["offset"], 0);
        assert!(map["php"].data.get("versions").is_some());
        assert!(map["settings"].data.get("settings").is_some());
    }

    #[test]
    fn diff_reports_changed_and_removed_keys() {
        let current = BTreeMap::from([(
            "php".into(),
            Section {
                section: "php".into(),
                target_id: String::new(),
                fingerprint: "a".repeat(64),
                data: json!({"selected":"8.4"}),
            },
        )]);
        let previous = BTreeMap::from([
            ("php".into(), "b".repeat(64)),
            (format!("project:{}", uuid::Uuid::new_v4()), "c".repeat(64)),
        ]);
        let (changed, removed) = diff(&current, &previous, &BTreeSet::new());
        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0].section, "php");
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].0, "project");
        // Identical snapshots never trigger a second POST. Live diagnostics may
        // legitimately change between scans, so this invariant uses fixed data.
        let (unchanged, missing) = diff(&current, &fingerprint_map(&current), &BTreeSet::new());
        assert!(unchanged.is_empty());
        assert!(missing.is_empty());
    }

    #[test]
    fn failed_read_keeps_previous_sql_fingerprint_until_it_can_be_retried() {
        let home = tempfile::tempdir().unwrap();
        let path = tempfile::tempdir().unwrap();
        std::fs::create_dir(path.path().join("public")).unwrap();
        std::fs::write(path.path().join("public/index.php"), "<?php echo 'ok';").unwrap();
        std::fs::write(path.path().join(".env"), "APP_KEY=initial").unwrap();
        let manager = Manager::new(home.path().into()).unwrap();
        let project = manager
            .add_project("demo".into(), path.path().into())
            .unwrap();
        let first = sections(&manager, &BTreeMap::new());
        let previous = first.fingerprints(&BTreeMap::new());
        let key = fingerprint_key("env", &project.id);
        assert!(previous.contains_key(&key));

        std::fs::write(path.path().join(".env"), "x".repeat(300 * 1024)).unwrap();
        let snapshot = sections(&manager, &previous);
        assert!(!snapshot.sections.contains_key(&key));
        let (_, removed) = diff(&snapshot.sections, &previous, &snapshot.protected);
        assert!(removed.is_empty());
        assert_eq!(snapshot.fingerprints(&previous)[&key], previous[&key]);
    }

    #[test]
    fn large_project_inventory_obeys_cloud_state_endpoint_limits() {
        let sections = (0..95)
            .map(|_| Section {
                section: "project".into(),
                target_id: uuid::Uuid::new_v4().to_string(),
                fingerprint: "a".repeat(64),
                data: json!({"name":"demo"}),
            })
            .collect::<Vec<_>>();
        let removed = (0..85)
            .map(|_| ("project".into(), uuid::Uuid::new_v4().to_string()))
            .collect::<Vec<_>>();
        let bodies = bodies(&sections, &removed).unwrap();
        assert!(bodies.len() > 2);
        assert_eq!(
            bodies
                .iter()
                .map(|body| body["sections"].as_array().unwrap().len())
                .sum::<usize>(),
            95
        );
        assert_eq!(
            bodies
                .iter()
                .map(|body| body["removed"].as_array().unwrap().len())
                .sum::<usize>(),
            85
        );
        for body in &bodies {
            assert!(body["sections"].as_array().unwrap().len() <= 40);
            assert!(body["removed"].as_array().unwrap().len() <= 40);
            assert!(serde_json::to_vec(body).unwrap().len() <= BODY_LIMIT);
        }
    }
}
