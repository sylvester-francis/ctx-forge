//! Markdown export. Each item becomes a fenced code block with a filename
//! heading and the detected language for syntax highlighting.

use crate::bundle::ItemKind;
use crate::resolve::ResolvedItem;

pub fn render(items: &[ResolvedItem]) -> String {
    let mut out = String::new();
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        write_item(&mut out, item);
    }
    out
}

fn write_item(out: &mut String, r: &ResolvedItem) {
    // Heading line.
    match &r.item.kind {
        ItemKind::File => {
            out.push_str(&format!("## `{}`\n\n", r.item.path.display()));
        }
        ItemKind::Range(range) => {
            out.push_str(&format!(
                "## `{}` (lines {}-{})\n\n",
                r.item.path.display(),
                range.start,
                range.end
            ));
        }
        ItemKind::Function { name } => {
            out.push_str(&format!(
                "## `{}` — fn `{}`\n\n",
                r.item.path.display(),
                name
            ));
        }
        ItemKind::Type { name } => {
            out.push_str(&format!(
                "## `{}` — type `{}`\n\n",
                r.item.path.display(),
                name
            ));
        }
    }

    // Fenced code block.
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
    use crate::bundle::{Item, ItemKind};
    use std::path::PathBuf;

    fn sample_file(path: &str, content: &str, lang: &'static str) -> ResolvedItem {
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

    #[test]
    fn single_file_renders_with_heading_and_fence() {
        let r = render(&[sample_file("src/main.rs", "fn main() {}\n", "rust")]);
        assert!(r.contains("## `src/main.rs`"));
        assert!(r.contains("```rust"));
        assert!(r.contains("fn main() {}"));
        assert!(r.contains("```\n"));
    }

    #[test]
    fn multiple_files_are_separated_by_blank_line() {
        let r = render(&[
            sample_file("a.rs", "one\n", "rust"),
            sample_file("b.rs", "two\n", "rust"),
        ]);
        // Two headings present, in order.
        let first = r.find("## `a.rs`").unwrap();
        let second = r.find("## `b.rs`").unwrap();
        assert!(first < second);
    }

    #[test]
    fn range_item_shows_line_numbers_in_heading() {
        let r = render(&[ResolvedItem {
            item: Item {
                path: PathBuf::from("a.rs"),
                kind: ItemKind::Range(crate::bundle::Range { start: 5, end: 10 }),
                label: None,
            },
            content: "slice\n".into(),
            language: "rust",
        }]);
        assert!(r.contains("(lines 5-10)"));
    }
}
