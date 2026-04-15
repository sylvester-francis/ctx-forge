//! TUI mode state machine.
//!
//! The `Mode` enum determines which keys are dispatched and what overlays
//! are rendered. `Mode::Normal` is the default two-panel browsing mode.
//! Other variants represent overlay/input modes added in later tasks.

/// Where a template was loaded from. Used by the TUI template picker
/// to show project-vs-global indicators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateSource {
    Project,
    Global,
}

/// Which input field is active in two-field overlays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputField {
    First,
    Second,
}

/// Application mode — determines what keys do and what renders.
#[derive(Debug, Clone, Default)]
pub enum Mode {
    /// Default two-panel browsing.
    #[default]
    Normal,
    /// Fuzzy search in the file tree.
    Search { query: String },
    /// Narrow a bundle item to a line range.
    Narrow {
        start: String,
        end: String,
        field: InputField,
    },
    /// Save current bundle as a named profile.
    SaveProfile { name: String },
    /// Pick a profile to load.
    LoadProfile {
        cursor: usize,
        profiles: Vec<String>,
    },
    /// Pipe-to-agent submenu.
    PipeMenu,
    /// Pick a model from the registry.
    ModelSwitch { cursor: usize },
    /// View memory notes (recall).
    MemoryPanel { cursor: usize, count: usize },
    /// Add a new note inline.
    AddNote {
        tag: String,
        body: String,
        field: InputField,
    },
    /// Pick a function to add (requires extract feature).
    #[cfg(feature = "extract")]
    FunctionPick {
        cursor: usize,
        items: Vec<(String, std::path::PathBuf)>,
    },
    /// Pick a type to add (requires extract feature).
    #[cfg(feature = "extract")]
    TypePick {
        cursor: usize,
        items: Vec<(String, std::path::PathBuf)>,
    },
    /// Pick changed files from a diff.
    DiffPick {
        branch: String,
        files: Vec<std::path::PathBuf>,
        selected: std::collections::HashSet<usize>,
        cursor: usize,
        entering_branch: bool,
    },
    /// Slash command palette open. Filtered live as the user types.
    CommandPalette { query: String, cursor: usize },
    /// Help overlay open. Toggled with `?` from Normal mode only.
    Help,
    /// Template picker overlay open. Loaded by `/template` command without args.
    TemplatePick {
        cursor: usize,
        templates: Vec<(String, TemplateSource)>,
    },
    /// Template task input open after a template is picked.
    TemplateTask { template_name: String, task: String },
    /// Scenario picker overlay.
    ScenarioPick {
        cursor: usize,
        scenarios: Vec<crate::tui::scenario::Scenario>,
    },
    /// Full-text preview of the composed prompt that would be delivered
    /// right now. Scrollable. Opened via `P` from Normal mode.
    FullPromptPreview { content: String, scroll: u16 },
    /// `@` file-mention popover. `all` is the cached project file list
    /// (walked once when the popover opens). `query` is whatever the
    /// user has typed after the `@`; `results` is the top-N fuzzy
    /// ranking refreshed on every key.
    AtPicker {
        all: Vec<std::path::PathBuf>,
        query: String,
        results: Vec<std::path::PathBuf>,
        cursor: usize,
    },
}

impl Mode {
    /// Whether the mode is Normal (for key dispatch).
    #[allow(dead_code)]
    pub fn is_normal(&self) -> bool {
        matches!(self, Mode::Normal)
    }

    /// Whether this mode renders an overlay on top of Normal content.
    /// Everything except `Normal` is an overlay.
    pub fn is_overlay(&self) -> bool {
        !matches!(self, Mode::Normal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_is_not_overlay() {
        assert!(!Mode::Normal.is_overlay());
    }

    #[test]
    fn help_is_overlay() {
        assert!(Mode::Help.is_overlay());
    }

    #[test]
    fn pipe_menu_is_overlay() {
        assert!(Mode::PipeMenu.is_overlay());
    }

    #[test]
    fn command_palette_is_overlay() {
        assert!(
            Mode::CommandPalette {
                query: String::new(),
                cursor: 0
            }
            .is_overlay()
        );
    }
}
