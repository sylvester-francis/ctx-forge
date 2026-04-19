//! Mode enum — the input mode stack for the v2 TUI.

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Mode {
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
    AtPicker {
        query: String,
        cursor: usize,
        files: Vec<std::path::PathBuf>,
    },
    TextPrompt {
        purpose: TextPromptPurpose,
        input: String,
    },
    PickerList {
        purpose: PickerPurpose,
        cursor: usize,
        items: Vec<String>,
    },
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
    AddGh,
    BundleRm,
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
            Self::AddGh => "GitHub attach — gh:///owner/repo/…",
            Self::BundleRm => "remove item — path or 1-based index",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerPurpose {
    LoadProfile,
    Model,
    ApplyTemplate,
    RemoveTemplate,
    Suggest,
}

impl PickerPurpose {
    pub fn label(&self) -> &'static str {
        match self {
            Self::LoadProfile => "load profile",
            Self::Model => "select model",
            Self::ApplyTemplate => "apply template",
            Self::RemoveTemplate => "remove template",
            Self::Suggest => "apply suggestions",
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
    Export(String),
    Pipe { target: String, content: String },
    Editor(String),
    TextPrompt(TextPromptPurpose),
    PickerList {
        purpose: PickerPurpose,
        items: Vec<String>,
    },
}
