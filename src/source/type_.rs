//! `TypeSource` — a named type extracted via tree-sitter.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeSource {
    pub path: PathBuf,
    pub name: String,
}
