//! Live prompt preview — the artifact the user is crafting.
//!
//! Data structures for the structured preview sections. Rendering is
//! handled by the TUI components (v2: `src/tui2/components/prompt_preview.rs`).

pub mod render;

pub struct PromptPreview {
    pub sections: Vec<Section>,
}

pub enum Section {
    ScenarioHeader {
        scenario: String,
        prefix_lines: usize,
        suffix_lines: usize,
    },
    Task {
        text: String,
    },
    Context {
        items: Vec<ContextItem>,
    },
    NoScenarioPlaceholder,
    TemplateError {
        scenario: String,
        error: String,
    },
}

pub struct ContextItem {
    pub path: std::path::PathBuf,
    pub tokens: usize,
}

fn format_tokens(n: usize) -> String {
    if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        format!("{n}")
    }
}

impl PromptPreview {
    /// Plain-text rendering (used by tests and fallback paths).
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for s in &self.sections {
            match s {
                Section::ScenarioHeader {
                    scenario,
                    prefix_lines,
                    suffix_lines,
                } => {
                    out.push_str(&format!("  scenario: {scenario}\n"));
                    out.push_str(&format!(
                        "  template: {} line{} prefix · {} line{} suffix\n",
                        prefix_lines,
                        if *prefix_lines == 1 { "" } else { "s" },
                        suffix_lines,
                        if *suffix_lines == 1 { "" } else { "s" },
                    ));
                    out.push_str("  (press P for full composed prompt)\n\n");
                }
                Section::Task { text } => {
                    out.push_str("task\n");
                    if text.is_empty() {
                        out.push_str("  (empty — press i to focus the prompt input)\n\n");
                    } else {
                        for line in text.lines() {
                            out.push_str(&format!("  {line}\n"));
                        }
                        out.push('\n');
                    }
                }
                Section::Context { items } => {
                    let total: usize = items.iter().map(|i| i.tokens).sum();
                    out.push_str(&format!(
                        "context  ({} file{} · {} tokens)\n",
                        items.len(),
                        if items.len() == 1 { "" } else { "s" },
                        format_tokens(total),
                    ));
                    if items.is_empty() {
                        out.push_str("  (empty — space in tree, or @ in prompt)\n");
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
                Section::NoScenarioPlaceholder => {
                    out.push_str("  no scenario selected\n\n");
                    out.push_str("  press / and type 'scenario' to pick one\n");
                    out.push_str(
                        "  built-in: bugfix · code-review · explain · refactor · migrate\n",
                    );
                }
                Section::TemplateError { scenario, error } => {
                    out.push_str(&format!("  ⚠  scenario '{scenario}' failed to load\n"));
                    out.push_str(&format!("     {error}\n"));
                }
            }
        }
        out
    }
}
