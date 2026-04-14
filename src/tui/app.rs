//! App state for the TUI.

use crate::bundle::{Bundle, Item, ItemKind, Range};
use crate::models;
use crate::paths::CtxforgeRoot;
use crate::resolve;
use crate::tokens;
use crate::tui::mode;
use crate::tui::motion::{AnimCtx, Clock, MotionLevel, SystemClock, constants};
use std::time::{Duration, Instant};
use crate::tui::tree::{self, TreeEntry};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use std::collections::HashSet;
use std::path::PathBuf;

/// Which panel has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    FileTree,
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
    pub fn set_mode(&mut self, next: mode::Mode) {
        let now = self.clock.now();
        let prev = std::mem::replace(&mut self.mode, next);
        let duration = match (prev.is_overlay(), self.mode.is_overlay()) {
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
    }

    /// Current animation context (clock time + motion level).
    pub fn anim_ctx(&self) -> AnimCtx {
        AnimCtx {
            now: self.clock.now(),
            motion: self.motion,
        }
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
        false
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
        };
        app.rebuild_bundled_paths();
        app.recalculate_tokens();
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
            self.status_message = format!("removed {}", path.display());
        } else {
            // Add to bundle.
            let item = Item {
                path: path.clone(),
                kind: ItemKind::File,
                label: None,
            };
            self.bundle.add(item);
            self.bundled_paths.insert(path.clone());
            self.status_message = format!("added {}", path.display());
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
                        self.status_message = format!(
                            "Copied {} items ({} tokens) to clipboard",
                            self.bundle.len(),
                            self.total_tokens
                        );
                    }
                    Err(e) => {
                        self.status_message = format!("Clipboard error: {e}");
                    }
                }
            }
            Err(e) => {
                self.status_message = format!("Resolve error: {e}");
            }
        }
    }

    pub fn move_tree_cursor(&mut self, delta: i32) {
        if self.visible_tree.is_empty() {
            return;
        }
        let new = self.tree_cursor as i32 + delta;
        self.tree_cursor = new.clamp(0, self.visible_tree.len() as i32 - 1) as usize;
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
                self.status_message = "Invalid start line".into();
                return;
            }
        };
        let end_num: usize = match end_str.parse() {
            Ok(n) if n >= start_num => n,
            _ => {
                self.status_message = "Invalid end line (must be >= start)".into();
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
        self.status_message = format!("Narrowed to lines {start_num}-{end_num}");
        self.set_mode(mode::Mode::Normal);
    }

    /// Save current bundle as a named profile under `.ctxforge/profiles/`.
    pub fn save_profile(&mut self, name: &str) {
        match crate::profile::save(&self.root, name, &self.bundle) {
            Ok(()) => {
                self.profile_name = Some(name.to_string());
                self.status_message = format!("Saved profile '{name}'");
            }
            Err(e) => {
                self.status_message = format!("Save error: {e}");
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
                    self.status_message = "No profiles saved yet".into();
                } else {
                    self.set_mode(mode::Mode::LoadProfile {
                        cursor: 0,
                        profiles,
                    });
                }
            }
            Err(e) => {
                self.status_message = format!("Profile list error: {e}");
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
                    self.status_message = format!("Loaded profile '{name}'");
                }
                Err(e) => {
                    self.status_message = format!("Load error: {e}");
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
                self.status_message = "Exported XML to stdout".into();
            }
            Err(e) => {
                self.status_message = format!("Export error: {e}");
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
        self.status_message = format!("Switched to {model_name}");
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
            self.status_message = "No functions found in project".into();
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
            self.status_message = "No types found in project".into();
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
            self.status_message = format!("Added fn:{name}");
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
            self.status_message = format!("Added type:{name}");
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
                    self.status_message = format!("No changes vs {branch}");
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
                self.status_message = format!("Diff error: {e}");
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
            self.status_message = format!("Added {count} changed file(s)");
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
                    self.status_message = "No memory notes yet".into();
                } else {
                    self.set_mode(mode::Mode::MemoryPanel { cursor: 0, count });
                }
            }
            Err(e) => {
                self.status_message = format!("Memory error: {e}");
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
            self.status_message = "Note body cannot be empty".into();
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
                self.status_message = format!("Noted: [{ts}] {}", note.body);
            }
            Err(e) => {
                self.status_message = format!("Note error: {e}");
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
                self.status_message = format!("Pipe error: {e}");
            }
        }
        self.set_mode(mode::Mode::Normal);
    }

    pub(crate) fn rebuild_bundled_paths(&mut self) {
        self.bundled_paths = self.bundle.items.iter().map(|i| i.path.clone()).collect();
    }

    /// Short status message setter (used by command table actions).
    pub fn set_status(&mut self, msg: &str) {
        self.status_message = msg.to_string();
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
                        self.status_message = format!("template error: {e}");
                        return;
                    }
                }
            }
        }
        let templates = scan_all_templates(&self.root);
        if templates.is_empty() {
            self.status_message =
                "no templates found; create one with `ctxforge templates new <name>`".into();
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
            self.status_message = "no templates found".into();
        } else {
            let names: Vec<String> = templates.iter().map(|(n, _)| n.clone()).collect();
            self.status_message = format!("templates: {}", names.join(", "));
        }
    }

    /// Scaffold a new project-local template via the command palette.
    pub fn run_template_new(&mut self, name: Option<String>) {
        let name = match name {
            Some(n) if !n.is_empty() => n,
            _ => {
                self.status_message = "usage: /template-new <name>".into();
                return;
            }
        };
        let dir = self.root.templates_dir();
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(format!("{name}.md"));
        if path.exists() {
            self.status_message = format!("template '{name}' already exists");
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
                self.status_message = format!("created template '{name}' at {}", path.display());
            }
            Err(e) => {
                self.status_message = format!("error creating template: {e}");
            }
        }
    }

    /// Delete a project-local template via the command palette.
    pub fn run_template_rm(&mut self, name: Option<String>) {
        let name = match name {
            Some(n) if !n.is_empty() => n,
            _ => {
                self.status_message = "usage: /template-rm <name>".into();
                return;
            }
        };
        let stem = name.strip_suffix(".md").unwrap_or(&name);
        let path = self.root.template_path(stem);
        if !path.exists() {
            self.status_message = format!("template '{stem}' not found");
            return;
        }
        match std::fs::remove_file(&path) {
            Ok(()) => {
                self.status_message = format!("deleted template '{stem}'");
            }
            Err(e) => {
                self.status_message = format!("error deleting template: {e}");
            }
        }
    }

    /// List built-in starter templates in a status message.
    pub fn run_template_starters(&mut self) {
        self.status_message =
            "starters: bugfix, code-review, explain, refactor, migrate (use CLI: ctxforge templates new <name> --from <starter>)".into();
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
                self.status_message = format!("resolve error: {e}");
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
                self.status_message = format!("template error: {e}");
                self.set_mode(mode::Mode::Normal);
                return;
            }
        };

        match crate::clipboard::set(&final_content) {
            Ok(()) => {
                self.status_message =
                    format!("copied template '{template_name}' with bundle + task");
            }
            Err(e) => {
                self.status_message = format!("clipboard error: {e}");
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
        let (app, _clock, _tmp) = test_app();
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
