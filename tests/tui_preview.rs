//! Tests for the live prompt preview.

#![cfg(feature = "tui")]

use ctxforge::paths::CtxforgeRoot;
use ctxforge::tui::app::App;
use ctxforge::tui::motion::{MockClock, MotionLevel};
use ctxforge::tui::preview::PromptPreview;

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
fn preview_renders_sections_when_scenario_set() {
    let (mut app, _tmp) = test_app();
    app.bundle.scenario = Some("bugfix".to_string());
    app.bundle.task_text = "fix jwt validation".to_string();
    ctxforge::test_helpers::seed_fixture(&mut app, &["src/auth.rs"]);

    let preview = PromptPreview::from_app(&app);
    let text = preview.to_text();
    assert!(
        text.contains("scenario: bugfix"),
        "scenario marker missing:\n{text}"
    );
    assert!(
        text.contains("template:"),
        "template summary missing:\n{text}"
    );
    assert!(text.contains("task"), "task section missing:\n{text}");
    assert!(
        text.contains("fix jwt validation"),
        "task text missing:\n{text}"
    );
    assert!(text.contains("context"), "context section missing:\n{text}");
    assert!(
        text.contains("src/auth.rs"),
        "context item missing:\n{text}"
    );
}

#[test]
fn preview_renders_no_scenario_placeholder() {
    let (app, _tmp) = test_app();
    let preview = PromptPreview::from_app(&app);
    let text = preview.to_text();
    assert!(
        text.contains("no scenario"),
        "expected no-scenario placeholder; got:\n{text}"
    );
}

#[test]
fn preview_empty_task_hints_at_input() {
    let (mut app, _tmp) = test_app();
    app.bundle.scenario = Some("bugfix".to_string());
    // task_text left empty
    let preview = PromptPreview::from_app(&app);
    let text = preview.to_text();
    assert!(text.contains("press i to focus the prompt input"));
}

#[test]
fn preview_unknown_scenario_shows_error_section() {
    let (mut app, _tmp) = test_app();
    app.bundle.scenario = Some("ghost".to_string());
    let preview = PromptPreview::from_app(&app);
    let text = preview.to_text();
    assert!(
        text.contains("scenario 'ghost'"),
        "expected error marker; got:\n{text}"
    );
}

#[test]
fn capital_p_opens_full_preview_overlay() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let (mut app, _tmp) = test_app();
    app.bundle.scenario = Some("bugfix".to_string());

    ctxforge::tui::events::handle(
        &mut app,
        KeyEvent::new(KeyCode::Char('P'), KeyModifiers::NONE),
    );
    assert!(matches!(
        app.mode(),
        ctxforge::tui::mode::Mode::FullPromptPreview { .. }
    ));
}

#[test]
fn esc_closes_full_preview() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let (mut app, _tmp) = test_app();
    app.bundle.scenario = Some("bugfix".to_string());
    ctxforge::tui::events::handle(
        &mut app,
        KeyEvent::new(KeyCode::Char('P'), KeyModifiers::NONE),
    );
    ctxforge::tui::events::handle(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(matches!(app.mode(), ctxforge::tui::mode::Mode::Normal));
}

#[test]
fn full_preview_scrolls_down_with_j() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let (mut app, _tmp) = test_app();
    app.bundle.scenario = Some("bugfix".to_string());
    ctxforge::tui::events::handle(
        &mut app,
        KeyEvent::new(KeyCode::Char('P'), KeyModifiers::NONE),
    );
    ctxforge::tui::events::handle(
        &mut app,
        KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE),
    );
    if let ctxforge::tui::mode::Mode::FullPromptPreview { scroll, .. } = app.mode() {
        assert_eq!(*scroll, 1);
    } else {
        panic!("expected FullPromptPreview mode");
    }
}

#[test]
fn wide_layout_renders_preview_in_right_column() {
    let (mut app, _tmp) = test_app();
    app.bundle.scenario = Some("bugfix".to_string());
    ctxforge::test_helpers::seed_fixture(&mut app, &["src/main.rs"]);

    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(150, 40)).unwrap();
    terminal.draw(|f| ctxforge::tui::ui::draw(f, &app)).unwrap();
    let s = ctxforge::test_helpers::buffer_to_ansi_string(terminal.backend().buffer());

    assert!(
        s.contains("prompt preview"),
        "preview block title missing:\n{s}"
    );
    assert!(s.contains("task"), "Task section missing:\n{s}");
    assert!(s.contains("context"), "Context section missing:\n{s}");
    // Bundle summary still visible in lower-left
    assert!(
        s.contains("bundle (1)") || s.contains("bundle (1) "),
        "bundle summary label missing:\n{s}"
    );
    assert!(s.contains("src/main.rs"), "bundle item missing:\n{s}");
}
