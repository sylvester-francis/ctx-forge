//! Shared TUI modules — used by `tui2` (iocraft v2 TUI).
//!
//! These were originally part of v1 (ratatui). Now that v2 is the sole TUI,
//! these modules remain here as shared domain code: theme palettes, file
//! tree building, scenario loading, prompt input widget, delivery choices,
//! and editor spawning. They have NO ratatui dependency.

pub mod deliver;
pub mod editor;
pub mod preview;
pub mod prompt_input;
pub mod scenario;
pub mod theme;
pub mod tree;
