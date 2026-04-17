//! Resolve bundle items into string content at export time.
//!
//! The `Bundle` holds references (Source variants); this module reads the
//! filesystem (or cache, for network sources in Task 9) to produce content
//! strings plus a `Provenance` record.

#![allow(dead_code)]

use crate::bundle::Item;
use crate::error::{CtxforgeError, Result};
use crate::source::{Provenance, Source};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ResolvedItem {
    pub item: Item,
    pub content: String,
    pub language: &'static str,
    pub provenance: Provenance,
}

pub fn resolve_all(items: &[Item], project_root: &Path) -> Result<Vec<ResolvedItem>> {
    items
        .iter()
        .map(|it| resolve_one(it, project_root))
        .collect()
}

pub fn resolve_one(item: &Item, project_root: &Path) -> Result<ResolvedItem> {
    match &item.source {
        Source::File(f) => resolve_file(item, project_root, &f.path, None),
        Source::Range(r) => resolve_file(item, project_root, &r.path, Some((r.start, r.end))),
        #[cfg(feature = "extract")]
        Source::Func(f) => resolve_symbol(item, project_root, &f.path, &f.name, true),
        #[cfg(feature = "extract")]
        Source::Type(t) => resolve_symbol(item, project_root, &t.path, &t.name, false),
        #[cfg(not(feature = "extract"))]
        Source::Func(_) | Source::Type(_) => Err(CtxforgeError::Msg(
            "function/type extraction requires `cargo install ctxforge --features=extract`".into(),
        )),
        Source::Url(_) => Err(CtxforgeError::Msg(
            "network sources require resolve::resolve_all_with_ctx (Task 8)".into(),
        )),
    }
}

fn resolve_file(
    item: &Item,
    project_root: &Path,
    rel: &Path,
    range: Option<(usize, usize)>,
) -> Result<ResolvedItem> {
    let full = project_root.join(rel);
    let language = crate::lang::detect(rel);
    let bytes = std::fs::read(&full)?;
    let all = match String::from_utf8(bytes) {
        Ok(s) => s,
        Err(e) => format!("<non-UTF-8 file, {} bytes — omitted>\n", e.as_bytes().len()),
    };
    let content = match range {
        None => all,
        Some((start, end)) => extract_range(&all, start, end),
    };
    let sha = crate::cache::sha256_hex(content.as_bytes());
    Ok(ResolvedItem {
        item: item.clone(),
        content,
        language,
        provenance: Provenance::local(item.source.to_uri().to_string(), sha),
    })
}

fn extract_range(raw: &str, start: usize, end: usize) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    if lines.is_empty() {
        return String::new();
    }
    let start = start.min(lines.len()).max(1);
    let end = end.min(lines.len());
    lines[start - 1..end].join("\n")
}

#[cfg(feature = "extract")]
fn resolve_symbol(
    item: &Item,
    project_root: &Path,
    rel: &Path,
    name: &str,
    is_func: bool,
) -> Result<ResolvedItem> {
    let full = project_root.join(rel);
    let language = crate::lang::detect(rel);
    let source_text = std::fs::read_to_string(&full)?;
    let extracted = if is_func {
        crate::extract::extract_function(&source_text, language, name).map_err(CtxforgeError::Msg)?
    } else {
        crate::extract::extract_type(&source_text, language, name).map_err(CtxforgeError::Msg)?
    };
    let content = extracted.ok_or_else(|| {
        CtxforgeError::Msg(format!(
            "{} `{}` not found in {}",
            if is_func { "function" } else { "type" },
            name,
            rel.display()
        ))
    })?;
    let sha = crate::cache::sha256_hex(content.as_bytes());
    Ok(ResolvedItem {
        item: item.clone(),
        content,
        language,
        provenance: Provenance::local(item.source.to_uri().to_string(), sha),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{FileSource, RangeSource};
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn write(root: &Path, rel: &str, c: &str) {
        let f = root.join(rel);
        std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        std::fs::write(f, c).unwrap();
    }

    #[test]
    fn file_has_local_provenance_with_sha() {
        let td = TempDir::new().unwrap();
        write(td.path(), "a.rs", "fn main() {}\n");
        let item = Item {
            source: Source::File(FileSource {
                path: "a.rs".into(),
            }),
            label: None,
        };
        let r = resolve_one(&item, td.path()).unwrap();
        assert_eq!(r.content, "fn main() {}\n");
        assert!(!r.provenance.sha256.is_empty());
        assert!(r.provenance.fetched_at.is_none());
        assert_eq!(r.provenance.uri, "file://a.rs");
    }

    #[test]
    fn range_extracts_slice() {
        let td = TempDir::new().unwrap();
        write(td.path(), "a.txt", "one\ntwo\nthree\nfour\n");
        let src = Source::Range(RangeSource::new("a.txt".into(), 2, 3).unwrap());
        let item = Item {
            source: src,
            label: None,
        };
        let r = resolve_one(&item, td.path()).unwrap();
        assert_eq!(r.content, "two\nthree");
    }

    #[test]
    fn range_clamps_past_end() {
        let td = TempDir::new().unwrap();
        write(td.path(), "a.txt", "one\ntwo\n");
        let src = Source::Range(RangeSource::new("a.txt".into(), 1, 100).unwrap());
        let item = Item {
            source: src,
            label: None,
        };
        let r = resolve_one(&item, td.path()).unwrap();
        assert_eq!(r.content, "one\ntwo");
    }

    #[test]
    fn non_utf8_produces_placeholder() {
        let td = TempDir::new().unwrap();
        std::fs::write(td.path().join("b.bin"), [0xFF, 0xFE, 0xFD]).unwrap();
        let item = Item {
            source: Source::File(FileSource {
                path: "b.bin".into(),
            }),
            label: None,
        };
        let r = resolve_one(&item, td.path()).unwrap();
        assert!(r.content.starts_with("<non-UTF-8"));
    }

    #[test]
    fn url_source_errors_in_base_resolve() {
        use crate::source::UrlSource;
        let item = Item {
            source: Source::Url(UrlSource {
                url: "https://x".into(),
            }),
            label: None,
        };
        assert!(resolve_one(&item, Path::new("/")).is_err());
    }

    #[test]
    fn resolve_all_succeeds_with_mixed_text_and_binary() {
        let td = TempDir::new().unwrap();
        write(td.path(), "a.rs", "fn a() {}\n");
        write(td.path(), "b.rs", "fn b() {}\n");
        std::fs::write(td.path().join("bad.bin"), [0xFF, 0xFE, 0xFD]).unwrap();

        let items = vec![
            Item {
                source: Source::File(FileSource {
                    path: PathBuf::from("a.rs"),
                }),
                label: None,
            },
            Item {
                source: Source::File(FileSource {
                    path: PathBuf::from("bad.bin"),
                }),
                label: None,
            },
            Item {
                source: Source::File(FileSource {
                    path: PathBuf::from("b.rs"),
                }),
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
