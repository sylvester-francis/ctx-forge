//! Mode enum — the input mode stack for the v2 TUI.
//!
//! Overlay variants carry their own state (e.g. picker cursor) so the App
//! component stays thin: the `mode` signal owns the overlay's transient
//! state.

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Normal,
    Search {
        query: String,
    },
    Help,
    ScenarioPicker {
        cursor: usize,
    },
}

impl Mode {
    pub fn is_overlay(&self) -> bool {
        matches!(self, Mode::Help | Mode::ScenarioPicker { .. })
    }
}
