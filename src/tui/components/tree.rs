//! File tree — distinctive indented rendering with status dots and nesting rails.
//!
//! Design language:
//! - `│  ` rails for nesting (visual hierarchy)
//! - `▾`/`▸` expanded/collapsed dir markers
//! - `●` bundled file (accent), `○` available file (dim)
//! - `▶` cursor indicator (left gutter) when focused
//! - Selected row: inverted colors with leading `▶`

use crate::tree::TreeEntry;
use crate::tui::theme::Theme;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use iocraft::prelude::*;
use std::collections::HashSet;
use std::path::PathBuf;

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

pub fn render_tree_rows(
    entries: &[(usize, TreeEntry)],
    cursor: usize,
    focused: bool,
    bundled_paths: &HashSet<PathBuf>,
    theme: &Theme,
) -> Vec<AnyElement<'static>> {
    entries
        .iter()
        .map(|(vi, entry)| {
            let selected = *vi == cursor && focused;
            let is_bundled = bundled_paths.contains(&entry.rel_path);

            let rails = "│  ".repeat(entry.depth);
            let gutter = if selected { "▶ " } else { "  " };
            let marker = if entry.is_dir {
                if entry.expanded {
                    "▾ "
                } else {
                    "▸ "
                }
            } else if is_bundled {
                "● "
            } else {
                "○ "
            };

            let (gutter_color, rail_color, marker_color, name_color) = if selected {
                (
                    Some(theme.accent),
                    Some(theme.selected_fg),
                    Some(theme.selected_fg),
                    Some(theme.selected_fg),
                )
            } else if entry.is_dir {
                (
                    Some(theme.muted),
                    Some(theme.muted),
                    Some(theme.dir),
                    Some(theme.dir),
                )
            } else if is_bundled {
                (
                    Some(theme.muted),
                    Some(theme.muted),
                    Some(theme.accent),
                    Some(theme.accent),
                )
            } else {
                (Some(theme.muted), Some(theme.muted), Some(theme.muted), None)
            };

            let bg = if selected { Some(theme.selected_bg) } else { None };
            let name = entry.name.clone();
            let bold_name = is_bundled && !selected;

            element! {
                View(background_color: bg, width: 100pct) {
                    MixedText(contents: vec![
                        span(gutter.to_string(), gutter_color, false),
                        span(rails, rail_color, false),
                        span(marker.to_string(), marker_color, false),
                        span(name, name_color, bold_name),
                    ])
                }
            }
            .into_any()
        })
        .collect()
}

/// Render fuzzy-ranked search results across all files (no directories).
/// Each row shows the full relative path instead of the leaf name.
pub fn render_search_rows(
    entries: &[TreeEntry],
    query: &str,
    cursor: usize,
    bundled_paths: &HashSet<PathBuf>,
    theme: &Theme,
    max_rows: usize,
) -> Vec<AnyElement<'static>> {
    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<(usize, i64)> = entries
        .iter()
        .enumerate()
        .filter(|(_, e)| !e.is_dir)
        .filter_map(|(i, e)| {
            let path_str = e.rel_path.to_string_lossy();
            matcher
                .fuzzy_match(&path_str, query)
                .map(|score| (i, score))
        })
        .collect();
    scored.sort_by(|a, b| b.1.cmp(&a.1));
    scored.truncate(max_rows);

    scored
        .into_iter()
        .enumerate()
        .filter_map(|(vi, (actual, _))| entries.get(actual).map(|e| (vi, e)))
        .map(|(vi, entry)| {
            let selected = vi == cursor;
            let is_bundled = bundled_paths.contains(&entry.rel_path);
            let marker = if is_bundled { "● " } else { "○ " };
            let gutter = if selected { "▶ " } else { "  " };
            let path = entry.rel_path.to_string_lossy().to_string();

            let (gutter_c, marker_c, name_c) = if selected {
                (
                    Some(theme.accent),
                    Some(theme.selected_fg),
                    Some(theme.selected_fg),
                )
            } else if is_bundled {
                (Some(theme.muted), Some(theme.accent), Some(theme.accent))
            } else {
                (Some(theme.muted), Some(theme.muted), None)
            };
            let bg = if selected { Some(theme.selected_bg) } else { None };
            let bold_name = is_bundled && !selected;

            element! {
                View(background_color: bg, width: 100pct) {
                    MixedText(contents: vec![
                        span(gutter.to_string(), gutter_c, false),
                        span(marker.to_string(), marker_c, false),
                        span(path, name_c, bold_name),
                    ])
                }
            }
            .into_any()
        })
        .collect()
}
