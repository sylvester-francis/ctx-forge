//! Bundle items: files, line ranges, functions, types, notes.
//!
//! An `Item` is a reference into the project — it does NOT store content.
//! Content is resolved lazily at export time by `resolve.rs`.

#![allow(dead_code)]

use crate::error::{CtxforgeError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Inclusive 1-based line range, e.g. `45-120` in `src/check.go:45-120`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Range {
    pub start: usize,
    pub end: usize,
}

impl Range {
    /// Parse a range string like `"45-120"`.
    pub fn parse(input: &str) -> Result<Self> {
        let (start_s, end_s) =
            input
                .split_once('-')
                .ok_or_else(|| CtxforgeError::InvalidRange {
                    input: input.to_string(),
                    reason: "expected `start-end`".into(),
                })?;

        let start: usize = start_s.parse().map_err(|_| CtxforgeError::InvalidRange {
            input: input.to_string(),
            reason: format!("start `{start_s}` is not a number"),
        })?;
        let end: usize = end_s.parse().map_err(|_| CtxforgeError::InvalidRange {
            input: input.to_string(),
            reason: format!("end `{end_s}` is not a number"),
        })?;

        if start == 0 || end == 0 {
            return Err(CtxforgeError::InvalidRange {
                input: input.to_string(),
                reason: "lines are 1-indexed; 0 is invalid".into(),
            });
        }
        if start > end {
            return Err(CtxforgeError::InvalidRange {
                input: input.to_string(),
                reason: format!("start {start} > end {end}"),
            });
        }
        Ok(Range { start, end })
    }
}

/// The kind of thing a bundle item references.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ItemKind {
    /// Whole file.
    File,
    /// Specific line range.
    Range(Range),
    /// Tree-sitter function extraction (populated in Plan 7).
    Function { name: String },
    /// Tree-sitter type extraction (populated in Plan 7).
    Type { name: String },
}

/// A bundle item: a reference to something in the project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    pub path: PathBuf,
    pub kind: ItemKind,
    /// Optional human-readable label override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

impl Item {
    /// Parse a user-supplied argument of the form `path[:start-end]`.
    pub fn parse_add_argument(input: &str) -> Result<Self> {
        if let Some((path, range_str)) = input.rsplit_once(':') {
            // Only treat `:` as range separator if what follows looks like a range
            // attempt: contains `-` and has no path separators. This excludes URLs
            // like `https://example.com/file-name` while still catching bad ranges
            // like `foo-bar` so users get a helpful error.
            if range_str.contains('-') && !range_str.contains('/') && !range_str.contains('\\') {
                let range = Range::parse(range_str)?;
                return Ok(Item {
                    path: PathBuf::from(path),
                    kind: ItemKind::Range(range),
                    label: None,
                });
            }
        }
        Ok(Item {
            path: PathBuf::from(input),
            kind: ItemKind::File,
            label: None,
        })
    }

    /// Display string for `ctxforge status` output.
    pub fn display(&self) -> String {
        match &self.kind {
            ItemKind::File => self.path.display().to_string(),
            ItemKind::Range(r) => format!("{}:{}-{}", self.path.display(), r.start, r.end),
            ItemKind::Function { name } => format!("fn:{name} ({})", self.path.display()),
            ItemKind::Type { name } => format!("type:{name} ({})", self.path.display()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_parses_valid() {
        assert_eq!(Range::parse("1-5").unwrap(), Range { start: 1, end: 5 });
        assert_eq!(
            Range::parse("45-120").unwrap(),
            Range {
                start: 45,
                end: 120
            }
        );
    }

    #[test]
    fn range_rejects_zero() {
        assert!(Range::parse("0-5").is_err());
        assert!(Range::parse("1-0").is_err());
    }

    #[test]
    fn range_rejects_reversed() {
        assert!(Range::parse("10-5").is_err());
    }

    #[test]
    fn range_rejects_non_numeric() {
        assert!(Range::parse("abc-5").is_err());
        assert!(Range::parse("1-xyz").is_err());
        assert!(Range::parse("no-dash").is_err());
    }

    #[test]
    fn item_parses_plain_path() {
        let item = Item::parse_add_argument("src/main.rs").unwrap();
        assert_eq!(item.path, PathBuf::from("src/main.rs"));
        assert_eq!(item.kind, ItemKind::File);
    }

    #[test]
    fn item_parses_path_with_range() {
        let item = Item::parse_add_argument("src/main.rs:10-50").unwrap();
        assert_eq!(item.path, PathBuf::from("src/main.rs"));
        assert_eq!(item.kind, ItemKind::Range(Range { start: 10, end: 50 }));
    }

    #[test]
    fn item_rejects_bad_range() {
        assert!(Item::parse_add_argument("src/main.rs:foo-bar").is_err());
    }

    #[test]
    fn item_does_not_eat_colon_in_non_range() {
        // A URL-like suffix should NOT be parsed as a range.
        let item = Item::parse_add_argument("https://example.com/file").unwrap();
        assert_eq!(item.kind, ItemKind::File);
    }

    #[test]
    fn item_display_formats_each_variant() {
        let file = Item {
            path: "a.rs".into(),
            kind: ItemKind::File,
            label: None,
        };
        assert_eq!(file.display(), "a.rs");

        let range = Item {
            path: "a.rs".into(),
            kind: ItemKind::Range(Range { start: 1, end: 5 }),
            label: None,
        };
        assert_eq!(range.display(), "a.rs:1-5");
    }
}
