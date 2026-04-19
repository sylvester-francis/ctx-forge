//! Manifest + lock file parsers, one per ecosystem. Each returns direct
//! deps with the most specific version available (lock > manifest spec).

pub mod cargo;
pub mod go;
pub mod npm;
pub mod python;

use crate::source::Ecosystem;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedDep {
    pub name: String,
    pub version: String,
    pub ecosystem: Ecosystem,
    pub manifest_path: PathBuf,
}
