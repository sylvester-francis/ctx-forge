//! `ctxforge save <name>` — snapshot the current bundle as a named profile.

use crate::bundle::Bundle;
use crate::error::Result;
use crate::paths::CtxforgeRoot;
use crate::profile;

pub fn run(root: &CtxforgeRoot, name: &str) -> Result<()> {
    let bundle = Bundle::load_or_default(root)?;
    profile::save(root, name, &bundle)?;
    println!("saved profile `{name}` ({} items)", bundle.len());
    Ok(())
}
