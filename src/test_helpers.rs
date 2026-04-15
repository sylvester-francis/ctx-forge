//! Shared helpers used by integration tests.
//!
//! Publicly exposed because the `ctxforge` crate is a library — integration
//! tests see it as an external dependency and cannot reach `#[cfg(test)]`
//! items. Each function here is only useful for tests; the cost of making
//! them `pub` is negligible.

#![allow(dead_code)]

#[cfg(feature = "tui")]
use crate::tui::app::App;
#[cfg(feature = "tui")]
use ratatui::buffer::Buffer;

/// Add a set of files to the bundle of the given `App`. Creates empty files
/// on disk under `app.project_root` so resolve + token counting have
/// something to read.
#[cfg(feature = "tui")]
pub fn seed_fixture(app: &mut App, paths: &[&str]) {
    use crate::bundle::{Item, ItemKind};
    for p in paths {
        let full = app.project_root.join(p);
        if let Some(parent) = full.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&full, "");
        app.bundle.items.push(Item {
            path: std::path::PathBuf::from(p),
            kind: ItemKind::File,
            label: None,
        });
        app.bundled_paths.insert(std::path::PathBuf::from(p));
    }
    app.recalculate_tokens();
}

/// Serialise a `Buffer` into a plain-text grid (symbols only, no styling).
/// Useful for snapshot tests that want a stable textual representation.
#[cfg(feature = "tui")]
pub fn buffer_to_ansi_string(buf: &Buffer) -> String {
    let mut out = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}
