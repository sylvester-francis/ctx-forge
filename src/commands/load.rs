//! `ctxforge load <name>` — replace the current bundle with a saved profile.

use crate::error::Result;
use crate::paths::CtxforgeRoot;
use crate::profile;

pub fn run(root: &CtxforgeRoot, name: &str) -> Result<()> {
    let bundle = profile::load(root, name)?;
    bundle.save(root)?;
    println!("loaded profile `{name}` ({} items)", bundle.len());
    Ok(())
}
