//! `ctxforge clear` — remove all items from the current bundle.

use crate::bundle::Bundle;
use crate::error::Result;
use crate::paths::CtxforgeRoot;

pub fn run(root: &CtxforgeRoot) -> Result<()> {
    let mut bundle = Bundle::load_or_default(root)?;
    let n = bundle.len();
    bundle.clear();
    bundle.save(root)?;
    crate::output::success(&format!("cleared {n} item(s)"));
    Ok(())
}
