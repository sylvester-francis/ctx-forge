//! Markdown export. Each item becomes a fenced code block with a filename
//! heading and the detected language for syntax highlighting.
//!
//! When memory notes are provided, a `## Memory` section is emitted at
//! the top of the output, before any items.

use crate::memory::Note;
use crate::resolve::ResolvedItem;
use crate::source::Source;

pub fn render(items: &[ResolvedItem], memory: &[Note]) -> String {
    let mut out = String::new();

    if !memory.is_empty() {
        write_memory(&mut out, memory);
    }

    for (i, item) in items.iter().enumerate() {
        if i > 0 || !memory.is_empty() {
            out.push('\n');
        }
        write_item(&mut out, item);
    }
    out
}

fn write_memory(out: &mut String, memory: &[Note]) {
    out.push_str("## Memory\n\n");
    for note in memory {
        let ts = note.timestamp.format("%Y-%m-%d");
        match &note.tag {
            Some(t) => out.push_str(&format!("> [{ts}] [{t}] {}\n", note.body)),
            None => out.push_str(&format!("> [{ts}] {}\n", note.body)),
        }
    }
    out.push_str("\n---\n");
}

fn write_item(out: &mut String, r: &ResolvedItem) {
    match &r.item.source {
        Source::File(f) => {
            out.push_str(&format!("## `{}`\n\n", f.path.display()));
        }
        Source::Range(range) => {
            out.push_str(&format!(
                "## `{}` (lines {}-{})\n\n",
                range.path.display(),
                range.start,
                range.end
            ));
        }
        Source::Func(func) => {
            out.push_str(&format!(
                "## `{}` — fn `{}`\n\n",
                func.path.display(),
                func.name
            ));
        }
        Source::Type(t) => {
            out.push_str(&format!(
                "## `{}` — type `{}`\n\n",
                t.path.display(),
                t.name
            ));
        }
        Source::Url(u) => {
            out.push_str(&format!("## `{}`\n\n", u.url));
        }
    }

    out.push_str("```");
    out.push_str(r.language);
    out.push('\n');
    out.push_str(&r.content);
    if !r.content.ends_with('\n') {
        out.push('\n');
    }
    out.push_str("```\n");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::Item;
    use crate::memory::Note;
    use crate::source::{FileSource, RangeSource};
    use std::path::PathBuf;

    fn sample_file(path: &str, content: &str, lang: &'static str) -> ResolvedItem {
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
    fn single_file_renders_with_heading_and_fence() {
        let r = render(&[sample_file("src/main.rs", "fn main() {}\n", "rust")], &[]);
        assert!(r.contains("## `src/main.rs`"));
        assert!(r.contains("```rust"));
        assert!(r.contains("fn main() {}"));
        assert!(r.contains("```\n"));
    }

    #[test]
    fn multiple_files_are_separated_by_blank_line() {
        let r = render(
            &[
                sample_file("a.rs", "one\n", "rust"),
                sample_file("b.rs", "two\n", "rust"),
            ],
            &[],
        );
        let first = r.find("## `a.rs`").unwrap();
        let second = r.find("## `b.rs`").unwrap();
        assert!(first < second);
    }

    #[test]
    fn range_item_shows_line_numbers_in_heading() {
        let item = Item {
            source: Source::Range(RangeSource::new("a.rs".into(), 5, 10).unwrap()),
            label: None,
        };
        let r = render(
            &[ResolvedItem {
                provenance: crate::source::Provenance::local(
                    item.source.to_uri().to_string(),
                    String::new(),
                ),
                item,
                content: "slice\n".into(),
                language: "rust",
            }],
            &[],
        );
        assert!(r.contains("(lines 5-10)"));
    }

    #[test]
    fn memory_section_rendered_when_notes_present() {
        let notes = vec![Note::new("JWT in header", Some("auth".into()))];
        let r = render(&[sample_file("a.rs", "", "rust")], &notes);
        assert!(r.contains("## Memory"));
        assert!(r.contains("[auth]"));
        assert!(r.contains("JWT in header"));
        let mem_idx = r.find("## Memory").unwrap();
        let item_idx = r.find("## `a.rs`").unwrap();
        assert!(mem_idx < item_idx);
    }

    #[test]
    fn memory_section_omitted_when_empty() {
        let r = render(&[sample_file("a.rs", "", "rust")], &[]);
        assert!(!r.contains("## Memory"));
    }
}
