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
    /// Single-line text-entry prompt. Populated by command-palette
    /// actions that need a value (SaveProfile name, Note body, etc.).
    TextPrompt {
        purpose: TextPromptPurpose,
        input: String,
    },
    /// Selectable list overlay — user picks from a pre-computed set of
    /// strings. Used for LoadProfile, Model picker, Template pickers.
    PickerList {
        purpose: PickerPurpose,
        cursor: usize,
        items: Vec<String>,
    },
    /// Scrolling output overlay — shows computed text the user wanted
    /// to see (memory recall, template list, starters). No input.
    TextOutput {
        title: String,
        body: String,
        scroll: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextPromptPurpose {
    SaveProfile,
    Narrow,
    Note,
    FindFn,
    FindType,
    FindDiff,
    TemplateNew,
    DocsAdd,
    DocsRm,
    AddUrl,
}

impl TextPromptPurpose {
    pub fn label(&self) -> &'static str {
        match self {
            Self::SaveProfile => "profile name",
            Self::Narrow => "narrow (path:start-end)",
            Self::Note => "memory note (body)",
            Self::FindFn => "function name",
            Self::FindType => "type name",
            Self::FindDiff => "git branch",
            Self::TemplateNew => "template name",
            Self::DocsAdd => "docs add — dep name",
            Self::DocsRm => "docs rm — dep name",
            Self::AddUrl => "URL (https://…)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerPurpose {
    LoadProfile,
    Model,
    ApplyTemplate,
    RemoveTemplate,
}

impl PickerPurpose {
    pub fn label(&self) -> &'static str {
        match self {
            Self::LoadProfile => "load profile",
            Self::Model => "select model",
            Self::ApplyTemplate => "apply template",
            Self::RemoveTemplate => "remove template",
        }
    }
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
                | Mode::TextPrompt { .. }
                | Mode::PickerList { .. }
                | Mode::TextOutput { .. }
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
    /// Suspend TUI, prompt for a single-line value via dialoguer,
    /// execute the matching CLI action, then resume.
    TextPrompt(TextPromptPurpose),
    /// Suspend TUI, show a dialoguer::Select populated with `items`,
    /// execute the matching CLI action for the selection, then resume.
    PickerList {
        purpose: PickerPurpose,
        items: Vec<String>,
    },
}
