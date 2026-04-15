//! Tests for the multi-line PromptInput widget.

#![cfg(feature = "tui")]

use ctxforge::tui::prompt_input::PromptInput;

#[test]
fn insert_text_advances_cursor() {
    let mut p = PromptInput::new();
    for c in "hello".chars() {
        p.insert_char(c);
    }
    assert_eq!(p.text(), "hello");
    assert_eq!(p.cursor(), 5);
}

#[test]
fn backspace_removes_char_behind_cursor() {
    let mut p = PromptInput::new();
    for c in "hello".chars() {
        p.insert_char(c);
    }
    p.backspace();
    assert_eq!(p.text(), "hell");
    assert_eq!(p.cursor(), 4);
}

#[test]
fn backspace_at_start_is_noop() {
    let mut p = PromptInput::new();
    p.backspace();
    assert_eq!(p.text(), "");
    assert_eq!(p.cursor(), 0);
}

#[test]
fn left_right_arrow_moves_cursor() {
    let mut p = PromptInput::new();
    for c in "abc".chars() {
        p.insert_char(c);
    }
    p.move_left();
    assert_eq!(p.cursor(), 2);
    p.move_right();
    assert_eq!(p.cursor(), 3);
    p.move_right();
    assert_eq!(p.cursor(), 3, "past end is a no-op");
}

#[test]
fn move_left_at_start_is_noop() {
    let mut p = PromptInput::new();
    p.move_left();
    assert_eq!(p.cursor(), 0);
}

#[test]
fn home_end_go_to_line_bounds() {
    let mut p = PromptInput::new();
    for c in "first\nsecond".chars() {
        p.insert_char(c);
    }
    assert_eq!(p.cursor(), 12);
    p.move_home();
    assert_eq!(p.cursor(), 6, "home should land after the '\\n'");
    p.move_end();
    assert_eq!(p.cursor(), 12);
}

#[test]
fn newline_increments_line_count() {
    let mut p = PromptInput::new();
    p.insert_char('a');
    p.insert_newline();
    p.insert_char('b');
    assert_eq!(p.text(), "a\nb");
    assert_eq!(p.line_count(), 2);
}

#[test]
fn insert_str_pastes_multiple_chars() {
    let mut p = PromptInput::new();
    p.insert_char('a');
    p.insert_str("BC");
    assert_eq!(p.text(), "aBC");
    assert_eq!(p.cursor(), 3);
}

#[test]
fn delete_word_back_removes_previous_word() {
    let mut p = PromptInput::new();
    p.insert_str("hello world");
    p.delete_word_back();
    assert_eq!(p.text(), "hello ");
    assert_eq!(p.cursor(), 6);
}

#[test]
fn unicode_insert_and_backspace_preserve_boundaries() {
    let mut p = PromptInput::new();
    p.insert_char('a');
    p.insert_char('日');
    p.insert_char('b');
    assert_eq!(p.text(), "a日b");
    p.move_left();
    p.backspace();
    assert_eq!(
        p.text(),
        "ab",
        "backspace should drop the full multibyte char"
    );
}

#[test]
fn set_text_and_take_text_round_trip() {
    let mut p = PromptInput::new();
    p.set_text("initial".to_string());
    assert_eq!(p.text(), "initial");
    assert_eq!(p.cursor(), "initial".len());
    let taken = p.take_text();
    assert_eq!(taken, "initial");
    assert_eq!(p.text(), "");
    assert_eq!(p.cursor(), 0);
}

// ---------------------------------------------------------------------------
// Integration tests: focus + event routing.
// ---------------------------------------------------------------------------

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ctxforge::paths::CtxforgeRoot;
use ctxforge::tui::app::{App, Focus};
use ctxforge::tui::motion::{MockClock, MotionLevel};

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
fn i_key_focuses_prompt_input() {
    let (mut app, _tmp) = test_app();
    app.bundle.scenario = Some("bugfix".to_string());
    assert_eq!(app.focus, Focus::FileTree);

    ctxforge::tui::events::handle(
        &mut app,
        KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE),
    );
    assert_eq!(app.focus, Focus::Prompt);
}

#[test]
fn typing_while_prompt_focused_builds_text() {
    let (mut app, _tmp) = test_app();
    app.bundle.scenario = Some("bugfix".to_string());
    app.focus_prompt();

    for c in "fix it".chars() {
        ctxforge::tui::events::handle(
            &mut app,
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
        );
    }
    assert_eq!(app.prompt_input.text(), "fix it");
}

#[test]
fn esc_while_prompt_focused_returns_to_last_panel() {
    let (mut app, _tmp) = test_app();
    app.bundle.scenario = Some("bugfix".to_string());
    app.focus = Focus::BundleList;
    app.last_panel_focus = Focus::BundleList;

    ctxforge::tui::events::handle(
        &mut app,
        KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE),
    );
    assert_eq!(app.focus, Focus::Prompt);

    ctxforge::tui::events::handle(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(app.focus, Focus::BundleList);
}

#[test]
fn backspace_in_prompt_removes_char() {
    let (mut app, _tmp) = test_app();
    app.bundle.scenario = Some("bugfix".to_string());
    app.focus_prompt();
    app.prompt_input.set_text("hello".to_string());

    ctxforge::tui::events::handle(
        &mut app,
        KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE),
    );
    assert_eq!(app.prompt_input.text(), "hell");
}

#[test]
fn ctrl_w_deletes_word_back_in_prompt() {
    let (mut app, _tmp) = test_app();
    app.bundle.scenario = Some("bugfix".to_string());
    app.focus_prompt();
    app.prompt_input.set_text("foo bar".to_string());

    ctxforge::tui::events::handle(
        &mut app,
        KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL),
    );
    assert_eq!(app.prompt_input.text(), "foo ");
}

#[test]
fn shift_enter_inserts_newline_when_prompt_focused() {
    let (mut app, _tmp) = test_app();
    app.bundle.scenario = Some("bugfix".to_string());
    app.focus_prompt();
    app.prompt_input.set_text("line1".to_string());

    ctxforge::tui::events::handle(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT));
    assert_eq!(app.prompt_input.text(), "line1\n");
}
