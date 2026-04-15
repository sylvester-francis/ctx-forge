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
                    out.push_str(&format!("── template: {scenario} (prefix) ──\n"));
                    out.push_str(&truncate_body(body, 4));
                    out.push('\n');
                }
                Section::Task { text } => {
                    out.push_str("── task ──\n");
                    if text.is_empty() {
                        out.push_str("(empty — press i to focus the prompt input)\n");
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
                        "── context ({} file{} · {} tokens) ──\n",
                        items.len(),
                        if items.len() == 1 { "" } else { "s" },
                        format_tokens(total),
                    ));
                    if items.is_empty() {
                        out.push_str("(empty — add files from the tree or via @ in the prompt)\n");
                    } else {
                        for (i, item) in items.iter().enumerate() {
                            out.push_str(&format!(
                                "  {:>2}  {:<40}  {}\n",
                                i + 1,
                                item.path.display(),
                                format_tokens(item.tokens),
                            ));
                        }
                    }
                    out.push('\n');
                }
                Section::TemplateSuffix { scenario, body } => {
                    out.push_str(&format!("── template: {scenario} (suffix) ──\n"));
                    out.push_str(&truncate_body(body, 4));
                }
                Section::NoScenarioPlaceholder => {
                    out.push_str("no scenario selected\n\n");
                    out.push_str("press / and type 'scenario' to pick one\n");
                    out.push_str("built-in: bugfix · code-review · explain · refactor · migrate\n");
                }
                Section::TemplateError { scenario, error } => {
                    out.push_str(&format!("⚠  scenario '{scenario}' failed to load\n"));
                    out.push_str(&format!("   {error}\n"));
                }
            }
        }
        out
    }
}

/// Limit a multi-line template body to the first `max_lines` meaningful
/// lines so the preview doesn't get dominated by the template wrapper.
/// Appends an ellipsis line when truncated.
fn truncate_body(body: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = body.lines().filter(|l| !l.trim().is_empty()).collect();
    let shown = lines.iter().take(max_lines);
    let mut out = String::new();
    for line in shown {
        out.push_str(line);
        out.push('\n');
    }
    if lines.len() > max_lines {
        out.push_str(&format!("… ({} more lines)\n", lines.len() - max_lines));
    }
    out
}

/// Format a token count as `1.2k` / `456` — keeps column widths stable.
fn format_tokens(n: usize) -> String {
    if n >= 10_000 {
        format!("{}k", n / 1000)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1000.0)
    } else {
        n.to_string()
    }
}
