//! Crate-wide error type.
#![allow(dead_code)]

use thiserror::Error;

pub type Result<T> = std::result::Result<T, CtxforgeError>;

#[derive(Error, Debug)]
pub enum CtxforgeError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("invalid range `{input}`: {reason}")]
    InvalidRange { input: String, reason: String },

    #[error("item not found at index {0}")]
    ItemNotFound(usize),

    #[error("profile `{0}` not found")]
    ProfileNotFound(String),

    #[error("no .ctxforge directory and no current working directory fallback")]
    NoCtxforgeRoot,

    #[error("git error: {0}")]
    Git(#[from] git2::Error),

    #[error("glob error: {0}")]
    Glob(#[from] globset::Error),

    #[error("clipboard error: {0}")]
    Clipboard(String),

    #[error("{0}")]
    Msg(String),

    #[error("file not found: {}", path.display())]
    NotFound {
        path: std::path::PathBuf,
        suggestions: Vec<String>,
    },

    #[error("fetch error: {0}")]
    Fetch(String),

    #[error("cache error: {0}")]
    Cache(String),

    #[error("URI parse error: {0}")]
    UriParse(String),
}

impl From<crate::source::UriParseError> for CtxforgeError {
    fn from(e: crate::source::UriParseError) -> Self {
        CtxforgeError::UriParse(e.to_string())
    }
}

impl From<String> for CtxforgeError {
    fn from(s: String) -> Self {
        CtxforgeError::Msg(s)
    }
}

impl From<&str> for CtxforgeError {
    fn from(s: &str) -> Self {
        CtxforgeError::Msg(s.to_string())
    }
}
