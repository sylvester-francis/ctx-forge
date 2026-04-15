//! Integration tests for the scenario state + display.

#![cfg(feature = "tui")]

use ctxforge::paths::CtxforgeRoot;
use ctxforge::tui::app::App;
use ctxforge::tui::motion::{MockClock, MotionLevel};
use ctxforge::tui::ui;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

fn test_app() -> (App, tempfile::TempDir) {
    let tmp = tempfile::tempdir().unwrap();
    let root = CtxforgeRoot::find_or_create(tmp.path()).unwrap();
    let mut app = App::new(root);
    app.clock = Box::new(MockClock::new());
    app.motion = MotionLevel::None;
    app.startup_fade.snap(1.0);
    (app, tmp)
}

#[test]
fn header_renders_scenario_when_set() {
    let (mut app, _tmp) = test_app();
    app.bundle.scenario = Some("bugfix".to_string());

    let mut terminal = Terminal::new(TestBackend::new(140, 10)).unwrap();
    terminal.draw(|f| ui::draw(f, &app)).unwrap();
    let rendered = ctxforge::test_helpers::buffer_to_ansi_string(terminal.backend().buffer());
    assert!(
        rendered.contains("scenario: bugfix"),
        "expected scenario in header; got:\n{rendered}"
    );
}

#[test]
fn header_renders_none_placeholder_when_scenario_unset() {
    let (app, _tmp) = test_app();
    assert!(app.bundle.scenario.is_none());

    let mut terminal = Terminal::new(TestBackend::new(140, 10)).unwrap();
    terminal.draw(|f| ui::draw(f, &app)).unwrap();
    let rendered = ctxforge::test_helpers::buffer_to_ansi_string(terminal.backend().buffer());
    assert!(
        rendered.contains("scenario: (none)"),
        "expected (none) placeholder; got:\n{rendered}"
    );
}
