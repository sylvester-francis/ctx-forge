//! Integration tests for the code viewer.
#![cfg(feature = "tui")]

use ctxforge::tui::viewer::{ViewerError, ViewerLoad};

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
