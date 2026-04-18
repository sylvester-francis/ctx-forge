//! Manifest + lock file parsers, one per ecosystem.
//!
//! Each parser returns Vec<DetectedDep>: direct deps only, with the most
//! specific version available (lock file > manifest spec).

pub mod cargo;
pub mod npm;
pub mod python;

use crate::source::Ecosystem;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedDep {
    pub name: String,
    pub version: String,
    pub ecosystem: Ecosystem,
    /// Absolute or project-relative path of the manifest this dep was read from.
    pub manifest_path: PathBuf,
}
