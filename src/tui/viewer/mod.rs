//! File preview pane with syntect syntax highlighting.
//!
//! `ViewerState` owns the toggle flag, scroll position, cached highlighted
//! lines, and a `Highlighter`. `App` holds a single `ViewerState`; the UI
//! reads from it during render.
//!
//! See `docs/superpowers/specs/2026-04-14-code-viewer-design.md`.

pub mod highlight;
pub mod load;

pub use load::{ViewerError, ViewerLoad};
