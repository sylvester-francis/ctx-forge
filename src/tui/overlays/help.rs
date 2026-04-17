//! Help modal — grouped keybinding list.

use crate::tui::theme::Theme;
use iocraft::prelude::*;

fn section_header(label: &'static str, theme: &Theme) -> AnyElement<'static> {
    element! {
        MixedText(contents: vec![
            MixedTextContent::new(label).color(theme.accent).weight(Weight::Bold),
        ])
    }
    .into_any()
}

fn binding(key: &'static str, desc: &'static str, theme: &Theme) -> AnyElement<'static> {
    element! {
        MixedText(contents: vec![
            MixedTextContent::new(format!("  {:<12} ", key))
                .color(theme.accent)
                .weight(Weight::Bold),
            MixedTextContent::new(desc).color(theme.fg),
        ])
    }
    .into_any()
}

fn blank() -> AnyElement<'static> {
    element! { Text(content: " ") }.into_any()
}

pub fn render_body(theme: &Theme) -> Vec<AnyElement<'static>> {
    vec![
        section_header("NAVIGATION", theme),
        binding("tab / ⇧tab", "cycle focus", theme),
        binding("j / k", "move cursor", theme),
        binding("g / G", "top / bottom", theme),
        binding("ctrl-u/d", "half page", theme),
        blank(),
        section_header("FILE TREE", theme),
        binding("space", "toggle bundle / expand dir", theme),
        binding("enter", "expand directory", theme),
        binding("E / C", "expand / collapse all", theme),
        binding("/", "fuzzy search", theme),
        blank(),
        section_header("PROMPT", theme),
        binding("i", "focus prompt input", theme),
        binding("esc", "leave prompt focus", theme),
        binding("ctrl-w", "delete word back", theme),
        blank(),
        section_header("OVERLAYS", theme),
        binding("?", "this help", theme),
        binding("S", "scenario picker", theme),
        blank(),
        section_header("APP", theme),
        binding("q", "quit", theme),
        blank(),
        element! {
            Text(content: "press esc or ? to close", color: theme.muted, weight: Weight::Light)
        }
        .into_any(),
    ]
}
