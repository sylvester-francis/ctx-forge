//! `ctxforge export` — render the current bundle and write to stdout or file.

use crate::bundle::Bundle;
use crate::error::Result;
use crate::format::{self, Format};
use crate::memory;
use crate::paths::CtxforgeRoot;
use crate::resolve;
use std::io::Write;
use std::path::PathBuf;

pub fn run(
    root: &CtxforgeRoot,
    output: Option<PathBuf>,
    no_memory: bool,
    memory_tag: Option<String>,
    memory_limit: usize,
) -> Result<()> {
    let bundle = Bundle::load_or_default(root)?;
    let resolved = resolve::resolve_all(&bundle.items, root.project_root())?;
    let memory_notes =
        memory::collect_for_attach(root, no_memory, memory_tag.as_deref(), memory_limit)?;

    let rendered = format::render(Format::Markdown, &resolved, &memory_notes);

    match output {
        Some(path) => {
            std::fs::write(&path, &rendered)?;
            eprintln!("wrote {} bytes to {}", rendered.len(), path.display());
        }
        None => {
            let stdout = std::io::stdout();
            let mut lock = stdout.lock();
            lock.write_all(rendered.as_bytes())?;
        }
    }
    Ok(())
}
