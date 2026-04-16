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
    bundle_summary::render_bundle_rows,
    header::Header,
    prompt_input::PromptInput,
    prompt_preview::render_preview,
    status_bar::render_footer,
    tree::render_tree_rows,
};
use crate::tui2::motion::use_animated;
use crate::tui2::theme::Theme;
use iocraft::prelude::*;
use std::collections::HashSet;
use std::path::PathBuf;

/// ─── Startup state (loaded once, owned by App) ────────────────────────

struct StartupData {
    bundle: Bundle,
    tree_entries: Vec<TreeEntry>,
    item_tokens: Vec<usize>,
    total_tokens: usize,
    model_name: String,
    model_window: usize,
    theme: Theme,
    preview: PromptPreview,
    bundled_paths: HashSet<PathBuf>,
}

fn load_startup(root: &CtxforgeRoot) -> StartupData {
    let bundle = Bundle::load_or_default(root).unwrap_or_default();
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

    let preview = build_preview(root, &bundle, &item_tokens);

    let _ = project_root; // consumed by tree::build above
    StartupData {
        bundle,
        tree_entries,
        item_tokens,
        total_tokens,
        model_name,
        model_window: model.window,
        theme,
        preview,
        bundled_paths,
    }
}

/// Build a PromptPreview from startup data (avoids needing a full v1 App).
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

/// ─── Focus ────────────────────────────────────────────────────────────

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

// ─── Entry point ──────────────────────────────────────────────────────

// Store startup data in a thread-local so the component closure can read it
// without threading it through props. `Option<StartupData>` — taken once at
// first render.
thread_local! {
    static STARTUP: std::cell::RefCell<Option<StartupData>> = const { std::cell::RefCell::new(None) };
}

pub async fn run(root: CtxforgeRoot) -> Result<()> {
    let data = load_startup(&root);
    STARTUP.with(|s| *s.borrow_mut() = Some(data));
    element!(App).render_loop().fullscreen().await?;
    Ok(())
}

/// ─── App component ───────────────────────────────────────────────────

#[component]
fn App(hooks: &mut Hooks) -> impl Into<AnyElement<'static>> {
    // Pull startup data out of the thread-local on first render. Keep it in
    // a hook-state slot so subsequent renders have it too.
    let startup = hooks.use_state(|| {
        STARTUP
            .with(|s| s.borrow_mut().take())
            .unwrap_or_else(|| panic!("tui2 startup data missing"))
    });

    let mut focus: State<Focus> = hooks.use_state(|| Focus::FileTree);
    let mut cursor: State<usize> = hooks.use_state(|| 0usize);

    // Filter visible entries (for Phase 1, all entries are visible — dir
    // expansion comes in Phase 2).
    let visible: Vec<(usize, TreeEntry)> = startup
        .read()
        .tree_entries
        .iter()
        .enumerate()
        .map(|(i, e)| (i, e.clone()))
        .collect();
    let max_cursor = visible.len().saturating_sub(1);

    hooks.use_terminal_events({
        move |event| {
            if let TerminalEvent::Key(k) = event {
                if k.kind != KeyEventKind::Press {
                    return;
                }
                match k.code {
                    KeyCode::Tab => {
                        let next = match *focus.read() {
                            Focus::FileTree => Focus::Viewer,
                            Focus::Viewer => Focus::BundleList,
                            Focus::BundleList => Focus::Prompt,
                            Focus::Prompt => Focus::FileTree,
                        };
                        *focus.write() = next;
                    }
                    KeyCode::BackTab => {
                        let prev = match *focus.read() {
                            Focus::FileTree => Focus::Prompt,
                            Focus::Viewer => Focus::FileTree,
                            Focus::BundleList => Focus::Viewer,
                            Focus::Prompt => Focus::BundleList,
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
                    _ => {}
                }
            }
        }
    });

    // Animated focus-border RGB. Tweens smoothly when focus changes.
    let cur_focus = *focus.read();
    let s = startup.read();
    let (tr, tg, tb) = focus_color_rgb(cur_focus, &s.theme);
    let anim_r = use_animated(hooks, tr, constants::FOCUS_BORDER, ease_out_cubic);
    let anim_g = use_animated(hooks, tg, constants::FOCUS_BORDER, ease_out_cubic);
    let anim_b = use_animated(hooks, tb, constants::FOCUS_BORDER, ease_out_cubic);
    let focus_color = Color::Rgb {
        r: anim_r,
        g: anim_g,
        b: anim_b,
    };

    let cur = *cursor.read();
    let theme = s.theme;

    // Tree rows.
    let tree_rows = render_tree_rows(&visible, cur, cur_focus == Focus::FileTree, &s.bundled_paths, &theme);
    let tree_title = format!(" files ({}) ", visible.len());
    let tree_border = if cur_focus == Focus::FileTree {
        focus_color
    } else {
        theme.border
    };

    // Bundle rows.
    let (bundle_title, bundle_rows) = render_bundle_rows(&s.bundle, &s.item_tokens, &theme);
    let bundle_border = if cur_focus == Focus::BundleList {
        focus_color
    } else {
        theme.border
    };

    // Preview lines.
    let preview_lines = render_preview(&s.preview, &theme);
    let preview_border = if cur_focus == Focus::Viewer {
        focus_color
    } else {
        theme.border
    };

    // Scenario string for prompt input.
    let scenario = s.bundle.scenario.clone().unwrap_or_default();

    element! {
        View(
            flex_direction: FlexDirection::Column,
            background_color: theme.bg,
            width: 100pct,
            height: 100pct,
        ) {
            Header(
                total_tokens: s.total_tokens,
                model_window: s.model_window,
                model_name: s.model_name.clone(),
                scenario: scenario.clone(),
                theme: Some(theme),
            )
            View(flex_direction: FlexDirection::Row, height: 100pct) {
                // ─── Left column: file tree + bundle summary ────────
                View(flex_direction: FlexDirection::Column, width: 28pct) {
                    View(
                        flex_direction: FlexDirection::Column,
                        border_style: BorderStyle::Round,
                        border_color: tree_border,
                        background_color: theme.bg,
                        height: 65pct,
                    ) {
                        Text(content: tree_title.leak() as &str, color: theme.muted, weight: Weight::Bold)
                        #(tree_rows)
                    }
                    View(
                        flex_direction: FlexDirection::Column,
                        border_style: BorderStyle::Round,
                        border_color: bundle_border,
                        background_color: theme.bg,
                        height: 35pct,
                    ) {
                        Text(content: bundle_title.leak() as &str, color: theme.muted, weight: Weight::Bold)
                        #(bundle_rows)
                    }
                }
                // ─── Center column: viewer placeholder ───────────────
                View(
                    flex_direction: FlexDirection::Column,
                    border_style: BorderStyle::Round,
                    border_color: if cur_focus == Focus::Viewer { focus_color } else { theme.border },
                    background_color: theme.bg,
                    width: 42pct,
                    padding: 1,
                ) {
                    Text(content: " viewer ", color: theme.muted, weight: Weight::Bold)
                    Text(content: "")
                    Text(
                        content: "  (code viewer lands in Phase 2)",
                        color: theme.muted,
                        weight: Weight::Light,
                    )
                    Text(content: "")
                    Text(content: "  Press v to toggle, drag to select,", weight: Weight::Light)
                    Text(content: "  a to add selection to bundle.", weight: Weight::Light)
                }
                // ─── Right column: prompt preview ────────────────────
                View(
                    flex_direction: FlexDirection::Column,
                    border_style: BorderStyle::Round,
                    border_color: preview_border,
                    background_color: theme.bg,
                    width: 30pct,
                    padding_left: 1,
                    padding_right: 1,
                    padding_top: 1,
                ) {
                    Text(content: " prompt preview ", color: theme.muted, weight: Weight::Bold)
                    Text(content: "")
                    #(preview_lines)
                }
            }
            PromptInput(
                focused: cur_focus == Focus::Prompt,
                border_color: Some(focus_color),
                scenario: scenario,
                theme: Some(theme),
            )
            #(vec![render_footer("Ready", &theme)])
        }
    }
}
