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
    element!(App).render_loop().fullscreen().await?;
    Ok(())
}

// Max tree rows rendered per frame. Phase 1 does no scrolling; we clip to
// a reasonable viewport so the layout doesn't overflow. Phase 2 adds
// proper viewport tracking + scrolling.
const TREE_VIEWPORT: usize = 24;

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

                match k.code {
                    KeyCode::Char('q') => *should_quit.write() = true,
                    KeyCode::Char('/') => {
                        *mode.write() = crate::tui2::mode::Mode::Search { query: String::new() };
                    }
                    KeyCode::Tab => {
                        let next = match *focus.read() {
                            Focus::FileTree => Focus::BundleList,
                            Focus::BundleList => Focus::Prompt,
                            Focus::Prompt => Focus::FileTree,
                            Focus::Viewer => Focus::FileTree,
                        };
                        *focus.write() = next;
                    }
                    KeyCode::BackTab => {
                        let prev = match *focus.read() {
                            Focus::FileTree => Focus::Prompt,
                            Focus::BundleList => Focus::FileTree,
                            Focus::Prompt => Focus::BundleList,
                            Focus::Viewer => Focus::FileTree,
                        };
                        *focus.write() = prev;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        let c = *cursor.read();
                        if c > 0 {
                            *cursor.write() = c - 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        let c = *cursor.read();
                        if c < max_cursor {
                            *cursor.write() = c + 1;
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
                    _ => {}
                }
            }
        }
    });

    if *should_quit.read() {
        std::process::exit(0);
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
    let start = cur.saturating_sub(TREE_VIEWPORT / 2);
    let end = (start + TREE_VIEWPORT).min(visible_count);
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

    let prompt_title = if scenario.is_empty() {
        " ⌥ prompt ".to_string()
    } else {
        format!(" ⌥ prompt · {} ", scenario)
    };

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
            // Phase 1 default: two-column. Viewer is opt-in (Phase 2).
            #(if wide {
                // Two-column: tree+bundle | preview (35/65 split favoring the preview)
                element! {
                    View(
                        flex_direction: FlexDirection::Row,
                        width: 100pct,
                        flex_grow: 1.0,
                    ) {
                        // Left: tree + bundle (narrow, list-focused)
                        View(
                            flex_direction: FlexDirection::Column,
                            width: 35pct,
                            height: 100pct,
                        ) {
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
                                #(render_tree_rows(&visible, cur, cur_focus == Focus::FileTree, &data.bundled_paths, &theme))
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
                        // Right: preview (primary — the crafted prompt output)
                        View(
                            flex_direction: FlexDirection::Column,
                            border_style: BorderStyle::Round,
                            border_color: preview_border,
                            background_color: theme.bg,
                            width: 65pct,
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
                        Focus::Viewer => (
                            "VIEWER".to_string(),
                            viewer_border,
                            vec![
                                element! { Text(content: "  ∘ code viewer arrives in Phase 2", color: theme.muted) }.into_any(),
                                element! { Text(content: "") }.into_any(),
                                element! { Text(content: "  ▸ v  toggle viewer", color: theme.muted, weight: Weight::Light) }.into_any(),
                            ],
                        ),
                        Focus::Prompt => (
                            preview_title_styled.clone(),
                            preview_border,
                            render_preview(&data.preview, &theme),
                        ),
                        Focus::FileTree => (
                            tree_title_styled.clone(),
                            tree_border,
                            render_tree_rows(&visible, cur, cur_focus == Focus::FileTree, &data.bundled_paths, &theme),
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

            // ─── PROMPT INPUT (height: 3) ──
            View(
                flex_direction: FlexDirection::Column,
                border_style: BorderStyle::Round,
                border_color: prompt_border,
                background_color: theme.bg,
                width: 100pct,
                height: 3,
                padding_left: 1,
                padding_right: 1,
            ) {
                MixedText(contents: vec![
                    MixedTextContent::new(prompt_title).color(theme.muted).weight(Weight::Bold),
                    MixedTextContent::new(" press i to edit — full editor arrives in Phase 2").color(theme.muted).weight(Weight::Light),
                ])
            }

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
                    MixedTextContent::new("ready").color(theme.muted),
                ])
                #(if term_w < 100 {
                    // Narrow mode: show only essential bindings
                    element! {
                        MixedText(contents: vec![
                            MixedTextContent::new("tab").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" switch  ").color(theme.muted),
                            MixedTextContent::new("j/k").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" move  ").color(theme.muted),
                            MixedTextContent::new("space").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" toggle  ").color(theme.muted),
                            MixedTextContent::new("?").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" help  ").color(theme.muted),
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
                            MixedTextContent::new("v").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" viewer  ").color(theme.muted),
                            MixedTextContent::new("d").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" deliver  ").color(theme.muted),
                            MixedTextContent::new("?").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" help  ").color(theme.muted),
                            MixedTextContent::new("q").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(" quit").color(theme.muted),
                        ])
                    }
                })
            }
        }
    }
}
