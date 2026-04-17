//! v1 → v2 migration regression suite. Covers bundles AND profiles.

use ctxforge::bundle::{Bundle, migrate};
use ctxforge::paths::CtxforgeRoot;
use tempfile::TempDir;

fn load_v1_bundle(fixture: &str) -> Bundle {
    let td = TempDir::new().unwrap();
    let root = CtxforgeRoot::find_or_create(td.path()).unwrap();
    std::fs::write(root.bundle_path(), fixture).unwrap();
    Bundle::load_or_default(&root).unwrap()
}

#[test]
fn basic_v1_migrates() {
    let b = load_v1_bundle(include_str!("fixtures/v1_bundles/basic.json"));
    assert_eq!(b.version, 2);
    assert_eq!(b.len(), 2);
    assert_eq!(b.items[0].display(), "src/main.rs");
    assert_eq!(b.items[1].label, Some("entry".into()));
}

#[test]
fn all_kinds_migrate() {
    let b = load_v1_bundle(include_str!("fixtures/v1_bundles/all_kinds.json"));
    assert_eq!(b.len(), 4);
    assert!(b.items[1].display().contains("45-120"));
    assert!(b.items[2].display().contains("parse_expr"));
    assert!(b.items[3].display().contains("Config"));
}

#[test]
fn v2_roundtrip_after_first_save() {
    let td = TempDir::new().unwrap();
    let root = CtxforgeRoot::find_or_create(td.path()).unwrap();
    std::fs::write(
        root.bundle_path(),
        include_str!("fixtures/v1_bundles/basic.json"),
    )
    .unwrap();
    let b = Bundle::load_or_default(&root).unwrap();
    b.save(&root).unwrap();
    let b2 = Bundle::load_or_default(&root).unwrap();
    assert_eq!(b2.version, 2);
    assert_eq!(b2.len(), b.len());
    let backup = root.bundle_path().with_extension("json.v1.bak");
    assert!(backup.exists());
}

#[test]
fn profile_v1_migrates() {
    use ctxforge::profile;
    let td = TempDir::new().unwrap();
    let root = CtxforgeRoot::find_or_create(td.path()).unwrap();
    std::fs::write(
        root.profile_path("sample"),
        include_str!("fixtures/v1_profiles/sample.json"),
    )
    .unwrap();
    let b = profile::load(&root, "sample").unwrap();
    assert_eq!(b.version, 2);
    assert_eq!(b.len(), 1);
}

#[test]
fn migrate_json_is_pure() {
    let v1 = include_str!("fixtures/v1_bundles/basic.json");
    let (a, _) = migrate::migrate_json(v1).unwrap();
    let (b, _) = migrate::migrate_json(v1).unwrap();
    assert_eq!(a.items.len(), b.items.len());
}
