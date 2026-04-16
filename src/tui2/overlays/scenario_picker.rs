//! Scenario picker — list of available scenarios with arrow + Enter selection.

use crate::tui::scenario::{self, Scenario, Source};
use crate::tui2::theme::Theme;
use iocraft::prelude::*;

pub fn load(root: &crate::paths::CtxforgeRoot) -> Vec<Scenario> {
    scenario::available(root)
}

fn source_badge(source: Source) -> &'static str {
    match source {
        Source::BuiltIn => "built-in",
        Source::Project => "project",
        Source::Global => "global",
    }
}

pub fn render_body(
    scenarios: &[Scenario],
    cursor: usize,
    current: Option<&str>,
    theme: &Theme,
) -> Vec<AnyElement<'static>> {
    if scenarios.is_empty() {
        return vec![
            element! {
                Text(content: "  no scenarios found", color: theme.muted, weight: Weight::Light)
            }
            .into_any(),
        ];
    }

    let mut rows: Vec<AnyElement<'static>> = Vec::new();
    for (i, sc) in scenarios.iter().enumerate() {
        let selected = i == cursor;
        let is_current = current.map(|n| n == sc.name).unwrap_or(false);

        let gutter = if selected { "▶ " } else { "  " };
        let check = if is_current { "● " } else { "  " };
        let name = sc.name.clone();
        let badge = format!(" [{}]", source_badge(sc.source));

        let (fg, bg) = if selected {
            (Some(theme.selected_fg), Some(theme.selected_bg))
        } else if is_current {
            (Some(theme.accent), None)
        } else {
            (None, None)
        };

        rows.push(
            element! {
                View(background_color: bg, width: 100pct) {
                    MixedText(contents: vec![
                        {
                            let mut c = MixedTextContent::new(gutter);
                            if let Some(col) = fg { c = c.color(col); }
                            c.weight(Weight::Bold)
                        },
                        {
                            let mut c = MixedTextContent::new(check);
                            if let Some(col) = fg { c = c.color(col); }
                            else if is_current { c = c.color(theme.accent); }
                            c.weight(Weight::Bold)
                        },
                        {
                            let mut c = MixedTextContent::new(name);
                            if let Some(col) = fg { c = c.color(col); }
                            if is_current && !selected { c = c.weight(Weight::Bold); }
                            c
                        },
                        {
                            let mut c = MixedTextContent::new(badge);
                            if let Some(col) = fg { c = c.color(col); }
                            else { c = c.color(theme.muted); }
                            c
                        },
                    ])
                }
            }
            .into_any(),
        );
    }

    rows.push(element! { Text(content: " ") }.into_any());
    rows.push(
        element! {
            Text(
                content: "↑/↓ or j/k · enter select · esc cancel",
                color: theme.muted,
                weight: Weight::Light,
            )
        }
        .into_any(),
    );

    rows
}
