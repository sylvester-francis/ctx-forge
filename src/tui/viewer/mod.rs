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
    /// Active mouse-drag selection, stored as 0-based line indices. Always
    /// normalized so `start <= end`. `None` means no selection.
    selection: Option<(usize, usize)>,
    /// Anchor line for an in-progress drag. `Some(n)` between MouseDown and
    /// MouseUp; used to compute the selection range as the mouse moves.
    drag_anchor: Option<usize>,
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
            selection: None,
            drag_anchor: None,
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
    /// scroll + selection when called with a different path.
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
        self.selection = None;
        self.drag_anchor = None;
    }

    /// Clear the cache — used when no file is under the cursor.
    pub fn clear(&mut self) {
        self.lines.clear();
        self.error = None;
        self.truncated = false;
        self.cached_path = None;
        self.scroll = 0;
        self.selection = None;
        self.drag_anchor = None;
    }

    /// Current selection as `(start_line, end_line)`, 0-based inclusive.
    pub fn selection(&self) -> Option<(usize, usize)> {
        self.selection
    }

    /// Begin a new drag selection anchored at `line` (0-based index into
    /// `lines`). Clamps to the valid range.
    pub fn begin_selection(&mut self, line: usize) {
        if self.lines.is_empty() {
            return;
        }
        let line = line.min(self.lines.len() - 1);
        self.drag_anchor = Some(line);
        self.selection = Some((line, line));
    }

    /// Extend the active drag selection to `line`. No-op if no drag is in
    /// progress.
    pub fn extend_selection(&mut self, line: usize) {
        let Some(anchor) = self.drag_anchor else {
            return;
        };
        if self.lines.is_empty() {
            return;
        }
        let line = line.min(self.lines.len() - 1);
        let (a, b) = if anchor <= line {
            (anchor, line)
        } else {
            (line, anchor)
        };
        self.selection = Some((a, b));
    }

    /// End the active drag. The selection itself persists until the user
    /// commits it or navigates away.
    pub fn end_drag(&mut self) {
        self.drag_anchor = None;
    }

    /// Clear any selection (used by Esc and after adding to bundle).
    pub fn clear_selection(&mut self) {
        self.selection = None;
        self.drag_anchor = None;
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
