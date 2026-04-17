//! File loading for the viewer: binary detection, 2 MB truncation, error variants.

use std::fmt;
use std::fs;
use std::io::{ErrorKind, Read};
use std::path::Path;

use super::Highlighter;
use iocraft::components::MixedTextContent;

const MAX_READ_BYTES: usize = 2 * 1024 * 1024;
const BINARY_PEEK_BYTES: usize = 4096;

#[derive(Debug, Clone)]
pub enum ViewerError {
    Binary(u64),
    Directory,
    NotFound,
    PermissionDenied,
    Io(String),
}

impl fmt::Display for ViewerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Binary(size) => write!(f, "binary file — {} bytes", size),
            Self::Directory => write!(f, "directory — select a file"),
            Self::NotFound => write!(f, "file not found"),
            Self::PermissionDenied => write!(f, "permission denied"),
            Self::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

#[derive(Default)]
pub struct ViewerLoad {
    pub lines: Vec<Vec<MixedTextContent>>,
    pub truncated: bool,
    pub error: Option<ViewerError>,
}

fn io_error(e: &std::io::Error) -> ViewerError {
    match e.kind() {
        ErrorKind::NotFound => ViewerError::NotFound,
        ErrorKind::PermissionDenied => ViewerError::PermissionDenied,
        _ => ViewerError::Io(e.to_string()),
    }
}

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

    let mut buf = peek[..peek_n].to_vec();
    let remaining = MAX_READ_BYTES.saturating_sub(peek_n);
    let mut truncated = false;
    if remaining > 0 {
        let mut rest = Vec::with_capacity(remaining);
        if let Err(e) = file.by_ref().take(remaining as u64).read_to_end(&mut rest) {
            return ViewerLoad {
                error: Some(ViewerError::Io(e.to_string())),
                ..Default::default()
            };
        }
        buf.extend(rest);
    }

    let mut one_more = [0u8; 1];
    if let Ok(n) = file.read(&mut one_more) {
        if n > 0 {
            truncated = true;
        }
    }

    let content = String::from_utf8_lossy(&buf).into_owned();
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let lines = highlighter.highlight(ext, &content);

    ViewerLoad {
        lines,
        truncated,
        error: None,
    }
}
