//! Mode enum — the input mode stack for the v2 TUI.
//!
//! Phase 2a only needs `Normal` and `Search`. Phase 2c adds overlay modes
//! (help, scenario picker, etc.).

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Normal,
    Search {
        query: String,
    },
}
