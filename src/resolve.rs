//! Resolve bundle items into string content at export time.
//!
//! The `Bundle` itself holds only references (paths, ranges, function names).
//! This module reads the filesystem to produce the content strings that the
//! format module will render.

#![allow(dead_code)]

use crate::bundle::{Item, ItemKind, Range};
use crate::error::{CtxforgeError, Result};
use std::path::Path;

/// The resolved content of a single item.
#[derive(Debug, Clone)]
pub struct ResolvedItem {
    pub item: Item,
    pub content: String,
    pub language: &'static str,
}

/// Resolve every item in `items` against the project rooted at `project_root`.
/// Returns the ordered list of resolved items.
pub fn resolve_all(items: &[Item], project_root: &Path) -> Result<Vec<ResolvedItem>> {
    items
        .iter()
        .map(|it| resolve_one(it, project_root))
        .collect()
}

pub fn resolve_one(item: &Item, project_root: &Path) -> Result<ResolvedItem> {
    let full = project_root.join(&item.path);
    let language = crate::lang::detect(&item.path);

    let content = match &item.kind {
        ItemKind::File => std::fs::read_to_string(&full)?,
        ItemKind::Range(range) => read_range(&full, *range)?,
        #[cfg(feature = "extract")]
        ItemKind::Function { name } => {
            let source = std::fs::read_to_string(&full)?;
            crate::extract::extract_function(&source, language, name)
                .map_err(CtxforgeError::Msg)?
                .ok_or_else(|| {
                    CtxforgeError::Msg(format!(
                        "function `{name}` not found in {}",
                        item.path.display()
                    ))
                })?
        }
        #[cfg(feature = "extract")]
        ItemKind::Type { name } => {
            let source = std::fs::read_to_string(&full)?;
            crate::extract::extract_type(&source, language, name)
                .map_err(CtxforgeError::Msg)?
                .ok_or_else(|| {
                    CtxforgeError::Msg(format!(
                        "type `{name}` not found in {}",
                        item.path.display()
                    ))
                })?
        }
        #[cfg(not(feature = "extract"))]
        ItemKind::Function { .. } | ItemKind::Type { .. } => {
            return Err(CtxforgeError::Msg(
                "function/type extraction requires `cargo install ctxforge --features=extract`"
                    .into(),
            ));
        }
    };

    Ok(ResolvedItem {
        item: item.clone(),
        content,
        language,
    })
}

/// Read a 1-indexed inclusive line range from a file.
fn read_range(path: &Path, range: Range) -> Result<String> {
    let raw = std::fs::read_to_string(path)?;
    let lines: Vec<&str> = raw.lines().collect();

    if lines.is_empty() {
        return Ok(String::new());
    }

    // Clamp to the file's actual line count.
    let start = range.start.min(lines.len());
    let end = range.end.min(lines.len());

    let slice = &lines[start - 1..end];
    Ok(slice.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::Range;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn write(root: &Path, rel: &str, content: &str) {
        let full = root.join(rel);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(full, content).unwrap();
    }

    #[test]
    fn resolve_whole_file() {
        let td = TempDir::new().unwrap();
        write(td.path(), "a.rs", "fn main() {}\n");

        let item = Item {
            path: PathBuf::from("a.rs"),
            kind: ItemKind::File,
            label: None,
        };
        let r = resolve_one(&item, td.path()).unwrap();
        assert_eq!(r.content, "fn main() {}\n");
        assert_eq!(r.language, "rust");
    }

    #[test]
    fn resolve_range_returns_slice() {
        let td = TempDir::new().unwrap();
        write(td.path(), "a.txt", "one\ntwo\nthree\nfour\nfive\n");

        let item = Item {
            path: PathBuf::from("a.txt"),
            kind: ItemKind::Range(Range { start: 2, end: 4 }),
            label: None,
        };
        let r = resolve_one(&item, td.path()).unwrap();
        assert_eq!(r.content, "two\nthree\nfour");
    }

    #[test]
    fn resolve_range_clamps_to_file_length() {
        let td = TempDir::new().unwrap();
        write(td.path(), "a.txt", "one\ntwo\n");

        let item = Item {
            path: PathBuf::from("a.txt"),
            kind: ItemKind::Range(Range { start: 1, end: 100 }),
            label: None,
        };
        let r = resolve_one(&item, td.path()).unwrap();
        assert_eq!(r.content, "one\ntwo");
    }

    #[test]
    fn resolve_missing_file_errors() {
        let td = TempDir::new().unwrap();
        let item = Item {
            path: PathBuf::from("nope.rs"),
            kind: ItemKind::File,
            label: None,
        };
        assert!(resolve_one(&item, td.path()).is_err());
    }
}
