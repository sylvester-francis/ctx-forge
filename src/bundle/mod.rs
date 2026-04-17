//! Bundle data model and persistence — v2 schema with v1 migration.

#![allow(dead_code, unused_imports)]

pub mod item;
pub mod migrate;
pub mod persist;

use crate::error::{CtxforgeError, Result};
use crate::paths::CtxforgeRoot;
use crate::source::{FileSource, Source};
pub use item::{Item, Range};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Bundle {
    #[serde(default = "default_version")]
    pub version: u32,
    pub items: Vec<Item>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub task_text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scenario: Option<String>,
}

fn default_version() -> u32 {
    2
}

impl Bundle {
    pub fn new() -> Self {
        Self {
            version: 2,
            items: vec![],
            model: None,
            task_text: String::new(),
            scenario: None,
        }
    }

    pub fn add(&mut self, item: Item) {
        if !self
            .items
            .iter()
            .any(|i| i.source == item.source && i.label == item.label)
        {
            self.items.push(item);
        }
    }

    pub fn remove_by_index(&mut self, idx: usize) -> Result<Item> {
        if idx == 0 || idx > self.items.len() {
            return Err(CtxforgeError::ItemNotFound(idx));
        }
        Ok(self.items.remove(idx - 1))
    }

    pub fn remove_by_path(&mut self, path: &std::path::Path) -> usize {
        let before = self.items.len();
        self.items
            .retain(|i| i.source.display_path().is_none_or(|p| p.as_path() != path));
        before - self.items.len()
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Load from disk. v1 bundles migrate in memory and emit a one-time
    /// stderr line; disk is not rewritten until the next explicit `save`.
    pub fn load_or_default(root: &CtxforgeRoot) -> Result<Self> {
        let path = root.bundle_path();
        if !path.exists() {
            return Ok(Bundle::new());
        }
        let raw = std::fs::read_to_string(&path)?;
        let (bundle, outcome) = migrate::migrate_json(&raw)?;
        if outcome == migrate::MigrationOutcome::MigratedFromV1 {
            eprintln!(
                "ctxforge: bundle loaded as v2 (migrated from v1; will rewrite on next save)"
            );
        }
        Ok(bundle)
    }

    /// Save as v2. Creates a one-time `<bundle>.v1.bak` if a v1 file exists
    /// on disk and the in-memory bundle is v2 (i.e. migration just happened
    /// and the user is about to overwrite v1 state).
    pub fn save(&self, root: &CtxforgeRoot) -> Result<()> {
        let path = root.bundle_path();
        if path.exists() {
            let raw = std::fs::read_to_string(&path)?;
            let on_disk_version = serde_json::from_str::<serde_json::Value>(&raw)
                .ok()
                .and_then(|v| v.get("version").and_then(|x| x.as_u64()))
                .unwrap_or(1);
            if on_disk_version < 2 {
                let backup = path.with_extension("json.v1.bak");
                if !backup.exists() {
                    std::fs::copy(&path, &backup)?;
                }
            }
        }
        let raw = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, raw)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_item(p: &str) -> Item {
        Item {
            source: Source::File(FileSource { path: p.into() }),
            label: None,
        }
    }

    #[test]
    fn new_is_v2_empty() {
        let b = Bundle::new();
        assert!(b.is_empty());
        assert_eq!(b.version, 2);
    }

    #[test]
    fn add_dedups() {
        let mut b = Bundle::new();
        b.add(sample_item("a.rs"));
        b.add(sample_item("a.rs"));
        assert_eq!(b.len(), 1);
    }

    #[test]
    fn remove_by_index_is_1_based() {
        let mut b = Bundle::new();
        b.add(sample_item("a.rs"));
        b.add(sample_item("b.rs"));
        let removed = b.remove_by_index(1).unwrap();
        assert!(matches!(removed.source, Source::File(ref f) if f.path == std::path::PathBuf::from("a.rs")));
    }

    #[test]
    fn remove_by_path_returns_count() {
        let mut b = Bundle::new();
        b.add(sample_item("a.rs"));
        b.add(sample_item("b.rs"));
        assert_eq!(b.remove_by_path("a.rs".as_ref()), 1);
        assert_eq!(b.remove_by_path("nope.rs".as_ref()), 0);
    }

    #[test]
    fn save_then_reload_v2() {
        let td = TempDir::new().unwrap();
        let root = CtxforgeRoot::find_or_create(td.path()).unwrap();
        let mut b = Bundle::new();
        b.add(sample_item("a.rs"));
        b.model = Some("claude-sonnet-4".into());
        b.save(&root).unwrap();
        let reloaded = Bundle::load_or_default(&root).unwrap();
        assert_eq!(reloaded.version, 2);
        assert_eq!(reloaded.len(), 1);
        assert_eq!(reloaded.model, b.model);
    }

    #[test]
    fn v1_migrates_on_load_without_writing() {
        let td = TempDir::new().unwrap();
        let root = CtxforgeRoot::find_or_create(td.path()).unwrap();
        let v1 = r#"{
            "version": 1,
            "items": [
                {"path": "src/main.rs", "kind": {"kind": "file"}},
                {"path": "src/hub.rs", "kind": {"kind": "range", "start": 10, "end": 20}}
            ],
            "model": "claude-sonnet-4"
        }"#;
        std::fs::write(root.bundle_path(), v1).unwrap();

        let b = Bundle::load_or_default(&root).unwrap();
        assert_eq!(b.version, 2);
        assert_eq!(b.len(), 2);
        let on_disk = std::fs::read_to_string(root.bundle_path()).unwrap();
        assert!(
            on_disk.contains(r#""version": 1"#) || on_disk.contains(r#""version":1"#),
            "disk must still be v1 until explicit save",
        );
    }

    #[test]
    fn save_after_v1_migration_writes_backup_once() {
        let td = TempDir::new().unwrap();
        let root = CtxforgeRoot::find_or_create(td.path()).unwrap();
        std::fs::write(
            root.bundle_path(),
            r#"{"version":1,"items":[{"path":"a.rs","kind":{"kind":"file"}}]}"#,
        )
        .unwrap();
        let b = Bundle::load_or_default(&root).unwrap();
        b.save(&root).unwrap();
        let backup = root.bundle_path().with_extension("json.v1.bak");
        assert!(backup.exists());

        let meta = std::fs::metadata(&backup).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        b.save(&root).unwrap();
        let meta2 = std::fs::metadata(&backup).unwrap();
        assert_eq!(meta.len(), meta2.len());
    }

    #[test]
    fn future_version_errors() {
        let td = TempDir::new().unwrap();
        let root = CtxforgeRoot::find_or_create(td.path()).unwrap();
        std::fs::write(root.bundle_path(), r#"{"version":99,"items":[]}"#).unwrap();
        assert!(Bundle::load_or_default(&root).is_err());
    }
}
