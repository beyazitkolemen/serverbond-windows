//! Panel state sections pushed to Cloud without waiting for read commands.

use crate::Manager;
use anyhow::Result;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

const BODY_LIMIT: usize = 768 * 1024;

#[derive(Clone, Debug)]
pub(super) struct Section {
    pub section: String,
    pub target_id: String,
    pub fingerprint: String,
    pub data: Value,
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
    } else {
        256 * 1024
    }
}

fn read(manager: &Manager, name: &str, parameters: Value) -> Result<Value> {
    super::operations::Operation::parse(name, &parameters)?.execute(manager)
}

fn insert(map: &mut BTreeMap<String, Section>, section: &str, target_id: &str, data: Value) {
    let Ok(encoded) = serde_json::to_vec(&data) else {
        return;
    };
    if encoded.len() > byte_limit(section) {
        return;
    }
    let Ok(fingerprint) = fingerprint(&data) else {
        return;
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
}

/// Build every Cloud panel section that this agent can report.
pub(super) fn sections(manager: &Manager) -> BTreeMap<String, Section> {
    let mut map = BTreeMap::new();
    if let Ok(data) = read(manager, "projects.list", json!({})) {
        insert(&mut map, "projects", "", data);
    }
    if let Ok(data) = read(manager, "php.list", json!({})) {
        insert(&mut map, "php", "", data);
    }
    if let Ok(data) = read(manager, "settings.show", json!({})) {
        insert(&mut map, "settings", "", data);
    }
    if let Ok(data) = read(manager, "tunnel.show", json!({})) {
        insert(&mut map, "tunnel", "", data);
    }
    if let Ok(data) = read(manager, "system.diagnostics", json!({})) {
        insert(&mut map, "diagnostics", "", data);
    }
    if manager.desktop_api().is_some() {
        if let Ok(data) = read(manager, "desktop.show", json!({})) {
            insert(&mut map, "desktop", "", data);
        }
    }
    let projects = manager
        .snapshot()
        .map(|snapshot| {
            snapshot
                .projects
                .into_iter()
                .map(|state| state.project.id)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for id in projects {
        if let Ok(data) = read(manager, "projects.show", json!({"id": id})) {
            insert(&mut map, "project", &id, data);
        }
        if let Ok(data) = read(manager, "jobs.show", json!({"id": id})) {
            insert(&mut map, "jobs", &id, data);
        }
        if let Ok(data) = read(manager, "database.show", json!({"id": id})) {
            insert(&mut map, "database", &id, data);
        }
        if let Ok(data) = read(manager, "env.read", json!({"id": id})) {
            insert(&mut map, "env", &id, data);
        }
    }
    map
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
) -> (Vec<Section>, Vec<(String, String)>) {
    let mut changed = Vec::new();
    for (key, section) in current {
        if previous.get(key).map(String::as_str) != Some(section.fingerprint.as_str()) {
            changed.push(section.clone());
        }
    }
    let mut removed = Vec::new();
    for key in previous.keys() {
        if !current.contains_key(key) {
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

/// Pack changed sections into one or more POST bodies under the Cloud size limit.
pub(super) fn bodies(sections: &[Section], removed: &[(String, String)]) -> Result<Vec<Value>> {
    let mut out = Vec::new();
    let mut batch_sections = Vec::new();
    let mut batch_removed: Vec<Value> = removed
        .iter()
        .map(|(section, target)| removed_item(section, target))
        .collect();
    for section in sections {
        let item = section_item(section);
        let mut candidate_sections = batch_sections.clone();
        candidate_sections.push(item.clone());
        let candidate = json!({"sections": candidate_sections, "removed": batch_removed});
        if serde_json::to_vec(&candidate)?.len() > BODY_LIMIT {
            if !batch_sections.is_empty() || !batch_removed.is_empty() {
                out.push(json!({"sections": batch_sections, "removed": batch_removed}));
                batch_sections = Vec::new();
                batch_removed = Vec::new();
            }
            let solo = json!({"sections": [item.clone()], "removed": []});
            anyhow::ensure!(
                serde_json::to_vec(&solo)?.len() <= BODY_LIMIT,
                "Durum bölümü çok büyük."
            );
            batch_sections.push(item);
        } else {
            batch_sections.push(item);
        }
    }
    if !batch_sections.is_empty() || !batch_removed.is_empty() {
        out.push(json!({"sections": batch_sections, "removed": batch_removed}));
    }
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
        let map = sections(&manager);
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
        let (changed, removed) = diff(&current, &previous);
        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0].section, "php");
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].0, "project");
        // Identical snapshots never trigger a second POST. Live diagnostics may
        // legitimately change between scans, so this invariant uses fixed data.
        let (unchanged, missing) = diff(&current, &fingerprint_map(&current));
        assert!(unchanged.is_empty());
        assert!(missing.is_empty());
    }
}
