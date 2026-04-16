//! Bundle data model and persistence.

#![allow(dead_code, unused_imports)]

pub mod item;
pub mod persist;

use crate::error::{CtxforgeError, Result};
use crate::paths::CtxforgeRoot;
pub use item::{Item, ItemKind, Range};
use serde::{Deserialize, Serialize};

/// A bundle is an ordered list of items. It is the central data model of ctxforge.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Bundle {
    #[serde(default = "default_version")]
    pub version: u32,
    pub items: Vec<Item>,
    /// Target model for token counting. Defaults via `models::DEFAULT_MODEL`.
    #[serde(default)]
    pub model: Option<String>,
    /// Natural-language task text the user is authoring. Rendered in the
    /// prompt preview's `## Task` section and delivered together with the
    /// bundle contents.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub task_text: String,
    /// Named scenario (template alias) — drives the template wrapper
    /// rendered around task + context on delivery.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scenario: Option<String>,
}

fn default_version() -> u32 {
    1
}

impl Bundle {
    pub fn new() -> Self {
        Self {
            version: 1,
            items: Vec::new(),
            model: None,
            task_text: String::new(),
            scenario: None,
        }
    }

    pub fn add(&mut self, item: Item) {
        // Avoid duplicates of the exact same (path, kind, label).
        if !self.items.contains(&item) {
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
        self.items.retain(|i| i.path != path);
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

    /// Load from disk, returning an empty bundle if none exists.
    pub fn load_or_default(root: &CtxforgeRoot) -> Result<Self> {
        let path = root.bundle_path();
        if !path.exists() {
            return Ok(Bundle::new());
        }
        let raw = std::fs::read_to_string(&path)?;
        let bundle: Bundle = serde_json::from_str(&raw)?;
        Ok(bundle)
    }

    /// Save to `.ctxforge/bundle.json`.
    pub fn save(&self, root: &CtxforgeRoot) -> Result<()> {
        let path = root.bundle_path();
        let raw = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, raw)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::CtxforgeRoot;
    use tempfile::TempDir;

    fn sample_item(path: &str) -> Item {
        Item {
            path: path.into(),
            kind: ItemKind::File,
            label: None,
        }
    }

    #[test]
    fn new_bundle_is_empty() {
        let b = Bundle::new();
        assert!(b.is_empty());
        assert_eq!(b.version, 1);
    }

    #[test]
    fn add_appends_unique_items() {
        let mut b = Bundle::new();
        b.add(sample_item("a.rs"));
        b.add(sample_item("b.rs"));
        assert_eq!(b.len(), 2);
    }

    #[test]
    fn add_deduplicates_exact_matches() {
        let mut b = Bundle::new();
        b.add(sample_item("a.rs"));
        b.add(sample_item("a.rs"));
        assert_eq!(b.len(), 1);
    }

    #[test]
    fn remove_by_index_uses_1_based() {
        let mut b = Bundle::new();
        b.add(sample_item("a.rs"));
        b.add(sample_item("b.rs"));
        let removed = b.remove_by_index(1).unwrap();
        assert_eq!(removed.path, std::path::PathBuf::from("a.rs"));
        assert_eq!(b.len(), 1);
    }

    #[test]
    fn remove_by_index_rejects_out_of_range() {
        let mut b = Bundle::new();
        assert!(b.remove_by_index(0).is_err());
        assert!(b.remove_by_index(1).is_err());
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
    fn load_or_default_returns_empty_when_no_file() {
        let td = TempDir::new().unwrap();
        let root = CtxforgeRoot::find_or_create(td.path()).unwrap();
        let b = Bundle::load_or_default(&root).unwrap();
        assert!(b.is_empty());
    }

    #[test]
    fn save_and_reload_roundtrip() {
        let td = TempDir::new().unwrap();
        let root = CtxforgeRoot::find_or_create(td.path()).unwrap();

        let mut b = Bundle::new();
        b.add(sample_item("a.rs"));
        b.model = Some("claude-sonnet-4".into());
        b.save(&root).unwrap();

        let reloaded = Bundle::load_or_default(&root).unwrap();
        assert_eq!(reloaded.items, b.items);
        assert_eq!(reloaded.model, b.model);
    }
}
