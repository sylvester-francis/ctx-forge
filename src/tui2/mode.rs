//! Mode enum — the input mode stack for the v2 TUI.

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
    CommandPalette {
        query: String,
        cursor: usize,
    },
}

impl Mode {
    pub fn is_overlay(&self) -> bool {
        matches!(
            self,
            Mode::Help | Mode::ScenarioPicker { .. } | Mode::CommandPalette { .. }
        )
    }
}
