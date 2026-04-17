//! Delivery picker — the 7 delivery choices with cursor selection.

use crate::tui::deliver::DeliverChoice;
use crate::tui2::theme::Theme;
use iocraft::prelude::*;

pub fn render_body(cursor: usize, theme: &Theme) -> Vec<AnyElement<'static>> {
    let choices = DeliverChoice::all();
    let mut rows: Vec<AnyElement<'static>> = Vec::new();

    for (i, choice) in choices.iter().enumerate() {
        let selected = i == cursor;
        let gutter = if selected { "▶ " } else { "  " };
        let label = choice.label().to_string();

        let (fg, bg) = if selected {
            (Some(theme.selected_fg), Some(theme.selected_bg))
        } else {
            (None, None)
        };

        rows.push(
            element! {
                View(background_color: bg, width: 100pct) {
                    MixedText(contents: vec![
                        {
                            let mut c = MixedTextContent::new(gutter);
                            if let Some(col) = fg { c = c.color(col); } else { c = c.color(theme.accent); }
                            c.weight(Weight::Bold)
                        },
                        {
                            let mut c = MixedTextContent::new(label);
                            if let Some(col) = fg { c = c.color(col); }
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
                content: "↑/↓ or j/k · enter deliver · esc cancel",
                color: theme.muted,
                weight: Weight::Light,
            )
        }
        .into_any(),
    );

    rows
}
