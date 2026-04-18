//! `ctxforge copy` — render the bundle and place it on the system clipboard.

use crate::bundle::Bundle;
use crate::error::Result;
use crate::format::{self, Format};
use crate::memory;
use crate::paths::CtxforgeRoot;
use crate::resolve;

pub fn run(
    root: &CtxforgeRoot,
    format: Format,
    no_memory: bool,
    memory_tag: Option<String>,
    memory_limit: usize,
    template_name: Option<&str>,
    task: Option<String>,
) -> Result<()> {
    let bundle = Bundle::load_or_default(root)?;
    let resolved = resolve::resolve_all(&bundle.items, root.project_root())?;
    let memory_notes =
        memory::collect_for_attach(root, no_memory, memory_tag.as_deref(), memory_limit)?;

    // Provenance is suppressed by default — the clipboard usually feeds an
    // LLM prompt directly, and audit-trail metadata wastes tokens there.
    let rendered = format::render(format, &resolved, &memory_notes, true);

    let final_content = match template_name {
        Some(name) => crate::template::apply_template(root, name, &rendered, task.as_deref())?,
        None => rendered,
    };

    crate::clipboard::set(&final_content)?;

    let chars = final_content.chars().count();
    let note_count = memory_notes.len();
    let tmpl_info = template_name
        .map(|n| format!(" (template: {n})"))
        .unwrap_or_default();
    if note_count > 0 {
        println!(
            "copied {} items + {} memory note(s) ({}, {} chars){tmpl_info} to clipboard",
            bundle.len(),
            note_count,
            format.name(),
            chars
        );
    } else {
        println!(
            "copied {} items ({}, {} chars){tmpl_info} to clipboard",
            bundle.len(),
            format.name(),
            chars
        );
    }
    Ok(())
}
