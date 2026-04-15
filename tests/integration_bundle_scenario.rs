//! Regression tests for the scenario + task_text additions to Bundle.

use ctxforge::bundle::Bundle;
use ctxforge::paths::CtxforgeRoot;
use tempfile::tempdir;

#[test]
fn bundle_serializes_scenario_and_task_text() {
    let tmp = tempdir().unwrap();
    let root = CtxforgeRoot::find_or_create(tmp.path()).unwrap();

    let mut b = Bundle::new();
    b.scenario = Some("bugfix".to_string());
    b.task_text = "fix the null pointer".to_string();
    b.save(&root).unwrap();

    let loaded = Bundle::load_or_default(&root).unwrap();
    assert_eq!(loaded.scenario, Some("bugfix".to_string()));
    assert_eq!(loaded.task_text, "fix the null pointer");
}

#[test]
fn bundle_without_scenario_loads_from_legacy_format() {
    let tmp = tempdir().unwrap();
    let root = CtxforgeRoot::find_or_create(tmp.path()).unwrap();
    // Legacy bundle — no scenario or task_text fields.
    std::fs::write(root.bundle_path(), r#"{"version":1,"items":[]}"#).unwrap();

    let loaded = Bundle::load_or_default(&root).unwrap();
    assert_eq!(loaded.scenario, None);
    assert_eq!(loaded.task_text, "");
}
