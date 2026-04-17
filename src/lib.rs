//! Library entry point. Exposes internal modules for integration tests and
//! external consumers. The `ctxforge` binary (see `src/main.rs`) wires these
//! into a CLI.

pub mod bundle;
pub mod cli;
pub mod clipboard;
pub mod commands;
pub mod deliver;
pub mod editor;
pub mod error;
#[cfg(feature = "extract")]
pub mod extract;
pub mod format;
pub mod git;
pub mod lang;
#[cfg(feature = "mcp")]
pub mod mcp;
pub mod memory;
pub mod models;
pub mod motion_core;
pub mod output;
pub mod paths;
pub mod preview;
pub mod profile;
pub mod prompt_input;
pub mod resolve;
pub mod scenario;
pub mod source;
pub mod template;
pub mod test_helpers;
pub mod theme;
pub mod tokens;
pub mod tree;
#[cfg(feature = "tui-v2")]
pub mod tui;
pub mod walk;
