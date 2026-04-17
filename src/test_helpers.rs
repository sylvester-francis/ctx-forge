//! Shared helpers used by integration tests.
//!
//! Publicly exposed because the `ctxforge` crate is a library — integration
//! tests see it as an external dependency and cannot reach `#[cfg(test)]`
//! items. Each function here is only useful for tests; the cost of making
//! them `pub` is negligible.

#![allow(dead_code)]

use crate::paths::CtxforgeRoot;

/// Create a CtxforgeRoot pointing at a temporary directory. Useful for
/// integration tests that need a project-like environment.
pub fn temp_root(dir: &std::path::Path) -> CtxforgeRoot {
    CtxforgeRoot::find_or_create(dir).expect("temp_root")
}
