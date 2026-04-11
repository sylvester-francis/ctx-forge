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

/// Read a file to a String. If the file exists but is not valid UTF-8
/// (e.g. `.git/index`, an image, a compiled binary), returns a short
/// placeholder string instead of erroring. Missing or unreadable files
/// still propagate as `io::Error`.
///
/// Rationale: before v1.0.3, a single non-UTF-8 file in the bundle made
/// `resolve_all` fail, which caused `recalculate_tokens` to zero out the
/// entire bundle's token count via `.unwrap_or_default()`. One binary
/// file silently broke the gauge for every other item. The placeholder
/// approach keeps the item visible in the bundle but flags it clearly
/// for both the user and the downstream LLM.
fn read_to_utf8_or_placeholder(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)?;
    match String::from_utf8(bytes) {
        Ok(s) => Ok(s),
        Err(e) => Ok(format!(
            "<non-UTF-8 file, {} bytes — omitted>\n",
            e.as_bytes().len()
        )),
    }
}

pub fn resolve_one(item: &Item, project_root: &Path) -> Result<ResolvedItem> {
    let full = project_root.join(&item.path);
    let language = crate::lang::detect(&item.path);

    let content = match &item.kind {
        ItemKind::File => read_to_utf8_or_placeholder(&full)?,
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

/// Read a 1-indexed inclusive line range from a file. Non-UTF-8 files
/// return the same placeholder as `read_to_utf8_or_placeholder`.
fn read_range(path: &Path, range: Range) -> Result<String> {
    let raw = read_to_utf8_or_placeholder(path)?;
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

    #[test]
    fn resolve_non_utf8_file_returns_placeholder() {
        // Regression test for v1.0.3: before this fix, a single non-UTF-8
        // file in the bundle (e.g. `.git/index`, an image, a compiled
        // binary) caused `resolve_all` to error out, which then made the
        // TUI's `recalculate_tokens` (via `.unwrap_or_default()`) zero the
        // ENTIRE bundle's token count. Now non-UTF-8 content becomes a
        // short placeholder and the rest of the bundle resolves normally.
        let td = TempDir::new().unwrap();
        // Bytes that are definitely not valid UTF-8 (truncated multibyte
        // sequence, plus a lone 0xFF continuation byte).
        let bad_bytes: Vec<u8> = vec![0xC3, 0x28, 0xFF, 0xFE, 0xFD];
        std::fs::write(td.path().join("binary.bin"), &bad_bytes).unwrap();

        let item = Item {
            path: PathBuf::from("binary.bin"),
            kind: ItemKind::File,
            label: None,
        };
        let r = resolve_one(&item, td.path()).expect("should not error");
        assert!(r.content.starts_with("<non-UTF-8 file"));
        assert!(r.content.contains(&format!("{} bytes", bad_bytes.len())));
    }

    #[test]
    fn resolve_all_succeeds_with_mixed_text_and_binary() {
        // Regression test for v1.0.3: the bundle had two text files and one
        // binary file. Previously the binary file would kill the whole
        // resolve, zeroing tokens for everything. Now resolve_all succeeds
        // and produces three ResolvedItems.
        let td = TempDir::new().unwrap();
        write(td.path(), "a.rs", "fn a() {}\n");
        write(td.path(), "b.rs", "fn b() {}\n");
        std::fs::write(td.path().join("bad.bin"), [0xFF, 0xFE, 0xFD]).unwrap();

        let items = vec![
            Item {
                path: "a.rs".into(),
                kind: ItemKind::File,
                label: None,
            },
            Item {
                path: "bad.bin".into(),
                kind: ItemKind::File,
                label: None,
            },
            Item {
                path: "b.rs".into(),
                kind: ItemKind::File,
                label: None,
            },
        ];
        let resolved = resolve_all(&items, td.path()).expect("should not error");
        assert_eq!(resolved.len(), 3);
        assert_eq!(resolved[0].content, "fn a() {}\n");
        assert!(resolved[1].content.starts_with("<non-UTF-8 file"));
        assert_eq!(resolved[2].content, "fn b() {}\n");
    }
}
