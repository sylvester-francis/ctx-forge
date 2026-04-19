//! Prompt preview — section markers, aligned rows, right-aligned tokens.

use crate::preview::{PromptPreview, Section};
use crate::tui::theme::Theme;
use iocraft::prelude::*;

fn format_tokens(n: usize) -> String {
    if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        format!("{n}")
    }
}

fn span(text: String, color: Option<Color>, bold: bool) -> MixedTextContent {
    let mut c = MixedTextContent::new(text);
    if let Some(col) = color {
        c = c.color(col);
    }
    if bold {
        c = c.weight(Weight::Bold);
    }
    c
}

fn span_light(text: String, color: Color) -> MixedTextContent {
    MixedTextContent::new(text)
        .color(color)
        .weight(Weight::Light)
}

fn divider(theme: &Theme) -> AnyElement<'static> {
    element! {
        View(width: 100pct) {
            Text(
                content: "───────────────────────────────",
                color: theme.muted,
                weight: Weight::Light,
            )
        }
    }
    .into_any()
}

fn blank() -> AnyElement<'static> {
    element! { Text(content: " ") }.into_any()
}

pub fn render_preview(preview: &PromptPreview, theme: &Theme) -> Vec<AnyElement<'static>> {
    let mut out: Vec<AnyElement<'static>> = Vec::new();
    let section_count = preview.sections.len();

    for (idx, section) in preview.sections.iter().enumerate() {
        match section {
            Section::ScenarioHeader {
                scenario,
                prefix_lines,
                suffix_lines,
            } => {
                let scenario_name = scenario.clone();
                let tmpl = format!("{}↑ prefix · {}↓ suffix", prefix_lines, suffix_lines);

                out.push(
                    element! {
                        View(width: 100pct) {
                            MixedText(contents: vec![
                                span("▍ ".to_string(), Some(theme.accent), true),
                                span("SCENARIO  ".to_string(), Some(theme.accent), true),
                                span(scenario_name, None, true),
                            ])
                        }
                    }
                    .into_any(),
                );
                out.push(
                    element! {
                        View(width: 100pct) {
                            MixedText(contents: vec![
                                span("   template  ".to_string(), Some(theme.muted), false),
                                span(tmpl, Some(theme.muted), false),
                            ])
                        }
                    }
                    .into_any(),
                );
                out.push(
                    element! {
                        Text(
                            content: "   press P for full composed prompt",
                            color: theme.muted,
                            weight: Weight::Light,
                        )
                    }
                    .into_any(),
                );
            }
            Section::Task { text } => {
                out.push(
                    element! {
                        View(width: 100pct) {
                            MixedText(contents: vec![
                                span("▍ ".to_string(), Some(theme.accent), true),
                                span("TASK".to_string(), Some(theme.accent), true),
                            ])
                        }
                    }
                    .into_any(),
                );
                if text.is_empty() {
                    out.push(
                        element! {
                            Text(
                                content: "   empty — press i to focus prompt",
                                color: theme.muted,
                                weight: Weight::Light,
                            )
                        }
                        .into_any(),
                    );
                } else {
                    for l in text.lines() {
                        let line = format!("   {l}");
                        out.push(
                            element! {
                                Text(content: line.leak() as &str)
                            }
                            .into_any(),
                        );
                    }
                }
            }
            Section::Context { items } => {
                let total: usize = items.iter().map(|i| i.tokens).sum();
                let meta = format!(
                    "{} file{} · {}",
                    items.len(),
                    if items.len() == 1 { "" } else { "s" },
                    format_tokens(total),
                );
                out.push(
                    element! {
                        View(width: 100pct) {
                            MixedText(contents: vec![
                                span("▍ ".to_string(), Some(theme.accent), true),
                                span("CONTEXT  ".to_string(), Some(theme.accent), true),
                                span(meta, Some(theme.muted), false),
                            ])
                        }
                    }
                    .into_any(),
                );
                if items.is_empty() {
                    out.push(
                        element! {
                            Text(
                                content: "   empty — space in tree, or @ in prompt",
                                color: theme.muted,
                                weight: Weight::Light,
                            )
                        }
                        .into_any(),
                    );
                } else {
                    for (i, item) in items.iter().enumerate() {
                        let badge = format!(" {:02} ", i + 1);
                        let path_str = item.path.display().to_string();
                        let path = super::smart_truncate_path(&path_str, 24);
                        let path_padded = format!(" {:<24} ", path);
                        let toks = format!("{:>5}", format_tokens(item.tokens));

                        out.push(
                            element! {
                                View(width: 100pct) {
                                    MixedText(contents: vec![
                                        span(badge, Some(theme.accent), true),
                                        span(path_padded, None, false),
                                        span(toks, Some(theme.muted), false),
                                    ])
                                }
                            }
                            .into_any(),
                        );
                    }
                }
            }
            Section::NoScenarioPlaceholder => {
                out.push(
                    element! {
                        View(width: 100pct) {
                            MixedText(contents: vec![
                                span(" ▶ SCENARIO ".to_string(), Some(theme.muted), true),
                                span_light("(none)".to_string(), theme.muted),
                            ])
                        }
                    }
                    .into_any(),
                );
                out.push(blank());
                out.push(
                    element! {
                        Text(
                            content: "   / scenario  to pick one",
                            color: theme.muted,
                            weight: Weight::Light,
                        )
                    }
                    .into_any(),
                );
                out.push(
                    element! {
                        View(width: 100pct) {
                            MixedText(contents: vec![
                                span_light("   built-in: ".to_string(), theme.muted),
                                span("bugfix · code-review · explain · refactor · migrate".to_string(), Some(theme.muted), false),
                            ])
                        }
                    }
                    .into_any(),
                );
            }
            Section::TemplateError { scenario, error } => {
                let line1 = format!(" ⚠  scenario '{}' failed to load", scenario);
                let line2 = format!("    {}", error);
                out.push(
                    element! {
                        Text(content: line1.leak() as &str, color: theme.danger, weight: Weight::Bold)
                    }
                    .into_any(),
                );
                out.push(
                    element! {
                        Text(content: line2.leak() as &str, color: theme.muted)
                    }
                    .into_any(),
                );
            }
        }

        if idx + 1 < section_count {
            out.push(blank());
            out.push(divider(theme));
            out.push(blank());
        }
    }

    out
}
