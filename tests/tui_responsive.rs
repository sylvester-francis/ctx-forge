//! Regression tests for responsive layout breakpoints.

#![cfg(feature = "tui")]

use ctxforge::paths::CtxforgeRoot;
use ctxforge::tui::app::App;
use ctxforge::tui::motion::{MockClock, MotionLevel};
use ctxforge::tui::ui;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

fn screen(cols: u16, rows: u16) -> String {
    let tmp = tempfile::tempdir().unwrap();
    let root = CtxforgeRoot::find_or_create(tmp.path()).unwrap();
    let mut app = App::new(root);
    app.clock = Box::new(MockClock::new());
    app.motion = MotionLevel::None;
    app.startup_fade.snap(1.0);
    app.bundle.scenario = Some("bugfix".to_string());
    ctxforge::test_helpers::seed_fixture(&mut app, &["src/main.rs"]);
    let mut terminal = Terminal::new(TestBackend::new(cols, rows)).unwrap();
    terminal.draw(|f| ui::draw(f, &app)).unwrap();
    ctxforge::test_helpers::buffer_to_ansi_string(terminal.backend().buffer())
}

#[test]
fn wide_layout_renders_tree_and_preview_and_prompt() {
    let s = screen(150, 40);
    assert!(s.contains("files"), "tree column missing:\n{s}");
    assert!(s.contains("prompt preview"), "preview missing:\n{s}");
    assert!(
        s.contains("prompt · scenario"),
        "prompt strip missing:\n{s}"
    );
}

#[test]
fn medium_layout_still_shows_preview_and_prompt() {
    let s = screen(125, 40);
    assert!(s.contains("files"));
    assert!(s.contains("prompt preview"));
    assert!(s.contains("prompt · scenario"));
}

#[test]
fn narrow_layout_stacks_vertically_and_keeps_prompt() {
    let s = screen(100, 50);
    assert!(
        s.contains("prompt preview"),
        "preview missing at narrow:\n{s}"
    );
    assert!(
        s.contains("prompt · scenario"),
        "prompt strip missing:\n{s}"
    );
}
