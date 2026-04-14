//! File loading for the viewer: binary detection, 2 MB truncation,
//! `ViewerError` variants.

use std::fmt;

/// Why the viewer can't display the current file. Each variant has a
/// canonical `Display` impl used by the render code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewerError {
    /// Cursor is on a directory — nothing to preview.
    Directory,
    /// File contains NUL bytes in the first 4 KB — treated as binary.
    Binary(u64),
    PermissionDenied,
    NotFound,
    Io(String),
}

impl fmt::Display for ViewerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ViewerError::Directory => write!(f, "↳ select a file to preview"),
            ViewerError::Binary(n) => write!(f, "[binary file — {n} bytes]"),
            ViewerError::PermissionDenied => write!(f, "permission denied"),
            ViewerError::NotFound => write!(f, "file not found"),
            ViewerError::Io(msg) => write!(f, "error: {msg}"),
        }
    }
}

/// Result of reading a file for the viewer. Contains either highlighted
/// lines (on success) or an error state. `truncated` is true when the file
/// was larger than the 2 MB limit and only the leading portion was read.
#[derive(Default)]
pub struct ViewerLoad {
    pub lines: Vec<ratatui::text::Line<'static>>,
    pub truncated: bool,
    pub error: Option<ViewerError>,
}
