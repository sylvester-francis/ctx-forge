//! Fully-ported prompt preview for v2.
//!
//! Consumes the same `PromptPreview` data model as v1 (sections: scenario
//! header, task, context) and renders with iocraft `MixedText` for inline
//! styled spans. Section headers get the theme accent; metadata lines are
//! muted; template errors use the danger color.
//!
//! This function returns a `Vec<AnyElement>` rather than being a component
//! so the parent (`App`) can place it inside whatever containing view it
//! wants (framed, padded, scrollable, etc.) without nesting constraints.

use crate::tui::preview::{PromptPreview, Section};
use crate::tui2::theme::Theme;
use iocraft::prelude::*;

fn format_tokens(n: usize) -> String {
    if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        format!("{n}")
    }
}

pub fn render_preview(preview: &PromptPreview, theme: &Theme) -> Vec<AnyElement<'static>> {
    let mut lines: Vec<AnyElement<'static>> = Vec::new();

    for section in &preview.sections {
        match section {
            Section::ScenarioHeader {
                scenario,
                prefix_lines,
                suffix_lines,
            } => {
                let scenario = scenario.clone();
                let tmpl = format!(
                    "{} line{} prefix · {} line{} suffix",
                    prefix_lines,
                    if *prefix_lines == 1 { "" } else { "s" },
                    suffix_lines,
                    if *suffix_lines == 1 { "" } else { "s" },
                );
                lines.push(
                    element! {
                        MixedText(contents: vec![
                            MixedTextContent::new("  scenario  ").color(theme.muted),
                            MixedTextContent::new(scenario).color(theme.accent).weight(Weight::Bold),
                        ])
                    }
                    .into_any(),
                );
                lines.push(
                    element! {
                        MixedText(contents: vec![
                            MixedTextContent::new("  template  ").color(theme.muted),
                            MixedTextContent::new(tmpl),
                        ])
                    }
                    .into_any(),
                );
                lines.push(
                    element! {
                        Text(content: "  (P for full composed prompt)", weight: Weight::Light)
                    }
                    .into_any(),
                );
                lines.push(element! { Text(content: "") }.into_any());
            }
            Section::Task { text } => {
                lines.push(
                    element! {
                        Text(content: "task", color: theme.accent, weight: Weight::Bold)
                    }
                    .into_any(),
                );
                if text.is_empty() {
                    lines.push(
                        element! {
                            Text(
                                content: "  (empty — press i to focus the prompt input)",
                                weight: Weight::Light,
                            )
                        }
                        .into_any(),
                    );
                } else {
                    for l in text.lines() {
                        let line = format!("  {l}");
                        lines.push(
                            element! {
                                Text(content: line.leak() as &str)
                            }
                            .into_any(),
                        );
                    }
                }
                lines.push(element! { Text(content: "") }.into_any());
            }
            Section::Context { items } => {
                let total: usize = items.iter().map(|i| i.tokens).sum();
                let header = format!(
                    "  {} file{} · {} tokens",
                    items.len(),
                    if items.len() == 1 { "" } else { "s" },
                    format_tokens(total),
                );
                lines.push(
                    element! {
                        MixedText(contents: vec![
                            MixedTextContent::new("context").color(theme.accent).weight(Weight::Bold),
                            MixedTextContent::new(header).color(theme.muted),
                        ])
                    }
                    .into_any(),
                );
                if items.is_empty() {
                    lines.push(
                        element! {
                            Text(
                                content: "  (empty — space in tree, or @ in prompt)",
                                weight: Weight::Light,
                            )
                        }
                        .into_any(),
                    );
                } else {
                    for (i, item) in items.iter().enumerate() {
                        let idx = format!("  {:>2}  ", i + 1);
                        let path = format!("{:<40}", item.path.display());
                        let toks = format!("  {}", format_tokens(item.tokens));
                        lines.push(
                            element! {
                                MixedText(contents: vec![
                                    MixedTextContent::new(idx).color(theme.muted),
                                    MixedTextContent::new(path),
                                    MixedTextContent::new(toks).color(theme.muted),
                                ])
                            }
                            .into_any(),
                        );
                    }
                }
                lines.push(element! { Text(content: "") }.into_any());
            }
            Section::NoScenarioPlaceholder => {
                lines.push(
                    element! {
                        Text(content: "  no scenario selected", weight: Weight::Light)
                    }
                    .into_any(),
                );
                lines.push(element! { Text(content: "") }.into_any());
                lines.push(
                    element! {
                        Text(
                            content: "  press / and type 'scenario' to pick one",
                            weight: Weight::Light,
                        )
                    }
                    .into_any(),
                );
                lines.push(
                    element! {
                        Text(content: "  built-in: bugfix · code-review · explain · refactor · migrate")
                    }
                    .into_any(),
                );
            }
            Section::TemplateError { scenario, error } => {
                let line1 = format!("  ⚠  scenario '{}' failed to load", scenario);
                let line2 = format!("     {}", error);
                lines.push(
                    element! {
                        Text(content: line1.leak() as &str, color: theme.danger)
                    }
                    .into_any(),
                );
                lines.push(
                    element! {
                        Text(content: line2.leak() as &str)
                    }
                    .into_any(),
                );
            }
        }
    }

    lines
}
