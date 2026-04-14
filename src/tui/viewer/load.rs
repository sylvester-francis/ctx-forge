//! File loading for the viewer: binary detection, 2 MB truncation,
//! `ViewerError` variants.

use std::fmt;
use std::fs;
use std::io::{ErrorKind, Read};
use std::path::Path;

use super::Highlighter;

/// Maximum bytes read from a single file. Larger files are truncated at
/// this boundary with `truncated = true`.
const MAX_READ_BYTES: usize = 2 * 1024 * 1024;
/// Bytes peeked for binary detection (NUL scan).
const BINARY_PEEK_BYTES: usize = 4096;

/// Load and highlight a file for the viewer. Resolves errors to
/// `ViewerError` variants rather than `Result` so the UI can display them
/// directly.
pub fn read_and_highlight(path: &Path, highlighter: &Highlighter) -> ViewerLoad {
    match fs::metadata(path) {
        Ok(md) if md.is_dir() => {
            return ViewerLoad {
                error: Some(ViewerError::Directory),
                ..Default::default()
            };
        }
        Ok(_) => {}
        Err(e) => {
            return ViewerLoad {
                error: Some(io_error(&e)),
                ..Default::default()
            };
        }
    }

    let mut file = match fs::File::open(path) {
        Ok(f) => f,
        Err(e) => {
            return ViewerLoad {
                error: Some(io_error(&e)),
                ..Default::default()
            };
        }
    };

    // Peek the first 4 KB for NUL bytes — the universal binary heuristic.
    let mut peek = [0u8; BINARY_PEEK_BYTES];
    let peek_n = match file.read(&mut peek) {
        Ok(n) => n,
        Err(e) => {
            return ViewerLoad {
                error: Some(ViewerError::Io(e.to_string())),
                ..Default::default()
            };
        }
    };
    if peek[..peek_n].contains(&0u8) {
        let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        return ViewerLoad {
            error: Some(ViewerError::Binary(size)),
            ..Default::default()
        };
    }

    // Read up to MAX_READ_BYTES total (peek is already consumed).
    let mut buf = peek[..peek_n].to_vec();
    let remaining = MAX_READ_BYTES.saturating_sub(peek_n);
    let mut truncated = false;
    if remaining > 0 {
        let mut rest = Vec::with_capacity(remaining);
        let mut take = file.take(remaining as u64);
        if let Err(e) = take.read_to_end(&mut rest) {
            return ViewerLoad {
                error: Some(ViewerError::Io(e.to_string())),
                ..Default::default()
            };
        }
        buf.extend(rest);
    }
    if let Ok(md) = fs::metadata(path) {
        if md.len() as usize > buf.len() {
            truncated = true;
        }
    }

    let content = String::from_utf8_lossy(&buf).into_owned();
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let lines = highlighter.highlight(extension, &content);

    ViewerLoad {
        lines,
        truncated,
        error: None,
    }
}

fn io_error(e: &std::io::Error) -> ViewerError {
    match e.kind() {
        ErrorKind::NotFound => ViewerError::NotFound,
        ErrorKind::PermissionDenied => ViewerError::PermissionDenied,
        _ => ViewerError::Io(e.to_string()),
    }
}

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
