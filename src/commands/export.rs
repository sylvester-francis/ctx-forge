//! `ctxforge export` — render the current bundle and write to stdout or file.

use crate::bundle::Bundle;
use crate::error::{CtxforgeError, Result};
use crate::format::{self, Format};
use crate::memory;
use crate::paths::CtxforgeRoot;
use std::io::Write;
use std::path::PathBuf;

#[allow(clippy::too_many_arguments)]
pub fn run(
    root: &CtxforgeRoot,
    output: Option<PathBuf>,
    format: Format,
    no_memory: bool,
    memory_tag: Option<String>,
    memory_limit: usize,
    template_name: Option<&str>,
    task: Option<String>,
    strict: bool,
    offline: bool,
    no_provenance: bool,
) -> Result<()> {
    let bundle = Bundle::load_or_default(root)?;

    let cache = if bundle.items.iter().any(|i| i.source.is_cacheable()) {
        let cache_dir = crate::paths::global_cache_dir().ok_or_else(|| {
            CtxforgeError::Msg(
                "cannot determine cache directory (no HOME / XDG_CACHE_HOME)".into(),
            )
        })?;
        Some(crate::cache::ContentCache::open(cache_dir)?)
    } else {
        None
    };

    let mut ctx = crate::resolve::ResolveCtx::cli(root.project_root(), cache.as_ref());
    ctx.strict = strict;
    ctx.offline = offline;

    let resolved = crate::resolve::resolve_all_with_ctx(&bundle.items, &ctx)?;
    for w in ctx.drain_warnings() {
        eprintln!("ctxforge: {w}");
    }

    let memory_notes =
        memory::collect_for_attach(root, no_memory, memory_tag.as_deref(), memory_limit)?;

    let rendered = format::render(format, &resolved, &memory_notes, no_provenance);

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
