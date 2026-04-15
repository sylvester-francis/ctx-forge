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
    assert_eq!(p.text(), "ab", "backspace should drop the full multibyte char");
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
