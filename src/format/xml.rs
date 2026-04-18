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

pub fn render(items: &[ResolvedItem], memory: &[Note], no_provenance: bool) -> String {
    let (docs, other) = crate::format::partition_docs(items);

    let mut out = String::new();
    out.push_str(&format!("<context items=\"{}\">\n", other.len()));

    if !memory.is_empty() {
        write_memory(&mut out, memory);
    }

    if !docs.is_empty() {
        write_project_stack(&mut out, &docs);
    }

    for item in &other {
        write_item(&mut out, item, no_provenance);
    }

    out.push_str("</context>\n");
    out
}

fn write_project_stack(out: &mut String, docs: &[&ResolvedItem]) {
    out.push_str("  <project-stack>\n");
    for r in docs {
        if let Source::Docs(d) = &r.item.source {
            let tier_str = format!("{:?}", d.tier).to_lowercase();
            let desc_attr = d
                .description
                .as_deref()
                .map(|x| format!(" description=\"{}\"", escape_attr(x)))
                .unwrap_or_default();
            out.push_str(&format!(
                "    <dep tier=\"{}\" ecosystem=\"{}\" name=\"{}\" version=\"{}\" url=\"{}\"{}/>\n",
                tier_str,
                d.ecosystem.as_str(),
                escape_attr(&d.name),
                escape_attr(&d.version),
                escape_attr(&d.url),
                desc_attr,
            ));
        }
    }
    out.push_str("  </project-stack>\n");
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

fn write_item(out: &mut String, r: &ResolvedItem, no_provenance: bool) {
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
        // Docs items render in <project-stack>; this branch is unreachable
        // when called via `render` but kept for exhaustiveness.
        Source::Docs(_) => return,
    };

    out.push_str("  <");
    out.push_str(tag_name);
    out.push_str(&format!(
        " path=\"{}\" language=\"{}\"",
        escape_attr(&path_str),
        r.language
    ));

    if !no_provenance {
        let p = &r.provenance;
        out.push_str(&format!(" uri=\"{}\"", escape_attr(&p.uri)));
        if let Some(ts) = p.fetched_at_str() {
            out.push_str(&format!(" fetched=\"{}\"", ts));
        }
        if !p.sha256.is_empty() {
            out.push_str(&format!(
                " sha256=\"{}\"",
                &p.sha256[..p.sha256.len().min(16)]
            ));
        }
        if let Some(etag) = &p.etag {
            out.push_str(&format!(" etag=\"{}\"", escape_attr(etag)));
        }
        if p.stale {
            out.push_str(" stale=\"true\"");
        }
        if p.failed {
            let reason = p.reason.as_deref().unwrap_or("unknown");
            out.push_str(&format!(
                " failed=\"true\" reason=\"{}\"",
                escape_attr(reason)
            ));
        }
    }

    match &r.item.source {
        Source::File(_) | Source::Url(_) | Source::Docs(_) => {}
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
        let out = render(&[], &[], true);
        assert!(out.starts_with("<context items=\"0\">"));
        assert!(out.trim_end().ends_with("</context>"));
    }

    #[test]
    fn single_source_file_uses_source_tag() {
        let out = render(
            &[sample("src/main.rs", "fn main() {}\n", "rust")],
            &[],
            true,
        );
        assert!(out.contains("<source path=\"src/main.rs\" language=\"rust\""));
        assert!(out.contains("<![CDATA[fn main() {}\n]]></source>"));
    }

    #[test]
    fn markdown_file_uses_documentation_tag() {
        let out = render(&[sample("README.md", "# Project\n", "markdown")], &[], true);
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
        let out = render(&[resolved], &[], true);
        assert!(out.contains("lines=\"45-120\""));
    }

    #[test]
    fn memory_section_renders_inside_context() {
        let notes = vec![
            Note::new("JWT in header", Some("auth".into())),
            Note::new("general note", None),
        ];
        let out = render(&[], &notes, true);
        assert!(out.contains("<memory count=\"2\">"));
        assert!(out.contains("tag=\"auth\""));
        assert!(out.contains("JWT in header"));
        assert!(out.contains("general note"));
        assert!(out.contains("</memory>"));
    }

    #[test]
    fn path_with_special_chars_is_attribute_escaped() {
        let out = render(&[sample("src/a&b.rs", "", "rust")], &[], true);
        assert!(out.contains("path=\"src/a&amp;b.rs\""));
    }

    #[test]
    fn cdata_split_handles_content_with_closing_cdata() {
        let out = render(&[sample("a.rs", "let s = \"]]>\";\n", "rust")], &[], true);
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
            true,
        );
        assert!(out.contains("<context items=\"3\">"));
    }

    #[test]
    fn memory_note_body_is_text_escaped() {
        let notes = vec![Note::new("A < B && C > D", None)];
        let out = render(&[], &notes, true);
        assert!(out.contains("A &lt; B &amp;&amp; C &gt; D"));
    }
}
