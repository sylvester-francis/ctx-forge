//! Root component and entry point for the v2 TUI.
//!
//! Loads bundle, tree, and theme at startup, computes token counts, and
//! renders the full three-column layout with keyboard navigation (Tab
//! cycles focus, j/k move the tree cursor, q quits) and animated focus
//! borders via `use_animated`.

use crate::bundle::Bundle;
use crate::error::Result;
use crate::models;
use crate::motion_core::{constants, ease_out_cubic};
use crate::paths::{CtxforgeRoot, config_file_path};
use crate::preview::PromptPreview;
use crate::prompt_input::PromptInput;
use crate::resolve;
use crate::theme::{AppTheme, config::resolve_theme_name, registry};
use crate::tokens;
use crate::tree::{self, TreeEntry};
use crate::tui::components::{
    bundle_summary::render_bundle_rows, prompt_preview::render_preview, tree::render_tree_rows,
};
use crate::tui::motion::use_animated;
use crate::tui::theme::Theme;
use iocraft::hooks::UseTerminalSize;
use iocraft::prelude::*;
use std::collections::HashSet;
use std::path::PathBuf;

// ─── App state (mutable, held in use_state) ───────────────────────────

struct AppData {
    bundle: Bundle,
    tree_entries: Vec<TreeEntry>,
    item_tokens: Vec<usize>,
    total_tokens: usize,
    model_name: String,
    model_window: usize,
    theme: Theme,
    preview: PromptPreview,
    bundled_paths: HashSet<PathBuf>,
    // Needed so mutations can persist to disk and recompute tokens.
    root: CtxforgeRoot,
    project_root: PathBuf,
    // Transient status message shown in the footer. Auto-clears after 3s.
    status: String,
    status_set_at: Option<std::time::Instant>,
    // Code viewer pane state.
    viewer: crate::tui::viewer::ViewerState,
    // Action that requires leaving the render loop (export / pipe / editor).
    pending_action: Option<crate::tui::mode::PendingAction>,
    // Hand-edited prompt body saved by `edit-prompt`. When set,
    // `render_payload` returns this verbatim instead of re-rendering.
    // Persisted at `root/prompt-override.md` across TUI restarts.
    prompt_override: Option<String>,
}

fn load_app_data(root: CtxforgeRoot) -> AppData {
    // Pre-warm the shared syntect highlighter so the first viewer toggle
    // doesn't stall on SyntaxSet deserialization (~200ms).
    let _ = crate::tui::viewer::highlight::shared();

    let bundle = Bundle::load_or_default(&root).unwrap_or_default();
    let project_root = root.project_root().to_path_buf();
    let tree_entries = tree::build(&project_root);

    let model_name = bundle
        .model
        .clone()
        .unwrap_or_else(|| models::DEFAULT_MODEL.to_string());
    let model = models::lookup(&model_name);

    let resolved = resolve::resolve_all(&bundle.items, &project_root).unwrap_or_default();
    let item_tokens: Vec<usize> = resolved
        .iter()
        .map(|r| tokens::count(&r.content, &model).tokens)
        .collect();
    let total_tokens: usize = item_tokens.iter().sum();

    let theme_name = config_file_path()
        .map(|p| resolve_theme_name(&p))
        .unwrap_or_else(|| "ctxforge".to_string());
    let raw: &'static AppTheme =
        registry::by_name(&theme_name).unwrap_or_else(|| registry::default_theme());
    let theme = Theme::from_app_theme(raw);

    let bundled_paths: HashSet<PathBuf> = bundle
        .items
        .iter()
        .filter_map(|i| i.source.display_path().cloned())
        .collect();
    let preview = build_preview(&root, &bundle, &item_tokens);

    let prompt_override = std::fs::read_to_string(root.prompt_override_path()).ok();

    AppData {
        bundle,
        tree_entries,
        item_tokens,
        total_tokens,
        model_name,
        model_window: model.window,
        theme,
        preview,
        bundled_paths,
        root,
        project_root,
        status: String::new(),
        status_set_at: None,
        viewer: crate::tui::viewer::ViewerState::new(),
        pending_action: None,
        prompt_override,
    }
}

impl AppData {
    /// Toggle file at `rel_path` in/out of the bundle. No-op on directories.
    /// Only the affected item is resolved + tokenized — existing items keep
    /// their cached token counts.
    fn toggle_bundle(&mut self, rel_path: &std::path::Path) {
        use crate::bundle::Item;
        use crate::source::{FileSource, Source};
        if self.bundled_paths.contains(rel_path) {
            // Remove path: drop the matching token entry too. Bundle stores
            // items ordered; find the first File-kind item matching the path
            // and remove its parallel token entry.
            if let Some(idx) = self
                .bundle
                .items
                .iter()
                .position(|it| matches!(&it.source, Source::File(f) if f.path == rel_path))
            {
                self.bundle.items.remove(idx);
                if idx < self.item_tokens.len() {
                    let removed = self.item_tokens.remove(idx);
                    self.total_tokens = self.total_tokens.saturating_sub(removed);
                }
            } else {
                // Fallback: path exists but not a File-kind entry. Use the
                // existing helper which scans for any path match, and recount.
                self.bundle.remove_by_path(rel_path);
                self.recompute_tokens();
            }
            self.bundled_paths.remove(rel_path);
        } else {
            let item = Item {
                source: Source::File(FileSource {
                    path: rel_path.to_path_buf(),
                }),
                label: None,
            };
            let model = models::lookup(&self.model_name);
            let new_tokens: usize =
                resolve::resolve_all(std::slice::from_ref(&item), &self.project_root)
                    .map(|res| {
                        res.iter()
                            .map(|r| tokens::count(&r.content, &model).tokens)
                            .sum()
                    })
                    .unwrap_or(0);
            self.bundle.add(item);
            self.bundled_paths.insert(rel_path.to_path_buf());
            self.item_tokens.push(new_tokens);
            self.total_tokens += new_tokens;
        }
        self.preview = build_preview(&self.root, &self.bundle, &self.item_tokens);
        let _ = self.bundle.save(&self.root);
    }

    /// Toggle expanded state of the directory at `tree_idx` (absolute index
    /// into `tree_entries`). No-op on files.
    fn toggle_expanded(&mut self, tree_idx: usize) {
        if let Some(entry) = self.tree_entries.get_mut(tree_idx) {
            if entry.is_dir {
                entry.expanded = !entry.expanded;
            }
        }
    }

    fn expand_all(&mut self) {
        tree::expand_all(&mut self.tree_entries);
    }

    fn collapse_all(&mut self) {
        tree::collapse_all(&mut self.tree_entries);
    }

    fn set_status(&mut self, msg: String) {
        self.status = msg;
        self.status_set_at = Some(std::time::Instant::now());
    }

    /// Dispatch a command action. Returns the new Mode to enter, or None
    /// for actions that only set status / quit / stay in Normal mode.
    fn dispatch_command(
        &mut self,
        action: crate::tui::command_registry::CommandAction,
    ) -> Option<crate::tui::mode::Mode> {
        use crate::tui::command_registry::CommandAction as A;
        use crate::tui::mode::{Mode, PickerPurpose, TextPromptPurpose};
        match action {
            A::Help => Some(Mode::Help),
            A::Scenario => Some(Mode::ScenarioPicker { cursor: 0 }),
            A::Find => Some(Mode::Search {
                query: String::new(),
            }),
            A::Quit => {
                self.set_status("quit requested".to_string());
                None
            }
            A::Theme => Some(crate::tui::mode::Mode::ThemePicker { cursor: 0 }),
            A::ToggleViewer => {
                self.viewer.toggle();
                let enabled = self.viewer.enabled;
                self.set_status(if enabled {
                    "viewer on".to_string()
                } else {
                    "viewer off".to_string()
                });
                None
            }
            A::AddSelection => {
                self.add_viewer_selection_to_bundle();
                None
            }
            A::Deliver => Some(crate::tui::mode::Mode::DeliveryPicker { cursor: 0 }),
            A::EditPrompt => {
                let content = self
                    .render_payload(crate::format::Format::Markdown)
                    .unwrap_or_else(|e| format!("Error: {e}"));
                self.pending_action = Some(crate::tui::mode::PendingAction::Editor(content));
                None
            }
            A::Copy => {
                self.run_delivery(crate::deliver::DeliverChoice::CopyMarkdown);
                None
            }
            A::CopyXml => {
                self.run_delivery(crate::deliver::DeliverChoice::CopyXml);
                None
            }
            A::CopyJson => {
                self.run_delivery(crate::deliver::DeliverChoice::CopyJson);
                None
            }
            A::Export => {
                self.run_delivery(crate::deliver::DeliverChoice::Export);
                None
            }
            A::ExportXml => {
                self.run_delivery(crate::deliver::DeliverChoice::ExportXml);
                None
            }
            A::ExportJson => {
                self.run_delivery(crate::deliver::DeliverChoice::ExportJson);
                None
            }
            A::Pipe => Some(crate::tui::mode::Mode::DeliveryPicker { cursor: 0 }),
            A::SaveProfile => {
                self.pending_action = Some(crate::tui::mode::PendingAction::TextPrompt(
                    TextPromptPurpose::SaveProfile,
                ));
                None
            }
            A::LoadProfile => {
                let items = crate::profile::list(&self.root).unwrap_or_default();
                self.pending_action = Some(crate::tui::mode::PendingAction::PickerList {
                    purpose: PickerPurpose::LoadProfile,
                    items,
                });
                None
            }
            A::Narrow => {
                self.pending_action = Some(crate::tui::mode::PendingAction::TextPrompt(
                    TextPromptPurpose::Narrow,
                ));
                None
            }
            A::Model => {
                let items = crate::models::list_ids();
                self.pending_action = Some(crate::tui::mode::PendingAction::PickerList {
                    purpose: PickerPurpose::Model,
                    items,
                });
                None
            }
            A::Memory => {
                self.show_memory();
                None
            }
            A::Note => {
                self.pending_action = Some(crate::tui::mode::PendingAction::TextPrompt(
                    TextPromptPurpose::Note,
                ));
                None
            }
            A::FindFn => {
                self.pending_action = Some(crate::tui::mode::PendingAction::TextPrompt(
                    TextPromptPurpose::FindFn,
                ));
                None
            }
            A::FindType => {
                self.pending_action = Some(crate::tui::mode::PendingAction::TextPrompt(
                    TextPromptPurpose::FindType,
                ));
                None
            }
            A::FindDiff => {
                self.pending_action = Some(crate::tui::mode::PendingAction::TextPrompt(
                    TextPromptPurpose::FindDiff,
                ));
                None
            }
            A::Template => {
                let items = crate::template::list_all_names(&self.root);
                self.pending_action = Some(crate::tui::mode::PendingAction::PickerList {
                    purpose: PickerPurpose::ApplyTemplate,
                    items,
                });
                None
            }
            A::TemplateNew => {
                self.pending_action = Some(crate::tui::mode::PendingAction::TextPrompt(
                    TextPromptPurpose::TemplateNew,
                ));
                None
            }
            A::TemplateRm => {
                let items = crate::template::list_project_names(&self.root);
                self.pending_action = Some(crate::tui::mode::PendingAction::PickerList {
                    purpose: PickerPurpose::RemoveTemplate,
                    items,
                });
                None
            }
            A::TemplateStarters => {
                self.show_template_starters();
                None
            }
            A::TemplateList => {
                self.show_template_list();
                None
            }
            A::DocsAdd => {
                self.pending_action = Some(crate::tui::mode::PendingAction::TextPrompt(
                    TextPromptPurpose::DocsAdd,
                ));
                None
            }
            A::DocsRm => {
                self.pending_action = Some(crate::tui::mode::PendingAction::TextPrompt(
                    TextPromptPurpose::DocsRm,
                ));
                None
            }
            A::AddUrl => {
                self.pending_action = Some(crate::tui::mode::PendingAction::TextPrompt(
                    TextPromptPurpose::AddUrl,
                ));
                None
            }
            A::ClearPromptOverride => {
                self.clear_prompt_override();
                None
            }
            A::DocsDetect => {
                self.run_docs_detect(false);
                None
            }
            A::DocsDetectAll => {
                self.run_docs_detect(true);
                None
            }
            A::DocsRefresh => {
                self.run_docs_refresh();
                None
            }
            A::DocsList => {
                self.run_docs_list();
                None
            }
            A::UrlRefresh => {
                self.run_url_refresh();
                None
            }
            A::CacheList => {
                self.run_cache_list();
                None
            }
            A::CacheClear => {
                self.run_cache_clear();
                None
            }
            A::CacheVerify => {
                self.run_cache_verify();
                None
            }
        }
    }

    fn show_memory(&mut self) {
        use crate::memory;
        let notes = memory::index::read_all(&self.root).unwrap_or_default();
        if notes.is_empty() {
            self.set_status("no memory notes yet".to_string());
            return;
        }
        let body = notes
            .iter()
            .rev()
            .take(50)
            .map(|n| {
                let ts = n.timestamp.format("%Y-%m-%d %H:%M").to_string();
                match &n.tag {
                    Some(t) => format!("[{ts}] [{t}] {}", n.body),
                    None => format!("[{ts}] {}", n.body),
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        self.pending_action = Some(crate::tui::mode::PendingAction::Editor(body));
    }

    fn show_template_list(&mut self) {
        let project = crate::template::list_project_names(&self.root);
        let global = crate::template::list_all_names(&self.root)
            .into_iter()
            .filter(|n| !project.contains(n))
            .collect::<Vec<_>>();
        let summary = format!(
            "templates — {} project, {} global",
            project.len(),
            global.len(),
        );
        if project.is_empty() && global.is_empty() {
            self.set_status("no templates yet — ctxforge templates new <name>".to_string());
            return;
        }
        self.set_status(format!(
            "{summary} · {}",
            project
                .iter()
                .chain(global.iter())
                .cloned()
                .collect::<Vec<_>>()
                .join(", "),
        ));
    }

    fn show_template_starters(&mut self) {
        let names = crate::template::list_starter_names()
            .into_iter()
            .map(|(n, _)| n.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        self.set_status(format!("starters: {names}"));
    }

    /// Submit a text-prompt value. Called by the key handler when the user
    /// presses Enter inside `Mode::TextPrompt` (reserved for future in-TUI
    /// overlay; current flow uses `PendingAction::TextPrompt` +
    /// `handle_text_prompt`).
    #[allow(dead_code)]
    pub(crate) fn submit_text_prompt(
        &mut self,
        purpose: crate::tui::mode::TextPromptPurpose,
        input: String,
    ) {
        use crate::tui::mode::TextPromptPurpose as P;
        let trimmed = input.trim().to_string();
        if trimmed.is_empty() {
            self.set_status("input was empty".to_string());
            return;
        }
        match purpose {
            P::SaveProfile => match crate::profile::save(&self.root, &trimmed, &self.bundle) {
                Ok(()) => self.set_status(format!("saved profile `{trimmed}`")),
                Err(e) => self.set_status(format!("save profile: {e}")),
            },
            P::Narrow => {
                // format: path:start-end
                match parse_narrow(&trimmed) {
                    Ok((path, start, end)) => {
                        use crate::source::{RangeSource, Source};
                        match RangeSource::new(path.into(), start, end) {
                            Ok(rs) => {
                                self.bundle.add(crate::bundle::Item {
                                    source: Source::Range(rs),
                                    label: None,
                                });
                                let _ = self.bundle.save(&self.root);
                                self.recompute_tokens();
                                self.preview =
                                    build_preview(&self.root, &self.bundle, &self.item_tokens);
                                self.set_status(format!("narrowed {trimmed}"));
                            }
                            Err(e) => self.set_status(format!("narrow: {e}")),
                        }
                    }
                    Err(e) => self.set_status(format!("narrow: {e}")),
                }
            }
            P::Note => {
                // Input is body; tag is ignored for MVP (the CLI supports a
                // separate --tag flag; the TUI wraps it in body for now).
                use crate::memory::{self, Note};
                let note = Note::new(trimmed.clone(), None);
                match memory::index::append(&self.root, &note) {
                    Ok(()) => self.set_status("note saved".to_string()),
                    Err(e) => self.set_status(format!("note: {e}")),
                }
            }
            P::FindFn => match crate::commands::add::run(
                &self.root,
                &self.project_root,
                Vec::new(),
                Vec::new(),
                None,
                vec![trimmed.clone()],
                Vec::new(),
                false,
                false,
            ) {
                Ok(()) => {
                    self.reload_bundle();
                    self.set_status(format!("added fn:{trimmed}"));
                }
                Err(e) => self.set_status(format!("find fn: {e}")),
            },
            P::FindType => match crate::commands::add::run(
                &self.root,
                &self.project_root,
                Vec::new(),
                Vec::new(),
                None,
                Vec::new(),
                vec![trimmed.clone()],
                false,
                false,
            ) {
                Ok(()) => {
                    self.reload_bundle();
                    self.set_status(format!("added type:{trimmed}"));
                }
                Err(e) => self.set_status(format!("find type: {e}")),
            },
            P::FindDiff => match crate::commands::add::run(
                &self.root,
                &self.project_root,
                Vec::new(),
                Vec::new(),
                Some(trimmed.clone()),
                Vec::new(),
                Vec::new(),
                false,
                false,
            ) {
                Ok(()) => {
                    self.reload_bundle();
                    self.set_status(format!("added diff vs {trimmed}"));
                }
                Err(e) => self.set_status(format!("find diff: {e}")),
            },
            P::TemplateNew => {
                use crate::cli::TemplatesAction;
                match crate::commands::template::run(
                    &self.root,
                    Some(TemplatesAction::New {
                        name: trimmed.clone(),
                        from: None,
                    }),
                ) {
                    Ok(()) => self.set_status(format!("template new: {trimmed}")),
                    Err(e) => self.set_status(format!("template new: {e}")),
                }
            }
            P::DocsAdd => match crate::commands::docs_cmd::add(&self.root, trimmed.clone(), None) {
                Ok(()) => {
                    self.reload_bundle();
                    self.set_status(format!("docs add: {trimmed}"));
                }
                Err(e) => self.set_status(format!("docs add: {e}")),
            },
            P::DocsRm => match crate::commands::docs_cmd::rm(&self.root, trimmed.clone()) {
                Ok(()) => {
                    self.reload_bundle();
                    self.set_status(format!("docs rm: {trimmed}"));
                }
                Err(e) => self.set_status(format!("docs rm: {e}")),
            },
            P::AddUrl => {
                use crate::bundle::Item;
                use crate::source::{Source, UrlSource};
                match crate::source::url::validate_url(&trimmed, false, false) {
                    Ok(()) => {
                        self.bundle.add(Item {
                            source: Source::Url(UrlSource {
                                url: trimmed.clone(),
                            }),
                            label: None,
                        });
                        let _ = self.bundle.save(&self.root);
                        self.reload_bundle();
                        self.set_status(format!("added url: {trimmed}"));
                    }
                    Err(e) => self.set_status(format!("url add: {e}")),
                }
            }
        }
    }

    /// Submit a picker selection from `Mode::PickerList` (reserved for
    /// future in-TUI overlay; current flow uses `PendingAction::PickerList`
    /// + `handle_picker`).
    #[allow(dead_code)]
    pub(crate) fn submit_picker(
        &mut self,
        purpose: crate::tui::mode::PickerPurpose,
        choice: String,
    ) {
        use crate::tui::mode::PickerPurpose as P;
        match purpose {
            P::LoadProfile => match crate::commands::load::run(&self.root, &choice) {
                Ok(()) => {
                    self.reload_bundle();
                    self.set_status(format!("loaded profile `{choice}`"));
                }
                Err(e) => self.set_status(format!("load profile: {e}")),
            },
            P::Model => {
                self.model_name = choice.clone();
                let m = crate::models::lookup(&self.model_name);
                self.model_window = m.window;
                self.recompute_tokens();
                self.preview = build_preview(&self.root, &self.bundle, &self.item_tokens);
                self.set_status(format!("model → {choice}"));
            }
            P::ApplyTemplate => {
                self.bundle.scenario = Some(choice.clone());
                let _ = self.bundle.save(&self.root);
                self.preview = build_preview(&self.root, &self.bundle, &self.item_tokens);
                self.set_status(format!("scenario → {choice}"));
            }
            P::RemoveTemplate => {
                use crate::cli::TemplatesAction;
                match crate::commands::template::run(
                    &self.root,
                    Some(TemplatesAction::Rm {
                        name: choice.clone(),
                    }),
                ) {
                    Ok(()) => self.set_status(format!("template rm: {choice}")),
                    Err(e) => self.set_status(format!("template rm: {e}")),
                }
            }
        }
    }

    fn clear_prompt_override(&mut self) {
        let path = self.root.prompt_override_path();
        if path.exists() {
            match std::fs::remove_file(&path) {
                Ok(()) => {
                    self.prompt_override = None;
                    self.set_status("prompt override cleared".to_string());
                }
                Err(e) => self.set_status(format!("clear override: {e}")),
            }
        } else {
            self.set_status("no prompt override to clear".to_string());
        }
    }

    #[allow(dead_code)]
    fn reload_bundle(&mut self) {
        match crate::bundle::Bundle::load_or_default(&self.root) {
            Ok(b) => {
                self.bundle = b;
                self.recompute_tokens();
                self.preview = build_preview(&self.root, &self.bundle, &self.item_tokens);
            }
            Err(e) => self.set_status(format!("bundle reload: {e}")),
        }
    }
}

/// Suspended-TUI handler for `PendingAction::TextPrompt`. Reads the root
/// via the thread-local `ROOT_STASH` so it doesn't need `AppData`.
fn handle_text_prompt(purpose: crate::tui::mode::TextPromptPurpose) {
    use crate::tui::mode::TextPromptPurpose as P;
    let Some(root) = ROOT_STASH.with(|r| r.borrow().clone()) else {
        eprintln!("ctxforge root not available");
        return;
    };

    eprintln!("ctxforge · {}", purpose.label());
    let input: String = match dialoguer::Input::new()
        .with_prompt(purpose.label())
        .allow_empty(false)
        .interact_text()
    {
        Ok(v) => v,
        Err(_) => {
            eprintln!("cancelled");
            return;
        }
    };
    let trimmed = input.trim().to_string();

    let outcome: std::result::Result<String, String> = match purpose {
        P::SaveProfile => (|| -> std::result::Result<String, String> {
            let bundle =
                crate::bundle::Bundle::load_or_default(&root).map_err(|e| e.to_string())?;
            crate::profile::save(&root, &trimmed, &bundle).map_err(|e| e.to_string())?;
            Ok(format!("saved profile `{trimmed}`"))
        })(),
        P::Narrow => (|| -> std::result::Result<String, String> {
            let (path, start, end) = parse_narrow(&trimmed)?;
            use crate::bundle::{Bundle, Item};
            use crate::source::{RangeSource, Source};
            let mut bundle = Bundle::load_or_default(&root).map_err(|e| e.to_string())?;
            let rs = RangeSource::new(path.into(), start, end).map_err(|e| e.to_string())?;
            bundle.add(Item {
                source: Source::Range(rs),
                label: None,
            });
            bundle.save(&root).map_err(|e| e.to_string())?;
            Ok(format!("narrowed {trimmed}"))
        })(),
        P::Note => {
            use crate::memory::{self, Note};
            let note = Note::new(trimmed.clone(), None);
            memory::index::append(&root, &note)
                .map(|()| "note saved".to_string())
                .map_err(|e| e.to_string())
        }
        P::FindFn => crate::commands::add::run(
            &root,
            root.project_root(),
            Vec::new(),
            Vec::new(),
            None,
            vec![trimmed.clone()],
            Vec::new(),
            false,
            false,
        )
        .map(|()| format!("added fn:{trimmed}"))
        .map_err(|e| e.to_string()),
        P::FindType => crate::commands::add::run(
            &root,
            root.project_root(),
            Vec::new(),
            Vec::new(),
            None,
            Vec::new(),
            vec![trimmed.clone()],
            false,
            false,
        )
        .map(|()| format!("added type:{trimmed}"))
        .map_err(|e| e.to_string()),
        P::FindDiff => crate::commands::add::run(
            &root,
            root.project_root(),
            Vec::new(),
            Vec::new(),
            Some(trimmed.clone()),
            Vec::new(),
            Vec::new(),
            false,
            false,
        )
        .map(|()| format!("added diff vs {trimmed}"))
        .map_err(|e| e.to_string()),
        P::TemplateNew => {
            use crate::cli::TemplatesAction;
            crate::commands::template::run(
                &root,
                Some(TemplatesAction::New {
                    name: trimmed.clone(),
                    from: None,
                }),
            )
            .map(|()| format!("template new: {trimmed}"))
            .map_err(|e| e.to_string())
        }
        P::DocsAdd => crate::commands::docs_cmd::add(&root, trimmed.clone(), None)
            .map(|()| format!("docs add: {trimmed}"))
            .map_err(|e| e.to_string()),
        P::DocsRm => crate::commands::docs_cmd::rm(&root, trimmed.clone())
            .map(|()| format!("docs rm: {trimmed}"))
            .map_err(|e| e.to_string()),
        P::AddUrl => (|| -> std::result::Result<String, String> {
            use crate::bundle::{Bundle, Item};
            use crate::source::{Source, UrlSource};
            crate::source::url::validate_url(&trimmed, false, false)?;
            let mut bundle = Bundle::load_or_default(&root).map_err(|e| e.to_string())?;
            bundle.add(Item {
                source: Source::Url(UrlSource {
                    url: trimmed.clone(),
                }),
                label: None,
            });
            bundle.save(&root).map_err(|e| e.to_string())?;
            Ok(format!("added url: {trimmed}"))
        })(),
    };

    match outcome {
        Ok(msg) => eprintln!("✓ {msg}"),
        Err(e) => eprintln!("✗ {e}"),
    }
}

/// Suspended-TUI handler for `PendingAction::PickerList`.
fn handle_picker(purpose: crate::tui::mode::PickerPurpose, items: Vec<String>) {
    use crate::tui::mode::PickerPurpose as P;
    let Some(root) = ROOT_STASH.with(|r| r.borrow().clone()) else {
        eprintln!("ctxforge root not available");
        return;
    };

    if items.is_empty() {
        eprintln!("no items to pick — nothing to do");
        return;
    }

    eprintln!("ctxforge · {}", purpose.label());
    let selection = match dialoguer::Select::new()
        .with_prompt(purpose.label())
        .items(&items)
        .default(0)
        .interact()
    {
        Ok(idx) => items[idx].clone(),
        Err(_) => {
            eprintln!("cancelled");
            return;
        }
    };

    let outcome: std::result::Result<String, String> = match purpose {
        P::LoadProfile => crate::commands::load::run(&root, &selection)
            .map(|()| format!("loaded profile `{selection}`"))
            .map_err(|e| e.to_string()),
        P::Model => {
            // Model picker only stashes the choice; persistence happens
            // via the config file, which the TUI reads on restart. For
            // one-session use, the user will re-pick next launch.
            let path = crate::paths::config_file_path();
            if let Some(p) = path {
                let existing = crate::theme::config::load_from(&p).unwrap_or_default();
                let _ = crate::theme::config::save_to(
                    &p,
                    &crate::theme::config::Config {
                        theme: existing.theme,
                        default_send: existing.default_send,
                    },
                );
            }
            Ok(format!("model → {selection} (restart TUI to apply)"))
        }
        P::ApplyTemplate => (|| -> std::result::Result<String, String> {
            let mut bundle =
                crate::bundle::Bundle::load_or_default(&root).map_err(|e| e.to_string())?;
            bundle.scenario = Some(selection.clone());
            bundle.save(&root).map_err(|e| e.to_string())?;
            Ok(format!("scenario → {selection}"))
        })(),
        P::RemoveTemplate => {
            use crate::cli::TemplatesAction;
            crate::commands::template::run(
                &root,
                Some(TemplatesAction::Rm {
                    name: selection.clone(),
                }),
            )
            .map(|()| format!("template rm: {selection}"))
            .map_err(|e| e.to_string())
        }
    };

    match outcome {
        Ok(msg) => eprintln!("✓ {msg}"),
        Err(e) => eprintln!("✗ {e}"),
    }
}

fn parse_narrow(input: &str) -> std::result::Result<(String, usize, usize), String> {
    // Format: path:start-end
    let (path, range) = input.rsplit_once(':').ok_or("expected path:start-end")?;
    let (start_s, end_s) = range.split_once('-').ok_or("expected start-end")?;
    let start: usize = start_s.parse().map_err(|_| "start not numeric")?;
    let end: usize = end_s.parse().map_err(|_| "end not numeric")?;
    Ok((path.to_string(), start, end))
}

impl AppData {
    fn run_docs_detect(&mut self, all: bool) {
        match crate::commands::docs_cmd::detect(&self.root, all, None) {
            Ok(()) => {
                // Reload bundle so the freshly-attached docs items show up in
                // the preview + status line without restarting the TUI.
                match crate::bundle::Bundle::load_or_default(&self.root) {
                    Ok(b) => {
                        self.bundle = b;
                        self.recompute_tokens();
                        self.preview = build_preview(&self.root, &self.bundle, &self.item_tokens);
                        let docs_count = self
                            .bundle
                            .items
                            .iter()
                            .filter(|i| matches!(&i.source, crate::source::Source::Docs(_)))
                            .count();
                        self.set_status(format!("docs detected ({docs_count} attached)"));
                    }
                    Err(e) => self.set_status(format!("docs detect reload: {e}")),
                }
            }
            Err(e) => self.set_status(format!("docs detect: {e}")),
        }
    }

    fn run_docs_refresh(&mut self) {
        match crate::commands::docs_cmd::refresh(&self.root) {
            Ok(()) => {
                if let Ok(b) = crate::bundle::Bundle::load_or_default(&self.root) {
                    self.bundle = b;
                    self.recompute_tokens();
                    self.preview = build_preview(&self.root, &self.bundle, &self.item_tokens);
                }
                self.set_status("docs refreshed".to_string());
            }
            Err(e) => self.set_status(format!("docs refresh: {e}")),
        }
    }

    fn run_docs_list(&mut self) {
        use crate::source::Source;
        let docs_items: Vec<&crate::source::DocsSource> = self
            .bundle
            .items
            .iter()
            .filter_map(|i| match &i.source {
                Source::Docs(d) => Some(d),
                _ => None,
            })
            .collect();
        if docs_items.is_empty() {
            self.set_status("no docs items in bundle".to_string());
            return;
        }
        let mut by_eco: std::collections::BTreeMap<&str, usize> = Default::default();
        for d in &docs_items {
            *by_eco.entry(d.ecosystem.as_str()).or_insert(0) += 1;
        }
        let breakdown = by_eco
            .iter()
            .map(|(k, v)| format!("{v} {k}"))
            .collect::<Vec<_>>()
            .join(" · ");
        self.set_status(format!("{} docs items ({breakdown})", docs_items.len()));
    }

    fn run_url_refresh(&mut self) {
        match crate::commands::refresh::run(&self.root, None, false) {
            Ok(()) => self.set_status("URL sources refreshed".to_string()),
            Err(e) => self.set_status(format!("url refresh: {e}")),
        }
    }

    fn run_cache_list(&mut self) {
        let Some(dir) = crate::paths::global_cache_dir() else {
            self.set_status("cache: no XDG/HOME dir".to_string());
            return;
        };
        if !dir.exists() {
            self.set_status("cache is empty".to_string());
            return;
        }
        match crate::cache::ContentCache::open(dir) {
            Ok(cache) => match cache.list() {
                Ok(metas) => self.set_status(format!("{} cached entries", metas.len())),
                Err(e) => self.set_status(format!("cache list: {e}")),
            },
            Err(e) => self.set_status(format!("cache open: {e}")),
        }
    }

    fn run_cache_clear(&mut self) {
        let Some(dir) = crate::paths::global_cache_dir() else {
            self.set_status("cache: no XDG/HOME dir".to_string());
            return;
        };
        if !dir.exists() {
            self.set_status("cache already empty".to_string());
            return;
        }
        match crate::cache::ContentCache::open(dir) {
            Ok(cache) => match cache.clear() {
                Ok(count) => self.set_status(format!("cleared {count} cached entries")),
                Err(e) => self.set_status(format!("cache clear: {e}")),
            },
            Err(e) => self.set_status(format!("cache open: {e}")),
        }
    }

    fn run_cache_verify(&mut self) {
        let Some(dir) = crate::paths::global_cache_dir() else {
            self.set_status("cache: no XDG/HOME dir".to_string());
            return;
        };
        if !dir.exists() {
            self.set_status("cache is empty — nothing to verify".to_string());
            return;
        }
        match crate::cache::ContentCache::open(dir) {
            Ok(cache) => match cache.verify() {
                Ok(r) => {
                    if r.body_mismatch.is_empty()
                        && r.hmac_mismatch.is_empty()
                        && r.read_error.is_empty()
                    {
                        self.set_status(format!("cache verified — {} entries OK", r.ok));
                    } else {
                        self.set_status(format!(
                            "cache integrity issues: {} body, {} hmac, {} read",
                            r.body_mismatch.len(),
                            r.hmac_mismatch.len(),
                            r.read_error.len(),
                        ));
                    }
                }
                Err(e) => self.set_status(format!("cache verify: {e}")),
            },
            Err(e) => self.set_status(format!("cache open: {e}")),
        }
    }

    /// Apply a theme by name. Swaps the active theme, rebuilds the preview
    /// (so any theme-dependent colors re-render), and persists the selection
    /// to `~/.config/ctxforge/config.toml`.
    fn apply_theme(&mut self, name: &str) -> std::result::Result<(), String> {
        let raw = crate::theme::registry::by_name(name)
            .ok_or_else(|| format!("unknown theme '{name}'"))?;
        self.theme = Theme::from_app_theme(raw);
        self.preview = build_preview(&self.root, &self.bundle, &self.item_tokens);

        if let Some(path) = crate::paths::config_file_path() {
            let existing = crate::theme::config::load_from(&path).unwrap_or_default();
            let next = crate::theme::config::Config {
                theme: name.to_string(),
                default_send: existing.default_send,
            };
            let _ = crate::theme::config::save_to(&path, &next);
        }
        self.set_status(format!("theme → {name}"));
        Ok(())
    }

    /// Add the viewer's current selection to the bundle as a Range item.
    /// Only the NEW item is resolved and tokenized — existing bundle items
    /// keep their cached token counts. Noticeably faster than a full
    /// `recompute_tokens()` call for large bundles.
    fn add_viewer_selection_to_bundle(&mut self) {
        use crate::bundle::Item;
        use crate::source::{RangeSource, Source};
        let (start0, end0) = match self.viewer.selection {
            Some(range) => range,
            None => {
                self.set_status("no selection — click-drag in the viewer first".to_string());
                return;
            }
        };
        let path = match &self.viewer.cached_path {
            Some(p) => p.clone(),
            None => {
                self.set_status("viewer has no file loaded".to_string());
                return;
            }
        };
        let rel_path = path
            .strip_prefix(&self.project_root)
            .unwrap_or(&path)
            .to_path_buf();
        let range_source = match RangeSource::new(rel_path.clone(), start0 + 1, end0 + 1) {
            Ok(r) => r,
            Err(e) => {
                self.set_status(format!("invalid selection: {e}"));
                return;
            }
        };
        let item = Item {
            source: Source::Range(range_source),
            label: None,
        };

        // Tokenize ONLY the new item — resolve_all is O(items × file size).
        let model = models::lookup(&self.model_name);
        let new_tokens: usize =
            resolve::resolve_all(std::slice::from_ref(&item), &self.project_root)
                .map(|res| {
                    res.iter()
                        .map(|r| tokens::count(&r.content, &model).tokens)
                        .sum()
                })
                .unwrap_or(0);

        self.bundle.add(item);
        self.bundled_paths.insert(rel_path.clone());
        self.item_tokens.push(new_tokens);
        self.total_tokens += new_tokens;
        self.viewer.clear_selection();
        self.preview = build_preview(&self.root, &self.bundle, &self.item_tokens);
        let _ = self.bundle.save(&self.root);
        self.set_status(format!(
            "added {}:{}-{} (+{} tokens)",
            rel_path.display(),
            start0 + 1,
            end0 + 1,
            new_tokens,
        ));
    }

    /// Render the delivery payload for the given format. If the user has
    /// a prompt-override saved (via `edit-prompt`), that text is returned
    /// verbatim regardless of format — the hand-edited prompt is the
    /// authoritative artifact.
    fn render_payload(&self, format: crate::format::Format) -> std::result::Result<String, String> {
        if let Some(override_body) = &self.prompt_override {
            return Ok(override_body.clone());
        }
        let resolved = resolve::resolve_all(&self.bundle.items, &self.project_root)
            .map_err(|e| format!("resolve bundle: {e}"))?;
        let notes = crate::memory::index::read_all(&self.root).unwrap_or_default();
        let bundle_rendered = crate::format::render(format, &resolved, &notes, true);

        if let Some(scenario) = &self.bundle.scenario {
            let body = crate::scenario::load_body(&self.root, scenario)
                .map_err(|e| format!("load scenario: {e}"))?;
            crate::template::substitute(scenario, &body, &bundle_rendered, &self.bundle.task_text)
                .map_err(|e| format!("render template: {e}"))
        } else {
            Ok(bundle_rendered)
        }
    }

    /// Execute a delivery choice. Copy goes to clipboard (no suspend needed).
    /// Pipe/Export stash a PendingAction for the outer run() loop.
    fn run_delivery(&mut self, choice: crate::deliver::DeliverChoice) {
        use crate::deliver::DeliverChoice as DC;
        use crate::tui::mode::PendingAction;

        let format = match choice {
            DC::PipeClaude | DC::CopyXml | DC::ExportXml => crate::format::Format::Xml,
            DC::CopyJson | DC::ExportJson => crate::format::Format::Json,
            _ => crate::format::Format::Markdown,
        };

        let content = match self.render_payload(format) {
            Ok(c) => c,
            Err(e) => {
                self.set_status(format!("delivery failed: {e}"));
                return;
            }
        };

        match choice {
            DC::CopyMarkdown | DC::CopyXml | DC::CopyJson => {
                match crate::clipboard::set(&content) {
                    Ok(()) => {
                        let label = match choice {
                            DC::CopyMarkdown => "markdown",
                            DC::CopyXml => "XML",
                            DC::CopyJson => "JSON",
                            _ => "content",
                        };
                        self.set_status(format!(
                            "copied as {} · {} chars",
                            label,
                            content.chars().count()
                        ));
                    }
                    Err(e) => self.set_status(format!("clipboard: {e}")),
                }
            }
            DC::PipeClaude => {
                self.pending_action = Some(PendingAction::Pipe {
                    target: "claude".to_string(),
                    content,
                });
            }
            DC::PipeAgent => {
                self.pending_action = Some(PendingAction::Pipe {
                    target: "agent".to_string(),
                    content,
                });
            }
            DC::PipeGemini => {
                self.pending_action = Some(PendingAction::Pipe {
                    target: "gemini".to_string(),
                    content,
                });
            }
            DC::Export | DC::ExportXml | DC::ExportJson => {
                self.pending_action = Some(PendingAction::Export(content));
            }
        }
    }

    /// Set the active scenario and rebuild the preview. Persists to disk.
    fn set_scenario(&mut self, name: Option<String>) {
        self.bundle.scenario = name;
        self.preview = build_preview(&self.root, &self.bundle, &self.item_tokens);
        let _ = self.bundle.save(&self.root);
    }

    /// Write task_text into the bundle and rebuild the preview. Persists to disk.
    fn sync_task_text(&mut self, text: String) {
        self.bundle.task_text = text;
        self.preview = build_preview(&self.root, &self.bundle, &self.item_tokens);
        let _ = self.bundle.save(&self.root);
    }

    /// Recompute `item_tokens` and `total_tokens` from the current bundle.
    fn recompute_tokens(&mut self) {
        let model = models::lookup(&self.model_name);
        self.model_window = model.window;
        let resolved =
            resolve::resolve_all(&self.bundle.items, &self.project_root).unwrap_or_default();
        self.item_tokens = resolved
            .iter()
            .map(|r| tokens::count(&r.content, &model).tokens)
            .collect();
        self.total_tokens = self.item_tokens.iter().sum();
    }
}

/// Payload written by a completed background viewer load. The generation
/// tag lets `apply_bg_load` drop stale results (from files the user has
/// since navigated past while the load was in flight).
type ViewerBgResult = (u64, PathBuf, crate::tui::viewer::ViewerLoad);
type ViewerBgSlot = State<Option<ViewerBgResult>>;

/// Fire a background file load and return immediately. The task runs on
/// smol's global executor; the blocking read+highlight runs on
/// `blocking`'s dedicated thread pool via `smol::unblock`, so neither
/// the render thread nor smol's worker thread blocks while the file is
/// read and syntect-highlighted. Uses the shared LazyLock highlighter so
/// the SyntaxSet deserialization is paid once across the app lifetime.
///
/// Multiple in-flight loads are safe because each task tags its result
/// with its own generation; `apply_bg_load` drops stale ones. The task
/// is detached so we don't hold the `Task` handle (dropping it would
/// cancel the work before the result landed).
fn spawn_viewer_load(path: PathBuf, generation: u64, mut result_slot: ViewerBgSlot) {
    smol::spawn(async move {
        let path_for_task = path.clone();
        let load = smol::unblock(move || {
            crate::tui::viewer::read_and_highlight(
                &path_for_task,
                crate::tui::viewer::highlight::shared(),
            )
        })
        .await;
        // Single `set()`, no retry: `State::set` uses `try_write` and
        // silently drops on contention, which would lose this result.
        // In practice the slot is virtually never contended (only during
        // the render's drain, which is a few microseconds), and the
        // user can retrigger by moving the cursor if a load does get
        // lost. An earlier retry-loop version hung startup on some
        // terminals.
        result_slot.set(Some((generation, path, load)));
    })
    .detach();
}

/// Reload the viewer for the file at `new_cursor` in the visible tree.
/// No-op on directories, on the cursor's current file, or when the viewer
/// is disabled. Dispatches the load to a background task so the event
/// thread stays responsive.
fn reload_viewer(data: &mut AppData, new_cursor: usize, slot: ViewerBgSlot) {
    if !data.viewer.enabled {
        return;
    }
    let visible = tree::visible_indices(&data.tree_entries);
    let Some(&idx) = visible.get(new_cursor) else {
        return;
    };
    let Some(entry) = data.tree_entries.get(idx).cloned() else {
        return;
    };
    if entry.is_dir {
        return;
    }
    let abs = data.project_root.join(&entry.rel_path);
    if data.viewer.cached_path.as_deref() == Some(abs.as_path()) {
        return;
    }
    let generation = data.viewer.begin_load();
    spawn_viewer_load(abs, generation, slot);
}

fn build_preview(root: &CtxforgeRoot, bundle: &Bundle, item_tokens: &[usize]) -> PromptPreview {
    use crate::preview::{ContextItem, Section};
    let mut sections = Vec::new();
    match &bundle.scenario {
        None => sections.push(Section::NoScenarioPlaceholder),
        Some(name) => match crate::preview::render::load_wrapped(root, name) {
            Ok(wrapped) => {
                sections.push(Section::ScenarioHeader {
                    scenario: name.clone(),
                    prefix_lines: wrapped
                        .prefix
                        .lines()
                        .filter(|l| !l.trim().is_empty())
                        .count(),
                    suffix_lines: wrapped
                        .suffix
                        .lines()
                        .filter(|l| !l.trim().is_empty())
                        .count(),
                });
                sections.push(Section::Task {
                    text: bundle.task_text.clone(),
                });
                let items =
                    bundle
                        .items
                        .iter()
                        .zip(item_tokens.iter())
                        .map(|(it, tok)| ContextItem {
                            path: it.source.display_path().cloned().unwrap_or_else(|| {
                                std::path::PathBuf::from(it.source.display_label())
                            }),
                            tokens: *tok,
                        })
                        .collect();
                sections.push(Section::Context { items });
            }
            Err(e) => sections.push(Section::TemplateError {
                scenario: name.clone(),
                error: e,
            }),
        },
    }
    PromptPreview { sections }
}

// ─── Focus ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Focus {
    FileTree,
    Viewer,
    BundleList,
    Prompt,
}

fn focus_color_rgb(focus: Focus, theme: &Theme) -> (u8, u8, u8) {
    let c = match focus {
        Focus::FileTree => theme.focus_tree,
        Focus::Viewer => theme.focus_viewer,
        Focus::BundleList => theme.focus_bundle,
        Focus::Prompt => theme.accent,
    };
    match c {
        iocraft::Color::Rgb { r, g, b } => (r, g, b),
        _ => (0, 255, 255),
    }
}

fn format_tokens(n: usize) -> String {
    if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        format!("{n}")
    }
}

fn gauge_color(pct: f64, theme: &Theme) -> Color {
    if pct > 90.0 {
        theme.danger
    } else if pct > 75.0 {
        theme.hotspot
    } else if pct > 40.0 {
        theme.warning
    } else {
        theme.success
    }
}

// ─── Entry point ──────────────────────────────────────────────────────

thread_local! {
    static STARTUP: std::cell::RefCell<Option<AppData>> = const { std::cell::RefCell::new(None) };
    static PENDING: std::cell::RefCell<Option<crate::tui::mode::PendingAction>> = const { std::cell::RefCell::new(None) };
    static ROOT_STASH: std::cell::RefCell<Option<CtxforgeRoot>> = const { std::cell::RefCell::new(None) };
}

pub async fn run(root: CtxforgeRoot) -> Result<()> {
    use crossterm::event::{DisableBracketedPaste, DisableMouseCapture};

    // Stash root so we can reload AppData between render-loop iterations.
    ROOT_STASH.with(|r| *r.borrow_mut() = Some(root.clone()));
    STARTUP.with(|s| *s.borrow_mut() = Some(load_app_data(root)));

    loop {
        let result = element!(App).render_loop().fullscreen().await;

        // Clean terminal state after render loop exits.
        let _ = crossterm::execute!(
            std::io::stdout(),
            DisableMouseCapture,
            DisableBracketedPaste,
        );

        result?;

        // Check if the render loop exited because of a PendingAction.
        let action = PENDING.with(|p| p.borrow_mut().take());
        match action {
            None => return Ok(()), // Normal quit — no action, exit app.
            Some(crate::tui::mode::PendingAction::Export(content)) => {
                println!("{content}");
                eprintln!("\nPress any key to return to ctxforge...");
                let _ = crossterm::event::read();
            }
            Some(crate::tui::mode::PendingAction::Pipe { target, content }) => {
                match std::process::Command::new(&target)
                    .stdin(std::process::Stdio::piped())
                    .spawn()
                {
                    Ok(mut child) => {
                        if let Some(mut stdin) = child.stdin.take() {
                            use std::io::Write;
                            let _ = stdin.write_all(content.as_bytes());
                        }
                        let _ = child.wait();
                    }
                    Err(e) => {
                        eprintln!("Failed to start `{target}`: {e}");
                        eprintln!("Press any key to return...");
                        let _ = crossterm::event::read();
                    }
                }
            }
            Some(crate::tui::mode::PendingAction::Editor(starting)) => {
                match crate::editor::spawn_editor(&starting) {
                    Ok(updated) => {
                        // Persist the edited text as a prompt override.
                        // load_app_data will pick it up on TUI re-entry and
                        // render_payload will return it verbatim, so delivery
                        // uses the hand-edit rather than re-rendering.
                        if updated.trim().is_empty() || updated == starting {
                            // Editor closed without changes — do nothing.
                        } else if let Some(root) = ROOT_STASH.with(|r| r.borrow().clone()) {
                            if let Err(e) = std::fs::write(root.prompt_override_path(), &updated) {
                                eprintln!("failed to save prompt override: {e}");
                                eprintln!("Press any key to return...");
                                let _ = crossterm::event::read();
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("editor: {e}");
                        eprintln!("Press any key to return...");
                        let _ = crossterm::event::read();
                    }
                }
            }
            Some(crate::tui::mode::PendingAction::TextPrompt(purpose)) => {
                handle_text_prompt(purpose);
                eprintln!("\nPress any key to return to ctxforge...");
                let _ = crossterm::event::read();
            }
            Some(crate::tui::mode::PendingAction::PickerList { purpose, items }) => {
                handle_picker(purpose, items);
                eprintln!("\nPress any key to return to ctxforge...");
                let _ = crossterm::event::read();
            }
        }

        // Reload AppData from disk so the next render-loop iteration
        // picks up any changes the external action made (e.g. editor).
        let root = ROOT_STASH
            .with(|r| r.borrow().clone())
            .expect("ROOT_STASH should be set");
        STARTUP.with(|s| *s.borrow_mut() = Some(load_app_data(root)));
    }
}

// Max tree rows rendered per frame. Phase 1 does no scrolling; we clip to
// a reasonable viewport so the layout doesn't overflow. Phase 2 adds
// proper viewport tracking + scrolling.
// Chrome rows: header (3) + prompt input min (4) + footer (2) = 9.
const CHROME_ROWS: usize = 9;

/// Height available for the tree panel's scrollable content.
/// Tree panel is 60% of the left column's main-row height; subtract the
/// panel's own 4 rows of chrome (2 borders + title + blank).
fn tree_viewport(term_h: u16) -> usize {
    ((term_h as usize).saturating_sub(CHROME_ROWS) * 60 / 100)
        .saturating_sub(4)
        .max(6)
}

/// Height available for the viewer panel's scrollable content.
/// Viewer takes the full main-row height when enabled.
fn viewer_viewport(term_h: u16) -> usize {
    (term_h as usize)
        .saturating_sub(CHROME_ROWS)
        .saturating_sub(4)
        .max(6)
}

// ─── App component ───────────────────────────────────────────────────

#[component]
fn App(hooks: &mut Hooks) -> impl Into<AnyElement<'static>> {
    let mut app_data = hooks.use_state(|| {
        STARTUP
            .with(|s| s.borrow_mut().take())
            .unwrap_or_else(|| panic!("tui2 app data missing"))
    });

    let mut focus: State<Focus> = hooks.use_state(|| Focus::FileTree);
    let mut cursor: State<usize> = hooks.use_state(|| 0usize);
    let mut should_quit: State<bool> = hooks.use_state(|| false);
    let mut mode: State<crate::tui::mode::Mode> = hooks.use_state(crate::tui::mode::Mode::default);
    let mut prompt_input: State<PromptInput> = hooks.use_state(|| {
        let initial = app_data.read().bundle.task_text.clone();
        PromptInput::with_text(initial)
    });
    let viewer_events: State<Vec<crate::tui::components::viewer::ViewerMouseEvent>> =
        hooks.use_state(Vec::new);

    // Auto-clear status message after 3 seconds. The future polls the
    // status_set_at timestamp; when 3s have elapsed, it clears the message.
    {
        let mut app_data_for_timer = app_data;
        hooks.use_future(async move {
            loop {
                smol::Timer::after(std::time::Duration::from_secs(1)).await;
                let should_clear = {
                    let d = app_data_for_timer.read();
                    d.status_set_at
                        .map(|t| {
                            t.elapsed() >= std::time::Duration::from_secs(3) && !d.status.is_empty()
                        })
                        .unwrap_or(false)
                };
                if should_clear {
                    let mut d = app_data_for_timer.write();
                    d.status.clear();
                    d.status_set_at = None;
                }
            }
        });
    }
    // Background file-load result slot. `spawn_viewer_load` writes a
    // (generation, path, ViewerLoad) tuple here when done; the render
    // body below reads it and applies only if the generation still
    // matches the current viewer load (so an old slow load can't clobber
    // a newer one).
    let viewer_bg_result: ViewerBgSlot = hooks.use_state(|| None);

    // Startup fade: animate opacity 0→1 over 260ms. On first render the
    // flag is false → target 0 → invisible. Flag flips to true on first
    // render → target 1 → tween fires on the second render.
    let mut startup_flag = hooks.use_state(|| false);
    if !startup_flag.get() {
        startup_flag.set(true);
    }
    let startup_target = if startup_flag.get() { 1.0f32 } else { 0.0f32 };
    let _startup_opacity = use_animated(
        hooks,
        startup_target,
        crate::motion_core::constants::STARTUP,
        crate::motion_core::ease_out_cubic,
    );

    let (raw_term_w, raw_term_h) = hooks.use_terminal_size();
    // Fall back to a live `terminal::size()` query only when iocraft
    // still reports zero, which happens on some terminals for one frame
    // at startup. If that also fails we leave raw as-is (never synthesize
    // dimensions that might mismatch the real terminal and paint into
    // a corner).
    let (term_w, term_h) = if raw_term_w == 0 || raw_term_h == 0 {
        crossterm::terminal::size()
            .ok()
            .filter(|(w, h)| *w > 0 && *h > 0)
            .unwrap_or((raw_term_w, raw_term_h))
    } else {
        (raw_term_w, raw_term_h)
    };

    let data = app_data.read();
    // Only show entries that aren't inside a collapsed directory
    let visible_indices = tree::visible_indices(&data.tree_entries);
    let visible_count = visible_indices.len();
    let max_cursor = visible_count.saturating_sub(1);

    hooks.use_terminal_events({
        move |event| {
            if let TerminalEvent::Key(k) = event {
                if k.kind != KeyEventKind::Press {
                    return;
                }
                // ── Welcome splash: q quits, any other key enters the app ──
                if matches!(*mode.read(), crate::tui::mode::Mode::Welcome) {
                    if matches!(k.code, KeyCode::Char('q')) {
                        *should_quit.write() = true;
                    } else {
                        *mode.write() = crate::tui::mode::Mode::Normal;
                    }
                    return;
                }

                // ── Search mode: accumulate chars, Esc cancels, Enter confirms ──
                if matches!(*mode.read(), crate::tui::mode::Mode::Search { .. }) {
                    match k.code {
                        KeyCode::Esc | KeyCode::Enter => {
                            *mode.write() = crate::tui::mode::Mode::Normal;
                            return;
                        }
                        KeyCode::Backspace => {
                            if let crate::tui::mode::Mode::Search { query } = &mut *mode.write() {
                                query.pop();
                            }
                            return;
                        }
                        KeyCode::Char(c) => {
                            if let crate::tui::mode::Mode::Search { query } = &mut *mode.write() {
                                query.push(c);
                            }
                            return;
                        }
                        _ => return,
                    }
                }

                // ── Help overlay ────────────────────────────────
                if matches!(*mode.read(), crate::tui::mode::Mode::Help) {
                    match k.code {
                        KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => {
                            *mode.write() = crate::tui::mode::Mode::Normal;
                        }
                        _ => {}
                    }
                    return;
                }

                // ── Scenario picker overlay ─────────────────────
                if matches!(*mode.read(), crate::tui::mode::Mode::ScenarioPicker { .. }) {
                    let scenarios =
                        crate::tui::overlays::scenario_picker::load(&app_data.read().root);
                    let count = scenarios.len();
                    match k.code {
                        KeyCode::Esc => {
                            *mode.write() = crate::tui::mode::Mode::Normal;
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if let crate::tui::mode::Mode::ScenarioPicker { cursor } =
                                &mut *mode.write()
                            {
                                if *cursor > 0 {
                                    *cursor -= 1;
                                }
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if let crate::tui::mode::Mode::ScenarioPicker { cursor } =
                                &mut *mode.write()
                            {
                                if *cursor + 1 < count {
                                    *cursor += 1;
                                }
                            }
                        }
                        KeyCode::Enter => {
                            let cur = match *mode.read() {
                                crate::tui::mode::Mode::ScenarioPicker { cursor } => cursor,
                                _ => 0,
                            };
                            if let Some(picked) = scenarios.get(cur) {
                                let name = picked.name.clone();
                                app_data.write().set_scenario(Some(name));
                            }
                            *mode.write() = crate::tui::mode::Mode::Normal;
                        }
                        _ => {}
                    }
                    return;
                }

                // ── Theme picker overlay ────────────────────────
                if matches!(*mode.read(), crate::tui::mode::Mode::ThemePicker { .. }) {
                    let themes = crate::tui::overlays::theme_picker::all();
                    let count = themes.len();
                    match k.code {
                        KeyCode::Esc => {
                            *mode.write() = crate::tui::mode::Mode::Normal;
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if let crate::tui::mode::Mode::ThemePicker { cursor } =
                                &mut *mode.write()
                            {
                                if *cursor > 0 {
                                    *cursor -= 1;
                                }
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if let crate::tui::mode::Mode::ThemePicker { cursor } =
                                &mut *mode.write()
                            {
                                if *cursor + 1 < count {
                                    *cursor += 1;
                                }
                            }
                        }
                        KeyCode::Enter => {
                            let cur = match *mode.read() {
                                crate::tui::mode::Mode::ThemePicker { cursor } => cursor,
                                _ => 0,
                            };
                            if let Some(picked) = themes.get(cur) {
                                let name = picked.name.to_string();
                                let _ = app_data.write().apply_theme(&name);
                            }
                            *mode.write() = crate::tui::mode::Mode::Normal;
                        }
                        _ => {}
                    }
                    return;
                }

                // ── @ file picker ──────────────────────────────
                if matches!(*mode.read(), crate::tui::mode::Mode::AtPicker { .. }) {
                    match k.code {
                        KeyCode::Esc => {
                            // Cancel — remove the '@' we inserted
                            prompt_input.write().backspace();
                            let text = prompt_input.read().text().to_string();
                            app_data.write().sync_task_text(text);
                            *mode.write() = crate::tui::mode::Mode::Normal;
                        }
                        KeyCode::Backspace => {
                            let query_empty = matches!(
                                &*mode.read(),
                                crate::tui::mode::Mode::AtPicker { query, .. } if query.is_empty()
                            );
                            if query_empty {
                                prompt_input.write().backspace();
                                let text = prompt_input.read().text().to_string();
                                app_data.write().sync_task_text(text);
                                *mode.write() = crate::tui::mode::Mode::Normal;
                            } else {
                                if let crate::tui::mode::Mode::AtPicker { query, cursor, .. } =
                                    &mut *mode.write()
                                {
                                    query.pop();
                                    *cursor = 0;
                                }
                                prompt_input.write().backspace();
                                let text = prompt_input.read().text().to_string();
                                app_data.write().sync_task_text(text);
                            }
                        }
                        KeyCode::Up => {
                            if let crate::tui::mode::Mode::AtPicker { cursor, .. } =
                                &mut *mode.write()
                            {
                                if *cursor > 0 {
                                    *cursor -= 1;
                                }
                            }
                        }
                        KeyCode::Down => {
                            if let crate::tui::mode::Mode::AtPicker {
                                cursor,
                                query,
                                files,
                            } = &mut *mode.write()
                            {
                                let ranked = crate::prompt_input::at_picker::rank(
                                    files,
                                    query,
                                    crate::prompt_input::at_picker::RESULT_LIMIT,
                                );
                                if *cursor + 1 < ranked.len() {
                                    *cursor += 1;
                                }
                            }
                        }
                        KeyCode::Enter => {
                            let picked = {
                                let m = mode.read();
                                if let crate::tui::mode::Mode::AtPicker {
                                    cursor,
                                    query,
                                    files,
                                } = &*m
                                {
                                    let ranked = crate::prompt_input::at_picker::rank(
                                        files,
                                        query,
                                        crate::prompt_input::at_picker::RESULT_LIMIT,
                                    );
                                    ranked.get(*cursor).cloned()
                                } else {
                                    None
                                }
                            };
                            if let Some(path) = picked {
                                // Replace the @query with @full_path
                                let cursor_pos = prompt_input.read().cursor();
                                let text = prompt_input.read().text().to_string();
                                let query_len = match &*mode.read() {
                                    crate::tui::mode::Mode::AtPicker { query, .. } => query.len(),
                                    _ => 0,
                                };
                                // The prompt text has "@<query>" before the cursor.
                                // Replace the query portion with the full path.
                                let at_start = cursor_pos.saturating_sub(query_len);
                                let path_str = path.display().to_string();
                                let mut new_text = text;
                                new_text.replace_range(at_start..cursor_pos, &path_str);
                                let new_cursor = at_start + path_str.len();
                                prompt_input.write().set_text(new_text.clone());
                                prompt_input.write().set_cursor(new_cursor);
                                app_data
                                    .write()
                                    .sync_task_text(prompt_input.read().text().to_string());
                                // Add to bundle if not already there
                                if !app_data.read().bundled_paths.contains(&path) {
                                    app_data.write().toggle_bundle(&path);
                                }
                            }
                            *mode.write() = crate::tui::mode::Mode::Normal;
                        }
                        KeyCode::Char(c) => {
                            if let crate::tui::mode::Mode::AtPicker { query, cursor, .. } =
                                &mut *mode.write()
                            {
                                query.push(c);
                                *cursor = 0;
                            }
                            prompt_input.write().insert_char(c);
                            let text = prompt_input.read().text().to_string();
                            app_data.write().sync_task_text(text);
                        }
                        _ => {}
                    }
                    return;
                }

                // ── Full prompt preview overlay ─────────────────
                if matches!(
                    *mode.read(),
                    crate::tui::mode::Mode::FullPromptPreview { .. }
                ) {
                    match k.code {
                        KeyCode::Esc | KeyCode::Char('P') | KeyCode::Char('q') => {
                            *mode.write() = crate::tui::mode::Mode::Normal;
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if let crate::tui::mode::Mode::FullPromptPreview { scroll, .. } =
                                &mut *mode.write()
                            {
                                *scroll = scroll.saturating_sub(1);
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if let crate::tui::mode::Mode::FullPromptPreview { scroll, .. } =
                                &mut *mode.write()
                            {
                                *scroll += 1;
                            }
                        }
                        KeyCode::Char('g') => {
                            if let crate::tui::mode::Mode::FullPromptPreview { scroll, .. } =
                                &mut *mode.write()
                            {
                                *scroll = 0;
                            }
                        }
                        KeyCode::Char('G') => {
                            if let crate::tui::mode::Mode::FullPromptPreview { scroll, content } =
                                &mut *mode.write()
                            {
                                let total = content.lines().count();
                                *scroll = total.saturating_sub(20);
                            }
                        }
                        _ => {}
                    }
                    return;
                }

                // ── Delivery picker overlay ─────────────────────
                if matches!(*mode.read(), crate::tui::mode::Mode::DeliveryPicker { .. }) {
                    let count = crate::deliver::DeliverChoice::all().len();
                    match k.code {
                        KeyCode::Esc => {
                            *mode.write() = crate::tui::mode::Mode::Normal;
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if let crate::tui::mode::Mode::DeliveryPicker { cursor } =
                                &mut *mode.write()
                            {
                                if *cursor > 0 {
                                    *cursor -= 1;
                                }
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if let crate::tui::mode::Mode::DeliveryPicker { cursor } =
                                &mut *mode.write()
                            {
                                if *cursor + 1 < count {
                                    *cursor += 1;
                                }
                            }
                        }
                        KeyCode::Enter => {
                            let cur = match *mode.read() {
                                crate::tui::mode::Mode::DeliveryPicker { cursor } => cursor,
                                _ => 0,
                            };
                            let choices = crate::deliver::DeliverChoice::all();
                            if let Some(&choice) = choices.get(cur) {
                                app_data.write().run_delivery(choice);
                                // If a pending action was set (pipe/export), need
                                // to exit the render loop. The outer run() handles it.
                                if app_data.read().pending_action.is_some() {
                                    *should_quit.write() = true;
                                }
                            }
                            *mode.write() = crate::tui::mode::Mode::Normal;
                        }
                        _ => {}
                    }
                    return;
                }

                // ── Command palette overlay ────────────────────
                if matches!(*mode.read(), crate::tui::mode::Mode::CommandPalette { .. }) {
                    match k.code {
                        KeyCode::Esc => {
                            *mode.write() = crate::tui::mode::Mode::Normal;
                        }
                        KeyCode::Up => {
                            if let crate::tui::mode::Mode::CommandPalette { cursor, .. } =
                                &mut *mode.write()
                            {
                                if *cursor > 0 {
                                    *cursor -= 1;
                                }
                            }
                        }
                        KeyCode::Down => {
                            let count = {
                                let m = mode.read();
                                if let crate::tui::mode::Mode::CommandPalette { query, .. } = &*m {
                                    crate::tui::command_registry::filter(query).len()
                                } else {
                                    0
                                }
                            };
                            if let crate::tui::mode::Mode::CommandPalette { cursor, .. } =
                                &mut *mode.write()
                            {
                                if *cursor + 1 < count {
                                    *cursor += 1;
                                }
                            }
                        }
                        KeyCode::Backspace => {
                            if let crate::tui::mode::Mode::CommandPalette { query, cursor } =
                                &mut *mode.write()
                            {
                                query.pop();
                                *cursor = 0;
                            }
                        }
                        KeyCode::Char(c) => {
                            if let crate::tui::mode::Mode::CommandPalette { query, cursor } =
                                &mut *mode.write()
                            {
                                query.push(c);
                                *cursor = 0;
                            }
                        }
                        KeyCode::Enter => {
                            let (query_owned, cursor_idx) = match &*mode.read() {
                                crate::tui::mode::Mode::CommandPalette { query, cursor } => {
                                    (query.clone(), *cursor)
                                }
                                _ => (String::new(), 0),
                            };
                            let results = crate::tui::command_registry::filter(&query_owned);
                            if let Some(cmd) = results.get(cursor_idx) {
                                let action = cmd.action;
                                let next_mode = app_data.write().dispatch_command(action);
                                match next_mode {
                                    Some(m) => *mode.write() = m,
                                    None => {
                                        if matches!(
                                            action,
                                            crate::tui::command_registry::CommandAction::Quit
                                        ) {
                                            *should_quit.write() = true;
                                        }
                                        *mode.write() = crate::tui::mode::Mode::Normal;
                                    }
                                }
                            } else {
                                *mode.write() = crate::tui::mode::Mode::Normal;
                            }
                        }
                        _ => {}
                    }
                    return;
                }

                // ── Prompt focus: route text keys into PromptInput ──
                if *focus.read() == Focus::Prompt {
                    let mut handled = true;
                    match k.code {
                        KeyCode::Esc => {
                            *focus.write() = Focus::FileTree;
                        }
                        KeyCode::Enter => {
                            prompt_input.write().insert_newline();
                        }
                        KeyCode::Backspace => {
                            prompt_input.write().backspace();
                        }
                        KeyCode::Left => prompt_input.write().move_left(),
                        KeyCode::Right => prompt_input.write().move_right(),
                        KeyCode::Home => prompt_input.write().move_home(),
                        KeyCode::End => prompt_input.write().move_end(),
                        KeyCode::Char('w') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                            prompt_input.write().delete_word_back();
                        }
                        KeyCode::Char('@') => {
                            prompt_input.write().insert_char('@');
                            // Open the @ file picker
                            let files = crate::prompt_input::at_picker::walk_files(
                                &app_data.read().project_root,
                            );
                            *mode.write() = crate::tui::mode::Mode::AtPicker {
                                query: String::new(),
                                cursor: 0,
                                files,
                            };
                        }
                        KeyCode::Char(c) => {
                            prompt_input.write().insert_char(c);
                        }
                        _ => {
                            handled = false;
                        }
                    }
                    if handled {
                        let text = prompt_input.read().text().to_string();
                        app_data.write().sync_task_text(text);
                        return;
                    }
                    // Fall through: Tab, BackTab, etc. bubble to normal dispatch below
                }

                match k.code {
                    KeyCode::Char('q') => *should_quit.write() = true,
                    KeyCode::Char('?') => {
                        *mode.write() = crate::tui::mode::Mode::Help;
                    }
                    KeyCode::Char('S') => {
                        *mode.write() = crate::tui::mode::Mode::ScenarioPicker { cursor: 0 };
                    }
                    KeyCode::Char('P') => {
                        let content = {
                            let d = app_data.read();
                            d.render_payload(crate::format::Format::Markdown)
                                .unwrap_or_else(|e| format!("Error: {e}"))
                        };
                        *mode.write() =
                            crate::tui::mode::Mode::FullPromptPreview { content, scroll: 0 };
                    }
                    KeyCode::Char('d') if !k.modifiers.contains(KeyModifiers::CONTROL) => {
                        *mode.write() = crate::tui::mode::Mode::DeliveryPicker { cursor: 0 };
                    }
                    // x = export to stdout (shortcut for /deliver → export)
                    KeyCode::Char('x') => {
                        app_data
                            .write()
                            .run_delivery(crate::deliver::DeliverChoice::Export);
                        if app_data.read().pending_action.is_some() {
                            *should_quit.write() = true;
                        }
                    }
                    KeyCode::Char('/') => {
                        *mode.write() = crate::tui::mode::Mode::CommandPalette {
                            query: String::new(),
                            cursor: 0,
                        };
                    }
                    KeyCode::Char('f') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                        *mode.write() = crate::tui::mode::Mode::Search {
                            query: String::new(),
                        };
                    }
                    KeyCode::Char('i') if *focus.read() != Focus::Prompt => {
                        *focus.write() = Focus::Prompt;
                    }
                    KeyCode::Char('v') => {
                        // Toggle viewer and kick off a background file load
                        // if the cursor's file isn't already cached. The
                        // toggle itself is instant — the load runs on smol's
                        // blocking pool via `spawn_viewer_load` and wakes
                        // the render loop when done (see the bg-drain below).
                        let (now_enabled, load_target) = {
                            let mut d = app_data.write();
                            d.viewer.toggle();
                            let enabled = d.viewer.enabled;
                            let mut target: Option<(PathBuf, u64)> = None;
                            if enabled {
                                let visible = tree::visible_indices(&d.tree_entries);
                                if let Some(&idx) = visible.get(*cursor.read()) {
                                    if let Some(entry) = d.tree_entries.get(idx).cloned() {
                                        if !entry.is_dir {
                                            let abs = d.project_root.join(&entry.rel_path);
                                            if d.viewer.cached_path.as_deref()
                                                != Some(abs.as_path())
                                            {
                                                let generation = d.viewer.begin_load();
                                                target = Some((abs, generation));
                                            }
                                        }
                                    }
                                }
                                d.set_status("viewer on".to_string());
                            } else {
                                d.set_status("viewer off".to_string());
                            }
                            (enabled, target)
                        };
                        if let Some((abs, generation)) = load_target {
                            spawn_viewer_load(abs, generation, viewer_bg_result);
                        }
                        if now_enabled {
                            focus.set(Focus::Viewer);
                        } else if *focus.read() == Focus::Viewer {
                            focus.set(Focus::FileTree);
                        }
                    }
                    KeyCode::Tab => {
                        let viewer_on = app_data.read().viewer.enabled;
                        let next = match *focus.read() {
                            Focus::FileTree => {
                                if viewer_on {
                                    Focus::Viewer
                                } else {
                                    Focus::BundleList
                                }
                            }
                            Focus::Viewer => Focus::BundleList,
                            Focus::BundleList => Focus::Prompt,
                            Focus::Prompt => Focus::FileTree,
                        };
                        *focus.write() = next;
                    }
                    KeyCode::BackTab => {
                        let viewer_on = app_data.read().viewer.enabled;
                        let prev = match *focus.read() {
                            Focus::FileTree => Focus::Prompt,
                            Focus::Viewer => Focus::FileTree,
                            Focus::BundleList => {
                                if viewer_on {
                                    Focus::Viewer
                                } else {
                                    Focus::FileTree
                                }
                            }
                            Focus::Prompt => Focus::BundleList,
                        };
                        *focus.write() = prev;
                    }
                    KeyCode::Up | KeyCode::Char('k') => match *focus.read() {
                        Focus::Viewer => {
                            app_data
                                .write()
                                .viewer
                                .scroll_by(-1, viewer_viewport(term_h));
                        }
                        _ => {
                            let c = *cursor.read();
                            if c > 0 {
                                let new = c - 1;
                                cursor.set(new);
                                reload_viewer(&mut app_data.write(), new, viewer_bg_result);
                            }
                        }
                    },
                    KeyCode::Down | KeyCode::Char('j') => match *focus.read() {
                        Focus::Viewer => {
                            app_data
                                .write()
                                .viewer
                                .scroll_by(1, viewer_viewport(term_h));
                        }
                        _ => {
                            let c = *cursor.read();
                            if c < max_cursor {
                                let new = c + 1;
                                cursor.set(new);
                                reload_viewer(&mut app_data.write(), new, viewer_bg_result);
                            }
                        }
                    },
                    // Space: toggle bundle (on file) or expand/collapse (on dir).
                    KeyCode::Char(' ') if *focus.read() == Focus::FileTree => {
                        let (actual_idx, is_dir, rel_path) = {
                            let d = app_data.read();
                            let visible = tree::visible_indices(&d.tree_entries);
                            let Some(&actual) = visible.get(*cursor.read()) else {
                                return;
                            };
                            match d.tree_entries.get(actual) {
                                Some(e) => (actual, e.is_dir, e.rel_path.clone()),
                                None => return,
                            }
                        };
                        let mut d = app_data.write();
                        if is_dir {
                            d.toggle_expanded(actual_idx);
                        } else {
                            d.toggle_bundle(&rel_path);
                        }
                    }
                    // Enter: expand/collapse directory only.
                    KeyCode::Enter if *focus.read() == Focus::FileTree => {
                        let (actual_idx, is_dir) = {
                            let d = app_data.read();
                            let visible = tree::visible_indices(&d.tree_entries);
                            let Some(&actual) = visible.get(*cursor.read()) else {
                                return;
                            };
                            let is_dir = d
                                .tree_entries
                                .get(actual)
                                .map(|e| e.is_dir)
                                .unwrap_or(false);
                            (actual, is_dir)
                        };
                        if is_dir {
                            app_data.write().toggle_expanded(actual_idx);
                        }
                    }
                    KeyCode::Char('E') if *focus.read() == Focus::FileTree => {
                        app_data.write().expand_all();
                    }
                    KeyCode::Char('C') if *focus.read() == Focus::FileTree => {
                        app_data.write().collapse_all();
                    }
                    // Viewer: a adds selection, Esc clears selection.
                    KeyCode::Char('a') if *focus.read() == Focus::Viewer => {
                        app_data.write().add_viewer_selection_to_bundle();
                    }
                    KeyCode::Esc
                        if *focus.read() == Focus::Viewer
                            && app_data.read().viewer.selection.is_some() =>
                    {
                        app_data.write().viewer.clear_selection();
                    }
                    // Navigation: g/G top/bottom, Ctrl-U/D half-page
                    KeyCode::Char('g') if *focus.read() == Focus::FileTree => {
                        cursor.set(0);
                        reload_viewer(&mut app_data.write(), 0, viewer_bg_result);
                    }
                    KeyCode::Char('g') if *focus.read() == Focus::Viewer => {
                        app_data.write().viewer.scroll_to_top();
                    }
                    KeyCode::Char('G') if *focus.read() == Focus::FileTree => {
                        let d = app_data.read();
                        let visible = tree::visible_indices(&d.tree_entries);
                        if !visible.is_empty() {
                            let last = visible.len() - 1;
                            drop(d);
                            cursor.set(last);
                            reload_viewer(&mut app_data.write(), last, viewer_bg_result);
                        }
                    }
                    KeyCode::Char('G') if *focus.read() == Focus::Viewer => {
                        app_data
                            .write()
                            .viewer
                            .scroll_to_bottom(viewer_viewport(term_h));
                    }
                    KeyCode::Char('u')
                        if k.modifiers.contains(KeyModifiers::CONTROL)
                            && *focus.read() == Focus::FileTree =>
                    {
                        let half = tree_viewport(term_h) / 2;
                        let c = *cursor.read();
                        let new = c.saturating_sub(half);
                        cursor.set(new);
                        reload_viewer(&mut app_data.write(), new, viewer_bg_result);
                    }
                    KeyCode::Char('u')
                        if k.modifiers.contains(KeyModifiers::CONTROL)
                            && *focus.read() == Focus::Viewer =>
                    {
                        app_data.write().viewer.scroll_by(
                            -(viewer_viewport(term_h) as i32 / 2),
                            viewer_viewport(term_h),
                        );
                    }
                    KeyCode::Char('d')
                        if k.modifiers.contains(KeyModifiers::CONTROL)
                            && *focus.read() == Focus::FileTree =>
                    {
                        let d = app_data.read();
                        let visible = tree::visible_indices(&d.tree_entries);
                        let half = tree_viewport(term_h) / 2;
                        let c = *cursor.read();
                        let new = (c + half).min(visible.len().saturating_sub(1));
                        drop(d);
                        cursor.set(new);
                        reload_viewer(&mut app_data.write(), new, viewer_bg_result);
                    }
                    KeyCode::Char('d')
                        if k.modifiers.contains(KeyModifiers::CONTROL)
                            && *focus.read() == Focus::Viewer =>
                    {
                        app_data
                            .write()
                            .viewer
                            .scroll_by(viewer_viewport(term_h) as i32 / 2, viewer_viewport(term_h));
                    }
                    _ => {}
                }
            }
        }
    });

    // Re-read app_data after the event handler may have mutated it, and
    // clamp the cursor so it stays within the (possibly shrunken) visible
    // range — relevant after collapse-all.
    drop(data);

    if *should_quit.read() {
        // Stash pending action into thread_local so the outer run() loop
        // can drain it after the render loop exits.
        {
            let mut d = app_data.write();
            let pending = d.pending_action.take();
            if let Some(action) = pending {
                PENDING.with(|p| *p.borrow_mut() = Some(action));
            }
            let _ = d.bundle.save(&d.root);
        }

        let mut system = hooks.use_context_mut::<iocraft::SystemContext>();
        system.exit();
    }

    // Drain a completed background file load (if any). Peek via
    // `try_read` first: if the slot is empty (the common case, every
    // frame with no load in flight) we don't touch it. This matters
    // because a `StateMutRef::DerefMut` flips `did_change` on drop,
    // which wakes the render loop again — unconditionally touching
    // the slot each frame produces an infinite self-wake loop that
    // starves terminal-event polling (seen as "app stuck at startup").
    // We only grab `try_write` when there's real work to apply.
    let has_load = viewer_bg_result
        .try_read()
        .map(|g| g.is_some())
        .unwrap_or(false);
    if has_load {
        let taken: Option<ViewerBgResult> = {
            let mut slot = viewer_bg_result;
            slot.try_write().and_then(|mut g| g.take())
        };
        if let Some((generation, path, load)) = taken {
            app_data
                .write()
                .viewer
                .apply_bg_load(generation, path, load);
        }
    }

    // Drain viewer mouse events batched since last render (wheel, drag,
    // click). Peek via `try_read` first — `DerefMut` on an empty queue
    // still flips `did_change=true` and keeps `root.wait()` Ready,
    // producing an infinite render loop that starves terminal-event
    // polling (sample trace shows 100% CPU pinned in taffy). We only
    // take `try_write` when there's real work to do, and `std::mem::take`
    // leaves the state as the default Vec so the next frame's peek
    // sees empty again.
    let has_mouse = viewer_events
        .try_read()
        .map(|g| !g.is_empty())
        .unwrap_or(false);
    let pending_mouse: Vec<crate::tui::components::viewer::ViewerMouseEvent> = if has_mouse {
        let mut events = viewer_events;
        match events.try_write() {
            Some(mut guard) => std::mem::take(&mut *guard),
            None => Vec::new(),
        }
    } else {
        Vec::new()
    };
    if !pending_mouse.is_empty() {
        let viewport = viewer_viewport(term_h);
        let mut d = app_data.write();
        for ev in pending_mouse {
            use crate::tui::components::viewer::ViewerMouseEvent as VE;
            match ev {
                VE::ScrollUp => d.viewer.scroll_by(-3, viewport),
                VE::ScrollDown => d.viewer.scroll_by(3, viewport),
                VE::Down { line } => d.viewer.drag_start(line),
                VE::Drag { line } => d.viewer.drag_extend(line),
                VE::Up => d.viewer.drag_end(),
            }
        }
    }

    let data = app_data.read();
    let visible_indices = tree::visible_indices(&data.tree_entries);
    let visible_count = visible_indices.len();
    let max_cursor = visible_count.saturating_sub(1);
    if *cursor.read() > max_cursor {
        cursor.set(max_cursor);
    }

    let cur_focus = *focus.read();
    let cur = *cursor.read();
    let theme = data.theme;

    // Search query — cloned to &'static String so we can use it in both the
    // search bar and the tree branch selection.
    let search_query: Option<String> = match &*mode.read() {
        crate::tui::mode::Mode::Search { query } => Some(query.clone()),
        _ => None,
    };
    let search_active = search_query.as_ref().is_some_and(|q| !q.is_empty());

    // Animated focus-border RGB
    let (tr, tg, tb) = focus_color_rgb(cur_focus, &theme);
    let anim_r = use_animated(hooks, tr, constants::FOCUS_BORDER, ease_out_cubic);
    let anim_g = use_animated(hooks, tg, constants::FOCUS_BORDER, ease_out_cubic);
    let anim_b = use_animated(hooks, tb, constants::FOCUS_BORDER, ease_out_cubic);
    let focus_color = Color::Rgb {
        r: anim_r,
        g: anim_g,
        b: anim_b,
    };

    // Clip visible entries to viewport centered around the cursor
    let start = cur.saturating_sub(tree_viewport(term_h) / 2);
    let end = (start + tree_viewport(term_h)).min(visible_count);
    let visible: Vec<(usize, TreeEntry)> = visible_indices
        .iter()
        .enumerate()
        .skip(start)
        .take(end.saturating_sub(start))
        .filter_map(|(vi, &idx)| data.tree_entries.get(idx).map(|e| (vi, e.clone())))
        .collect();

    // Focus-aware border colors. Content rendering is done per-branch below
    // so `AnyElement` vectors (which aren't Clone) don't need to be duplicated.
    let tree_border = if cur_focus == Focus::FileTree {
        focus_color
    } else {
        theme.border
    };
    let bundle_border = if cur_focus == Focus::BundleList {
        focus_color
    } else {
        theme.border
    };
    let preview_border = if cur_focus == Focus::Viewer {
        focus_color
    } else {
        theme.border
    };
    let viewer_border = if cur_focus == Focus::Viewer {
        focus_color
    } else {
        theme.border
    };
    let prompt_border = if cur_focus == Focus::Prompt {
        focus_color
    } else {
        theme.border
    };

    // Compute the bundle title once — it's cheap and Clone.
    let bundle_title = {
        let total: usize = data.item_tokens.iter().sum();
        if data.bundle.is_empty() {
            "BUNDLE · empty".to_string()
        } else {
            let tokens = if total >= 1_000 {
                format!("{:.1}k", total as f64 / 1_000.0)
            } else {
                total.to_string()
            };
            format!("BUNDLE · {} · {} tokens", data.bundle.len(), tokens)
        }
    };

    // Header content — wordmark + scenario + model + animated gradient gauge
    let pct = if data.model_window == 0 {
        0.0
    } else {
        (data.total_tokens as f64 / data.model_window as f64) * 100.0
    };
    let ratio = (data.total_tokens as f32) / (data.model_window.max(1) as f32);
    let animated_ratio = use_animated(
        hooks,
        ratio.clamp(0.0, 1.0),
        constants::GAUGE_FILL,
        crate::motion_core::ease_out_quad,
    );
    let scenario = data.bundle.scenario.clone().unwrap_or_default();

    // Gradient gauge using ░▒▓█ — 4-step fill for finer visual granularity
    let bar_width = 24u32;
    let bar_color = gauge_color(pct, &theme);
    let raw_fill = animated_ratio * bar_width as f32 * 4.0;
    let full_cells = (raw_fill as u32) / 4;
    let frac = ((raw_fill as u32) % 4) as usize;
    let partial = ["", "░", "▒", "▓"][frac];
    let empty = bar_width.saturating_sub(full_cells) as usize;
    let empty = if partial.is_empty() {
        empty
    } else {
        empty.saturating_sub(1)
    };
    let gauge = format!(
        "{}{}{}",
        "█".repeat(full_cells as usize),
        partial,
        "░".repeat(empty)
    );

    // Header pieces — wordmark, separator, meta, gauge
    let scenario_chip = if scenario.is_empty() {
        " · ".to_string()
    } else {
        format!(" · {} · ", scenario)
    };
    let meta = format!(
        "{} · ~{} / {} · {:.1}% ",
        data.model_name,
        format_tokens(data.total_tokens),
        format_tokens(data.model_window),
        pct,
    );

    // Section-marker titles — left bar + uppercase label for consistent hierarchy
    let tree_title_styled = format!("FILES  {}", visible_count);
    let preview_title_styled = "PREVIEW".to_string();

    // Width-adaptive layout breakpoints.
    // Phase 1 does not ship the code viewer, so two-column is the default.
    // Three-column layout (with viewer) will come in Phase 2 when opt-in via `v`.
    let wide = term_w >= 100; // two-column for anything ≥ 100
    let _narrow = term_w < 100; // handled by the else branch below

    // Use explicit terminal dimensions instead of 100pct so the root View
    // actually fills the whole terminal. iocraft's fullscreen mode doesn't
    // force the root to match terminal size — we have to pin it ourselves.
    let w = term_w as u32;
    let h = term_h as u32;

    // ─── Welcome splash: full-screen, dismisses on any key ──
    if matches!(&*mode.read(), crate::tui::mode::Mode::Welcome) {
        return element! {
            View(
                flex_direction: FlexDirection::Column,
                background_color: theme.bg,
                width: w,
                height: h,
            ) {
                #(crate::tui::components::welcome::render_splash(
                    &theme,
                    visible_count,
                    term_w,
                    term_h,
                ))
            }
        };
    }

    element! {
        View(
            flex_direction: FlexDirection::Column,
            background_color: theme.bg,
            width: w,
            height: h,
        ) {
            // ─── HEADER (height: 3) ──────────────────────────────
            View(
                border_style: BorderStyle::Round,
                border_color: theme.border,
                background_color: theme.bg,
                height: 3,
                width: 100pct,
                padding_left: 1,
                padding_right: 1,
            ) {
                MixedText(contents: vec![
                    MixedTextContent::new("▍ ").color(theme.accent).weight(Weight::Bold),
                    MixedTextContent::new("ctxforge").color(theme.accent).weight(Weight::Bold),
                    MixedTextContent::new(scenario_chip).color(theme.muted),
                    MixedTextContent::new(meta).color(theme.muted),
                    MixedTextContent::new(gauge).color(bar_color).weight(Weight::Bold),
                ])
            }

            // ─── MAIN CONTENT ROW ────────────────────────────────
            // Two-column by default; three-column when viewer is enabled.
            #(if wide {
                let viewer_on = data.viewer.enabled;
                let viewer_file_title = data.viewer.cached_path.as_ref()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .map(|s| format!("VIEWER · {}", s))
                    .unwrap_or_else(|| "VIEWER".to_string());

                let (left_w, center_w, right_w) = if viewer_on {
                    (iocraft::Percent(25.0), iocraft::Percent(45.0), iocraft::Percent(30.0))
                } else {
                    (iocraft::Percent(35.0), iocraft::Percent(0.0), iocraft::Percent(65.0))
                };

                element! {
                    View(
                        flex_direction: FlexDirection::Row,
                        width: 100pct,
                        flex_grow: 1.0,
                    ) {
                        // Left: tree + bundle
                        View(
                            flex_direction: FlexDirection::Column,
                            width: left_w,
                            height: 100pct,
                        ) {
                            #(search_query.as_ref().map(|q| {
                                crate::tui::components::search_bar::render_search_bar(q, &theme)
                            }))
                            View(
                                flex_direction: FlexDirection::Column,
                                border_style: BorderStyle::Round,
                                border_color: tree_border,
                                background_color: theme.bg,
                                width: 100pct,
                                height: 60pct,
                                padding_left: 1,
                                padding_right: 1,
                            ) {
                                MixedText(contents: vec![
                                    MixedTextContent::new("▍ ").color(theme.accent).weight(Weight::Bold),
                                    MixedTextContent::new(tree_title_styled.clone()).color(theme.accent).weight(Weight::Bold),
                                ])
                                Text(content: "")
                                #(if search_active {
                                    crate::tui::components::tree::render_search_rows(
                                        &data.tree_entries,
                                        search_query.as_deref().unwrap_or(""),
                                        cur,
                                        &data.bundled_paths,
                                        &theme,
                                        tree_viewport(term_h),
                                    )
                                } else {
                                    render_tree_rows(&visible, cur, cur_focus == Focus::FileTree, &data.bundled_paths, &theme)
                                })
                            }
                            View(
                                flex_direction: FlexDirection::Column,
                                border_style: BorderStyle::Round,
                                border_color: bundle_border,
                                background_color: theme.bg,
                                width: 100pct,
                                height: 40pct,
                                padding_left: 1,
                                padding_right: 1,
                            ) {
                                MixedText(contents: vec![
                                    MixedTextContent::new("▍ ").color(theme.accent).weight(Weight::Bold),
                                    MixedTextContent::new(bundle_title.clone()).color(theme.accent).weight(Weight::Bold),
                                ])
                                Text(content: "")
                                #(render_bundle_rows(&data.bundle, &data.item_tokens, &theme).1)
                            }
                        }
                        // Center: viewer (only when enabled)
                        #(if viewer_on {
                            Some(element! {
                                View(
                                    flex_direction: FlexDirection::Column,
                                    border_style: BorderStyle::Round,
                                    border_color: viewer_border,
                                    background_color: theme.bg,
                                    width: center_w,
                                    height: 100pct,
                                    padding_left: 1,
                                    padding_right: 1,
                                    padding_top: 1,
                                ) {
                                    MixedText(contents: vec![
                                        MixedTextContent::new("▍ ").color(theme.accent).weight(Weight::Bold),
                                        MixedTextContent::new(viewer_file_title).color(theme.accent).weight(Weight::Bold),
                                    ])
                                    Text(content: "")
                                    crate::tui::components::viewer::Viewer(
                                        viewer: crate::tui::components::viewer::ViewerStateSnapshot::from_state(&data.viewer),
                                        theme: Some(theme),
                                        events: Some(viewer_events),
                                    )
                                }
                            })
                        } else {
                            None
                        })
                        // Right: preview
                        View(
                            flex_direction: FlexDirection::Column,
                            border_style: BorderStyle::Round,
                            border_color: preview_border,
                            background_color: theme.bg,
                            width: right_w,
                            height: 100pct,
                            padding_left: 2,
                            padding_right: 2,
                            padding_top: 1,
                        ) {
                            MixedText(contents: vec![
                                MixedTextContent::new("▍ ").color(theme.accent).weight(Weight::Bold),
                                MixedTextContent::new(preview_title_styled.clone()).color(theme.accent).weight(Weight::Bold),
                            ])
                            Text(content: "")
                            #(render_preview(&data.preview, &theme))
                        }
                    }
                }.into_any()
            } else {
                // Narrow: single-column focus-based — show just the focused panel
                let (panel_label, panel_border, panel_body): (String, Color, Vec<AnyElement<'static>>) =
                    match cur_focus {
                        Focus::BundleList => (
                            bundle_title.clone(),
                            bundle_border,
                            render_bundle_rows(&data.bundle, &data.item_tokens, &theme).1,
                        ),
                        Focus::Viewer => {
                            let title = data.viewer.cached_path.as_ref()
                                .and_then(|p| p.file_name())
                                .and_then(|n| n.to_str())
                                .map(|s| format!("VIEWER · {}", s))
                                .unwrap_or_else(|| "VIEWER".to_string());
                            let viewer_element = element! {
                                crate::tui::components::viewer::Viewer(
                                    viewer: crate::tui::components::viewer::ViewerStateSnapshot::from_state(&data.viewer),
                                    theme: Some(theme),
                                    events: Some(viewer_events),
                                )
                            }
                            .into_any();
                            (title, viewer_border, vec![viewer_element])
                        }
                        Focus::Prompt => (
                            preview_title_styled.clone(),
                            preview_border,
                            render_preview(&data.preview, &theme),
                        ),
                        Focus::FileTree => (
                            tree_title_styled.clone(),
                            tree_border,
                            if search_active {
                                crate::tui::components::tree::render_search_rows(
                                    &data.tree_entries,
                                    search_query.as_deref().unwrap_or(""),
                                    cur,
                                    &data.bundled_paths,
                                    &theme,
                                    tree_viewport(term_h),
                                )
                            } else {
                                render_tree_rows(&visible, cur, cur_focus == Focus::FileTree, &data.bundled_paths, &theme)
                            },
                        ),
                    };

                element! {
                    View(
                        flex_direction: FlexDirection::Column,
                        border_style: BorderStyle::Round,
                        border_color: panel_border,
                        background_color: theme.bg,
                        width: 100pct,
                        flex_grow: 1.0,
                        padding_left: 1,
                        padding_right: 1,
                    ) {
                        MixedText(contents: vec![
                            MixedTextContent::new("▍ ").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(panel_label).color(theme.accent).weight(Weight::Bold),
                        ])
                        Text(content: "")
                        #(panel_body)
                    }
                }.into_any()
            })

            // ─── PROMPT INPUT (auto-grow 4..=10 rows) ──
            #(crate::tui::components::prompt_input::render_prompt_input(
                &prompt_input.read(),
                cur_focus == Focus::Prompt,
                prompt_border,
                scenario.as_str(),
                &theme,
            ))

            // ─── FOOTER (height: 2) ──
            View(
                flex_direction: FlexDirection::Column,
                background_color: theme.bg,
                width: 100pct,
                height: 2,
                padding_left: 1,
                padding_right: 1,
            ) {
                MixedText(contents: vec![
                    MixedTextContent::new("● ").color(theme.success).weight(Weight::Bold),
                    MixedTextContent::new(
                        if data.status.is_empty() { "ready".to_string() } else { data.status.clone() }
                    ).color(theme.muted),
                ])
                #(if mode.read().is_overlay() {
                    element! {
                        MixedText(contents: vec![
                            MixedTextContent::new("↑/↓").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" navigate  ").color(theme.muted),
                            MixedTextContent::new("enter").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" select  ").color(theme.muted),
                            MixedTextContent::new("esc").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" close").color(theme.muted),
                        ])
                    }
                } else if cur_focus == Focus::Viewer {
                    element! {
                        MixedText(contents: vec![
                            MixedTextContent::new("j/k").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" scroll  ").color(theme.muted),
                            MixedTextContent::new("drag").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" select  ").color(theme.muted),
                            MixedTextContent::new("a").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" add  ").color(theme.muted),
                            MixedTextContent::new("esc").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" clear  ").color(theme.muted),
                            MixedTextContent::new("v").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" close").color(theme.muted),
                        ])
                    }
                } else if cur_focus == Focus::Prompt {
                    // Prompt focus: text-editing hints
                    element! {
                        MixedText(contents: vec![
                            MixedTextContent::new("type").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" text  ").color(theme.muted),
                            MixedTextContent::new("enter").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" newline  ").color(theme.muted),
                            MixedTextContent::new("ctrl-w").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" del word  ").color(theme.muted),
                            MixedTextContent::new("tab").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" focus  ").color(theme.muted),
                            MixedTextContent::new("esc").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" done").color(theme.muted),
                        ])
                    }
                } else if matches!(*mode.read(), crate::tui::mode::Mode::Search { .. }) {
                    // Search mode: typing into query
                    element! {
                        MixedText(contents: vec![
                            MixedTextContent::new("type").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" query  ").color(theme.muted),
                            MixedTextContent::new("enter").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" confirm  ").color(theme.muted),
                            MixedTextContent::new("esc").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" cancel").color(theme.muted),
                        ])
                    }
                } else if term_w < 100 {
                    // Narrow mode: show only essential bindings
                    element! {
                        MixedText(contents: vec![
                            MixedTextContent::new("tab").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" switch  ").color(theme.muted),
                            MixedTextContent::new("j/k").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" move  ").color(theme.muted),
                            MixedTextContent::new("space").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" toggle  ").color(theme.muted),
                            MixedTextContent::new("/").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" cmds  ").color(theme.muted),
                            MixedTextContent::new("q").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" quit").color(theme.muted),
                        ])
                    }
                } else {
                    element! {
                        MixedText(contents: vec![
                            MixedTextContent::new("tab").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" focus  ").color(theme.muted),
                            MixedTextContent::new("j/k").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" move  ").color(theme.muted),
                            MixedTextContent::new("space").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" toggle  ").color(theme.muted),
                            MixedTextContent::new("i").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" edit  ").color(theme.muted),
                            MixedTextContent::new("v").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" viewer  ").color(theme.muted),
                            MixedTextContent::new("/").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" commands  ").color(theme.muted),
                            MixedTextContent::new("ctrl-f").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" find  ").color(theme.muted),
                            MixedTextContent::new("q").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" quit").color(theme.muted),
                        ])
                    }
                })
            }

            // ─── OVERLAY LAYER (rendered above main layout) ──
            #(match &*mode.read() {
                crate::tui::mode::Mode::Help => Some(
                    crate::tui::overlays::card::render_card(
                        "HELP",
                        crate::tui::overlays::help::render_body(&theme),
                        &theme,
                        term_w,
                        term_h,
                    )
                ),
                crate::tui::mode::Mode::ScenarioPicker { cursor } => {
                    let scenarios = crate::tui::overlays::scenario_picker::load(&data.root);
                    let current = data.bundle.scenario.as_deref();
                    Some(crate::tui::overlays::card::render_card(
                        "SCENARIO",
                        crate::tui::overlays::scenario_picker::render_body(&scenarios, *cursor, current, &theme),
                        &theme,
                        term_w,
                        term_h,
                    ))
                }
                crate::tui::mode::Mode::CommandPalette { query, cursor } => {
                    let results = crate::tui::command_registry::filter(query);
                    Some(crate::tui::overlays::card::render_card(
                        "COMMANDS",
                        crate::tui::overlays::command_palette::render_body(query, &results, *cursor, &theme),
                        &theme,
                        term_w,
                        term_h,
                    ))
                }
                crate::tui::mode::Mode::ThemePicker { cursor } => {
                    let themes = crate::tui::overlays::theme_picker::all();
                    Some(crate::tui::overlays::card::render_card(
                        "THEME",
                        crate::tui::overlays::theme_picker::render_body(themes, *cursor, data.theme.name, &theme),
                        &theme,
                        term_w,
                        term_h,
                    ))
                }
                crate::tui::mode::Mode::DeliveryPicker { cursor } => {
                    Some(crate::tui::overlays::card::render_card(
                        "DELIVER",
                        crate::tui::overlays::delivery_picker::render_body(*cursor, &theme),
                        &theme,
                        term_w,
                        term_h,
                    ))
                }
                crate::tui::mode::Mode::AtPicker {
                    query, cursor, files,
                } => {
                    let ranked = crate::prompt_input::at_picker::rank(
                        files, query, 10,
                    );
                    let mut body: Vec<AnyElement<'static>> = Vec::new();
                    let display_query = format!("@{query}▏");
                    body.push(
                        element! {
                            MixedText(contents: vec![
                                MixedTextContent::new(display_query).color(theme.accent),
                            ])
                        }
                        .into_any(),
                    );
                    body.push(
                        element! {
                            Text(content: "─────────────────────────", color: theme.muted, weight: Weight::Light)
                        }
                        .into_any(),
                    );
                    if ranked.is_empty() {
                        body.push(
                            element! { Text(content: "  no matches", color: theme.muted) }
                                .into_any(),
                        );
                    } else {
                        for (i, path) in ranked.iter().enumerate() {
                            let selected = i == *cursor;
                            let gutter = if selected { "▶ " } else { "  " };
                            let path_str = path.display().to_string();
                            let (fg, bg) = if selected {
                                (Some(theme.selected_fg), Some(theme.selected_bg))
                            } else {
                                (None, None)
                            };
                            body.push(
                                element! {
                                    View(background_color: bg, width: 100pct) {
                                        MixedText(contents: vec![
                                            {
                                                let mut c = MixedTextContent::new(gutter);
                                                if let Some(col) = fg { c = c.color(col); }
                                                else { c = c.color(theme.accent); }
                                                c.weight(Weight::Bold)
                                            },
                                            {
                                                let mut c = MixedTextContent::new(path_str);
                                                if let Some(col) = fg { c = c.color(col); }
                                                c
                                            },
                                        ])
                                    }
                                }
                                .into_any(),
                            );
                        }
                    }
                    Some(crate::tui::overlays::card::render_card(
                        "@ FILE",
                        body,
                        &theme,
                        term_w,
                        term_h,
                    ))
                }
                crate::tui::mode::Mode::FullPromptPreview { content, scroll } => {
                    let lines: Vec<&str> = content.lines().collect();
                    let viewport = (term_h as usize).saturating_sub(8).max(5);
                    let start = (*scroll).min(lines.len().saturating_sub(viewport));
                    let end = (start + viewport).min(lines.len());
                    let mut body: Vec<AnyElement<'static>> = Vec::new();
                    for line in &lines[start..end] {
                        body.push(
                            element! { Text(content: line.to_string().leak() as &str) }
                                .into_any(),
                        );
                    }
                    if end < lines.len() {
                        let hint = format!("  …{} more lines (j/k to scroll, esc to close)", lines.len() - end);
                        body.push(
                            element! { Text(content: hint.leak() as &str, color: theme.muted, weight: Weight::Light) }
                                .into_any(),
                        );
                    }
                    Some(crate::tui::overlays::card::render_card(
                        "COMPOSED PROMPT",
                        body,
                        &theme,
                        term_w,
                        term_h,
                    ))
                }
                _ => None,
            })
        }
    }
}
