//! Tests for the deliver picker.

#![cfg(feature = "tui")]

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ctxforge::paths::CtxforgeRoot;
use ctxforge::tui::app::App;
use ctxforge::tui::deliver::DeliverChoice;
use ctxforge::tui::mode::Mode;
use ctxforge::tui::motion::{MockClock, MotionLevel};

fn setup() -> (App, tempfile::TempDir) {
    let tmp = tempfile::tempdir().unwrap();
    let root = CtxforgeRoot::find_or_create(tmp.path()).unwrap();
    let mut app = App::new(root);
    app.clock = Box::new(MockClock::new());
    app.motion = MotionLevel::None;
    app.startup_fade.snap(1.0);
    app.bundle.scenario = Some("bugfix".to_string());
    (app, tmp)
}

#[test]
fn ctrl_enter_in_prompt_opens_deliver_picker() {
    let (mut app, _tmp) = setup();
    app.focus_prompt();

    ctxforge::tui::events::handle(
        &mut app,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL),
    );
    assert!(matches!(app.mode(), Mode::DeliverPick { .. }));
}

#[test]
fn alt_enter_in_prompt_opens_deliver_picker() {
    let (mut app, _tmp) = setup();
    app.focus_prompt();

    ctxforge::tui::events::handle(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::ALT));
    assert!(matches!(app.mode(), Mode::DeliverPick { .. }));
}

#[test]
fn shift_enter_in_prompt_still_inserts_newline() {
    let (mut app, _tmp) = setup();
    app.focus_prompt();

    ctxforge::tui::events::handle(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT));
    assert!(matches!(app.mode(), Mode::Normal));
    assert_eq!(app.prompt_input.text(), "\n");
}

#[test]
fn esc_cancels_deliver_picker() {
    let (mut app, _tmp) = setup();
    app.focus_prompt();

    ctxforge::tui::events::handle(
        &mut app,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL),
    );
    ctxforge::tui::events::handle(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(matches!(app.mode(), Mode::Normal));
    assert!(app.deliver_last.is_none());
}

#[test]
fn down_advances_cursor_in_picker() {
    let (mut app, _tmp) = setup();
    app.focus_prompt();

    ctxforge::tui::events::handle(
        &mut app,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL),
    );
    ctxforge::tui::events::handle(&mut app, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    if let Mode::DeliverPick { cursor } = app.mode() {
        assert_eq!(*cursor, 1);
    } else {
        panic!("expected DeliverPick mode");
    }
}

#[test]
fn copy_markdown_writes_to_clipboard_and_records_last() {
    let (mut app, _tmp) = setup();
    app.bundle.task_text = "fix it".to_string();
    ctxforge::test_helpers::seed_fixture(&mut app, &["src/main.rs"]);

    match ctxforge::tui::deliver::run_choice(&mut app, DeliverChoice::CopyMarkdown) {
        Ok(content) => {
            assert!(
                content.contains("fix it"),
                "payload should include task text; got: {content}"
            );
            assert_eq!(app.deliver_last, Some(DeliverChoice::CopyMarkdown));
        }
        Err(e) if e.contains("clipboard") => {
            // Headless CI (no X11/Wayland) — skip gracefully.
            eprintln!("skipping clipboard test: {e}");
        }
        Err(e) => panic!("unexpected error: {e}"),
    }
}

#[test]
fn pipe_claude_stashes_pending_pipe() {
    let (mut app, _tmp) = setup();
    app.bundle.task_text = "fix it".to_string();
    ctxforge::test_helpers::seed_fixture(&mut app, &["src/main.rs"]);

    ctxforge::tui::deliver::run_choice(&mut app, DeliverChoice::PipeClaude)
        .expect("pipe claude should succeed");
    let (target, content) = app
        .pending_pipe
        .as_ref()
        .expect("pending_pipe should be set");
    assert_eq!(target, "claude");
    assert!(
        content.contains("fix it"),
        "payload should include task text; got: {content}"
    );
}

#[test]
fn export_stashes_pending_stdout() {
    let (mut app, _tmp) = setup();
    app.bundle.task_text = "fix it".to_string();
    ctxforge::test_helpers::seed_fixture(&mut app, &["src/main.rs"]);

    ctxforge::tui::deliver::run_choice(&mut app, DeliverChoice::Export)
        .expect("export should succeed");
    let content = app
        .pending_stdout
        .as_ref()
        .expect("pending_stdout should be set");
    assert!(content.contains("fix it"));
}

#[test]
fn prompt_override_wins_over_template_rendering() {
    let (mut app, _tmp) = setup();
    app.prompt_override = Some("HAND-EDITED".to_string());
    ctxforge::test_helpers::seed_fixture(&mut app, &["src/main.rs"]);

    // Use Export (pending_stdout) instead of CopyMarkdown to avoid
    // clipboard failures on headless CI.
    let content = ctxforge::tui::deliver::run_choice(&mut app, DeliverChoice::Export)
        .expect("export should succeed");
    assert_eq!(content, "HAND-EDITED");
    assert!(app.prompt_override.is_none());
}

#[test]
fn slash_deliver_opens_picker() {
    let (mut app, _tmp) = setup();
    // Simulate the command palette -> /deliver dispatch.
    let spec = ctxforge::tui::commands::COMMANDS
        .iter()
        .find(|c| c.name == "deliver")
        .expect("/deliver command should exist");
    (spec.action)(&mut app, None);
    assert!(matches!(app.mode(), Mode::DeliverPick { .. }));
}

#[test]
fn ctrl_enter_starts_picker_on_last_choice() {
    let (mut app, _tmp) = setup();
    app.focus_prompt();
    app.deliver_last = Some(DeliverChoice::CopyXml);

    ctxforge::tui::events::handle(
        &mut app,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL),
    );
    let expected_idx = DeliverChoice::all()
        .iter()
        .position(|&c| c == DeliverChoice::CopyXml)
        .unwrap();
    if let Mode::DeliverPick { cursor } = app.mode() {
        assert_eq!(*cursor, expected_idx);
    } else {
        panic!("expected DeliverPick mode");
    }
}
