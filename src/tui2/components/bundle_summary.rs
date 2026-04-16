//! Compact bundle summary (lower-left column).

use crate::bundle::Bundle;
use crate::tui2::theme::Theme;
use iocraft::prelude::*;

fn format_tokens(n: usize) -> String {
    if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        format!("{n}")
    }
}

pub fn render_bundle_rows(
    bundle: &Bundle,
    item_tokens: &[usize],
    theme: &Theme,
) -> (String, Vec<AnyElement<'static>>) {
    let total: usize = item_tokens.iter().sum();
    let title = if bundle.is_empty() {
        " bundle (0) ".to_string()
    } else {
        format!(
            " bundle ({}) · {} tokens ",
            bundle.len(),
            format_tokens(total)
        )
    };

    let mut rows: Vec<AnyElement<'static>> = Vec::new();
    if bundle.is_empty() {
        rows.push(
            element! {
                Text(
                    content: " (empty — space in tree, or @ in prompt)",
                    color: theme.muted,
                    weight: Weight::Light,
                )
            }
            .into_any(),
        );
    } else {
        for (i, (item, tok)) in bundle.items.iter().zip(item_tokens.iter()).enumerate() {
            let idx = format!(" {:>2}  ", i + 1);
            let path = format!("{:<35}  ", item.path.display());
            let toks = format_tokens(*tok);
            rows.push(
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

    (title, rows)
}
