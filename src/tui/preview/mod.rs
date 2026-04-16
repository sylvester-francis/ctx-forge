//! Live prompt preview — the artifact the user is crafting.
//!
//! Shows the crafted prompt's structure in three logical sections:
//! scenario + template summary at the top, the user's task text in the
//! middle, and the bundled context at the bottom. The actual template
//! body is NOT rendered here — it can be 100+ lines of boilerplate and
//! dominates the sidebar. Press `P` for the full composed text.

pub mod full;
pub mod render;

use crate::tui::app::App;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

pub struct PromptPreview {
    pub sections: Vec<Section>,
}

pub enum Section {
    /// Scenario is set. Carries the name + summary of the template
    /// (lines of prefix + suffix) so the user knows something's wrapping
    /// their prompt, without seeing the boilerplate.
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

impl PromptPreview {
    pub fn from_app(app: &App) -> Self {
        let mut sections = Vec::new();
        match &app.bundle.scenario {
            None => sections.push(Section::NoScenarioPlaceholder),
            Some(name) => match render::load_wrapped(&app.root, name) {
                Ok(wrapped) => {
                    sections.push(Section::ScenarioHeader {
                        scenario: name.clone(),
                        prefix_lines: wrapped
                            .prefix
                            .lines()
                            .filter(|l| !l.trim().is_empty())
                            .count(),
                        suffix_lines: wrapped
                            .suffix
                            .lines()
                            .filter(|l| !l.trim().is_empty())
                            .count(),
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
                }
                Err(e) => sections.push(Section::TemplateError {
                    scenario: name.clone(),
                    error: e,
                }),
            },
        }
        Self { sections }
    }

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

    /// Styled rendering used by the ratatui Paragraph. Section headers get
    /// the theme accent colour; metadata is DIM; body text uses fg.
    pub fn to_lines<'a>(&'a self, theme: &'a crate::tui::theme::AppTheme) -> Vec<Line<'a>> {
        let mut lines: Vec<Line<'a>> = Vec::new();
        let accent = Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD);
        let dim = Style::default().add_modifier(Modifier::DIM);
        let muted = Style::default().fg(theme.muted);

        for s in &self.sections {
            match s {
                Section::ScenarioHeader {
                    scenario,
                    prefix_lines,
                    suffix_lines,
                } => {
                    lines.push(Line::from(vec![
                        Span::styled("  scenario  ", muted),
                        Span::styled(scenario.clone(), accent),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled("  template  ", muted),
                        Span::raw(format!(
                            "{} line{} prefix · {} line{} suffix",
                            prefix_lines,
                            if *prefix_lines == 1 { "" } else { "s" },
                            suffix_lines,
                            if *suffix_lines == 1 { "" } else { "s" },
                        )),
                    ]));
                    lines.push(Line::styled("  (P for full composed prompt)", dim));
                    lines.push(Line::raw(""));
                }
                Section::Task { text } => {
                    lines.push(Line::styled("task", accent));
                    if text.is_empty() {
                        lines.push(Line::styled(
                            "  (empty — press i to focus the prompt input)",
                            dim,
                        ));
                    } else {
                        for l in text.lines() {
                            lines.push(Line::raw(format!("  {l}")));
                        }
                    }
                    lines.push(Line::raw(""));
                }
                Section::Context { items } => {
                    let total: usize = items.iter().map(|i| i.tokens).sum();
                    lines.push(Line::from(vec![
                        Span::styled("context", accent),
                        Span::styled(
                            format!(
                                "  {} file{} · {} tokens",
                                items.len(),
                                if items.len() == 1 { "" } else { "s" },
                                format_tokens(total),
                            ),
                            muted,
                        ),
                    ]));
                    if items.is_empty() {
                        lines.push(Line::styled(
                            "  (empty — space in tree, or @ in prompt)",
                            dim,
                        ));
                    } else {
                        for (i, item) in items.iter().enumerate() {
                            lines.push(Line::from(vec![
                                Span::styled(format!("  {:>2}  ", i + 1), muted),
                                Span::raw(format!("{:<40}", item.path.display())),
                                Span::styled(format!("  {}", format_tokens(item.tokens)), muted),
                            ]));
                        }
                    }
                    lines.push(Line::raw(""));
                }
                Section::NoScenarioPlaceholder => {
                    lines.push(Line::styled("  no scenario selected", accent));
                    lines.push(Line::raw(""));
                    lines.push(Line::raw("  press / and type 'scenario' to pick one"));
                    lines.push(Line::styled(
                        "  built-in: bugfix · code-review · explain · refactor · migrate",
                        dim,
                    ));
                }
                Section::TemplateError { scenario, error } => {
                    lines.push(Line::from(vec![
                        Span::styled("  ⚠  ", Style::default().fg(theme.danger)),
                        Span::raw(format!("scenario '{scenario}' failed to load")),
                    ]));
                    lines.push(Line::styled(format!("     {error}"), dim));
                }
            }
        }
        lines
    }
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
