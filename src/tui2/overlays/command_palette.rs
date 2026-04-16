//! Command palette overlay — inline search bar + fuzzy-filtered command list.

use crate::tui2::command_registry::CommandSpec;
use crate::tui2::theme::Theme;
use iocraft::prelude::*;

pub fn render_body(
    query: &str,
    results: &[&'static CommandSpec],
    cursor: usize,
    theme: &Theme,
) -> Vec<AnyElement<'static>> {
    let mut body: Vec<AnyElement<'static>> = Vec::new();

    let query_line = format!("{query}▏");
    body.push(
        element! {
            MixedText(contents: vec![
                MixedTextContent::new("/ ").color(theme.accent).weight(Weight::Bold),
                MixedTextContent::new(query_line).color(theme.fg),
            ])
        }
        .into_any(),
    );

    body.push(
        element! {
            Text(
                content: "─────────────────────────────────────────",
                color: theme.muted,
                weight: Weight::Light,
            )
        }
        .into_any(),
    );
    body.push(element! { Text(content: " ") }.into_any());

    if results.is_empty() {
        body.push(
            element! {
                Text(content: "  no commands match", color: theme.muted, weight: Weight::Light)
            }
            .into_any(),
        );
        return body;
    }

    const MAX_ROWS: usize = 16;
    let start = cursor.saturating_sub(MAX_ROWS / 2);
    let end = (start + MAX_ROWS).min(results.len());

    for (i, cmd) in results.iter().enumerate().skip(start).take(end - start) {
        let selected = i == cursor;
        let gutter = if selected { "▶ " } else { "  " };
        let name = cmd.name.to_string();
        let desc = format!("  {}", cmd.description);

        let (fg, bg) = if selected {
            (Some(theme.selected_fg), Some(theme.selected_bg))
        } else {
            (None, None)
        };

        body.push(
            element! {
                View(background_color: bg, width: 100pct) {
                    MixedText(contents: vec![
                        {
                            let mut c = MixedTextContent::new(gutter);
                            if let Some(col) = fg { c = c.color(col); }
                            else { c = c.color(theme.accent); }
                            c.weight(Weight::Bold)
                        },
                        {
                            let mut c = MixedTextContent::new(format!("/{name}"));
                            if let Some(col) = fg { c = c.color(col); }
                            else { c = c.color(theme.accent); }
                            c.weight(Weight::Bold)
                        },
                        {
                            let mut c = MixedTextContent::new(desc);
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

    if end < results.len() {
        let hint = format!("  …{} more below", results.len() - end);
        body.push(
            element! {
                Text(content: hint.leak() as &str, color: theme.muted, weight: Weight::Light)
            }
            .into_any(),
        );
    }

    body
}
