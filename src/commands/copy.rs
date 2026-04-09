//! `ctxforge copy` — render the bundle and place it on the system clipboard.

use crate::bundle::Bundle;
use crate::error::Result;
use crate::format::{self, Format};
use crate::memory;
use crate::paths::CtxforgeRoot;
use crate::resolve;

pub fn run(
    root: &CtxforgeRoot,
    no_memory: bool,
    memory_tag: Option<String>,
    memory_limit: usize,
) -> Result<()> {
    let bundle = Bundle::load_or_default(root)?;
    let resolved = resolve::resolve_all(&bundle.items, root.project_root())?;
    let memory_notes =
        memory::collect_for_attach(root, no_memory, memory_tag.as_deref(), memory_limit)?;

    let rendered = format::render(Format::Markdown, &resolved, &memory_notes);

    crate::clipboard::set(&rendered)?;

    let chars = rendered.chars().count();
    let note_count = memory_notes.len();
    if note_count > 0 {
        println!(
            "copied {} items + {} memory note(s) ({} chars) to clipboard",
            bundle.len(),
            note_count,
            chars
        );
    } else {
        println!(
            "copied {} items ({} chars) to clipboard",
            bundle.len(),
            chars
        );
    }
    Ok(())
}
