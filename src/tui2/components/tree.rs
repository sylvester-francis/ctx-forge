//! File tree — read-only render with cursor highlighting.
//!
//! Consumes `TreeEntry` values from `crate::tui::tree` and `bundled_paths`
//! from the app state. Returns rows as AnyElements so the parent can place
//! them inside a bordered/padded View.

use crate::tui::tree::TreeEntry;
use crate::tui2::theme::Theme;
use iocraft::prelude::*;
use std::collections::HashSet;
use std::path::PathBuf;

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
            let indent = "  ".repeat(entry.depth);
            let marker = if entry.is_dir {
                if entry.expanded {
                    "▾ "
                } else {
                    "▸ "
                }
            } else if bundled_paths.contains(&entry.rel_path) {
                "■ "
            } else {
                "▫ "
            };
            let text = format!("{indent}{marker}{}", entry.name);

            let selected = *vi == cursor && focused;
            let (fg, bg) = if selected {
                (Some(theme.selected_fg), Some(theme.selected_bg))
            } else if entry.is_dir {
                (Some(theme.dir), None)
            } else if bundled_paths.contains(&entry.rel_path) {
                (Some(theme.accent), None)
            } else {
                (None, None)
            };

            element! {
                View(background_color: bg) {
                    Text(content: text.leak() as &str, color: fg)
                }
            }
            .into_any()
        })
        .collect()
}
