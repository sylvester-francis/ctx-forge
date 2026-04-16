//! Per-theme text snapshots.
//!
//! Renders a fixed sample screen at 140×40 under each theme in the registry,
//! then snapshots the result via insta. These catch layout / text regressions
//! that accidentally affect a specific theme.
//!
//! Colour regressions are covered separately by:
//! - `tui_theme::every_theme_has_readable_accent_against_bg` (WCAG guard)
//! - `tui::theme::lint::forbidden_color_literals_are_absent` (no unthemed Color)

#![cfg(feature = "tui")]

use ctxforge::paths::CtxforgeRoot;
use ctxforge::tui::app::App;
use ctxforge::tui::motion::MockClock;
use ctxforge::tui::motion::MotionLevel;
use ctxforge::tui::ui;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

fn render_with_theme(theme_name: &str) -> String {
    let tmp = tempfile::tempdir().unwrap();
    let root = CtxforgeRoot::find_or_create(tmp.path()).unwrap();
    let mut app = App::new(root);
    let clock = MockClock::new();
    app.clock = Box::new(clock);
    app.motion = MotionLevel::None;
    app.theme = ctxforge::tui::theme::registry::by_name(theme_name).unwrap();
    app.startup_fade.snap(1.0);
    ctxforge::test_helpers::seed_fixture(&mut app, &["src/main.rs", "src/lib.rs"]);
    let mut terminal = Terminal::new(TestBackend::new(140, 40)).unwrap();
    terminal.draw(|f| ui::draw(f, &app)).unwrap();
    ctxforge::test_helpers::buffer_to_ansi_string(terminal.backend().buffer())
}

#[test]
fn ctxforge_snapshot() {
    insta::assert_snapshot!(render_with_theme("ctxforge"));
}

#[test]
fn zinc_snapshot() {
    insta::assert_snapshot!(render_with_theme("zinc"));
}

#[test]
fn tokyo_night_snapshot() {
    insta::assert_snapshot!(render_with_theme("tokyo-night"));
}

#[test]
fn gruvbox_snapshot() {
    insta::assert_snapshot!(render_with_theme("gruvbox"));
}
