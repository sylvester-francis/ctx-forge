//! File tree — distinctive indented rendering with status dots and nesting rails.
//!
//! Design language:
//! - `│  ` rails for nesting (visual hierarchy)
//! - `▾`/`▸` expanded/collapsed dir markers
//! - `●` bundled file (accent), `○` available file (dim)
//! - `▶` cursor indicator (left gutter) when focused
//! - Selected row: inverted colors with leading `▶`

use crate::tui::tree::TreeEntry;
use crate::tui2::theme::Theme;
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
