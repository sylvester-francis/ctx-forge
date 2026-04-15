//! Integration tests for the code viewer.
#![cfg(feature = "tui")]

use ctxforge::tui::viewer::{Highlighter, ViewerError, ViewerLoad, ViewerState};

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
    let big: Vec<u8> = std::iter::repeat_n(b'a', 2_500_000).collect();
    let path = write_file(&tmp, "big.txt", &big);
    let load = ctxforge::tui::viewer::read_and_highlight(&path, &h);
    assert!(load.truncated);
    assert!(load.error.is_none());
    assert!(!load.lines.is_empty());
}

#[test]
fn viewer_state_starts_disabled_and_empty() {
    let v = ViewerState::new();
    assert!(!v.enabled);
    assert_eq!(v.scroll, 0);
    assert!(v.cached_path.is_none());
    assert!(v.lines().is_empty());
    assert!(v.error().is_none());
    assert!(!v.truncated());
}

#[test]
fn viewer_toggle_flips_enabled() {
    let mut v = ViewerState::new();
    assert!(!v.enabled);
    v.toggle();
    assert!(v.enabled);
    v.toggle();
    assert!(!v.enabled);
}

#[test]
fn viewer_load_for_path_populates_cache() {
    let mut v = ViewerState::new();
    let tmp = TempDir::new().unwrap();
    let path = write_file(&tmp, "x.rs", b"fn main() {}\n");
    v.load_for_path(&path);
    assert_eq!(v.cached_path.as_deref(), Some(path.as_path()));
    assert_eq!(v.lines().len(), 1);
    assert_eq!(v.scroll, 0);
}

#[test]
fn viewer_load_for_same_path_is_idempotent() {
    let mut v = ViewerState::new();
    let tmp = TempDir::new().unwrap();
    let path = write_file(&tmp, "x.rs", b"fn main() {}\nfn other() {}\n");
    v.load_for_path(&path);
    v.scroll = 1;
    v.load_for_path(&path);
    assert_eq!(v.scroll, 1);
}

#[test]
fn viewer_load_for_different_path_resets_scroll() {
    let mut v = ViewerState::new();
    let tmp = TempDir::new().unwrap();
    let a = write_file(&tmp, "a.rs", b"a1\na2\na3\n");
    let b = write_file(&tmp, "b.rs", b"b1\nb2\nb3\n");
    v.load_for_path(&a);
    v.scroll = 2;
    v.load_for_path(&b);
    assert_eq!(v.scroll, 0);
}

#[test]
fn viewer_scroll_clamped_to_valid_range() {
    let mut v = ViewerState::new();
    let tmp = TempDir::new().unwrap();
    let path = write_file(&tmp, "x.txt", b"1\n2\n3\n4\n5\n");
    v.load_for_path(&path);
    v.scroll_by(10, 3);
    assert_eq!(v.scroll, 2);
    v.scroll_by(-100, 3);
    assert_eq!(v.scroll, 0);
}

#[test]
fn viewer_scroll_to_top_and_bottom_helpers() {
    let mut v = ViewerState::new();
    let tmp = TempDir::new().unwrap();
    let path = write_file(&tmp, "x.txt", b"1\n2\n3\n4\n5\n");
    v.load_for_path(&path);
    v.scroll_to_bottom(2);
    assert_eq!(v.scroll, 3);
    v.scroll_to_top();
    assert_eq!(v.scroll, 0);
}

#[test]
fn viewer_load_for_directory_sets_directory_error() {
    let mut v = ViewerState::new();
    let tmp = TempDir::new().unwrap();
    v.load_for_path(tmp.path());
    assert_eq!(v.error(), Some(&ViewerError::Directory));
    assert!(v.lines().is_empty());
}

// Silence unused warnings from the imports we'll need in later tasks.
#[allow(dead_code)]
fn _unused(_: ViewerLoad) {}

#[test]
fn viewer_renders_highlighted_lines_against_test_backend() {
    use ctxforge::paths::CtxforgeRoot;
    use ctxforge::tui::app::App;
    use ctxforge::tui::motion::{MockClock, MotionLevel};
    use ctxforge::tui::ui;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    let tmp = TempDir::new().unwrap();
    // 80-line file — guarantees the viewport can't show everything at once,
    // so scroll has somewhere to go.
    let lines: Vec<String> = (1..=80).map(|i| format!("fn f{i}() {{}}")).collect();
    std::fs::write(tmp.path().join("big.rs"), lines.join("\n") + "\n").unwrap();

    let root = CtxforgeRoot::find_or_create(tmp.path()).unwrap();
    let clock = MockClock::new();
    let mut app = App::new(root);
    app.clock = Box::new(clock.clone());
    app.motion = MotionLevel::Full;
    app.startup_fade.snap(1.0);

    app.toggle_viewer();

    let cached = app.viewer.cached_path.clone();
    assert!(cached.is_some(), "expected viewer to load on toggle");
    assert!(cached.unwrap().ends_with("big.rs"));
    assert_eq!(app.viewer.lines().len(), 80);

    // Render at 140x40 to hit the three-column layout; this records a
    // non-zero viewport_height on the app so scroll_by clamps meaningfully.
    let backend = TestBackend::new(140, 40);
    let mut term = Terminal::new(backend).unwrap();
    term.draw(|f| ui::draw(f, &app)).unwrap();

    app.move_viewer_scroll(2);
    assert_eq!(app.viewer.scroll, 2);
}
