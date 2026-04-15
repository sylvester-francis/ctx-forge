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

#[test]
fn scenario_picker_lists_builtins_and_project_templates() {
    let tmp = tempfile::tempdir().unwrap();
    let root = CtxforgeRoot::find_or_create(tmp.path()).unwrap();
    // Seed a project-local template.
    let templates = root.templates_dir();
    std::fs::create_dir_all(&templates).unwrap();
    std::fs::write(templates.join("my-template.md"), "{{bundle}}\n{{task}}\n").unwrap();

    let available = ctxforge::tui::scenario::available(&root);
    let names: Vec<_> = available.iter().map(|s| s.name.clone()).collect();
    assert!(names.contains(&"bugfix".to_string()));
    assert!(names.contains(&"code-review".to_string()));
    assert!(names.contains(&"explain".to_string()));
    assert!(names.contains(&"refactor".to_string()));
    assert!(names.contains(&"migrate".to_string()));
    assert!(names.contains(&"my-template".to_string()));
}

#[test]
fn set_scenario_persists_to_bundle_and_status() {
    let (mut app, tmp) = test_app();
    app.set_scenario("bugfix").unwrap();

    assert_eq!(app.bundle.scenario, Some("bugfix".to_string()));

    // Reload from disk — scenario survived.
    let reloaded = ctxforge::bundle::Bundle::load_or_default(
        &CtxforgeRoot::find_or_create(tmp.path()).unwrap(),
    )
    .unwrap();
    assert_eq!(reloaded.scenario, Some("bugfix".to_string()));
}

#[test]
fn set_scenario_rejects_unknown_name() {
    let (mut app, _tmp) = test_app();
    let err = app.set_scenario("nonexistent").unwrap_err();
    assert!(err.contains("unknown scenario"));
    assert_eq!(app.bundle.scenario, None);
}

#[test]
fn open_scenario_picker_enters_picker_mode() {
    let (mut app, _tmp) = test_app();
    app.open_scenario_picker();
    assert!(matches!(
        app.mode(),
        ctxforge::tui::mode::Mode::ScenarioPick { .. }
    ));
}

#[test]
fn picker_enter_picks_and_returns_to_normal() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let (mut app, _tmp) = test_app();
    app.open_scenario_picker();
    // Cursor starts at 0 = "bugfix"
    ctxforge::tui::events::handle(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert!(matches!(app.mode(), ctxforge::tui::mode::Mode::Normal));
    assert_eq!(app.bundle.scenario, Some("bugfix".to_string()));
}

#[test]
fn picker_esc_cancels_without_setting() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let (mut app, _tmp) = test_app();
    app.open_scenario_picker();
    ctxforge::tui::events::handle(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(matches!(app.mode(), ctxforge::tui::mode::Mode::Normal));
    assert_eq!(app.bundle.scenario, None);
}

#[test]
fn fresh_bundle_opens_scenario_picker() {
    let (mut app, _tmp) = test_app();
    assert!(app.bundle.scenario.is_none());

    app.auto_open_scenario_picker_if_needed();
    assert!(matches!(
        app.mode(),
        ctxforge::tui::mode::Mode::ScenarioPick { .. }
    ));
}

#[test]
fn bundle_with_scenario_skips_auto_picker() {
    let (mut app, _tmp) = test_app();
    app.bundle.scenario = Some("bugfix".to_string());

    app.auto_open_scenario_picker_if_needed();
    assert!(matches!(app.mode(), ctxforge::tui::mode::Mode::Normal));
}
