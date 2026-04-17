//! `RangeSource` — a 1-indexed inclusive line range from a file.

use crate::error::{CtxforgeError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RangeSource {
    pub path: PathBuf,
    pub start: usize,
    pub end: usize,
}

impl RangeSource {
    /// Validating constructor. Rejects zero bounds and reversed ranges so
    /// `start - 1` in the resolver can never underflow.
    pub fn new(path: PathBuf, start: usize, end: usize) -> Result<Self> {
        if start == 0 || end == 0 {
            return Err(CtxforgeError::InvalidRange {
                input: format!("{start}-{end}"),
                reason: "lines are 1-indexed; 0 is invalid".into(),
            });
        }
        if start > end {
            return Err(CtxforgeError::InvalidRange {
                input: format!("{start}-{end}"),
                reason: format!("start {start} > end {end}"),
            });
        }
        Ok(Self { path, start, end })
    }
}
