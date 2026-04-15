//! App state for the TUI.

use crate::bundle::{Bundle, Item, ItemKind, Range};
use crate::models;
use crate::paths::CtxforgeRoot;
use crate::resolve;
use crate::tokens;
use crate::tui::mode;
use crate::tui::motion::{AnimCtx, Clock, MotionLevel, SystemClock, constants};
use crate::tui::tree::{self, TreeEntry};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Which panel has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    FileTree,
    Viewer,
    BundleList,
}

/// In-flight mode transition — both the outgoing and incoming modes render
/// simultaneously during this window.
pub struct ModeTransition {
    pub prev: mode::Mode,
    pub started: Instant,
    pub duration: Duration,
}

/// Full application state.
pub struct App {
    pub root: CtxforgeRoot,
    pub project_root: PathBuf,
    pub bundle: Bundle,
    pub tree_entries: Vec<TreeEntry>,
    /// Indices into `tree_entries` for currently visible (non-collapsed) entries.
    pub visible_tree: Vec<usize>,
    pub tree_cursor: usize,
    pub bundle_cursor: usize,
    pub focus: Focus,
    pub model_name: String,
    pub model_window: usize,
    /// Per-item token counts, recalculated on every toggle.
    pub item_tokens: Vec<usize>,
    pub total_tokens: usize,
    pub exact_tokens: bool,
    /// Animated token gauge — smoothly tweens toward `total_tokens` on
    /// bundle mutations. UI reads via `token_gauge.current(now)`.
    pub token_gauge: crate::tui::motion::Gauge,
    /// Darkens the Normal content underneath any visible overlay.
    /// Fades in when an overlay opens, fades out when all overlays close.
    pub backdrop_dim: crate::tui::motion::Fade,
    /// Opacity of the status bar message. Fades in when `set_status` is
    /// called, holds for STATUS_HOLD, then fades out.
    pub status_fade: crate::tui::motion::Fade,
    /// Timestamp of the most recent `set_status` call. Used by
    /// `tick_status_fade` to schedule the fade-out.
    pub status_set_at: Option<Instant>,
    /// Persisted ratatui `ListState` so scroll offset survives across frames.
    /// Kept as `RefCell` because the render code only has `&App` and
    /// `StatefulWidget::render` needs `&mut ListState`.
    pub tree_list_state: std::cell::RefCell<ratatui::widgets::ListState>,
    pub bundle_list_state: std::cell::RefCell<ratatui::widgets::ListState>,
    /// Animated border highlight — tweens between FileTree and BundleList
    /// focus colors on Tab switch.
    pub focus_highlight: crate::tui::motion::Highlight,
    /// Fade-in applied to the entire TUI on first render.
    pub startup_fade: crate::tui::motion::Fade,
    /// Per-bundle-row fade animations keyed by path. Populated on add; each
    /// entry is removed once its Fade settles at 1.0.
    pub bundle_row_fades: std::collections::HashMap<PathBuf, crate::tui::motion::Fade>,
    /// File preview pane state (toggle flag + cached highlighted lines).
    pub viewer: crate::tui::viewer::ViewerState,
    /// Last-rendered body height of the viewer pane. Cell<usize> because
    /// `ui::draw` has only `&App` and needs to update this each frame so
    /// key handlers can clamp scroll without knowing layout dimensions.
    pub viewer_last_viewport_height: std::cell::Cell<usize>,
    /// Last-rendered rect of the viewer pane (including borders), in
    /// terminal-absolute coordinates. Used by mouse event handling to map
    /// click positions to line indices. `None` before the first frame.
    pub viewer_pane_rect: std::cell::Cell<Option<ratatui::layout::Rect>>,
    /// Set of relative paths currently in the bundle, for fast lookup.
    pub bundled_paths: HashSet<PathBuf>,
    /// Indices into `tree_entries` for fuzzy-search results, ranked by score.
    pub search_results: Vec<usize>,
    /// Currently loaded/saved profile name (shown in header).
    pub profile_name: Option<String>,
    /// Pending stdout output (for `x` export). The TUI run loop drains this
    /// each iteration: it restores the terminal, prints the content, waits
    /// for a keypress, and re-initializes the alternate screen.
    pub pending_stdout: Option<String>,
    /// Pending pipe (target, content). Drained by the run loop the same way
    /// as `pending_stdout` but spawns the target binary and writes to its stdin.
    pub pending_pipe: Option<(String, String)>,
    /// Active input/overlay mode. Drives key dispatch and overlay rendering.
    /// Private — mutate only through `set_mode` so transitions are tracked.
    mode: mode::Mode,
    /// In-flight mode transition. `None` when no cross-fade is animating.
    pub mode_transition: Option<ModeTransition>,
    /// Time source. `SystemClock` in production; `MockClock` in tests.
    pub clock: Box<dyn Clock>,
    /// Whether animations play or snap. Detected once at startup.
    pub motion: MotionLevel,
    pub should_quit: bool,
    pub status_message: String,
    pub show_help: bool,
    /// Last known height (in rows) of the file tree viewport, minus borders.
    /// Captured during render; used by PageUp/PageDown and half-page scrolls.
    /// `Cell<u16>` for interior mutability — `ui::draw` writes from `&App`.
    pub tree_viewport_height: std::cell::Cell<u16>,
    /// Last known height of the bundle list viewport, minus borders.
    pub bundle_viewport_height: std::cell::Cell<u16>,
    /// Active colour theme. Read from `~/.config/ctxforge/config.toml` at
    /// startup (Task 7); defaults to the built-in `ctxforge` palette.
    pub theme: &'static crate::tui::theme::AppTheme,
}

impl App {
    /// Read-only accessor for the current mode.
    pub fn mode(&self) -> &mode::Mode {
        &self.mode
    }

    /// Mutable accessor for in-place variant-field mutation only. DO NOT use
    /// this to switch modes — call `set_mode` so transitions stay tracked.
    pub fn mode_mut(&mut self) -> &mut mode::Mode {
        &mut self.mode
    }

    /// Transition to a new mode. If the transition crosses an overlay
    /// boundary, records a `ModeTransition` with the appropriate duration
    /// (MODAL_IN for Normal→Overlay, MODAL_OUT for Overlay→Normal,
    /// MODAL_CROSSFADE for Overlay→Overlay). Normal→Normal is a no-op.
    ///
    /// Also drives the backdrop-dim fade whenever overlay visibility
    /// changes (Overlay→Overlay keeps the dim at full — no flicker).
    pub fn set_mode(&mut self, next: mode::Mode) {
        let now = self.clock.now();
        let prev = std::mem::replace(&mut self.mode, next);
        let prev_overlay = prev.is_overlay();
        let next_overlay = self.mode.is_overlay();
        let duration = match (prev_overlay, next_overlay) {
            (false, false) => None,
            (false, true) => Some(constants::MODAL_IN),
            (true, false) => Some(constants::MODAL_OUT),
            (true, true) => Some(constants::MODAL_CROSSFADE),
        };
        if let Some(duration) = duration {
            self.mode_transition = Some(ModeTransition {
                prev,
                started: now,
                duration,
            });
        }

        // Backdrop dim tracks "any overlay visible?" — during overlay→overlay
        // the dim stays at full (no animation), during Normal↔Overlay it
        // fades in/out to match the modal's open/close timing.
        let ctx = self.anim_ctx();
        let target_dim = if next_overlay {
            constants::BACKDROP_DIM
        } else {
            0.0
        };
        match (prev_overlay, next_overlay) {
            (false, true) => self.backdrop_dim.set_over(
                target_dim,
                constants::MODAL_IN,
                crate::tui::motion::ease_out_cubic,
                &ctx,
            ),
            (true, false) => self.backdrop_dim.set_over(
                target_dim,
                constants::MODAL_OUT,
                crate::tui::motion::ease_in_cubic,
                &ctx,
            ),
            _ => {}
        }
    }

    /// Swap the active panel and tween the focus border color. Cycle order
    /// depends on whether the viewer is enabled:
    ///   viewer off: FileTree → BundleList → FileTree
    ///   viewer on:  FileTree → Viewer → BundleList → FileTree
    pub fn toggle_focus(&mut self) {
        let next = match (self.focus, self.viewer.enabled) {
            (Focus::FileTree, true) => Focus::Viewer,
            (Focus::FileTree, false) => Focus::BundleList,
            (Focus::Viewer, _) => Focus::BundleList,
            (Focus::BundleList, _) => Focus::FileTree,
        };
        self.focus = next;
        let target = self.theme.focus_tint(next);
        let ctx = self.anim_ctx();
        self.focus_highlight.transition_to(target, &ctx);
    }

    /// Toggle the file viewer pane on/off. On transition to `on`, kicks a
    /// load for whatever file the tree cursor is on and enables terminal
    /// mouse capture so drag-selection works. On transition to `off`,
    /// disables mouse capture (restores the terminal's native text
    /// selection) and snaps focus to FileTree if it was on the viewer.
    ///
    /// Note: on terminals narrower than 100 cols, the viewer flag is still
    /// set but the render layout suppresses it. Resizing wider activates it.
    pub fn toggle_viewer(&mut self) {
        self.viewer.toggle();
        if self.viewer.enabled {
            self.reload_viewer_for_cursor();
            self.set_mouse_capture(true);
        } else {
            self.set_mouse_capture(false);
            if self.focus == Focus::Viewer {
                self.focus = Focus::FileTree;
                let target = self.theme.focus_tint(Focus::FileTree);
                let ctx = self.anim_ctx();
                self.focus_highlight.transition_to(target, &ctx);
            }
        }
    }

    /// Enable or disable terminal mouse capture. Swallows I/O errors —
    /// on failure the viewer still works via keyboard; the user just can't
    /// drag-select.
    fn set_mouse_capture(&self, enable: bool) {
        use crossterm::execute;
        use std::io::stdout;
        let mut out = stdout();
        if enable {
            let _ = execute!(out, crossterm::event::EnableMouseCapture);
        } else {
            let _ = execute!(out, crossterm::event::DisableMouseCapture);
        }
    }

    /// Sync the viewer cache with whatever file is currently under the tree
    /// cursor. No-op when viewer is disabled.
    pub fn reload_viewer_for_cursor(&mut self) {
        if !self.viewer.enabled {
            return;
        }
        let Some(&actual_idx) = self.visible_tree.get(self.tree_cursor) else {
            self.viewer.clear();
            return;
        };
        let Some(entry) = self.tree_entries.get(actual_idx) else {
            self.viewer.clear();
            return;
        };
        let path = self.project_root.join(&entry.rel_path);
        if entry.is_dir {
            // Force a reload for directories too (the path is a dir; load will
            // set ViewerError::Directory). Reset cached_path first so the
            // idempotency short-circuit doesn't skip.
            self.viewer.cached_path = None;
        }
        self.viewer.load_for_path(&path);
    }

    pub fn move_viewer_scroll(&mut self, delta: i32) {
        let vh = self.viewer_last_viewport_height.get().max(1);
        self.viewer.scroll_by(delta, vh);
    }

    pub fn scroll_viewer_to_top(&mut self) {
        self.viewer.scroll_to_top();
    }

    pub fn scroll_viewer_to_bottom(&mut self) {
        let vh = self.viewer_last_viewport_height.get().max(1);
        self.viewer.scroll_to_bottom(vh);
    }

    pub fn set_viewer_viewport_height(&self, h: usize) {
        self.viewer_last_viewport_height.set(h);
    }

    pub fn set_viewer_pane_rect(&self, rect: ratatui::layout::Rect) {
        self.viewer_pane_rect.set(Some(rect));
    }

    /// Translate an absolute (col, row) mouse position to a 0-based line
    /// index into the viewer's cached lines. Returns `None` if the click
    /// fell outside the viewer's content area (on a border, in another
    /// pane, or below the last line).
    pub fn viewer_line_at(&self, col: u16, row: u16) -> Option<usize> {
        let rect = self.viewer_pane_rect.get()?;
        // Outside the pane entirely.
        if col < rect.x || col >= rect.x + rect.width || row < rect.y || row >= rect.y + rect.height
        {
            return None;
        }
        // Inside borders (skip top border and bottom border).
        if row == rect.y || row == rect.y + rect.height - 1 {
            return None;
        }
        let row_offset = (row - rect.y - 1) as usize;
        let line_idx = self.viewer.scroll + row_offset;
        if line_idx >= self.viewer.lines().len() {
            return None;
        }
        Some(line_idx)
    }

    /// Called on a left-button mouse-down event within the viewer pane.
    /// Clears any prior selection and anchors a new drag.
    pub fn viewer_mouse_down(&mut self, col: u16, row: u16) {
        if let Some(line) = self.viewer_line_at(col, row) {
            self.viewer.begin_selection(line);
        }
    }

    /// Extend the active drag as the mouse moves with the button held.
    pub fn viewer_mouse_drag(&mut self, col: u16, row: u16) {
        if let Some(line) = self.viewer_line_at(col, row) {
            self.viewer.extend_selection(line);
        }
    }

    /// Finalize the drag (selection stays visible until cleared or added).
    pub fn viewer_mouse_up(&mut self, _col: u16, _row: u16) {
        self.viewer.end_drag();
    }

    /// Add the current viewer selection to the bundle as a Range item.
    /// No-op if no selection or no cached file.
    pub fn add_viewer_selection_to_bundle(&mut self) {
        let Some((a, b)) = self.viewer.selection() else {
            self.set_status("no lines selected (drag in the viewer first)");
            return;
        };
        let Some(path) = self.viewer.cached_path.clone() else {
            self.set_status("no file in viewer");
            return;
        };
        // Path is absolute; bundle stores relative paths from project_root.
        let rel = path
            .strip_prefix(&self.project_root)
            .unwrap_or(&path)
            .to_path_buf();
        // Lines are 0-based internally; bundle Range is 1-based inclusive.
        let start = a + 1;
        let end = b + 1;
        self.bundle.add(Item {
            path: rel.clone(),
            kind: ItemKind::Range(Range { start, end }),
            label: None,
        });
        self.bundled_paths.insert(rel.clone());
        self.recalculate_tokens();
        let _ = self.bundle.save(&self.root);
        self.viewer.clear_selection();
        self.set_status(format!("added {} lines {start}-{end}", rel.display()));
    }

    /// Current animation context (clock time + motion level).
    pub fn anim_ctx(&self) -> AnimCtx {
        AnimCtx {
            now: self.clock.now(),
            motion: self.motion,
        }
    }

    /// Opacity of the currently-visible overlay (incoming during cross-fade).
    /// 0.0 = not visible, 1.0 = fully visible. Drives the overlay blend.
    pub fn incoming_overlay_opacity(&self) -> f32 {
        if !self.mode.is_overlay() {
            return 0.0;
        }
        let now = self.clock.now();
        match &self.mode_transition {
            Some(t) => {
                let elapsed = now.saturating_duration_since(t.started);
                if elapsed >= t.duration {
                    1.0
                } else {
                    crate::tui::motion::ease_in_out_cubic(
                        elapsed.as_secs_f32() / t.duration.as_secs_f32(),
                    )
                }
            }
            None => 1.0,
        }
    }

    /// Opacity of the outgoing overlay during a cross-fade. 0.0 when there's
    /// no outgoing overlay (i.e. the previous mode was Normal, or no
    /// transition is in flight).
    pub fn outgoing_overlay_opacity(&self) -> f32 {
        let now = self.clock.now();
        match &self.mode_transition {
            Some(t) if t.prev.is_overlay() => {
                let elapsed = now.saturating_duration_since(t.started);
                if elapsed >= t.duration {
                    0.0
                } else {
                    1.0 - crate::tui::motion::ease_in_out_cubic(
                        elapsed.as_secs_f32() / t.duration.as_secs_f32(),
                    )
                }
            }
            _ => 0.0,
        }
    }

    /// The outgoing overlay mode, if one is mid-fade-out.
    pub fn outgoing_overlay_mode(&self) -> Option<&mode::Mode> {
        self.mode_transition
            .as_ref()
            .filter(|t| t.prev.is_overlay())
            .map(|t| &t.prev)
    }

    /// True iff any animation is in flight. Drives the render-loop timeout.
    /// Task 12+ extend this to include per-feature animations (gauge, status,
    /// etc.).
    pub fn has_active_animations(&self) -> bool {
        let now = self.clock.now();
        if let Some(t) = &self.mode_transition {
            if now.saturating_duration_since(t.started) < t.duration {
                return true;
            }
        }
        if self.token_gauge.is_active(now) {
            return true;
        }
        if self.backdrop_dim.is_active(now) {
            return true;
        }
        if self.status_fade.is_active(now) {
            return true;
        }
        if self.focus_highlight.is_active(now) {
            return true;
        }
        if self.startup_fade.is_active(now) {
            return true;
        }
        if self.bundle_row_fades.values().any(|f| f.is_active(now)) {
            return true;
        }
        false
    }

    /// How long the render loop should wait before waking up to run the
    /// next animation frame or scheduled tick. Returns `Duration::ZERO` to
    /// tick immediately, a finite duration for scheduled events (e.g. the
    /// status fade-out trigger), or an effectively-infinite duration when
    /// idle so event polling blocks on input.
    pub fn next_wake_delay(&self) -> Duration {
        if self.has_active_animations() {
            return Duration::from_millis(16);
        }
        // Status fade-out is scheduled for after STATUS_IN + STATUS_HOLD.
        if let Some(set_at) = self.status_set_at {
            use crate::tui::motion::constants;
            let now = self.clock.now();
            let held = now.saturating_duration_since(set_at);
            let fade_out_starts = constants::STATUS_IN + constants::STATUS_HOLD;
            if held < fade_out_starts {
                return fade_out_starts - held;
            }
            // Hold expired; hint the loop to wake up now and trigger fade-out.
            return Duration::ZERO;
        }
        Duration::from_secs(3600)
    }

    /// Clear any transition whose duration has elapsed. Called from the
    /// render loop after each draw.
    pub fn cleanup_finished_animations(&mut self) {
        let now = self.clock.now();
        if let Some(t) = &self.mode_transition {
            if now.saturating_duration_since(t.started) >= t.duration {
                self.mode_transition = None;
            }
        }
        // Drop settled row fades so the map doesn't grow unbounded.
        self.bundle_row_fades.retain(|_, fade| fade.is_active(now));
    }

    /// Test-only constructor that installs a mock clock and forces Full
    /// motion (so animations are observable without depending on env vars).
    #[cfg(test)]
    pub fn with_clock(root: CtxforgeRoot, clock: Box<dyn Clock>) -> Self {
        let mut app = Self::new(root);
        app.clock = clock;
        app.motion = MotionLevel::Full;
        app
    }

    pub fn new(root: CtxforgeRoot) -> Self {
        let project_root = root.project_root().to_path_buf();
        let bundle = Bundle::load_or_default(&root).unwrap_or_default();
        let tree_entries = tree::build(&project_root);
        let visible_tree = tree::visible_indices(&tree_entries);

        let model_name = bundle
            .model
            .clone()
            .unwrap_or_else(|| models::DEFAULT_MODEL.to_string());

        let mut app = App {
            root,
            project_root,
            bundle,
            tree_entries,
            visible_tree,
            tree_cursor: 0,
            bundle_cursor: 0,
            focus: Focus::FileTree,
            model_name,
            model_window: 0,
            item_tokens: Vec::new(),
            total_tokens: 0,
            exact_tokens: false,
            token_gauge: crate::tui::motion::Gauge::new(0.0),
            backdrop_dim: crate::tui::motion::Fade::new_hidden(),
            status_fade: crate::tui::motion::Fade::new_hidden(),
            status_set_at: None,
            tree_list_state: std::cell::RefCell::new(ratatui::widgets::ListState::default()),
            bundle_list_state: std::cell::RefCell::new(ratatui::widgets::ListState::default()),
            focus_highlight: crate::tui::motion::Highlight::new(
                crate::tui::theme::registry::default_theme()
                    .focus_tint(Focus::FileTree),
            ),
            startup_fade: crate::tui::motion::Fade::new_hidden(),
            bundle_row_fades: std::collections::HashMap::new(),
            viewer: crate::tui::viewer::ViewerState::new(),
            viewer_last_viewport_height: std::cell::Cell::new(10),
            viewer_pane_rect: std::cell::Cell::new(None),
            bundled_paths: HashSet::new(),
            search_results: Vec::new(),
            profile_name: None,
            pending_stdout: None,
            pending_pipe: None,
            mode: mode::Mode::Normal,
            mode_transition: None,
            clock: Box::new(SystemClock),
            motion: crate::tui::motion::detect_motion(),
            should_quit: false,
            status_message: String::new(),
            show_help: false,
            tree_viewport_height: std::cell::Cell::new(0),
            bundle_viewport_height: std::cell::Cell::new(0),
            theme: {
                let name = crate::paths::config_file_path()
                    .map(|p| crate::tui::theme::config::resolve_theme_name(&p))
                    .unwrap_or_else(|| "ctxforge".to_string());
                crate::tui::theme::registry::by_name(&name)
                    .unwrap_or_else(|| crate::tui::theme::registry::default_theme())
            },
        };
        app.rebuild_bundled_paths();
        app.recalculate_tokens();
        // Snap the gauge to current total so startup doesn't fade from 0.
        app.token_gauge.snap(app.total_tokens as f32);
        // Kick the startup fade — the whole TUI fades in over STARTUP duration.
        let ctx = app.anim_ctx();
        app.startup_fade.set_over(
            1.0,
            crate::tui::motion::constants::STARTUP,
            crate::tui::motion::ease_out_cubic,
            &ctx,
        );
        app
    }

    /// Number of currently visible (non-collapsed) tree rows.
    pub fn visible_tree_len(&self) -> usize {
        self.visible_tree.len()
    }

    /// Run fuzzy search across all tree entries (files only) and populate
    /// `search_results` ranked by score (best first). An empty query falls
    /// back to the current visible tree so the result list is never empty
    /// when the user first opens search.
    pub fn run_search(&mut self, query: &str) {
        if query.is_empty() {
            self.search_results = self.visible_tree.clone();
            return;
        }
        let matcher = SkimMatcherV2::default();
        let mut scored: Vec<(usize, i64)> = self
            .tree_entries
            .iter()
            .enumerate()
            .filter(|(_, e)| !e.is_dir)
            .filter_map(|(i, e)| {
                let path_str = e.rel_path.to_string_lossy();
                matcher
                    .fuzzy_match(&path_str, query)
                    .map(|score| (i, score))
            })
            .collect();
        scored.sort_by(|a, b| b.1.cmp(&a.1));
        self.search_results = scored.into_iter().map(|(i, _)| i).collect();
    }

    /// Toggle expand/collapse for the directory at the current tree cursor.
    /// No-op if the cursor is on a file.
    pub fn toggle_expand(&mut self) {
        let Some(&actual_idx) = self.visible_tree.get(self.tree_cursor) else {
            return;
        };
        let Some(entry) = self.tree_entries.get_mut(actual_idx) else {
            return;
        };
        if !entry.is_dir {
            return;
        }
        entry.expanded = !entry.expanded;
        self.visible_tree = tree::visible_indices(&self.tree_entries);
        // Clamp cursor to new visible range.
        if !self.visible_tree.is_empty() && self.tree_cursor >= self.visible_tree.len() {
            self.tree_cursor = self.visible_tree.len() - 1;
        }
        self.reload_viewer_for_cursor();
    }

    /// Toggle selection of the file at the current tree cursor.
    pub fn toggle_current(&mut self) {
        let Some(&actual_idx) = self.visible_tree.get(self.tree_cursor) else {
            return;
        };
        let Some(entry) = self.tree_entries.get(actual_idx) else {
            return;
        };
        if entry.is_dir {
            return; // Can't select directories.
        }

        let path = entry.rel_path.clone();
        if self.bundled_paths.contains(&path) {
            // Remove from bundle.
            self.bundle.remove_by_path(&path);
            self.bundled_paths.remove(&path);
            self.bundle_row_fades.remove(&path);
            self.set_status(format!("removed {}", path.display()));
        } else {
            // Add to bundle.
            let item = Item {
                path: path.clone(),
                kind: ItemKind::File,
                label: None,
            };
            self.bundle.add(item);
            self.bundled_paths.insert(path.clone());
            // Kick a per-row fade-in.
            let ctx = self.anim_ctx();
            use crate::tui::motion::{Fade, constants, ease_out_cubic};
            let mut fade = Fade::new_hidden();
            fade.set_over(1.0, constants::ROW_IN, ease_out_cubic, &ctx);
            self.bundle_row_fades.insert(path.clone(), fade);
            self.set_status(format!("added {}", path.display()));
        }

        self.recalculate_tokens();
        let _ = self.bundle.save(&self.root);
    }

    /// Copy bundle to clipboard.
    pub fn copy_to_clipboard(&mut self) {
        let resolved = resolve::resolve_all(&self.bundle.items, &self.project_root);
        match resolved {
            Ok(items) => {
                let memory = crate::memory::collect_for_attach(&self.root, false, None, 10)
                    .unwrap_or_default();
                let rendered =
                    crate::format::render(crate::format::Format::Markdown, &items, &memory);
                match crate::clipboard::set(&rendered) {
                    Ok(()) => {
                        self.set_status(format!(
                            "Copied {} items ({} tokens) to clipboard",
                            self.bundle.len(),
                            self.total_tokens
                        ));
                    }
                    Err(e) => {
                        self.set_status(format!("Clipboard error: {e}"));
                    }
                }
            }
            Err(e) => {
                self.set_status(format!("Resolve error: {e}"));
            }
        }
    }

    pub fn move_tree_cursor(&mut self, delta: i32) {
        if self.visible_tree.is_empty() {
            return;
        }
        let new = self.tree_cursor as i32 + delta;
        self.tree_cursor = new.clamp(0, self.visible_tree.len() as i32 - 1) as usize;
        self.reload_viewer_for_cursor();
    }

    /// Page-sized movement for the file tree. Uses the last captured viewport
    /// height; falls back to 10 rows if nothing has been rendered yet.
    pub fn page_tree_cursor(&mut self, direction: i32) {
        let page = if self.tree_viewport_height.get() > 0 {
            self.tree_viewport_height.get() as i32
        } else {
            10
        };
        self.move_tree_cursor(direction * page);
    }

    /// Half-page movement (Ctrl-D / Ctrl-U style) for the file tree.
    pub fn half_page_tree_cursor(&mut self, direction: i32) {
        let half = if self.tree_viewport_height.get() > 0 {
            (self.tree_viewport_height.get() as i32 / 2).max(1)
        } else {
            5
        };
        self.move_tree_cursor(direction * half);
    }

    /// Page-sized movement for the bundle list.
    pub fn page_bundle_cursor(&mut self, direction: i32) {
        let page = if self.bundle_viewport_height.get() > 0 {
            self.bundle_viewport_height.get() as i32
        } else {
            10
        };
        self.move_bundle_cursor(direction * page);
    }

    /// Half-page movement for the bundle list.
    pub fn half_page_bundle_cursor(&mut self, direction: i32) {
        let half = if self.bundle_viewport_height.get() > 0 {
            (self.bundle_viewport_height.get() as i32 / 2).max(1)
        } else {
            5
        };
        self.move_bundle_cursor(direction * half);
    }

    /// Expand every directory in the tree. Keeps the cursor on the same file
    /// (by relative path) when possible.
    pub fn expand_all_dirs(&mut self) {
        let anchor = self.cursor_anchor_path();
        tree::expand_all(&mut self.tree_entries);
        self.visible_tree = tree::visible_indices(&self.tree_entries);
        self.restore_cursor_from_anchor(anchor);
        self.reload_viewer_for_cursor();
        self.set_status("expanded all directories");
    }

    /// Collapse every directory in the tree. Cursor is kept on the same file
    /// if it's still visible, otherwise clamped to the last visible row.
    pub fn collapse_all_dirs(&mut self) {
        let anchor = self.cursor_anchor_path();
        tree::collapse_all(&mut self.tree_entries);
        self.visible_tree = tree::visible_indices(&self.tree_entries);
        self.restore_cursor_from_anchor(anchor);
        self.reload_viewer_for_cursor();
        self.set_status("collapsed all directories");
    }

    fn cursor_anchor_path(&self) -> Option<PathBuf> {
        self.visible_tree
            .get(self.tree_cursor)
            .and_then(|&idx| self.tree_entries.get(idx))
            .map(|e| e.rel_path.clone())
    }

    fn restore_cursor_from_anchor(&mut self, anchor: Option<PathBuf>) {
        if self.visible_tree.is_empty() {
            self.tree_cursor = 0;
            return;
        }
        if let Some(path) = anchor {
            if let Some(pos) = self
                .visible_tree
                .iter()
                .position(|&i| self.tree_entries.get(i).map(|e| &e.rel_path) == Some(&path))
            {
                self.tree_cursor = pos;
                return;
            }
        }
        if self.tree_cursor >= self.visible_tree.len() {
            self.tree_cursor = self.visible_tree.len() - 1;
        }
    }

    pub fn move_bundle_cursor(&mut self, delta: i32) {
        if self.bundle.is_empty() {
            return;
        }
        let new = self.bundle_cursor as i32 + delta;
        self.bundle_cursor = new.clamp(0, self.bundle.len() as i32 - 1) as usize;
    }

    pub fn window_pct(&self) -> f64 {
        if self.model_window == 0 {
            return 0.0;
        }
        (self.total_tokens as f64 / self.model_window as f64) * 100.0
    }

    /// Start narrow mode for the current bundle item. No-op unless the
    /// BundleList panel is focused and the cursor is on a `File` item.
    pub fn start_narrow(&mut self) {
        if self.focus != Focus::BundleList {
            return;
        }
        if let Some(item) = self.bundle.items.get(self.bundle_cursor) {
            if matches!(item.kind, ItemKind::File) {
                self.set_mode(mode::Mode::Narrow {
                    start: String::new(),
                    end: String::new(),
                    field: mode::InputField::First,
                });
            }
        }
    }

    /// Confirm narrow: replace the current bundle item's kind with a Range.
    /// Validates that both inputs parse and that start <= end.
    pub fn confirm_narrow(&mut self) {
        // Pull start/end out of the mode before mutating self further.
        let (start_str, end_str) = match self.mode() {
            mode::Mode::Narrow { start, end, .. } => (start.clone(), end.clone()),
            _ => return,
        };

        let start_num: usize = match start_str.parse() {
            Ok(n) if n >= 1 => n,
            _ => {
                self.set_status("Invalid start line");
                return;
            }
        };
        let end_num: usize = match end_str.parse() {
            Ok(n) if n >= start_num => n,
            _ => {
                self.set_status("Invalid end line (must be >= start)");
                return;
            }
        };

        if let Some(item) = self.bundle.items.get_mut(self.bundle_cursor) {
            item.kind = ItemKind::Range(Range {
                start: start_num,
                end: end_num,
            });
        }
        self.recalculate_tokens();
        let _ = self.bundle.save(&self.root);
        self.set_status(format!("Narrowed to lines {start_num}-{end_num}"));
        self.set_mode(mode::Mode::Normal);
    }

    /// Save current bundle as a named profile under `.ctxforge/profiles/`.
    pub fn save_profile(&mut self, name: &str) {
        match crate::profile::save(&self.root, name, &self.bundle) {
            Ok(()) => {
                self.profile_name = Some(name.to_string());
                self.set_status(format!("Saved profile '{name}'"));
            }
            Err(e) => {
                self.set_status(format!("Save error: {e}"));
            }
        }
        self.set_mode(mode::Mode::Normal);
    }

    /// Open the load-profile picker. If no profiles exist, sets a status
    /// message and returns to Normal mode without entering LoadProfile mode.
    pub fn start_load_profile(&mut self) {
        match crate::profile::list(&self.root) {
            Ok(profiles) => {
                if profiles.is_empty() {
                    self.set_status("No profiles saved yet");
                } else {
                    self.set_mode(mode::Mode::LoadProfile {
                        cursor: 0,
                        profiles,
                    });
                }
            }
            Err(e) => {
                self.set_status(format!("Profile list error: {e}"));
            }
        }
    }

    /// Load whichever profile the cursor is on inside `LoadProfile` mode.
    pub fn load_selected_profile(&mut self) {
        // Pull the chosen name out of the mode before mutating self.
        let chosen = if let mode::Mode::LoadProfile { cursor, profiles } = self.mode() {
            profiles.get(*cursor).cloned()
        } else {
            None
        };
        if let Some(name) = chosen {
            match crate::profile::load(&self.root, &name) {
                Ok(bundle) => {
                    self.bundle = bundle;
                    self.rebuild_bundled_paths();
                    self.recalculate_tokens();
                    let _ = self.bundle.save(&self.root);
                    self.profile_name = Some(name.clone());
                    self.set_status(format!("Loaded profile '{name}'"));
                }
                Err(e) => {
                    self.set_status(format!("Load error: {e}"));
                }
            }
        }
        self.set_mode(mode::Mode::Normal);
    }

    /// Render the bundle as XML and stash it in `pending_stdout` so the run
    /// loop can drain the alternate screen before printing.
    pub fn export_xml_to_stdout(&mut self) {
        match resolve::resolve_all(&self.bundle.items, &self.project_root) {
            Ok(items) => {
                let memory = crate::memory::collect_for_attach(&self.root, false, None, 10)
                    .unwrap_or_default();
                let rendered = crate::format::render(crate::format::Format::Xml, &items, &memory);
                self.pending_stdout = Some(rendered);
                self.set_status("Exported XML to stdout");
            }
            Err(e) => {
                self.set_status(format!("Export error: {e}"));
            }
        }
    }

    /// Switch to a different model and recompute token counts. Persists the
    /// new model on the bundle so it survives restart.
    pub fn switch_model(&mut self, model_name: &str) {
        self.model_name = model_name.to_string();
        self.bundle.model = Some(model_name.to_string());
        self.recalculate_tokens();
        let _ = self.bundle.save(&self.root);
        self.set_status(format!("Switched to {model_name}"));
        self.set_mode(mode::Mode::Normal);
    }

    /// Start function pick mode (extract feature only). Performs a project-wide
    /// tree-sitter scan synchronously — fast on small projects, slower on
    /// large ones. Sets a status message instead of opening if no functions
    /// are found in any supported language.
    #[cfg(feature = "extract")]
    pub fn start_function_pick(&mut self) {
        let items = crate::extract::scan::scan_functions(&self.project_root);
        if items.is_empty() {
            self.set_status("No functions found in project");
        } else {
            self.set_mode(mode::Mode::FunctionPick { cursor: 0, items });
        }
    }

    /// Start type pick mode (extract feature only). Same semantics as
    /// `start_function_pick` but scans for type/struct/class/interface defs.
    #[cfg(feature = "extract")]
    pub fn start_type_pick(&mut self) {
        let items = crate::extract::scan::scan_types(&self.project_root);
        if items.is_empty() {
            self.set_status("No types found in project");
        } else {
            self.set_mode(mode::Mode::TypePick { cursor: 0, items });
        }
    }

    /// Add the function under the FunctionPick cursor to the bundle.
    #[cfg(feature = "extract")]
    pub fn add_picked_function(&mut self) {
        let chosen = if let mode::Mode::FunctionPick { cursor, items } = self.mode() {
            items.get(*cursor).cloned()
        } else {
            None
        };
        if let Some((name, path)) = chosen {
            let item = Item {
                path,
                kind: ItemKind::Function { name: name.clone() },
                label: None,
            };
            self.bundle.add(item);
            self.rebuild_bundled_paths();
            self.recalculate_tokens();
            let _ = self.bundle.save(&self.root);
            self.set_status(format!("Added fn:{name}"));
        }
        self.set_mode(mode::Mode::Normal);
    }

    /// Add the type under the TypePick cursor to the bundle.
    #[cfg(feature = "extract")]
    pub fn add_picked_type(&mut self) {
        let chosen = if let mode::Mode::TypePick { cursor, items } = self.mode() {
            items.get(*cursor).cloned()
        } else {
            None
        };
        if let Some((name, path)) = chosen {
            let item = Item {
                path,
                kind: ItemKind::Type { name: name.clone() },
                label: None,
            };
            self.bundle.add(item);
            self.rebuild_bundled_paths();
            self.recalculate_tokens();
            let _ = self.bundle.save(&self.root);
            self.set_status(format!("Added type:{name}"));
        }
        self.set_mode(mode::Mode::Normal);
    }

    /// Start diff pick mode — opens a branch-name input first, then a
    /// multi-select list of changed files.
    pub fn start_diff_pick(&mut self) {
        self.set_mode(mode::Mode::DiffPick {
            branch: "main".into(),
            files: Vec::new(),
            selected: std::collections::HashSet::new(),
            cursor: 0,
            entering_branch: true,
        });
    }

    /// Load the changed files for the entered branch and switch to the
    /// file-selection phase. On error or empty diff, drops back to Normal
    /// with a status message.
    pub fn load_diff_files(&mut self) {
        let branch = if let mode::Mode::DiffPick { branch, .. } = self.mode() {
            branch.clone()
        } else {
            return;
        };
        match crate::git::changed_files(&self.project_root, &branch) {
            Ok(changed) => {
                if changed.is_empty() {
                    self.set_status(format!("No changes vs {branch}"));
                    self.set_mode(mode::Mode::Normal);
                    return;
                }
                if let mode::Mode::DiffPick {
                    files,
                    entering_branch,
                    ..
                } = self.mode_mut()
                {
                    *files = changed;
                    *entering_branch = false;
                }
            }
            Err(e) => {
                self.set_status(format!("Diff error: {e}"));
                self.set_mode(mode::Mode::Normal);
            }
        }
    }

    /// Add all selected diff files to the bundle.
    pub fn add_selected_diff_files(&mut self) {
        let to_add: Vec<PathBuf> = if let mode::Mode::DiffPick {
            files, selected, ..
        } = &self.mode
        {
            selected
                .iter()
                .filter_map(|&idx| files.get(idx).cloned())
                .collect()
        } else {
            Vec::new()
        };
        let count = to_add.len();
        for path in to_add {
            self.bundle.add(Item {
                path,
                kind: ItemKind::File,
                label: None,
            });
        }
        if count > 0 {
            self.rebuild_bundled_paths();
            self.recalculate_tokens();
            let _ = self.bundle.save(&self.root);
            self.set_status(format!("Added {count} changed file(s)"));
        }
        self.set_mode(mode::Mode::Normal);
    }

    /// Open or close the memory recall panel. Reads notes from the JSONL
    /// index — sets a status message instead of opening if there are none.
    pub fn toggle_memory_panel(&mut self) {
        if matches!(self.mode(), mode::Mode::MemoryPanel { .. }) {
            self.set_mode(mode::Mode::Normal);
            return;
        }
        match crate::memory::index::read_all(&self.root) {
            Ok(notes) => {
                let count = notes.len();
                if count == 0 {
                    self.set_status("No memory notes yet");
                } else {
                    self.set_mode(mode::Mode::MemoryPanel { cursor: 0, count });
                }
            }
            Err(e) => {
                self.set_status(format!("Memory error: {e}"));
            }
        }
    }

    /// Persist the inline note from `Mode::AddNote`. Validates the body is
    /// not empty before writing. An empty tag becomes `None` (untagged → goes
    /// to `decisions.md`).
    pub fn write_note_inline(&mut self) {
        let (tag, body) = if let mode::Mode::AddNote { tag, body, .. } = self.mode() {
            (tag.clone(), body.clone())
        } else {
            return;
        };

        if body.trim().is_empty() {
            self.set_status("Note body cannot be empty");
            self.set_mode(mode::Mode::Normal);
            return;
        }
        let tag_opt = if tag.trim().is_empty() {
            None
        } else {
            Some(tag)
        };
        match crate::memory::write_note(&self.root, body, tag_opt) {
            Ok(note) => {
                let ts = note.timestamp.format("%Y-%m-%d %H:%M");
                self.set_status(format!("Noted: [{ts}] {}", note.body));
            }
            Err(e) => {
                self.set_status(format!("Note error: {e}"));
            }
        }
        self.set_mode(mode::Mode::Normal);
    }

    /// Pipe the rendered bundle to a local agent CLI. Targets `claude` get
    /// XML; everything else gets markdown. The actual subprocess spawn is
    /// deferred to the run loop via `pending_pipe`.
    pub fn pipe_to_agent(&mut self, target: &str) {
        let fmt = match target {
            "claude" => crate::format::Format::Xml,
            _ => crate::format::Format::Markdown,
        };
        match resolve::resolve_all(&self.bundle.items, &self.project_root) {
            Ok(items) => {
                let memory = crate::memory::collect_for_attach(&self.root, false, None, 10)
                    .unwrap_or_default();
                let rendered = crate::format::render(fmt, &items, &memory);
                self.pending_pipe = Some((target.to_string(), rendered));
            }
            Err(e) => {
                self.set_status(format!("Pipe error: {e}"));
            }
        }
        self.set_mode(mode::Mode::Normal);
    }

    pub(crate) fn rebuild_bundled_paths(&mut self) {
        self.bundled_paths = self.bundle.items.iter().map(|i| i.path.clone()).collect();
    }

    /// Switch to a named theme and persist the choice to
    /// `~/.config/ctxforge/config.toml`. Falls back with an error status
    /// message if the name does not match a known theme.
    pub fn set_theme_by_name(&mut self, name: &str) {
        let Some(theme) = crate::tui::theme::registry::by_name(name) else {
            self.set_status(format!("unknown theme: {name}"));
            return;
        };
        self.theme = theme;
        if let Some(path) = crate::paths::config_file_path() {
            let mut cfg = crate::tui::theme::config::load_from(&path).unwrap_or_default();
            cfg.theme = name.to_string();
            if let Err(e) = crate::tui::theme::config::save_to(&path, &cfg) {
                self.set_status(format!("theme set but config save failed: {e}"));
                return;
            }
        }
        self.set_status(format!("theme: {name}"));
    }

    /// Short status message setter. Kicks off a fade-in animation; the
    /// render loop handles the fade-out after STATUS_HOLD via
    /// `tick_status_fade`.
    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = msg.into();
        if self.status_message.is_empty() {
            self.status_set_at = None;
            self.status_fade.snap(0.0);
            return;
        }
        let now = self.clock.now();
        self.status_set_at = Some(now);
        let ctx = self.anim_ctx();
        use crate::tui::motion::{constants, ease_out_cubic};
        self.status_fade
            .set_over(1.0, constants::STATUS_IN, ease_out_cubic, &ctx);
    }

    /// Advance the status-message fade state machine. Should be called each
    /// render-loop iteration. After STATUS_IN + STATUS_HOLD, triggers the
    /// fade-out; after the fade-out completes, clears the message text.
    pub fn tick_status_fade(&mut self) {
        use crate::tui::motion::{constants, ease_in_cubic};
        let Some(set_at) = self.status_set_at else {
            return;
        };
        let now = self.clock.now();
        let held = now.saturating_duration_since(set_at);
        let fade_out_starts_at = constants::STATUS_IN + constants::STATUS_HOLD;
        let fully_gone_at = fade_out_starts_at + constants::STATUS_OUT;

        if held >= fade_out_starts_at
            && self.status_fade.opacity(now) > 0.0
            && !self.status_fade.is_active(now)
        {
            let ctx = self.anim_ctx();
            self.status_fade
                .set_over(0.0, constants::STATUS_OUT, ease_in_cubic, &ctx);
        }
        if held >= fully_gone_at {
            self.status_message.clear();
            self.status_set_at = None;
        }
    }

    /// Open the template flow: if a name is given inline, jump to task input;
    /// otherwise open the template picker.
    pub fn start_template_flow(&mut self, name: Option<String>) {
        if let Some(n) = name {
            if !n.is_empty() {
                match crate::template::resolve_template_path(&self.root, &n) {
                    Ok(_) => {
                        self.set_mode(mode::Mode::TemplateTask {
                            template_name: n,
                            task: String::new(),
                        });
                        return;
                    }
                    Err(e) => {
                        self.set_status(format!("template error: {e}"));
                        return;
                    }
                }
            }
        }
        let templates = scan_all_templates(&self.root);
        if templates.is_empty() {
            self.set_status("no templates found; create one with `ctxforge templates new <name>`");
            return;
        }
        self.set_mode(mode::Mode::TemplatePick {
            cursor: 0,
            templates,
        });
    }

    /// Show available templates in a status message.
    pub fn show_template_list(&mut self) {
        let templates = scan_all_templates(&self.root);
        if templates.is_empty() {
            self.set_status("no templates found");
        } else {
            let names: Vec<String> = templates.iter().map(|(n, _)| n.clone()).collect();
            self.set_status(format!("templates: {}", names.join(", ")));
        }
    }

    /// Scaffold a new project-local template via the command palette.
    pub fn run_template_new(&mut self, name: Option<String>) {
        let name = match name {
            Some(n) if !n.is_empty() => n,
            _ => {
                self.set_status("usage: /template-new <name>");
                return;
            }
        };
        let dir = self.root.templates_dir();
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(format!("{name}.md"));
        if path.exists() {
            self.set_status(format!("template '{name}' already exists"));
            return;
        }
        let content = format!(
            "# Template: {name}\n\n\
             You are an expert software engineer. Below is the relevant code and notes.\n\n\
             ## Task\n\n{{{{task}}}}\n\n\
             ## Code and notes\n\n{{{{bundle}}}}\n"
        );
        match std::fs::write(&path, content) {
            Ok(()) => {
                self.set_status(format!("created template '{name}' at {}", path.display()));
            }
            Err(e) => {
                self.set_status(format!("error creating template: {e}"));
            }
        }
    }

    /// Delete a project-local template via the command palette.
    pub fn run_template_rm(&mut self, name: Option<String>) {
        let name = match name {
            Some(n) if !n.is_empty() => n,
            _ => {
                self.set_status("usage: /template-rm <name>");
                return;
            }
        };
        let stem = name.strip_suffix(".md").unwrap_or(&name);
        let path = self.root.template_path(stem);
        if !path.exists() {
            self.set_status(format!("template '{stem}' not found"));
            return;
        }
        match std::fs::remove_file(&path) {
            Ok(()) => {
                self.set_status(format!("deleted template '{stem}'"));
            }
            Err(e) => {
                self.set_status(format!("error deleting template: {e}"));
            }
        }
    }

    /// List built-in starter templates in a status message.
    pub fn run_template_starters(&mut self) {
        self.set_status("starters: bugfix, code-review, explain, refactor, migrate (use CLI: ctxforge templates new <name> --from <starter>)");
    }

    /// Confirm template task: render bundle, apply template, copy to clipboard.
    pub fn confirm_template_task(&mut self) {
        let (template_name, task) = match self.mode() {
            mode::Mode::TemplateTask {
                template_name,
                task,
            } => (template_name.clone(), task.clone()),
            _ => return,
        };
        let resolved = match crate::resolve::resolve_all(&self.bundle.items, &self.project_root) {
            Ok(r) => r,
            Err(e) => {
                self.set_status(format!("resolve error: {e}"));
                self.set_mode(mode::Mode::Normal);
                return;
            }
        };
        let memory =
            crate::memory::collect_for_attach(&self.root, false, None, 10).unwrap_or_default();
        let rendered = crate::format::render(crate::format::Format::Markdown, &resolved, &memory);

        let final_content = match crate::template::apply_template(
            &self.root,
            &template_name,
            &rendered,
            Some(&task),
        ) {
            Ok(c) => c,
            Err(e) => {
                self.set_status(format!("template error: {e}"));
                self.set_mode(mode::Mode::Normal);
                return;
            }
        };

        match crate::clipboard::set(&final_content) {
            Ok(()) => {
                self.set_status(format!(
                    "copied template '{template_name}' with bundle + task"
                ));
            }
            Err(e) => {
                self.set_status(format!("clipboard error: {e}"));
            }
        }
        self.set_mode(mode::Mode::Normal);
    }

    pub(crate) fn recalculate_tokens(&mut self) {
        let model = models::lookup(&self.model_name);
        self.model_window = model.window;

        let resolved =
            resolve::resolve_all(&self.bundle.items, &self.project_root).unwrap_or_default();

        self.item_tokens = resolved
            .iter()
            .map(|r| tokens::count(&r.content, &model).tokens)
            .collect();

        self.total_tokens = self.item_tokens.iter().sum();
        self.exact_tokens = matches!(model.tokenizer, models::Tokenizer::Estimate)
            .then_some(false)
            .unwrap_or(true);
        // Animate the gauge toward the new total.
        let ctx = self.anim_ctx();
        self.token_gauge.set(self.total_tokens as f32, &ctx);
    }
}

/// Scan project-local and global template directories for available templates.
fn scan_all_templates(root: &CtxforgeRoot) -> Vec<(String, mode::TemplateSource)> {
    let mut result = Vec::new();
    let project_dir = root.templates_dir();
    if project_dir.is_dir() {
        for entry in std::fs::read_dir(&project_dir)
            .into_iter()
            .flatten()
            .flatten()
        {
            if let Some(name) = entry
                .path()
                .file_stem()
                .and_then(|s| s.to_str())
                .map(String::from)
            {
                result.push((name, mode::TemplateSource::Project));
            }
        }
    }
    if let Some(global_dir) = crate::paths::global_templates_dir() {
        if global_dir.is_dir() {
            for entry in std::fs::read_dir(&global_dir)
                .into_iter()
                .flatten()
                .flatten()
            {
                if let Some(name) = entry
                    .path()
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(String::from)
                {
                    if !result.iter().any(|(n, _)| n == &name) {
                        result.push((name, mode::TemplateSource::Global));
                    }
                }
            }
        }
    }
    result.sort_by(|a, b| a.0.cmp(&b.0));
    result
}

#[cfg(test)]
mod mode_transition_tests {
    use super::*;
    use crate::tui::mode::Mode;
    use crate::tui::motion::{MockClock, constants};
    use std::time::Duration;
    use tempfile::TempDir;

    fn test_app() -> (App, MockClock, TempDir) {
        let tmp = TempDir::new().unwrap();
        let root = CtxforgeRoot::find_or_create(tmp.path()).unwrap();
        let clock = MockClock::new();
        let app = App::with_clock(root, Box::new(clock.clone()));
        (app, clock, tmp)
    }

    #[test]
    fn normal_to_normal_has_no_transition() {
        let (mut app, _clock, _tmp) = test_app();
        app.set_mode(Mode::Normal);
        assert!(app.mode_transition.is_none());
    }

    #[test]
    fn normal_to_overlay_sets_modal_in_duration() {
        let (mut app, _clock, _tmp) = test_app();
        app.set_mode(Mode::Help);
        let t = app.mode_transition.as_ref().unwrap();
        assert_eq!(t.duration, constants::MODAL_IN);
    }

    #[test]
    fn overlay_to_normal_sets_modal_out_duration() {
        let (mut app, _clock, _tmp) = test_app();
        app.set_mode(Mode::Help);
        app.set_mode(Mode::Normal);
        let t = app.mode_transition.as_ref().unwrap();
        assert_eq!(t.duration, constants::MODAL_OUT);
    }

    #[test]
    fn overlay_to_overlay_sets_crossfade_duration() {
        let (mut app, _clock, _tmp) = test_app();
        app.set_mode(Mode::Help);
        app.set_mode(Mode::PipeMenu);
        let t = app.mode_transition.as_ref().unwrap();
        assert_eq!(t.duration, constants::MODAL_CROSSFADE);
    }

    #[test]
    fn has_active_animations_false_by_default() {
        let (mut app, clock, _tmp) = test_app();
        // The startup fade is active right after construction; advance past it.
        clock.advance(Duration::from_millis(500));
        app.cleanup_finished_animations();
        assert!(!app.has_active_animations());
    }

    #[test]
    fn has_active_animations_true_during_transition() {
        let (mut app, _clock, _tmp) = test_app();
        app.set_mode(Mode::Help);
        assert!(app.has_active_animations());
    }

    #[test]
    fn cleanup_clears_finished_transition() {
        let (mut app, clock, _tmp) = test_app();
        app.set_mode(Mode::Help);
        assert!(app.has_active_animations());
        clock.advance(Duration::from_millis(500));
        app.cleanup_finished_animations();
        assert!(!app.has_active_animations());
        assert!(app.mode_transition.is_none());
    }
}

#[cfg(test)]
mod viewer_integration_tests {
    use super::*;
    use crate::paths::CtxforgeRoot;
    use crate::tui::motion::MockClock;
    use tempfile::TempDir;

    fn test_app_with_files(files: &[(&str, &[u8])]) -> (App, TempDir) {
        let tmp = TempDir::new().unwrap();
        for (name, content) in files {
            std::fs::write(tmp.path().join(name), content).unwrap();
        }
        let root = CtxforgeRoot::find_or_create(tmp.path()).unwrap();
        let clock = MockClock::new();
        let app = App::with_clock(root, Box::new(clock));
        (app, tmp)
    }

    #[test]
    fn toggle_viewer_flips_enabled() {
        let (mut app, _tmp) = test_app_with_files(&[]);
        assert!(!app.viewer.enabled);
        app.toggle_viewer();
        assert!(app.viewer.enabled);
        app.toggle_viewer();
        assert!(!app.viewer.enabled);
    }

    #[test]
    fn toggle_viewer_on_triggers_load_for_current_cursor() {
        let (mut app, _tmp) =
            test_app_with_files(&[("a.rs", b"fn a() {}\n"), ("b.rs", b"fn b() {}\n")]);
        assert!(!app.visible_tree.is_empty());
        app.toggle_viewer();
        assert!(app.viewer.cached_path.is_some());
    }

    #[test]
    fn move_tree_cursor_reloads_viewer_when_enabled() {
        let (mut app, _tmp) =
            test_app_with_files(&[("a.rs", b"fn a() {}\n"), ("b.rs", b"fn b() {}\n")]);
        app.toggle_viewer();
        let first_path = app.viewer.cached_path.clone();
        app.move_tree_cursor(1);
        let second_path = app.viewer.cached_path.clone();
        assert_ne!(first_path, second_path);
    }

    #[test]
    fn move_tree_cursor_when_viewer_disabled_does_not_load() {
        let (mut app, _tmp) =
            test_app_with_files(&[("a.rs", b"fn a() {}\n"), ("b.rs", b"fn b() {}\n")]);
        assert!(!app.viewer.enabled);
        app.move_tree_cursor(1);
        assert!(app.viewer.cached_path.is_none());
    }

    #[test]
    fn disable_viewer_while_focused_snaps_focus_to_tree() {
        let (mut app, _tmp) = test_app_with_files(&[("a.rs", b"fn a() {}\n")]);
        app.toggle_viewer();
        app.focus = Focus::Viewer;
        app.toggle_viewer();
        assert_eq!(app.focus, Focus::FileTree);
    }

    #[test]
    fn tab_cycle_with_viewer_off_skips_viewer() {
        let (mut app, _tmp) = test_app_with_files(&[]);
        assert_eq!(app.focus, Focus::FileTree);
        app.toggle_focus();
        assert_eq!(app.focus, Focus::BundleList);
        app.toggle_focus();
        assert_eq!(app.focus, Focus::FileTree);
    }

    #[test]
    fn tab_cycle_with_viewer_on_includes_viewer() {
        let (mut app, _tmp) = test_app_with_files(&[]);
        app.toggle_viewer();
        assert_eq!(app.focus, Focus::FileTree);
        app.toggle_focus();
        assert_eq!(app.focus, Focus::Viewer);
        app.toggle_focus();
        assert_eq!(app.focus, Focus::BundleList);
        app.toggle_focus();
        assert_eq!(app.focus, Focus::FileTree);
    }

    #[test]
    fn add_selection_with_no_selection_sets_status_and_no_bundle_change() {
        let (mut app, _tmp) = test_app_with_files(&[("a.rs", b"fn a() {}\n")]);
        app.toggle_viewer();
        let before = app.bundle.len();
        app.add_viewer_selection_to_bundle();
        assert_eq!(app.bundle.len(), before);
        assert!(app.status_message.contains("no lines selected"));
    }

    #[test]
    fn add_selection_appends_range_item_to_bundle() {
        let (mut app, _tmp) = test_app_with_files(&[(
            "big.rs",
            b"fn one() {}\nfn two() {}\nfn three() {}\nfn four() {}\n",
        )]);
        app.toggle_viewer();
        // Select lines 1-2 (0-based) → stored as 1-based 2-3.
        app.viewer.begin_selection(1);
        app.viewer.extend_selection(2);

        let before = app.bundle.len();
        app.add_viewer_selection_to_bundle();
        assert_eq!(app.bundle.len(), before + 1);

        let last = app.bundle.items.last().unwrap();
        match &last.kind {
            crate::bundle::ItemKind::Range(r) => {
                assert_eq!(r.start, 2);
                assert_eq!(r.end, 3);
            }
            other => panic!("expected Range, got {other:?}"),
        }
        // Selection clears after add.
        assert!(app.viewer.selection().is_none());
    }
}
