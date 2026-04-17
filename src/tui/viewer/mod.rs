//! Viewer state — keeps the highlighted file cache, scroll position, and errors.

pub mod highlight;
pub mod load;

pub use highlight::Highlighter;
pub use load::{read_and_highlight, ViewerError, ViewerLoad};

use iocraft::components::MixedTextContent;
use std::path::PathBuf;

pub struct ViewerState {
    pub enabled: bool,
    pub scroll: usize,
    pub cached_path: Option<PathBuf>,
    pub lines: Vec<Vec<MixedTextContent>>,
    pub error: Option<ViewerError>,
    pub truncated: bool,
    pub highlighter: Highlighter,
    /// True while a background file load is in flight. Viewer shows a
    /// "loading…" hint during this window so the UI doesn't feel frozen.
    pub loading: bool,
    /// Monotonic counter incremented on each new load request. The bg task
    /// carries its generation; results with a stale generation are dropped
    /// when the render body drains the result slot. Prevents slow loads
    /// from clobbering faster subsequent loads.
    pub load_generation: u64,
    /// Active drag selection, 0-based inclusive line indices, normalized
    /// so `start <= end`. `None` = no selection.
    pub selection: Option<(usize, usize)>,
    /// Anchor line set on MouseDown, cleared on MouseUp. Drives selection
    /// range extension during drag.
    pub drag_anchor: Option<usize>,
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
            loading: false,
            load_generation: 0,
            selection: None,
            drag_anchor: None,
        }
    }

    /// Begin a new load. Bumps the generation counter and returns the new
    /// value so the caller can tag the bg task; when the result comes back
    /// we compare against `load_generation` to ignore stale loads.
    pub fn begin_load(&mut self) -> u64 {
        self.load_generation = self.load_generation.wrapping_add(1);
        self.loading = true;
        self.load_generation
    }

    /// Apply a result from a background load. Called from the render body
    /// when the bg task completes. Ignores the result if its generation is
    /// stale (user moved on to another file).
    pub fn apply_bg_load(&mut self, generation: u64, path: PathBuf, load: ViewerLoad) {
        if generation != self.load_generation {
            return;
        }
        self.lines = load.lines;
        self.error = load.error;
        self.truncated = load.truncated;
        self.cached_path = Some(path);
        self.scroll = 0;
        self.selection = None;
        self.drag_anchor = None;
        self.loading = false;
    }

    pub fn drag_start(&mut self, line_idx: usize) {
        self.drag_anchor = Some(line_idx);
        self.selection = Some((line_idx, line_idx));
    }

    pub fn drag_extend(&mut self, line_idx: usize) {
        if let Some(anchor) = self.drag_anchor {
            let (start, end) = if anchor <= line_idx {
                (anchor, line_idx)
            } else {
                (line_idx, anchor)
            };
            self.selection = Some((start, end));
        }
    }

    pub fn drag_end(&mut self) {
        self.drag_anchor = None;
    }

    pub fn clear_selection(&mut self) {
        self.selection = None;
        self.drag_anchor = None;
    }

    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_load(n_lines: usize) -> ViewerLoad {
        ViewerLoad {
            lines: (0..n_lines)
                .map(|i| vec![MixedTextContent::new(format!("line {i}"))])
                .collect(),
            truncated: false,
            error: None,
        }
    }

    #[test]
    fn begin_load_increments_generation_and_sets_loading() {
        let mut v = ViewerState::new();
        assert_eq!(v.load_generation, 0);
        assert!(!v.loading);

        let g1 = v.begin_load();
        assert_eq!(g1, 1);
        assert_eq!(v.load_generation, 1);
        assert!(v.loading);

        let g2 = v.begin_load();
        assert_eq!(g2, 2);
        assert_eq!(v.load_generation, 2);
    }

    #[test]
    fn apply_bg_load_applies_fresh_generation() {
        let mut v = ViewerState::new();
        let generation = v.begin_load();

        v.apply_bg_load(generation, PathBuf::from("/x.rs"), fake_load(3));

        assert!(!v.loading);
        assert_eq!(v.lines.len(), 3);
        assert_eq!(v.cached_path.as_deref(), Some(std::path::Path::new("/x.rs")));
    }

    #[test]
    fn apply_bg_load_drops_stale_generation() {
        let mut v = ViewerState::new();
        let old_gen = v.begin_load();
        // User navigated away — a new load starts, bumping generation.
        let _new_gen = v.begin_load();

        // Late-arriving result from the OLD load — must be ignored.
        v.apply_bg_load(old_gen, PathBuf::from("/old.rs"), fake_load(3));

        // loading is still true (the new load hasn't finished yet), and
        // the stale content did NOT land in lines.
        assert!(v.loading);
        assert!(v.lines.is_empty());
        assert!(v.cached_path.is_none());
    }

    #[test]
    fn apply_bg_load_clears_selection_and_resets_scroll() {
        let mut v = ViewerState::new();
        v.scroll = 42;
        v.drag_start(5);
        v.drag_extend(9);
        let generation = v.begin_load();

        v.apply_bg_load(generation, PathBuf::from("/x.rs"), fake_load(20));

        assert_eq!(v.scroll, 0);
        assert!(v.selection.is_none());
        assert!(v.drag_anchor.is_none());
    }
}
