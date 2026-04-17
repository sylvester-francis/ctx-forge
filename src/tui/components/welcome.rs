//! Welcome/landing screen — always shown in the preview panel.
//! ASCII wordmark + quick-start guide.

use crate::tui::theme::Theme;
use iocraft::prelude::*;

pub fn render_welcome(theme: &Theme, file_count: usize) -> Vec<AnyElement<'static>> {
    let mut body: Vec<AnyElement<'static>> = Vec::new();

    // ─── Wordmark ─────────────────────────────────────────────
    body.push(element! { Text(content: "") }.into_any());
    body.push(
        element! {
            MixedText(contents: vec![
                MixedTextContent::new("  ────── ").color(theme.muted).weight(Weight::Light),
                MixedTextContent::new("⚒ ").color(theme.accent).weight(Weight::Bold),
                MixedTextContent::new("c t x f o r g e").color(theme.accent).weight(Weight::Bold),
                MixedTextContent::new(" ──────").color(theme.muted).weight(Weight::Light),
            ])
        }
        .into_any(),
    );
    body.push(element! { Text(content: "") }.into_any());
    body.push(
        element! {
            Text(
                content: "  the prompt engineer for AI coding",
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
                MixedTextContent::new("  ▍ ").color(theme.accent).weight(Weight::Bold),
                MixedTextContent::new("QUICK START").color(theme.accent).weight(Weight::Bold),
            ])
        }
        .into_any(),
    );
    body.push(element! { Text(content: "") }.into_any());

    let steps: &[(&str, &str, String)] = &[
        ("S", "pick a scenario", "bugfix · code-review · explain · refactor · migrate".to_string()),
        ("space", "add files to bundle", format!("{file_count} files in project")),
        ("i", "write your task", "describe what you need".to_string()),
        ("P", "preview composed prompt", "see exactly what gets sent".to_string()),
        ("d", "deliver", "copy · pipe · export".to_string()),
    ];

    for (i, (key, action, detail)) in steps.iter().enumerate() {
        body.push(
            element! {
                MixedText(contents: vec![
                    MixedTextContent::new(format!("  {}  ", i + 1)).color(theme.muted),
                    MixedTextContent::new(format!("{key:<6}")).color(theme.accent).weight(Weight::Bold),
                    MixedTextContent::new(*action),
                ])
            }
            .into_any(),
        );
        body.push(
            element! {
                Text(
                    content: format!("         {detail}").leak() as &str,
                    color: theme.muted,
                    weight: Weight::Light,
                )
            }
            .into_any(),
        );
    }

    body.push(element! { Text(content: "") }.into_any());

    // ─── Extra hints ──────────────────────────────────────────
    body.push(
        element! {
            MixedText(contents: vec![
                MixedTextContent::new("  /").color(theme.accent).weight(Weight::Bold),
                MixedTextContent::new("  command palette").color(theme.muted),
                MixedTextContent::new("     ").color(theme.muted),
                MixedTextContent::new("?").color(theme.accent).weight(Weight::Bold),
                MixedTextContent::new("  keybinding help").color(theme.muted),
            ])
        }
        .into_any(),
    );
    body.push(element! { Text(content: "") }.into_any());

    body
}
