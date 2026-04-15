//! Live prompt preview — the artifact the user is crafting.
//!
//! `PromptPreview::from_app(&app)` snapshots the current scenario + bundle +
//! task text into a structural view. `to_text()` returns a flat plain-text
//! rendering suitable for display in the right column (and for testing).
//!
//! Sections:
//!   - TemplatePrefix   — template text before the first placeholder
//!   - Task             — the user's task text (synced with the prompt input)
//!   - Context          — bundle items with token counts
//!   - TemplateSuffix   — template text after the last placeholder
//!
//! Error / empty states:
//!   - No scenario  → `NoScenarioPlaceholder` single line
//!   - Template read fails → `TemplateError` block with the reason

pub mod full;
pub mod render;

use crate::tui::app::App;

pub struct PromptPreview {
    pub sections: Vec<Section>,
}

pub enum Section {
    TemplatePrefix { scenario: String, body: String },
    Task { text: String },
    Context { items: Vec<ContextItem> },
    TemplateSuffix { scenario: String, body: String },
    NoScenarioPlaceholder,
    TemplateError { scenario: String, error: String },
}

pub struct ContextItem {
    pub path: std::path::PathBuf,
    pub tokens: usize,
}

impl PromptPreview {
    pub fn from_app(app: &App) -> Self {
        let mut sections = Vec::new();
        match &app.bundle.scenario {
            None => sections.push(Section::NoScenarioPlaceholder),
            Some(name) => match render::load_wrapped(&app.root, name) {
                Ok(wrapped) => {
                    sections.push(Section::TemplatePrefix {
                        scenario: name.clone(),
                        body: wrapped.prefix,
                    });
                    sections.push(Section::Task {
                        text: app.bundle.task_text.clone(),
                    });
                    let items = app
                        .bundle
                        .items
                        .iter()
                        .zip(app.item_tokens.iter())
                        .map(|(it, tok)| ContextItem {
                            path: it.path.clone(),
                            tokens: *tok,
                        })
                        .collect();
                    sections.push(Section::Context { items });
                    sections.push(Section::TemplateSuffix {
                        scenario: name.clone(),
                        body: wrapped.suffix,
                    });
                }
                Err(e) => sections.push(Section::TemplateError {
                    scenario: name.clone(),
                    error: e,
                }),
            },
        }
        Self { sections }
    }

    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for s in &self.sections {
            match s {
                Section::TemplatePrefix { scenario, body } => {
                    out.push_str(&format!("╭─ template: {scenario} · prefix ─╮\n"));
                    for line in body.lines() {
                        out.push_str(&format!("│ {line}\n"));
                    }
                    out.push_str("╰───────────────────────────────────╯\n\n");
                }
                Section::Task { text } => {
                    out.push_str("## Task\n");
                    if text.is_empty() {
                        out.push_str("(empty — type in the prompt input below)\n");
                    } else {
                        out.push_str(text);
                        if !text.ends_with('\n') {
                            out.push('\n');
                        }
                    }
                    out.push('\n');
                }
                Section::Context { items } => {
                    let total: usize = items.iter().map(|i| i.tokens).sum();
                    out.push_str(&format!(
                        "## Context ({} file{} · {}k tokens)\n",
                        items.len(),
                        if items.len() == 1 { "" } else { "s" },
                        total / 1000,
                    ));
                    for (i, item) in items.iter().enumerate() {
                        out.push_str(&format!(
                            " {:>2} {} {:>8}\n",
                            i + 1,
                            item.path.display(),
                            item.tokens,
                        ));
                    }
                    out.push('\n');
                }
                Section::TemplateSuffix { scenario, body } => {
                    out.push_str(&format!("╭─ template: {scenario} · suffix ─╮\n"));
                    for line in body.lines() {
                        out.push_str(&format!("│ {line}\n"));
                    }
                    out.push_str("╰───────────────────────────────────╯\n");
                }
                Section::NoScenarioPlaceholder => {
                    out.push_str("no scenario · /scenario to pick one\n");
                }
                Section::TemplateError { scenario, error } => {
                    out.push_str(&format!("⚠ scenario {scenario}: {error}\n"));
                }
            }
        }
        out
    }
}
