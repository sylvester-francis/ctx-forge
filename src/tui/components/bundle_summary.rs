//! Bundle list — indexed badges, truncated paths, right-aligned tokens.
//!
//! Network sources (URL) render with a freshness dot (●/◐/◌) reflecting
//! their cache state: fresh / stale / uncached.

use crate::bundle::{Bundle, Item};
use crate::cache::{CacheRead, ContentCache};
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
        let cache = crate::paths::global_cache_dir().and_then(|d| ContentCache::open(d).ok());
        for (i, (item, tok)) in bundle.items.iter().zip(item_tokens.iter()).enumerate() {
            let badge = format!(" {:02} ", i + 1);
            let dot = freshness_dot(item, cache.as_ref());
            let dot_color = freshness_color(item, cache.as_ref(), theme);
            let path_str = item.source.display_label();
            let path = smart_truncate_path(&path_str, 26);
            let path_padded = format!(" {:<26} ", path);
            let toks = format!("{:>6}", format_tokens(*tok));

            rows.push(
                element! {
                    View(width: 100pct) {
                        MixedText(contents: vec![
                            span(badge, Some(theme.accent), true),
                            span(format!("{dot} "), Some(dot_color), false),
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

/// Freshness dot for a bundle item. Local sources always render as fresh;
/// cacheable sources reflect cache state.
fn freshness_dot(item: &Item, cache: Option<&ContentCache>) -> &'static str {
    if !item.source.is_cacheable() {
        return "●";
    }
    match cache.map(|c| c.get(&item.source.cache_key())) {
        Some(Ok(CacheRead::Fresh { .. })) => "●",
        Some(Ok(CacheRead::Stale { .. })) => "◐",
        _ => "◌",
    }
}

fn freshness_color(item: &Item, cache: Option<&ContentCache>, theme: &Theme) -> Color {
    if !item.source.is_cacheable() {
        return theme.muted;
    }
    match cache.map(|c| c.get(&item.source.cache_key())) {
        Some(Ok(CacheRead::Fresh { .. })) => theme.accent,
        Some(Ok(CacheRead::Stale { .. })) => theme.warning,
        _ => theme.muted,
    }
}
