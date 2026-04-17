//! v1 Bundle serde types. Read-only — consumed by `bundle::migrate` to
//! translate legacy on-disk bundles and profiles to the v2 `Source` model.

use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct BundleV1 {
    #[serde(default = "default_one")]
    pub version: u32,
    pub items: Vec<ItemV1>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub task_text: String,
    #[serde(default)]
    pub scenario: Option<String>,
}

fn default_one() -> u32 {
    1
}

#[derive(Debug, Deserialize)]
pub struct ItemV1 {
    pub path: PathBuf,
    pub kind: ItemKindV1,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ItemKindV1 {
    File,
    Range { start: usize, end: usize },
    Function { name: String },
    Type { name: String },
}
