//! Root component and entry point for the v2 TUI.
//!
//! Loads the bundle, tree, and theme at startup, then hands off to the
//! reactive `App` component. Keyboard focus, navigation, and mutations
//! happen inside `App`; this module is the bootstrapping layer.

use crate::error::Result;
use crate::paths::{config_file_path, CtxforgeRoot};
use crate::tui::theme::{config::resolve_theme_name, registry, AppTheme};
use crate::tui2::theme::Theme;
use iocraft::prelude::*;

/// Resolve the active theme from env var / config / default, returning the
/// v2 `Theme` bridge struct.
fn load_theme() -> Theme {
    let name = config_file_path()
        .map(|p| resolve_theme_name(&p))
        .unwrap_or_else(|| "ctxforge".to_string());
    let raw: &'static AppTheme =
        registry::by_name(&name).unwrap_or_else(|| registry::default_theme());
    Theme::from_app_theme(raw)
}

pub async fn run(_root: CtxforgeRoot) -> Result<()> {
    let _theme = load_theme();
    // Theme wiring through the component tree lands in Task 8 when the real
    // App layout is assembled. Task 5 just proves the bridge compiles and
    // produces a valid Theme at startup.
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
            Text(content: "ctxforge v2 — scaffold. Full layout lands in Task 8. Ctrl-C quits.")
        }
    }
}
