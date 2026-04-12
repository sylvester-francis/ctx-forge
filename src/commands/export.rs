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

    let rendered = format::render(format, &resolved, &memory_notes);

    let final_content = match template_name {
        Some(name) => crate::template::apply_template(root, name, &rendered, task.as_deref())?,
        None => rendered,
    };

    match output {
        Some(path) => {
            std::fs::write(&path, &final_content)?;
            eprintln!(
                "wrote {} bytes ({}) to {}",
                final_content.len(),
                format.name(),
                path.display()
            );
        }
        None => {
            let stdout = std::io::stdout();
            let mut lock = stdout.lock();
            lock.write_all(final_content.as_bytes())?;
        }
    }
    Ok(())
}
