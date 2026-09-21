use serde_json::Value;
use std::collections::BTreeSet;

#[test]
fn every_registered_desktop_command_has_a_documented_http_equivalent() {
    let source = include_str!("../../../src-tauri/src/main.rs");
    let handler = source
        .split("tauri::generate_handler![")
        .nth(1)
        .unwrap()
        .split("])")
        .next()
        .unwrap();
    let commands: BTreeSet<_> = handler
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    let coverage: Value = serde_json::from_str(include_str!("../api-coverage.json")).unwrap();
    let covered: BTreeSet<_> = coverage["commands"]
        .as_object()
        .unwrap()
        .keys()
        .chain(coverage["internal"].as_object().unwrap().keys())
        .map(String::as_str)
        .collect();
    assert_eq!(
        commands, covered,
        "New IPC operations need an HTTP equivalent and coverage entry."
    );
    let doc = serverbond_core::api::documentation();
    for group in ["commands", "plugins"] {
        for (command, endpoints) in coverage[group].as_object().unwrap() {
            assert!(!endpoints.as_array().unwrap().is_empty(), "{command}");
            for endpoint in endpoints.as_array().unwrap() {
                let (method, path) = endpoint.as_str().unwrap().split_once(' ').unwrap();
                assert!(
                    doc["document"]["paths"][path]
                        .get(method.to_lowercase())
                        .is_some(),
                    "{command}: {endpoint}"
                );
            }
        }
    }
}
