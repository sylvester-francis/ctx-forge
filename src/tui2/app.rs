//! Root component and entry point for the v2 TUI.

use crate::error::Result;
use crate::paths::CtxforgeRoot;
use iocraft::prelude::*;

pub async fn run(_root: CtxforgeRoot) -> Result<()> {
    element!(App).render_loop().fullscreen().await?;
    Ok(())
}

#[component]
fn App(hooks: &mut Hooks) -> impl Into<AnyElement<'static>> {
    let _ = hooks;
    element! {
        View(
            flex_direction: FlexDirection::Column,
            width: 100pct,
            height: 100pct,
        ) {
            Text(content: "ctxforge v2 — stub layout. Press Ctrl-C to quit.")
        }
    }
}
