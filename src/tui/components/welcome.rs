//! Welcome/landing screen — shown in the preview panel when no scenario
//! is set and the bundle is empty. First thing users see on launch.
//!
//! Design: distinctive ASCII wordmark + quick-start guide with the
//! terminal-ui-design skill's aesthetic.

use crate::tui::theme::Theme;
use iocraft::prelude::*;

pub fn render_welcome(theme: &Theme, file_count: usize) -> Vec<AnyElement<'static>> {
    let mut body: Vec<AnyElement<'static>> = Vec::new();

    // ─── Wordmark ─────────────────────────────────────────────
    let wordmark = [
        "          ╔═╗ ╔╦╗ ═╗ ╔═ ╔═╗ ╦═╗ ╔═╗ ╔═╗",
        "          ║   ║║║  ╠╣  ╠╣  ║ ║ ╠╦╝ ║ ╦ ╠╣ ",
        "          ╚═╝ ╩ ╩ ╩ ╚ ╚   ╚═╝ ╩╚═ ╚═╝ ╚═╝",
    ];

    body.push(element! { Text(content: "") }.into_any());
    for line in &wordmark {
        body.push(
            element! { Text(content: *line, color: theme.accent, weight: Weight::Bold) }
                .into_any(),
        );
    }
    body.push(element! { Text(content: "") }.into_any());
    body.push(
        element! {
            Text(
                content: "          the prompt engineer for AI coding",
                color: theme.muted,
                weight: Weight::Light,
            )
        }
        .into_any(),
    );
    body.push(element! { Text(content: "") }.into_any());
    body.push(
        element! {
            Text(
                content: "─────────────────────────────────────────────",
                color: theme.muted,
                weight: Weight::Light,
            )
        }
        .into_any(),
    );
    body.push(element! { Text(content: "") }.into_any());

    // ─── Quick start ──────────────────────────────────────────
    body.push(
        element! {
            MixedText(contents: vec![
                MixedTextContent::new("  QUICK START").color(theme.accent).weight(Weight::Bold),
            ])
        }
        .into_any(),
    );
    body.push(element! { Text(content: "") }.into_any());

    let steps = [
        ("S", "pick a scenario", "bugfix · code-review · explain · refactor · migrate"),
        ("space", "add files to bundle", &format!("{file_count} files in project")),
        ("i", "write your task", "describe what you need"),
        ("P", "preview composed prompt", "see exactly what gets sent"),
        ("d", "deliver", "copy · pipe · export"),
    ];

    for (i, (key, action, detail)) in steps.iter().enumerate() {
        let step_num = format!("  {}  ", i + 1);
        body.push(
            element! {
                MixedText(contents: vec![
                    MixedTextContent::new(step_num).color(theme.muted).weight(Weight::Bold),
                    MixedTextContent::new(*key).color(theme.accent).weight(Weight::Bold),
                    MixedTextContent::new(format!("  {action}")).weight(Weight::Bold),
                ])
            }
            .into_any(),
        );
        body.push(
            element! {
                MixedText(contents: vec![
                    MixedTextContent::new("       ").color(theme.muted),
                    MixedTextContent::new(*detail).color(theme.muted).weight(Weight::Light),
                ])
            }
            .into_any(),
        );
        body.push(element! { Text(content: "") }.into_any());
    }

    body.push(
        element! {
            Text(
                content: "─────────────────────────────────────────────",
                color: theme.muted,
                weight: Weight::Light,
            )
        }
        .into_any(),
    );
    body.push(element! { Text(content: "") }.into_any());
    body.push(
        element! {
            MixedText(contents: vec![
                MixedTextContent::new("  /").color(theme.accent).weight(Weight::Bold),
                MixedTextContent::new("  open command palette for all commands").color(theme.muted),
            ])
        }
        .into_any(),
    );
    body.push(
        element! {
            MixedText(contents: vec![
                MixedTextContent::new("  ?").color(theme.accent).weight(Weight::Bold),
                MixedTextContent::new("  show keybinding help").color(theme.muted),
            ])
        }
        .into_any(),
    );

    body
}
