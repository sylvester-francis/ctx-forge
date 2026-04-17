//! JSON export — structured format for API consumers and pipeline
//! integration (e.g. piping to LLM CLIs that accept JSON-RPC payloads).
//!
//! Backed by a typed `JsonContext` struct so serde handles all escaping.
//! The shape is intentionally stable — breaking changes should bump a
//! schema version field in the future.

use crate::memory::Note;
use crate::resolve::ResolvedItem;
use crate::source::Source;
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
    /// `"file"`, `"range"`, `"function"`, `"type"`, or `"url"`.
    pub kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines: Option<JsonLines>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<&'a str>,
    pub content: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance: Option<crate::source::Provenance>,
}

#[derive(Debug, Serialize)]
pub struct JsonLines {
    pub start: usize,
    pub end: usize,
}

pub fn render(items: &[ResolvedItem], memory: &[Note], no_provenance: bool) -> String {
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
            let (kind_str, lines, name, path) = match &r.item.source {
                Source::File(f) => ("file", None, None, f.path.display().to_string()),
                Source::Range(r2) => (
                    "range",
                    Some(JsonLines {
                        start: r2.start,
                        end: r2.end,
                    }),
                    None,
                    r2.path.display().to_string(),
                ),
                Source::Func(f) => (
                    "function",
                    None,
                    Some(f.name.as_str()),
                    f.path.display().to_string(),
                ),
                Source::Type(t) => (
                    "type",
                    None,
                    Some(t.name.as_str()),
                    t.path.display().to_string(),
                ),
                Source::Url(u) => ("url", None, None, u.url.clone()),
            };
            JsonItem {
                path,
                language: r.language,
                kind: kind_str,
                lines,
                name,
                content: &r.content,
                provenance: if no_provenance {
                    None
                } else {
                    Some(r.provenance.clone())
                },
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
    use crate::bundle::Item;
    use crate::memory::Note;
    use crate::source::{FileSource, RangeSource};
    use serde_json::Value;
    use std::path::PathBuf;

    fn sample(path: &str, content: &str, lang: &'static str) -> ResolvedItem {
        let item = Item {
            source: Source::File(FileSource {
                path: PathBuf::from(path),
            }),
            label: None,
        };
        ResolvedItem {
            provenance: crate::source::Provenance::local(
                item.source.to_uri().to_string(),
                String::new(),
            ),
            item,
            content: content.to_string(),
            language: lang,
        }
    }

    fn parse(s: &str) -> Value {
        serde_json::from_str(s).expect("render output must be valid JSON")
    }

    #[test]
    fn empty_renders_empty_arrays() {
        let v = parse(&render(&[], &[], true));
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
            true,
        ));
        assert_eq!(v["items_count"], 1);
        let item = &v["items"][0];
        assert_eq!(item["kind"], "file");
        assert_eq!(item["path"], "src/main.rs");
        assert_eq!(item["language"], "rust");
        assert_eq!(item["content"], "fn main() {}\n");
        assert!(item.get("lines").is_none() || item["lines"].is_null());
    }

    #[test]
    fn range_item_has_lines_object() {
        let item = Item {
            source: Source::Range(RangeSource::new("a.rs".into(), 5, 10).unwrap()),
            label: None,
        };
        let resolved = ResolvedItem {
            provenance: crate::source::Provenance::local(
                item.source.to_uri().to_string(),
                String::new(),
            ),
            item,
            content: "slice\n".into(),
            language: "rust",
        };
        let v = parse(&render(&[resolved], &[], true));
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
        let v = parse(&render(&[], &notes, true));
        assert_eq!(v["memory"].as_array().unwrap().len(), 2);
        let n0 = &v["memory"][0];
        assert_eq!(n0["tag"], "auth");
        assert_eq!(n0["body"], "JWT in header");
        let ts: &str = n0["timestamp"].as_str().unwrap();
        assert!(ts.contains('T'), "timestamp should be RFC3339");
    }

    #[test]
    fn output_is_pretty_printed() {
        let out = render(&[sample("a.rs", "", "rust")], &[], true);
        assert!(out.contains("\n"));
        assert!(out.contains("  "));
    }

    #[test]
    fn provenance_object_emitted_by_default() {
        let v = parse(&render(&[sample("a.rs", "x\n", "rust")], &[], false));
        let prov = &v["items"][0]["provenance"];
        assert_eq!(prov["uri"], "file://a.rs");
        assert!(prov["sha256"].is_string());
    }

    #[test]
    fn no_provenance_omits_the_object() {
        let v = parse(&render(&[sample("a.rs", "x\n", "rust")], &[], true));
        assert!(v["items"][0].get("provenance").is_none() || v["items"][0]["provenance"].is_null());
    }
}
