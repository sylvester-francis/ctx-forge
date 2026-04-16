//! iocraft-based TUI (v2).
//!
//! Selectable via `--tui-v2` CLI flag or `CTXFORGE_TUI=v2` env var.
//! v1 (`crate::tui`) continues to ship as default; this module is additive.

pub mod app;
pub mod command_registry;
pub mod components;
pub mod mode;
pub mod motion;
pub mod overlays;
pub mod theme;
pub mod viewer;

use crate::error::Result;
use crate::paths::CtxforgeRoot;

pub fn run(root: CtxforgeRoot) -> Result<()> {
    smol::block_on(app::run(root))
}
