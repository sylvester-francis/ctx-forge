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
use crate::paths::{config_file_path, CtxforgeRoot};
use crate::resolve;
use crate::tokens;
use crate::tui::preview::PromptPreview;
use crate::tui::prompt_input::PromptInput;
use crate::tui::theme::{config::resolve_theme_name, registry, AppTheme};
use crate::tui::tree::{self, TreeEntry};
use crate::tui2::components::{
    bundle_summary::render_bundle_rows, prompt_preview::render_preview, tree::render_tree_rows,
};
use crate::tui2::motion::use_animated;
use crate::tui2::theme::Theme;
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
    // Transient status message shown in the footer.
    status: String,
    // Code viewer pane state.
    viewer: crate::tui2::viewer::ViewerState,
}

fn load_app_data(root: CtxforgeRoot) -> AppData {
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

    let bundled_paths: HashSet<PathBuf> = bundle.items.iter().map(|i| i.path.clone()).collect();
    let preview = build_preview(&root, &bundle, &item_tokens);

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
        viewer: crate::tui2::viewer::ViewerState::new(),
    }
}

impl AppData {
    /// Toggle file at `rel_path` in/out of the bundle. No-op on directories.
    /// Recomputes token counts and the preview. Persists the bundle to disk.
    fn toggle_bundle(&mut self, rel_path: &std::path::Path) {
        use crate::bundle::{Item, ItemKind};
        if self.bundled_paths.contains(rel_path) {
            self.bundle.remove_by_path(rel_path);
            self.bundled_paths.remove(rel_path);
        } else {
            let item = Item {
                path: rel_path.to_path_buf(),
                kind: ItemKind::File,
                label: None,
            };
            self.bundle.add(item);
            self.bundled_paths.insert(rel_path.to_path_buf());
        }
        self.recompute_tokens();
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
    }

    /// Dispatch a command action. Returns the new Mode to enter, or None
    /// for actions that only set status / quit / stay in Normal mode.
    fn dispatch_command(
        &mut self,
        action: crate::tui2::command_registry::CommandAction,
    ) -> Option<crate::tui2::mode::Mode> {
        use crate::tui2::command_registry::CommandAction as A;
        use crate::tui2::mode::Mode;
        match action {
            A::Help => Some(Mode::Help),
            A::Scenario => Some(Mode::ScenarioPicker { cursor: 0 }),
            A::Find => Some(Mode::Search { query: String::new() }),
            A::Quit => {
                self.set_status("quit requested".to_string());
                None
            }
            A::Theme => Some(crate::tui2::mode::Mode::ThemePicker { cursor: 0 }),
            A::ToggleViewer => {
                self.viewer.toggle();
                let enabled = self.viewer.enabled;
                self.set_status(if enabled { "viewer on".to_string() } else { "viewer off".to_string() });
                None
            }
            A::Deliver | A::EditPrompt | A::AddSelection => {
                self.set_status("coming in a follow-up phase (deliver / editor / add-selection)".to_string());
                None
            }
            A::Copy => { self.set_status("use CLI: ctxforge copy".to_string()); None }
            A::CopyXml => { self.set_status("use CLI: ctxforge copy --xml".to_string()); None }
            A::CopyJson => { self.set_status("use CLI: ctxforge copy --json".to_string()); None }
            A::Export => { self.set_status("use CLI: ctxforge export".to_string()); None }
            A::ExportXml => { self.set_status("use CLI: ctxforge export --xml".to_string()); None }
            A::ExportJson => { self.set_status("use CLI: ctxforge export --json".to_string()); None }
            A::Pipe => { self.set_status("use CLI: ctxforge copy | your-agent".to_string()); None }
            A::SaveProfile => { self.set_status("use CLI: ctxforge profile save <name>".to_string()); None }
            A::LoadProfile => { self.set_status("use CLI: ctxforge profile load <name>".to_string()); None }
            A::Narrow => { self.set_status("use CLI: ctxforge narrow <path> <start> <end>".to_string()); None }
            A::Model => { self.set_status("set model via --model flag or config.toml".to_string()); None }
            A::Memory => { self.set_status("use CLI: ctxforge memory".to_string()); None }
            A::Note => { self.set_status("use CLI: ctxforge memory note".to_string()); None }
            A::FindFn => { self.set_status("use CLI: ctxforge add --fn <name> <path>".to_string()); None }
            A::FindType => { self.set_status("use CLI: ctxforge add --type <name> <path>".to_string()); None }
            A::FindDiff => { self.set_status("use CLI: ctxforge add --diff <branch>".to_string()); None }
            A::Template => { self.set_status("use CLI: ctxforge template".to_string()); None }
            A::TemplateNew => { self.set_status("use CLI: ctxforge template new <name>".to_string()); None }
            A::TemplateRm => { self.set_status("use CLI: ctxforge template rm <name>".to_string()); None }
            A::TemplateStarters => { self.set_status("use CLI: ctxforge template starters".to_string()); None }
            A::TemplateList => { self.set_status("use CLI: ctxforge template list".to_string()); None }
        }
    }

    /// Apply a theme by name. Swaps the active theme, rebuilds the preview
    /// (so any theme-dependent colors re-render), and persists the selection
    /// to `~/.config/ctxforge/config.toml`.
    fn apply_theme(&mut self, name: &str) -> std::result::Result<(), String> {
        let raw = crate::tui::theme::registry::by_name(name)
            .ok_or_else(|| format!("unknown theme '{name}'"))?;
        self.theme = Theme::from_app_theme(raw);
        self.preview = build_preview(&self.root, &self.bundle, &self.item_tokens);

        if let Some(path) = crate::paths::config_file_path() {
            let existing = crate::tui::theme::config::load_from(&path).unwrap_or_default();
            let next = crate::tui::theme::config::Config {
                theme: name.to_string(),
                default_send: existing.default_send,
            };
            let _ = crate::tui::theme::config::save_to(&path, &next);
        }
        self.set_status(format!("theme → {name}"));
        Ok(())
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

/// Reload the viewer with the file at `new_cursor` in the visible tree.
/// No-op when the viewer is disabled or the cursor lands on a directory.
fn reload_viewer(data: &mut AppData, new_cursor: usize) {
    if !data.viewer.enabled {
        return;
    }
    let visible = tree::visible_indices(&data.tree_entries);
    if let Some(&idx) = visible.get(new_cursor) {
        if let Some(entry) = data.tree_entries.get(idx).cloned() {
            if !entry.is_dir {
                let abs = data.project_root.join(&entry.rel_path);
                data.viewer.load_for_path(&abs);
            }
        }
    }
}

fn build_preview(root: &CtxforgeRoot, bundle: &Bundle, item_tokens: &[usize]) -> PromptPreview {
    use crate::tui::preview::{ContextItem, Section};
    let mut sections = Vec::new();
    match &bundle.scenario {
        None => sections.push(Section::NoScenarioPlaceholder),
        Some(name) => match crate::tui::preview::render::load_wrapped(root, name) {
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
                let items = bundle
                    .items
                    .iter()
                    .zip(item_tokens.iter())
                    .map(|(it, tok)| ContextItem {
                        path: it.path.clone(),
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
}

pub async fn run(root: CtxforgeRoot) -> Result<()> {
    STARTUP.with(|s| *s.borrow_mut() = Some(load_app_data(root)));
    let result = element!(App).render_loop().fullscreen().await;

    // Belt-and-suspenders cleanup: iocraft's fullscreen enables mouse
    // capture by default and disables it on graceful exit, but if the loop
    // exits abnormally, mouse reporting can leak into the shell and
    // garble the terminal. Sending these is idempotent — safe regardless.
    use crossterm::event::{DisableBracketedPaste, DisableMouseCapture};
    let _ = crossterm::execute!(
        std::io::stdout(),
        DisableMouseCapture,
        DisableBracketedPaste,
    );

    result?;
    Ok(())
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
    let mut mode: State<crate::tui2::mode::Mode> = hooks.use_state(crate::tui2::mode::Mode::default);
    let mut prompt_input: State<PromptInput> = hooks.use_state(|| {
        let initial = app_data.read().bundle.task_text.clone();
        PromptInput::with_text(initial)
    });

    let (term_w, term_h) = hooks.use_terminal_size();

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
                // ── Search mode: accumulate chars, Esc cancels, Enter confirms ──
                if matches!(*mode.read(), crate::tui2::mode::Mode::Search { .. }) {
                    match k.code {
                        KeyCode::Esc | KeyCode::Enter => {
                            *mode.write() = crate::tui2::mode::Mode::Normal;
                            return;
                        }
                        KeyCode::Backspace => {
                            if let crate::tui2::mode::Mode::Search { query } = &mut *mode.write() {
                                query.pop();
                            }
                            return;
                        }
                        KeyCode::Char(c) => {
                            if let crate::tui2::mode::Mode::Search { query } = &mut *mode.write() {
                                query.push(c);
                            }
                            return;
                        }
                        _ => return,
                    }
                }

                // ── Help overlay ────────────────────────────────
                if matches!(*mode.read(), crate::tui2::mode::Mode::Help) {
                    match k.code {
                        KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => {
                            *mode.write() = crate::tui2::mode::Mode::Normal;
                        }
                        _ => {}
                    }
                    return;
                }

                // ── Scenario picker overlay ─────────────────────
                if matches!(*mode.read(), crate::tui2::mode::Mode::ScenarioPicker { .. }) {
                    let scenarios = crate::tui2::overlays::scenario_picker::load(&app_data.read().root);
                    let count = scenarios.len();
                    match k.code {
                        KeyCode::Esc => {
                            *mode.write() = crate::tui2::mode::Mode::Normal;
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if let crate::tui2::mode::Mode::ScenarioPicker { cursor } = &mut *mode.write() {
                                if *cursor > 0 {
                                    *cursor -= 1;
                                }
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if let crate::tui2::mode::Mode::ScenarioPicker { cursor } = &mut *mode.write() {
                                if *cursor + 1 < count {
                                    *cursor += 1;
                                }
                            }
                        }
                        KeyCode::Enter => {
                            let cur = match *mode.read() {
                                crate::tui2::mode::Mode::ScenarioPicker { cursor } => cursor,
                                _ => 0,
                            };
                            if let Some(picked) = scenarios.get(cur) {
                                let name = picked.name.clone();
                                app_data.write().set_scenario(Some(name));
                            }
                            *mode.write() = crate::tui2::mode::Mode::Normal;
                        }
                        _ => {}
                    }
                    return;
                }

                // ── Theme picker overlay ────────────────────────
                if matches!(*mode.read(), crate::tui2::mode::Mode::ThemePicker { .. }) {
                    let themes = crate::tui2::overlays::theme_picker::all();
                    let count = themes.len();
                    match k.code {
                        KeyCode::Esc => {
                            *mode.write() = crate::tui2::mode::Mode::Normal;
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if let crate::tui2::mode::Mode::ThemePicker { cursor } = &mut *mode.write() {
                                if *cursor > 0 {
                                    *cursor -= 1;
                                }
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if let crate::tui2::mode::Mode::ThemePicker { cursor } = &mut *mode.write() {
                                if *cursor + 1 < count {
                                    *cursor += 1;
                                }
                            }
                        }
                        KeyCode::Enter => {
                            let cur = match *mode.read() {
                                crate::tui2::mode::Mode::ThemePicker { cursor } => cursor,
                                _ => 0,
                            };
                            if let Some(picked) = themes.get(cur) {
                                let name = picked.name.to_string();
                                let _ = app_data.write().apply_theme(&name);
                            }
                            *mode.write() = crate::tui2::mode::Mode::Normal;
                        }
                        _ => {}
                    }
                    return;
                }

                // ── Command palette overlay ────────────────────
                if matches!(*mode.read(), crate::tui2::mode::Mode::CommandPalette { .. }) {
                    match k.code {
                        KeyCode::Esc => {
                            *mode.write() = crate::tui2::mode::Mode::Normal;
                        }
                        KeyCode::Up => {
                            if let crate::tui2::mode::Mode::CommandPalette { cursor, .. } =
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
                                if let crate::tui2::mode::Mode::CommandPalette { query, .. } = &*m {
                                    crate::tui2::command_registry::filter(query).len()
                                } else {
                                    0
                                }
                            };
                            if let crate::tui2::mode::Mode::CommandPalette { cursor, .. } =
                                &mut *mode.write()
                            {
                                if *cursor + 1 < count {
                                    *cursor += 1;
                                }
                            }
                        }
                        KeyCode::Backspace => {
                            if let crate::tui2::mode::Mode::CommandPalette { query, cursor } =
                                &mut *mode.write()
                            {
                                query.pop();
                                *cursor = 0;
                            }
                        }
                        KeyCode::Char(c) => {
                            if let crate::tui2::mode::Mode::CommandPalette { query, cursor } =
                                &mut *mode.write()
                            {
                                query.push(c);
                                *cursor = 0;
                            }
                        }
                        KeyCode::Enter => {
                            let (query_owned, cursor_idx) = match &*mode.read() {
                                crate::tui2::mode::Mode::CommandPalette { query, cursor } => {
                                    (query.clone(), *cursor)
                                }
                                _ => (String::new(), 0),
                            };
                            let results = crate::tui2::command_registry::filter(&query_owned);
                            if let Some(cmd) = results.get(cursor_idx) {
                                let action = cmd.action;
                                let next_mode = app_data.write().dispatch_command(action);
                                match next_mode {
                                    Some(m) => *mode.write() = m,
                                    None => {
                                        if matches!(
                                            action,
                                            crate::tui2::command_registry::CommandAction::Quit
                                        ) {
                                            *should_quit.write() = true;
                                        }
                                        *mode.write() = crate::tui2::mode::Mode::Normal;
                                    }
                                }
                            } else {
                                *mode.write() = crate::tui2::mode::Mode::Normal;
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
                        *mode.write() = crate::tui2::mode::Mode::Help;
                    }
                    KeyCode::Char('S') => {
                        *mode.write() = crate::tui2::mode::Mode::ScenarioPicker { cursor: 0 };
                    }
                    KeyCode::Char('/') => {
                        *mode.write() = crate::tui2::mode::Mode::CommandPalette {
                            query: String::new(),
                            cursor: 0,
                        };
                    }
                    KeyCode::Char('f') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                        *mode.write() = crate::tui2::mode::Mode::Search { query: String::new() };
                    }
                    KeyCode::Char('i') if *focus.read() != Focus::Prompt => {
                        *focus.write() = Focus::Prompt;
                    }
                    KeyCode::Char('v') => {
                        // Snapshot what we need, release app_data lock, then do
                        // both state writes without nesting — iocraft's state
                        // updates are more reliable this way than trying to
                        // mutate focus while holding an app_data guard.
                        let (now_enabled, loaded_path) = {
                            let mut d = app_data.write();
                            d.viewer.toggle();
                            let enabled = d.viewer.enabled;
                            let path = if enabled {
                                let visible = tree::visible_indices(&d.tree_entries);
                                visible.get(*cursor.read())
                                    .copied()
                                    .and_then(|idx| d.tree_entries.get(idx).cloned())
                                    .filter(|e| !e.is_dir)
                                    .map(|e| d.project_root.join(&e.rel_path))
                            } else {
                                None
                            };
                            if let Some(p) = &path {
                                d.viewer.load_for_path(p);
                            }
                            d.set_status(if enabled { "viewer on".to_string() } else { "viewer off".to_string() });
                            (enabled, path)
                        };
                        let _ = loaded_path;
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
                                if viewer_on { Focus::Viewer } else { Focus::BundleList }
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
                                if viewer_on { Focus::Viewer } else { Focus::FileTree }
                            }
                            Focus::Prompt => Focus::BundleList,
                        };
                        *focus.write() = prev;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        match *focus.read() {
                            Focus::Viewer => {
                                app_data.write().viewer.scroll_by(-1, viewer_viewport(term_h));
                            }
                            _ => {
                                let c = *cursor.read();
                                if c > 0 {
                                    let new = c - 1;
                                    cursor.set(new);
                                    reload_viewer(&mut app_data.write(), new);
                                }
                            }
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        match *focus.read() {
                            Focus::Viewer => {
                                app_data.write().viewer.scroll_by(1, viewer_viewport(term_h));
                            }
                            _ => {
                                let c = *cursor.read();
                                if c < max_cursor {
                                    let new = c + 1;
                                    cursor.set(new);
                                    reload_viewer(&mut app_data.write(), new);
                                }
                            }
                        }
                    }
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
                            let is_dir = d.tree_entries.get(actual).map(|e| e.is_dir).unwrap_or(false);
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
                    // Navigation: g/G top/bottom, Ctrl-U/D half-page
                    KeyCode::Char('g') if *focus.read() == Focus::FileTree => {
                        cursor.set(0);
                        reload_viewer(&mut app_data.write(), 0);
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
                            reload_viewer(&mut app_data.write(), last);
                        }
                    }
                    KeyCode::Char('G') if *focus.read() == Focus::Viewer => {
                        app_data.write().viewer.scroll_to_bottom(viewer_viewport(term_h));
                    }
                    KeyCode::Char('u')
                        if k.modifiers.contains(KeyModifiers::CONTROL)
                            && *focus.read() == Focus::FileTree =>
                    {
                        let half = tree_viewport(term_h) / 2;
                        let c = *cursor.read();
                        let new = c.saturating_sub(half);
                        cursor.set(new);
                        reload_viewer(&mut app_data.write(), new);
                    }
                    KeyCode::Char('u')
                        if k.modifiers.contains(KeyModifiers::CONTROL)
                            && *focus.read() == Focus::Viewer =>
                    {
                        app_data.write().viewer.scroll_by(-(viewer_viewport(term_h) as i32 / 2), viewer_viewport(term_h));
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
                        reload_viewer(&mut app_data.write(), new);
                    }
                    KeyCode::Char('d')
                        if k.modifiers.contains(KeyModifiers::CONTROL)
                            && *focus.read() == Focus::Viewer =>
                    {
                        app_data.write().viewer.scroll_by(viewer_viewport(term_h) as i32 / 2, viewer_viewport(term_h));
                    }
                    _ => {}
                }
            }
        }
    });

    if *should_quit.read() {
        // Use iocraft's system.exit() so the render loop unwinds cleanly,
        // disabling mouse capture / bracketed paste / alternate screen.
        // std::process::exit() would bypass all of that and leave the
        // terminal in a dirty state (mouse reports leaking into the shell).
        let mut system = hooks.use_context_mut::<iocraft::SystemContext>();
        system.exit();
    }

    // Re-read app_data after the event handler may have mutated it, and
    // clamp the cursor so it stays within the (possibly shrunken) visible
    // range — relevant after collapse-all.
    drop(data);
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
        crate::tui2::mode::Mode::Search { query } => Some(query.clone()),
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
    let empty = if partial.is_empty() { empty } else { empty.saturating_sub(1) };
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
                                crate::tui2::components::search_bar::render_search_bar(q, &theme)
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
                                    crate::tui2::components::tree::render_search_rows(
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
                                    #(crate::tui2::components::viewer::render_body(&data.viewer, &theme))
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
                            (
                                title,
                                viewer_border,
                                crate::tui2::components::viewer::render_body(&data.viewer, &theme),
                            )
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
                                crate::tui2::components::tree::render_search_rows(
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
            #(crate::tui2::components::prompt_input::render_prompt_input(
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
                            MixedTextContent::new("g/G").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" top/bot  ").color(theme.muted),
                            MixedTextContent::new("ctrl-u/d").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" page  ").color(theme.muted),
                            MixedTextContent::new("v").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" close  ").color(theme.muted),
                            MixedTextContent::new("tab").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" focus").color(theme.muted),
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
                } else if matches!(*mode.read(), crate::tui2::mode::Mode::Search { .. }) {
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
                crate::tui2::mode::Mode::Help => Some(
                    crate::tui2::overlays::card::render_card(
                        "HELP",
                        crate::tui2::overlays::help::render_body(&theme),
                        &theme,
                        term_w,
                        term_h,
                    )
                ),
                crate::tui2::mode::Mode::ScenarioPicker { cursor } => {
                    let scenarios = crate::tui2::overlays::scenario_picker::load(&data.root);
                    let current = data.bundle.scenario.as_deref();
                    Some(crate::tui2::overlays::card::render_card(
                        "SCENARIO",
                        crate::tui2::overlays::scenario_picker::render_body(&scenarios, *cursor, current, &theme),
                        &theme,
                        term_w,
                        term_h,
                    ))
                }
                crate::tui2::mode::Mode::CommandPalette { query, cursor } => {
                    let results = crate::tui2::command_registry::filter(query);
                    Some(crate::tui2::overlays::card::render_card(
                        "COMMANDS",
                        crate::tui2::overlays::command_palette::render_body(query, &results, *cursor, &theme),
                        &theme,
                        term_w,
                        term_h,
                    ))
                }
                crate::tui2::mode::Mode::ThemePicker { cursor } => {
                    let themes = crate::tui2::overlays::theme_picker::all();
                    Some(crate::tui2::overlays::card::render_card(
                        "THEME",
                        crate::tui2::overlays::theme_picker::render_body(themes, *cursor, data.theme.name, &theme),
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
