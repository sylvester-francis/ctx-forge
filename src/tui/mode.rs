//! Mode enum — the input mode stack for the v2 TUI.

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Mode {
    /// Full-screen welcome splash. Any keypress transitions to Normal.
    #[default]
    Welcome,
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
    ThemePicker {
        cursor: usize,
    },
    DeliveryPicker {
        cursor: usize,
    },
    FullPromptPreview {
        content: String,
        scroll: usize,
    },
    /// `@` file picker inside the prompt input.
    AtPicker {
        query: String,
        cursor: usize,
        /// Cached file list from walk_files, populated on open.
        files: Vec<std::path::PathBuf>,
    },
}

impl Mode {
    pub fn is_overlay(&self) -> bool {
        matches!(
            self,
            Mode::Help
                | Mode::ScenarioPicker { .. }
                | Mode::CommandPalette { .. }
                | Mode::ThemePicker { .. }
                | Mode::DeliveryPicker { .. }
                | Mode::FullPromptPreview { .. }
                | Mode::AtPicker { .. }
        )
    }
}

/// An action that requires leaving the iocraft render loop — the outer
/// `run()` function handles it between render-loop iterations.
#[derive(Debug, Clone)]
pub enum PendingAction {
    /// Print content to stdout, wait for keypress, re-enter TUI.
    Export(String),
    /// Spawn a target binary and pipe content to its stdin.
    Pipe { target: String, content: String },
    /// Spawn $EDITOR with the given starting content. On save, the edited
    /// text replaces the prompt override for the next delivery.
    Editor(String),
}
