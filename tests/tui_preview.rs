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
        text.contains("template: bugfix · prefix"),
        "prefix marker missing:\n{text}"
    );
    assert!(text.contains("## Task"), "task section missing:\n{text}");
    assert!(
        text.contains("fix jwt validation"),
        "task text missing:\n{text}"
    );
    assert!(
        text.contains("## Context"),
        "context section missing:\n{text}"
    );
    assert!(
        text.contains("src/auth.rs"),
        "context item missing:\n{text}"
    );
    assert!(
        text.contains("template: bugfix · suffix"),
        "suffix marker missing:\n{text}"
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
    assert!(text.contains("empty — type in the prompt input"));
}

#[test]
fn preview_unknown_scenario_shows_error_section() {
    let (mut app, _tmp) = test_app();
    app.bundle.scenario = Some("ghost".to_string());
    let preview = PromptPreview::from_app(&app);
    let text = preview.to_text();
    assert!(
        text.contains("scenario ghost"),
        "expected error marker; got:\n{text}"
    );
}
