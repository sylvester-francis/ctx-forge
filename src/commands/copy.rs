//! `ctxforge copy` — render the bundle and place it on the system clipboard.

use crate::bundle::Bundle;
use crate::error::Result;
use crate::format::{self, Format};
use crate::paths::CtxforgeRoot;
use crate::resolve;

pub fn run(root: &CtxforgeRoot) -> Result<()> {
    let bundle = Bundle::load_or_default(root)?;
    let resolved = resolve::resolve_all(&bundle.items, root.project_root())?;
    let rendered = format::render(Format::Markdown, &resolved);

    crate::clipboard::set(&rendered)?;

    let chars = rendered.chars().count();
    println!(
        "copied {} items ({} chars) to clipboard",
        bundle.len(),
        chars
    );
    Ok(())
}
