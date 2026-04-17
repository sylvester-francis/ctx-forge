//! XML export — optimized for Claude/Anthropic prompting.
//!
//! Structure:
//!
//! ```xml
//! <context items="N">
//!   <memory count="M">
//!     <note timestamp="..." tag="...">body</note>
//!     ...
//!   </memory>
//!   <source path="..." language="..."><![CDATA[...code...]]></source>
//!   <source path="..." language="..." lines="45-120"><![CDATA[...]]></source>
//!   <documentation path="..." language="markdown"><![CDATA[...]]></documentation>
//! </context>
//! ```
//!
//! Items whose detected language is `markdown` render as `<documentation>`;
//! everything else renders as `<source>`. This gives the agent a semantic
//! signal for how to treat each item.
//!
//! Content is wrapped in `<![CDATA[...]]>`. If the content itself contains
//! the literal sequence `]]>`, the standard split trick applies: replace
//! `]]>` with `]]]]><![CDATA[>` so it is broken across two CDATA sections.

use crate::memory::Note;
use crate::resolve::ResolvedItem;
use crate::source::Source;

pub fn render(items: &[ResolvedItem], memory: &[Note]) -> String {
    let mut out = String::new();
    out.push_str(&format!("<context items=\"{}\">\n", items.len()));

    if !memory.is_empty() {
        write_memory(&mut out, memory);
    }

    for item in items {
        write_item(&mut out, item);
    }

    out.push_str("</context>\n");
    out
}

fn write_memory(out: &mut String, memory: &[Note]) {
    out.push_str(&format!("  <memory count=\"{}\">\n", memory.len()));
    for note in memory {
        let ts = note.timestamp.to_rfc3339();
        match &note.tag {
            Some(t) => out.push_str(&format!(
                "    <note timestamp=\"{}\" tag=\"{}\">{}</note>\n",
                ts,
                escape_attr(t),
                escape_text(&note.body),
            )),
            None => out.push_str(&format!(
                "    <note timestamp=\"{}\">{}</note>\n",
                ts,
                escape_text(&note.body),
            )),
        }
    }
    out.push_str("  </memory>\n");
}

fn write_item(out: &mut String, r: &ResolvedItem) {
    let tag_name = if r.language == "markdown" {
        "documentation"
    } else {
        "source"
    };

    let path_str = match &r.item.source {
        Source::File(f) => f.path.display().to_string(),
        Source::Range(r2) => r2.path.display().to_string(),
        Source::Func(f) => f.path.display().to_string(),
        Source::Type(t) => t.path.display().to_string(),
        Source::Url(u) => u.url.clone(),
    };

    out.push_str("  <");
    out.push_str(tag_name);
    out.push_str(&format!(
        " path=\"{}\" language=\"{}\"",
        escape_attr(&path_str),
        r.language
    ));

    match &r.item.source {
        Source::File(_) | Source::Url(_) => {}
        Source::Range(range) => {
            out.push_str(&format!(" lines=\"{}-{}\"", range.start, range.end));
        }
        Source::Func(f) => {
            out.push_str(&format!(" fn=\"{}\"", escape_attr(&f.name)));
        }
        Source::Type(t) => {
            out.push_str(&format!(" type=\"{}\"", escape_attr(&t.name)));
        }
    }

    out.push_str("><![CDATA[");
    out.push_str(&cdata_escape(&r.content));
    out.push_str("]]></");
    out.push_str(tag_name);
    out.push_str(">\n");
}

/// Escape the five XML attribute characters. CDATA handles everything else.
fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Escape XML text content (not inside CDATA). Used for memory note bodies.
fn escape_text(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Split `]]>` inside content across two CDATA sections so it can be safely
/// embedded inside a single outer `<![CDATA[...]]>` wrapper.
fn cdata_escape(s: &str) -> String {
    s.replace("]]>", "]]]]><![CDATA[>")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::Item;
    use crate::memory::Note;
    use crate::source::{FileSource, RangeSource};
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

    #[test]
    fn empty_bundle_renders_empty_context() {
        let out = render(&[], &[]);
        assert!(out.starts_with("<context items=\"0\">"));
        assert!(out.trim_end().ends_with("</context>"));
    }

    #[test]
    fn single_source_file_uses_source_tag() {
        let out = render(&[sample("src/main.rs", "fn main() {}\n", "rust")], &[]);
        assert!(out.contains("<source path=\"src/main.rs\" language=\"rust\""));
        assert!(out.contains("<![CDATA[fn main() {}\n]]></source>"));
    }

    #[test]
    fn markdown_file_uses_documentation_tag() {
        let out = render(&[sample("README.md", "# Project\n", "markdown")], &[]);
        assert!(out.contains("<documentation path=\"README.md\" language=\"markdown\""));
        assert!(out.contains("</documentation>"));
        assert!(!out.contains("<source"));
    }

    #[test]
    fn line_range_item_has_lines_attribute() {
        let item = Item {
            source: Source::Range(RangeSource::new("src/hub.rs".into(), 45, 120).unwrap()),
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
        let out = render(&[resolved], &[]);
        assert!(out.contains("lines=\"45-120\""));
    }

    #[test]
    fn memory_section_renders_inside_context() {
        let notes = vec![
            Note::new("JWT in header", Some("auth".into())),
            Note::new("general note", None),
        ];
        let out = render(&[], &notes);
        assert!(out.contains("<memory count=\"2\">"));
        assert!(out.contains("tag=\"auth\""));
        assert!(out.contains("JWT in header"));
        assert!(out.contains("general note"));
        assert!(out.contains("</memory>"));
    }

    #[test]
    fn path_with_special_chars_is_attribute_escaped() {
        let out = render(&[sample("src/a&b.rs", "", "rust")], &[]);
        assert!(out.contains("path=\"src/a&amp;b.rs\""));
    }

    #[test]
    fn cdata_split_handles_content_with_closing_cdata() {
        let out = render(&[sample("a.rs", "let s = \"]]>\";\n", "rust")], &[]);
        assert!(!out.contains("\"]]>\""));
        assert!(out.contains("]]]]><![CDATA[>"));
    }

    #[test]
    fn root_items_count_matches_input_len() {
        let out = render(
            &[
                sample("a.rs", "", "rust"),
                sample("b.rs", "", "rust"),
                sample("c.md", "", "markdown"),
            ],
            &[],
        );
        assert!(out.contains("<context items=\"3\">"));
    }

    #[test]
    fn memory_note_body_is_text_escaped() {
        let notes = vec![Note::new("A < B && C > D", None)];
        let out = render(&[], &notes);
        assert!(out.contains("A &lt; B &amp;&amp; C &gt; D"));
    }
}
