//! Bundle list — indexed badges, truncated paths, right-aligned tokens.

use crate::bundle::Bundle;
use crate::tui::theme::Theme;
use iocraft::prelude::*;

fn format_tokens(n: usize) -> String {
    if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        format!("{n}")
    }
}

use super::smart_truncate_path;

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

pub fn render_bundle_rows(
    bundle: &Bundle,
    item_tokens: &[usize],
    theme: &Theme,
) -> (String, Vec<AnyElement<'static>>) {
    let total: usize = item_tokens.iter().sum();
    let title = if bundle.is_empty() {
        "BUNDLE · empty".to_string()
    } else {
        format!(
            "BUNDLE · {} · {} tokens",
            bundle.len(),
            format_tokens(total)
        )
    };

    let mut rows: Vec<AnyElement<'static>> = Vec::new();
    if bundle.is_empty() {
        rows.push(
            element! {
                View(width: 100pct) {
                    Text(
                        content: "  no files yet — space in tree, or @ in prompt",
                        color: theme.muted,
                        weight: Weight::Light,
                    )
                }
            }
            .into_any(),
        );
    } else {
        for (i, (item, tok)) in bundle.items.iter().zip(item_tokens.iter()).enumerate() {
            let badge = format!(" {:02} ", i + 1);
            let path_str = item.path.display().to_string();
            let path = smart_truncate_path(&path_str, 28);
            let path_padded = format!(" {:<28} ", path);
            let toks = format!("{:>6}", format_tokens(*tok));

            rows.push(
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

    (title, rows)
}
