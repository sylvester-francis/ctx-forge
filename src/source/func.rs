//! `FuncSource` — a named function extracted via tree-sitter.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FuncSource {
    pub path: PathBuf,
    pub name: String,
}
