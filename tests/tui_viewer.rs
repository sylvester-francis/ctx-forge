//! Integration tests for the code viewer.
#![cfg(feature = "tui")]

use ctxforge::tui::viewer::{Highlighter, ViewerError, ViewerLoad};

#[test]
fn viewer_error_display_messages() {
    assert_eq!(
        format!("{}", ViewerError::Directory),
        "↳ select a file to preview"
    );
    assert_eq!(
        format!("{}", ViewerError::Binary(1024)),
        "[binary file — 1024 bytes]"
    );
    assert_eq!(
        format!("{}", ViewerError::PermissionDenied),
        "permission denied"
    );
    assert_eq!(format!("{}", ViewerError::NotFound), "file not found");
    assert_eq!(format!("{}", ViewerError::Io("bad".into())), "error: bad");
}

#[test]
fn viewer_load_default_is_empty() {
    let l = ViewerLoad::default();
    assert!(l.lines.is_empty());
    assert!(!l.truncated);
    assert!(l.error.is_none());
}

#[test]
fn highlighter_constructs_with_bundled_defaults() {
    let _h = Highlighter::new();
}

#[test]
fn highlights_rust_file_produces_lines_with_spans() {
    let h = Highlighter::new();
    let source = "fn main() {\n    let x = 42;\n}\n";
    let lines = h.highlight("rs", source);
    assert_eq!(lines.len(), 3);
    for line in &lines {
        assert!(!line.spans.is_empty());
    }
}

#[test]
fn unknown_extension_falls_back_to_plaintext() {
    let h = Highlighter::new();
    let lines = h.highlight("xyz", "plain text\nanother line\n");
    assert_eq!(lines.len(), 2);
    for line in &lines {
        assert!(!line.spans.is_empty());
    }
}

#[test]
fn empty_input_produces_no_lines() {
    let h = Highlighter::new();
    let lines = h.highlight("rs", "");
    assert!(lines.is_empty());
}

#[test]
fn highlight_preserves_line_count() {
    let h = Highlighter::new();
    let source = "one\ntwo\nthree\nfour\nfive\n";
    let lines = h.highlight("txt", source);
    assert_eq!(lines.len(), 5);
}
