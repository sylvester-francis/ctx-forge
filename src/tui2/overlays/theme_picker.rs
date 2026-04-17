//! Theme picker — list the available themes with a small color swatch per row.

use crate::tui::theme::{registry, AppTheme};
use crate::tui2::theme::Theme;
use iocraft::prelude::*;

pub fn all() -> &'static [AppTheme] {
    registry::all_themes()
}

pub fn render_body(
    themes: &[AppTheme],
    cursor: usize,
    current: &str,
    theme: &Theme,
) -> Vec<AnyElement<'static>> {
    if themes.is_empty() {
        return vec![
            element! {
                Text(content: "  no themes found", color: theme.muted, weight: Weight::Light)
            }
            .into_any(),
        ];
    }

    let mut rows: Vec<AnyElement<'static>> = Vec::new();
    for (i, t) in themes.iter().enumerate() {
        let selected = i == cursor;
        let is_current = current == t.name;

        let gutter = if selected { "▶ " } else { "  " };
        let check = if is_current { "● " } else { "  " };
        let name = t.name.to_string();

        let swatch_accent = t.accent;
        let swatch_dir = t.dir;
        let swatch_success = t.success;
        let swatch_warning = t.warning;

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
                            if let Some(col) = fg { c = c.color(col); } else { c = c.color(theme.accent); }
                            c.weight(Weight::Bold)
                        },
                        {
                            let mut c = MixedTextContent::new(check);
                            if let Some(col) = fg { c = c.color(col); }
                            else if is_current { c = c.color(theme.accent); }
                            c.weight(Weight::Bold)
                        },
                        MixedTextContent::new("██").color(swatch_accent),
                        MixedTextContent::new("██").color(swatch_dir),
                        MixedTextContent::new("██").color(swatch_success),
                        MixedTextContent::new("██").color(swatch_warning),
                        MixedTextContent::new("  "),
                        {
                            let mut c = MixedTextContent::new(name);
                            if let Some(col) = fg { c = c.color(col); }
                            if is_current && !selected { c = c.weight(Weight::Bold); }
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
                content: "↑/↓ or j/k · enter apply · esc cancel",
                color: theme.muted,
                weight: Weight::Light,
            )
        }
        .into_any(),
    );

    rows
}
