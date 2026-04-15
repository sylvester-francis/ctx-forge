//! Integration tests for the `@` file picker popover.

#![cfg(feature = "tui")]

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ctxforge::paths::CtxforgeRoot;
use ctxforge::tui::app::App;
use ctxforge::tui::mode::Mode;
use ctxforge::tui::motion::{MockClock, MotionLevel};

fn setup(files: &[&str]) -> (App, tempfile::TempDir) {
    let tmp = tempfile::tempdir().unwrap();
    for f in files {
        let p = tmp.path().join(f);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&p, "").unwrap();
    }
    let root = CtxforgeRoot::find_or_create(tmp.path()).unwrap();
    let mut app = App::new(root);
    app.clock = Box::new(MockClock::new());
    app.motion = MotionLevel::None;
    app.startup_fade.snap(1.0);
    app.bundle.scenario = Some("bugfix".to_string());
    (app, tmp)
}

#[test]
fn at_key_opens_picker_mode() {
    let (mut app, _tmp) = setup(&["auth.rs", "other.rs"]);
    app.focus_prompt();

    ctxforge::tui::events::handle(
        &mut app,
        KeyEvent::new(KeyCode::Char('@'), KeyModifiers::NONE),
    );
    assert!(matches!(app.mode(), Mode::AtPicker { .. }));
    // '@' was typed into the prompt buffer.
    assert_eq!(app.prompt_input.text(), "@");
}

#[test]
fn typing_after_at_filters_results() {
    let (mut app, _tmp) = setup(&["auth.rs", "authzone.rs", "other.rs"]);
    app.focus_prompt();

    ctxforge::tui::events::handle(
        &mut app,
        KeyEvent::new(KeyCode::Char('@'), KeyModifiers::NONE),
    );
    for c in "au".chars() {
        ctxforge::tui::events::handle(
            &mut app,
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
        );
    }

    if let Mode::AtPicker { results, .. } = app.mode() {
        let names: Vec<_> = results
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(names.iter().any(|n| n == "auth.rs"));
        assert!(names.iter().any(|n| n == "authzone.rs"));
        assert!(!names.iter().any(|n| n == "other.rs"));
    } else {
        panic!("expected AtPicker mode, got {:?}", app.mode());
    }
    // The query characters were also typed into the prompt so the user
    // sees `@au` growing in place.
    assert_eq!(app.prompt_input.text(), "@au");
}

#[test]
fn esc_closes_picker_without_further_insertion() {
    let (mut app, _tmp) = setup(&["auth.rs"]);
    app.focus_prompt();

    ctxforge::tui::events::handle(
        &mut app,
        KeyEvent::new(KeyCode::Char('@'), KeyModifiers::NONE),
    );
    ctxforge::tui::events::handle(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    assert!(matches!(app.mode(), Mode::Normal));
    // '@' was typed on open. Esc cancels the picker without removing it.
    assert_eq!(app.prompt_input.text(), "@");
}

#[test]
fn backspace_on_empty_query_closes_picker() {
    let (mut app, _tmp) = setup(&["auth.rs"]);
    app.focus_prompt();

    ctxforge::tui::events::handle(
        &mut app,
        KeyEvent::new(KeyCode::Char('@'), KeyModifiers::NONE),
    );
    // Empty query + Backspace: should close cleanly.
    ctxforge::tui::events::handle(
        &mut app,
        KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE),
    );
    assert!(matches!(app.mode(), Mode::Normal));
}

#[test]
fn backspace_trims_query_and_reranks() {
    let (mut app, _tmp) = setup(&["auth.rs", "authzone.rs", "other.rs"]);
    app.focus_prompt();

    ctxforge::tui::events::handle(
        &mut app,
        KeyEvent::new(KeyCode::Char('@'), KeyModifiers::NONE),
    );
    for c in "au".chars() {
        ctxforge::tui::events::handle(
            &mut app,
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
        );
    }
    ctxforge::tui::events::handle(
        &mut app,
        KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE),
    );

    if let Mode::AtPicker { query, .. } = app.mode() {
        assert_eq!(query, "a");
    } else {
        panic!("expected AtPicker mode");
    }
}

#[test]
fn down_key_advances_cursor() {
    let (mut app, _tmp) = setup(&["auth.rs", "authzone.rs"]);
    app.focus_prompt();

    ctxforge::tui::events::handle(
        &mut app,
        KeyEvent::new(KeyCode::Char('@'), KeyModifiers::NONE),
    );
    ctxforge::tui::events::handle(&mut app, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));

    if let Mode::AtPicker { cursor, .. } = app.mode() {
        assert_eq!(*cursor, 1);
    } else {
        panic!("expected AtPicker mode");
    }
}

#[test]
fn enter_inserts_path_and_adds_to_bundle() {
    let (mut app, _tmp) = setup(&["auth.rs", "other.rs"]);
    app.focus_prompt();

    ctxforge::tui::events::handle(
        &mut app,
        KeyEvent::new(KeyCode::Char('@'), KeyModifiers::NONE),
    );
    for c in "au".chars() {
        ctxforge::tui::events::handle(
            &mut app,
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
        );
    }
    ctxforge::tui::events::handle(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert!(matches!(app.mode(), Mode::Normal));
    // `@au` replaced with `@auth.rs` (the top-ranked match).
    assert_eq!(app.prompt_input.text(), "@auth.rs");
    assert_eq!(app.bundle.task_text, "@auth.rs");
    // File added to the bundle.
    let paths: Vec<_> = app.bundle.items.iter().map(|i| i.path.clone()).collect();
    assert!(paths.contains(&std::path::PathBuf::from("auth.rs")));
}

#[test]
fn enter_with_no_results_closes_cleanly() {
    let (mut app, _tmp) = setup(&["auth.rs"]);
    app.focus_prompt();

    ctxforge::tui::events::handle(
        &mut app,
        KeyEvent::new(KeyCode::Char('@'), KeyModifiers::NONE),
    );
    for c in "zzzzzz".chars() {
        ctxforge::tui::events::handle(
            &mut app,
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
        );
    }
    assert!(matches!(app.mode(), Mode::AtPicker { .. }));

    ctxforge::tui::events::handle(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert!(matches!(app.mode(), Mode::Normal));
    // No items added; prompt left as typed.
    assert!(app.bundle.items.is_empty());
}

#[test]
fn confirming_same_file_twice_does_not_duplicate() {
    let (mut app, _tmp) = setup(&["auth.rs", "other.rs"]);
    app.focus_prompt();

    // First mention
    ctxforge::tui::events::handle(
        &mut app,
        KeyEvent::new(KeyCode::Char('@'), KeyModifiers::NONE),
    );
    ctxforge::tui::events::handle(
        &mut app,
        KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE),
    );
    ctxforge::tui::events::handle(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(app.bundle.items.len(), 1);

    // Second mention of the same file
    ctxforge::tui::events::handle(
        &mut app,
        KeyEvent::new(KeyCode::Char('@'), KeyModifiers::NONE),
    );
    ctxforge::tui::events::handle(
        &mut app,
        KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE),
    );
    ctxforge::tui::events::handle(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(
        app.bundle.items.len(),
        1,
        "bundle should not gain a duplicate row"
    );
    // Status reflects dedupe.
    assert!(app.status_message.contains("already in bundle"));
}

#[test]
fn at_in_paste_does_not_open_picker() {
    // Pasted text with `@` should not trigger the picker — paste routes
    // through handle_paste, which inserts atomically.
    let (mut app, _tmp) = setup(&["auth.rs"]);
    app.focus_prompt();

    ctxforge::tui::events::handle_paste(&mut app, "me@example.com".to_string());
    assert!(matches!(app.mode(), Mode::Normal));
    assert_eq!(app.prompt_input.text(), "me@example.com");
}
