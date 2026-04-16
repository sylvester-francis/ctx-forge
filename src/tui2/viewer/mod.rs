//! Viewer state — keeps the highlighted file cache, scroll position, and errors.

pub mod highlight;
pub mod load;

pub use highlight::Highlighter;
pub use load::{read_and_highlight, ViewerError, ViewerLoad};

use iocraft::components::MixedTextContent;
use std::path::{Path, PathBuf};

pub struct ViewerState {
    pub enabled: bool,
    pub scroll: usize,
    pub cached_path: Option<PathBuf>,
    pub lines: Vec<Vec<MixedTextContent>>,
    pub error: Option<ViewerError>,
    pub truncated: bool,
    pub highlighter: Highlighter,
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

    pub fn load_for_path(&mut self, path: &Path) {
        if self.cached_path.as_deref() == Some(path) {
            return;
        }
        let load = read_and_highlight(path, &self.highlighter);
        self.lines = load.lines;
        self.truncated = load.truncated;
        self.error = load.error;
        self.cached_path = Some(path.to_path_buf());
        self.scroll = 0;
    }

    pub fn scroll_by(&mut self, delta: i32, viewport_height: usize) {
        let max = self.lines.len().saturating_sub(viewport_height.max(1));
        let new = (self.scroll as i32 + delta).max(0) as usize;
        self.scroll = new.min(max);
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll = 0;
    }

    pub fn scroll_to_bottom(&mut self, viewport_height: usize) {
        self.scroll = self.lines.len().saturating_sub(viewport_height.max(1));
    }
}

impl Default for ViewerState {
    fn default() -> Self {
        Self::new()
    }
}
