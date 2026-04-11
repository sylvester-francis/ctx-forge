//! `ctxforge rm` — remove an item by 1-based index or by path.

use crate::bundle::Bundle;
use crate::error::Result;
use crate::paths::CtxforgeRoot;

pub fn run(root: &CtxforgeRoot, target: String) -> Result<()> {
    let mut bundle = Bundle::load_or_default(root)?;

    // Try index first.
    if let Ok(idx) = target.parse::<usize>() {
        let removed = bundle.remove_by_index(idx)?;
        bundle.save(root)?;
        crate::output::success(&format!("removed #{idx} {}", removed.display()));
        return Ok(());
    }

    // Otherwise path-based removal.
    let count = bundle.remove_by_path(std::path::Path::new(&target));
    if count == 0 {
        return Err(format!("no items matching `{target}`").into());
    }
    bundle.save(root)?;
    crate::output::success(&format!("removed {count} item(s) matching `{target}`"));
    Ok(())
}
