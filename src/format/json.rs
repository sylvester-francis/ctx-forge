//! JSON export — structured format for API consumers and pipeline
//! integration (e.g. piping to LLM CLIs that accept JSON-RPC payloads).
//!
//! Backed by a typed `JsonContext` struct so serde handles all escaping.
//! The shape is intentionally stable — breaking changes should bump a
//! schema version field in the future.

use crate::bundle::ItemKind;
use crate::memory::Note;
use crate::resolve::ResolvedItem;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct JsonContext<'a> {
    pub schema_version: u32,
    pub items_count: usize,
    pub memory: Vec<JsonNote<'a>>,
    pub items: Vec<JsonItem<'a>>,
}

#[derive(Debug, Serialize)]
pub struct JsonNote<'a> {
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<&'a str>,
    pub body: &'a str,
}

#[derive(Debug, Serialize)]
pub struct JsonItem<'a> {
    pub path: String,
    pub language: &'a str,
    /// `"file"`, `"range"`, `"function"`, or `"type"`.
    pub kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines: Option<JsonLines>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<&'a str>,
    pub content: &'a str,
}

#[derive(Debug, Serialize)]
pub struct JsonLines {
    pub start: usize,
    pub end: usize,
}

pub fn render(items: &[ResolvedItem], memory: &[Note]) -> String {
    let json_memory: Vec<JsonNote> = memory
        .iter()
        .map(|n| JsonNote {
            timestamp: n.timestamp.to_rfc3339(),
            tag: n.tag.as_deref(),
            body: &n.body,
        })
        .collect();

    let json_items: Vec<JsonItem> = items
        .iter()
        .map(|r| {
            let (kind_str, lines, name) = match &r.item.kind {
                ItemKind::File => ("file", None, None),
                ItemKind::Range(range) => (
                    "range",
                    Some(JsonLines {
                        start: range.start,
                        end: range.end,
                    }),
                    None,
                ),
                ItemKind::Function { name } => ("function", None, Some(name.as_str())),
                ItemKind::Type { name } => ("type", None, Some(name.as_str())),
            };
            JsonItem {
                path: r.item.path.display().to_string(),
                language: r.language,
                kind: kind_str,
                lines,
                name,
                content: &r.content,
            }
        })
        .collect();

    let ctx = JsonContext {
        schema_version: 1,
        items_count: items.len(),
        memory: json_memory,
        items: json_items,
    };

    serde_json::to_string_pretty(&ctx).unwrap_or_else(|_| "{}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::{Item, ItemKind, Range};
    use crate::memory::Note;
    use serde_json::Value;
    use std::path::PathBuf;

    fn sample(path: &str, content: &str, lang: &'static str) -> ResolvedItem {
        ResolvedItem {
            item: Item {
                path: PathBuf::from(path),
                kind: ItemKind::File,
                label: None,
            },
            content: content.to_string(),
            language: lang,
        }
    }

    fn parse(s: &str) -> Value {
        serde_json::from_str(s).expect("render output must be valid JSON")
    }

    #[test]
    fn empty_renders_empty_arrays() {
        let v = parse(&render(&[], &[]));
        assert_eq!(v["schema_version"], 1);
        assert_eq!(v["items_count"], 0);
        assert_eq!(v["memory"].as_array().unwrap().len(), 0);
        assert_eq!(v["items"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn file_item_has_kind_file_and_content() {
        let v = parse(&render(
            &[sample("src/main.rs", "fn main() {}\n", "rust")],
            &[],
        ));
        assert_eq!(v["items_count"], 1);
        let item = &v["items"][0];
        assert_eq!(item["kind"], "file");
        assert_eq!(item["path"], "src/main.rs");
        assert_eq!(item["language"], "rust");
        assert_eq!(item["content"], "fn main() {}\n");
        assert!(item.get("lines").is_none() || item["lines"].is_null());
        assert!(item.get("name").is_none() || item["name"].is_null());
    }

    #[test]
    fn range_item_has_lines_object() {
        let item = ResolvedItem {
            item: Item {
                path: PathBuf::from("a.rs"),
                kind: ItemKind::Range(Range { start: 5, end: 10 }),
                label: None,
            },
            content: "slice\n".into(),
            language: "rust",
        };
        let v = parse(&render(&[item], &[]));
        let it = &v["items"][0];
        assert_eq!(it["kind"], "range");
        assert_eq!(it["lines"]["start"], 5);
        assert_eq!(it["lines"]["end"], 10);
    }

    #[test]
    fn memory_notes_are_serialized_with_rfc3339_timestamp() {
        let notes = vec![
            Note::new("JWT in header", Some("auth".into())),
            Note::new("untagged note", None),
        ];
        let v = parse(&render(&[], &notes));
        assert_eq!(v["memory"].as_array().unwrap().len(), 2);
        let n0 = &v["memory"][0];
        assert_eq!(n0["tag"], "auth");
        assert_eq!(n0["body"], "JWT in header");
        let ts: &str = n0["timestamp"].as_str().unwrap();
        assert!(ts.contains('T'), "timestamp should be RFC3339");
        let n1 = &v["memory"][1];
        assert!(n1.get("tag").is_none() || n1["tag"].is_null());
        assert_eq!(n1["body"], "untagged note");
    }

    #[test]
    fn special_chars_in_content_are_escaped_by_serde() {
        let v = parse(&render(
            &[sample("a.rs", "let s = \"a \\\"b\\\"\";\n", "rust")],
            &[],
        ));
        assert_eq!(v["items"][0]["content"], "let s = \"a \\\"b\\\"\";\n");
    }

    #[test]
    fn output_is_pretty_printed() {
        let out = render(&[sample("a.rs", "", "rust")], &[]);
        assert!(out.contains("\n"));
        assert!(out.contains("  "));
    }
}
