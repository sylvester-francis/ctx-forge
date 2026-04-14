//! File preview pane with syntect syntax highlighting.
//!
//! `ViewerState` owns the toggle flag, scroll position, cached highlighted
//! lines, and a `Highlighter`. `App` holds a single `ViewerState`; the UI
//! reads from it during render.
//!
//! See `docs/superpowers/specs/2026-04-14-code-viewer-design.md`.

pub mod highlight;
pub mod load;

pub use highlight::Highlighter;
pub use load::{ViewerError, ViewerLoad, read_and_highlight};

use ratatui::text::Line;
use std::path::{Path, PathBuf};

pub struct ViewerState {
    pub enabled: bool,
    pub scroll: usize,
    pub cached_path: Option<PathBuf>,
    lines: Vec<Line<'static>>,
    error: Option<ViewerError>,
    truncated: bool,
    highlighter: Highlighter,
}

impl ViewerState {
    pub fn new() -> Self {
        Self {
            enabled: false,
            scroll: 0,
            cached_path: None,
            lines: Vec::new(),
            error: None,
            truncated: false,
            highlighter: Highlighter::new(),
        }
    }

    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }

    pub fn lines(&self) -> &[Line<'static>] {
        &self.lines
    }

    pub fn error(&self) -> Option<&ViewerError> {
        self.error.as_ref()
    }

    pub fn truncated(&self) -> bool {
        self.truncated
    }

    /// Load and highlight the given file into the cache. Idempotent when
    /// called with the same path — preserves scroll position. Resets
    /// scroll when called with a different path.
    pub fn load_for_path(&mut self, path: &Path) {
        if self
            .cached_path
            .as_deref()
            .map(|p| p == path)
            .unwrap_or(false)
        {
            return;
        }
        let load = read_and_highlight(path, &self.highlighter);
        self.lines = load.lines;
        self.truncated = load.truncated;
        self.error = load.error;
        self.cached_path = Some(path.to_path_buf());
        self.scroll = 0;
    }

    /// Clear the cache — used when no file is under the cursor.
    pub fn clear(&mut self) {
        self.lines.clear();
        self.error = None;
        self.truncated = false;
        self.cached_path = None;
        self.scroll = 0;
    }

    /// Advance or rewind `scroll` by `delta`, clamped to `[0, max_scroll]`.
    pub fn scroll_by(&mut self, delta: i32, viewport_height: usize) {
        let max_scroll = self.lines.len().saturating_sub(viewport_height);
        let new = (self.scroll as i32 + delta).clamp(0, max_scroll as i32);
        self.scroll = new as usize;
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll = 0;
    }

    pub fn scroll_to_bottom(&mut self, viewport_height: usize) {
        self.scroll = self.lines.len().saturating_sub(viewport_height);
    }
}

impl Default for ViewerState {
    fn default() -> Self {
        Self::new()
    }
}
