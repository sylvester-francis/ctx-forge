//! App state for the TUI.

use crate::bundle::{Bundle, Item, ItemKind};
use crate::models;
use crate::paths::CtxforgeRoot;
use crate::resolve;
use crate::tokens;
use crate::tui::mode;
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
    /// Active input/overlay mode. Drives key dispatch and overlay rendering.
    pub mode: mode::Mode,
    pub should_quit: bool,
    pub status_message: String,
}

impl App {
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
            mode: mode::Mode::Normal,
            should_quit: false,
            status_message: String::new(),
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
                matcher.fuzzy_match(&path_str, query).map(|score| (i, score))
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

    pub(crate) fn rebuild_bundled_paths(&mut self) {
        self.bundled_paths = self.bundle.items.iter().map(|i| i.path.clone()).collect();
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
