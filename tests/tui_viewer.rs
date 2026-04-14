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

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn write_file(dir: &TempDir, name: &str, content: &[u8]) -> PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, content).unwrap();
    path
}

#[test]
fn read_small_text_file_returns_highlighted_lines() {
    let h = Highlighter::new();
    let tmp = TempDir::new().unwrap();
    let path = write_file(&tmp, "x.rs", b"fn main() {}\n");
    let load = ctxforge::tui::viewer::read_and_highlight(&path, &h);
    assert!(load.error.is_none());
    assert!(!load.truncated);
    assert_eq!(load.lines.len(), 1);
}

#[test]
fn read_binary_file_returns_binary_error() {
    let h = Highlighter::new();
    let tmp = TempDir::new().unwrap();
    let path = write_file(&tmp, "img.bin", &[0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    let load = ctxforge::tui::viewer::read_and_highlight(&path, &h);
    assert!(matches!(load.error, Some(ViewerError::Binary(_))));
    assert!(load.lines.is_empty());
}

#[test]
fn read_missing_file_returns_not_found() {
    let h = Highlighter::new();
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("nope.txt");
    let load = ctxforge::tui::viewer::read_and_highlight(&path, &h);
    assert_eq!(load.error, Some(ViewerError::NotFound));
}

#[test]
fn read_directory_returns_directory_error() {
    let h = Highlighter::new();
    let tmp = TempDir::new().unwrap();
    let load = ctxforge::tui::viewer::read_and_highlight(tmp.path(), &h);
    assert_eq!(load.error, Some(ViewerError::Directory));
}

#[test]
fn read_over_2mb_truncates_and_flags_truncated() {
    let h = Highlighter::new();
    let tmp = TempDir::new().unwrap();
    let big: Vec<u8> = std::iter::repeat(b'a').take(2_500_000).collect();
    let path = write_file(&tmp, "big.txt", &big);
    let load = ctxforge::tui::viewer::read_and_highlight(&path, &h);
    assert!(load.truncated);
    assert!(load.error.is_none());
    assert!(!load.lines.is_empty());
}
