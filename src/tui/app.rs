//! App state for the TUI.

use crate::bundle::{Bundle, Item, ItemKind};
use crate::models;
use crate::paths::CtxforgeRoot;
use crate::resolve;
use crate::tokens;
use crate::tui::mode;
use crate::tui::tree::{self, TreeEntry};
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

        let model_name = bundle
            .model
            .clone()
            .unwrap_or_else(|| models::DEFAULT_MODEL.to_string());

        let mut app = App {
            root,
            project_root,
            bundle,
            tree_entries,
            tree_cursor: 0,
            bundle_cursor: 0,
            focus: Focus::FileTree,
            model_name,
            model_window: 0,
            item_tokens: Vec::new(),
            total_tokens: 0,
            exact_tokens: false,
            bundled_paths: HashSet::new(),
            mode: mode::Mode::Normal,
            should_quit: false,
            status_message: String::new(),
        };
        app.rebuild_bundled_paths();
        app.recalculate_tokens();
        app
    }

    /// Toggle selection of the file at the current tree cursor.
    pub fn toggle_current(&mut self) {
        let Some(entry) = self.tree_entries.get(self.tree_cursor) else {
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
        if self.tree_entries.is_empty() {
            return;
        }
        let new = self.tree_cursor as i32 + delta;
        self.tree_cursor = new.clamp(0, self.tree_entries.len() as i32 - 1) as usize;
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
