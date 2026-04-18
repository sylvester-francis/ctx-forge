//! Markdown export. Each item becomes a fenced code block with a filename
//! heading and the detected language for syntax highlighting.
//!
//! When memory notes are provided, a `## Memory` section is emitted at
//! the top of the output, before any items.

use crate::memory::Note;
use crate::resolve::ResolvedItem;
use crate::source::Source;

pub fn render(items: &[ResolvedItem], memory: &[Note], no_provenance: bool) -> String {
    let mut out = String::new();

    if !memory.is_empty() {
        write_memory(&mut out, memory);
    }

    let (docs, other) = crate::format::partition_docs(items);
    if !docs.is_empty() {
        write_project_stack(&mut out, &docs, items);
    }

    for (i, item) in other.iter().enumerate() {
        if i > 0 || !memory.is_empty() || !docs.is_empty() {
            out.push('\n');
        }
        write_item(&mut out, item, no_provenance);
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

fn write_item(out: &mut String, r: &ResolvedItem, no_provenance: bool) {
    if !no_provenance {
        write_provenance_comment(out, r);
    }
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
        Source::Docs(_) => {
            // Docs items are rendered in the Project stack section, not here.
            // This branch is unreachable when called via `render` (which
            // partitions them out), but kept for exhaustiveness.
            return;
        }
        Source::Gh(g) => {
            out.push_str(&format!("## `{}`\n\n", g.resource.browser_url()));
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

fn write_project_stack(out: &mut String, docs: &[&ResolvedItem], all_items: &[ResolvedItem]) {
    use crate::source::{DocsSource, DocsTier};
    use std::collections::BTreeMap;
    use std::collections::HashSet;

    // Compute attention weight per manifest dir.
    fn weight_for(dir: &std::path::Path, items: &[ResolvedItem]) -> usize {
        items
            .iter()
            .filter(|r| !matches!(r.item.source, Source::Docs(_)))
            .filter_map(|r| r.item.source.display_path())
            .filter(|p| p.starts_with(dir))
            .count()
    }

    let manifest_dirs: HashSet<(String, std::path::PathBuf)> = docs
        .iter()
        .filter_map(|r| match &r.item.source {
            Source::Docs(d) => d.manifest_path.as_ref().map(|p| {
                let parent = p.parent().map(|x| x.to_path_buf()).unwrap_or_default();
                (p.display().to_string(), parent)
            }),
            _ => None,
        })
        .collect();

    let mut by_manifest: BTreeMap<String, Vec<&DocsSource>> = BTreeMap::new();
    let mut no_manifest: Vec<&DocsSource> = Vec::new();
    for r in docs {
        if let Source::Docs(d) = &r.item.source {
            match d.manifest_path.as_ref() {
                Some(p) => by_manifest
                    .entry(p.display().to_string())
                    .or_default()
                    .push(d),
                None => no_manifest.push(d),
            }
        }
    }

    let mut ordered_manifests: Vec<String> = by_manifest.keys().cloned().collect();
    ordered_manifests.sort_by(|a, b| {
        let wa = manifest_dirs
            .iter()
            .find(|(m, _)| m == a)
            .map(|(_, dir)| weight_for(dir, all_items))
            .unwrap_or(0);
        let wb = manifest_dirs
            .iter()
            .find(|(m, _)| m == b)
            .map(|(_, dir)| weight_for(dir, all_items))
            .unwrap_or(0);
        wb.cmp(&wa).then_with(|| a.cmp(b))
    });

    out.push_str("## Project stack\n\n");
    for manifest in ordered_manifests {
        let deps = &by_manifest[&manifest];
        let eco = deps.first().unwrap().ecosystem;
        out.push_str(&format!("### `{}` ({})\n\n", manifest, eco.display_name(),));
        let mut tiered: Vec<&&DocsSource> = deps
            .iter()
            .filter(|d| d.tier != DocsTier::Library)
            .collect();
        tiered.sort_by_key(|d| d.tier_order());
        for d in &tiered {
            out.push_str(&d.render_line());
            out.push('\n');
        }
        let library_count = deps.iter().filter(|d| d.tier == DocsTier::Library).count();
        if library_count > 0 {
            out.push_str(&format!(
                "\n_{library_count} other direct dep(s) — `ctxforge docs add <name>` to include._\n"
            ));
        }
        out.push('\n');
    }

    if !no_manifest.is_empty() {
        out.push_str("### Added manually\n\n");
        for d in &no_manifest {
            out.push_str(&d.render_line());
            out.push('\n');
        }
        out.push('\n');
    }
}

fn write_provenance_comment(out: &mut String, r: &ResolvedItem) {
    let p = &r.provenance;
    if p.failed {
        let reason = p.reason.as_deref().unwrap_or("unknown");
        out.push_str(&format!(
            "<!-- ctxforge: FAILED {}\n     reason: {reason}\n     last_attempt: {} -->\n",
            p.uri,
            p.fetched_at_str().unwrap_or_else(|| "unknown".into()),
        ));
        return;
    }
    let mut line = format!(
        "<!-- ctxforge: {}, sha256:{}",
        p.uri,
        &p.sha256[..p.sha256.len().min(16)]
    );
    if let Some(ts) = p.fetched_at_str() {
        line.push_str(&format!(", fetched {ts}"));
    }
    if let Some(etag) = &p.etag {
        line.push_str(&format!(", etag:{etag}"));
    }
    if p.stale {
        line.push_str(", stale");
    }
    line.push_str(" -->\n");
    out.push_str(&line);
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
        let r = render(
            &[sample_file("src/main.rs", "fn main() {}\n", "rust")],
            &[],
            true,
        );
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
            true,
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
            true,
        );
        assert!(r.contains("(lines 5-10)"));
    }

    #[test]
    fn memory_section_rendered_when_notes_present() {
        let notes = vec![Note::new("JWT in header", Some("auth".into()))];
        let r = render(&[sample_file("a.rs", "", "rust")], &notes, true);
        assert!(r.contains("## Memory"));
        assert!(r.contains("[auth]"));
        assert!(r.contains("JWT in header"));
        let mem_idx = r.find("## Memory").unwrap();
        let item_idx = r.find("## `a.rs`").unwrap();
        assert!(mem_idx < item_idx);
    }

    #[test]
    fn memory_section_omitted_when_empty() {
        let r = render(&[sample_file("a.rs", "", "rust")], &[], true);
        assert!(!r.contains("## Memory"));
    }

    #[test]
    fn provenance_header_emitted_by_default() {
        let r = render(&[sample_file("a.rs", "fn a() {}\n", "rust")], &[], false);
        assert!(r.contains("<!-- ctxforge:"));
        assert!(r.contains("file://a.rs"));
    }

    #[test]
    fn no_provenance_flag_strips_header() {
        let r = render(&[sample_file("a.rs", "fn a() {}\n", "rust")], &[], true);
        assert!(!r.contains("<!-- ctxforge:"));
    }

    #[test]
    fn project_stack_renders_subsection_per_manifest() {
        use crate::source::{DocsSource, DocsTier, Ecosystem};
        let item = Item {
            source: Source::Docs(DocsSource {
                name: "axum".into(),
                version: "0.7.5".into(),
                ecosystem: Ecosystem::Rust,
                tier: DocsTier::Framework,
                url: "https://docs.rs/axum/0.7.5/".into(),
                description: Some("Ergonomic web framework".into()),
                manifest_path: Some("Cargo.toml".into()),
                forge: None,
            }),
            label: None,
        };
        let resolved = ResolvedItem {
            provenance: crate::source::Provenance::local(
                item.source.to_uri().to_string(),
                String::new(),
            ),
            item,
            content: String::new(),
            language: "markdown",
        };
        let out = render(&[resolved], &[], true);
        assert!(out.contains("## Project stack"));
        assert!(out.contains("### `Cargo.toml` (Rust)"));
        assert!(out.contains("axum 0.7.5"));
        assert!(out.contains("https://docs.rs/axum/0.7.5/"));
    }

    #[test]
    fn subsections_ordered_by_attention_weight() {
        use crate::source::{DocsSource, DocsTier, Ecosystem, FileSource};

        fn docs_item(manifest: &str, name: &str, tier: DocsTier, eco: Ecosystem) -> ResolvedItem {
            let item = Item {
                source: Source::Docs(DocsSource {
                    name: name.into(),
                    version: "1.0".into(),
                    ecosystem: eco,
                    tier,
                    url: "https://example/".into(),
                    description: None,
                    manifest_path: Some(manifest.into()),
                    forge: None,
                }),
                label: None,
            };
            ResolvedItem {
                provenance: crate::source::Provenance::local(
                    item.source.to_uri().to_string(),
                    String::new(),
                ),
                item,
                content: String::new(),
                language: "markdown",
            }
        }

        fn file_item(path: &str) -> ResolvedItem {
            let item = Item {
                source: Source::File(FileSource { path: path.into() }),
                label: None,
            };
            ResolvedItem {
                provenance: crate::source::Provenance::local(
                    item.source.to_uri().to_string(),
                    String::new(),
                ),
                item,
                content: "x\n".into(),
                language: "text",
            }
        }

        // 3 files under services/api, 1 file under apps/web.
        // Expect services/api subsection before apps/web.
        let items = vec![
            docs_item(
                "services/api/go.mod",
                "gin",
                DocsTier::Framework,
                Ecosystem::Go,
            ),
            docs_item(
                "apps/web/package.json",
                "next",
                DocsTier::Framework,
                Ecosystem::Js,
            ),
            file_item("services/api/main.go"),
            file_item("services/api/handler.go"),
            file_item("services/api/db.go"),
            file_item("apps/web/src/App.tsx"),
        ];

        let rendered = render(&items, &[], true);
        let api_idx = rendered.find("services/api").unwrap();
        let web_idx = rendered.find("apps/web").unwrap();
        assert!(api_idx < web_idx);
    }
}
