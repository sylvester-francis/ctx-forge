//! Compile-time reminder that hardcoded colours are forbidden in the TUI.
//!
//! Runs as a unit test. Walks `src/tui/` and asserts no file outside the
//! allowlist contains a `Color::<variant>` literal. Allowlisted paths:
//!
//! - `src/tui/theme/**`  — palettes themselves
//! - `src/tui/motion.rs` — animation primitives accept colours as inputs
//! - `src/tui/mod.rs`    — module declarations only
//! - `src/tui/ui.rs`     — retains `Color::Gray` as a fallback in blend calls
//! - `src/tui/viewer/highlight.rs` — converts syntect palette into ratatui Color
//!
//! The literal matcher is deliberately string-based so it runs without
//! parsing Rust. Comments starting with `//` on the same line are skipped.

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    fn is_allowlisted(p: &Path) -> bool {
        let s = p.to_string_lossy().replace('\\', "/");
        s.contains("src/tui/theme/")
            || s.ends_with("src/tui/motion.rs")
            || s.ends_with("src/tui/mod.rs")
            || s.ends_with("src/tui/ui.rs")
            || s.ends_with("src/tui/viewer/highlight.rs")
    }

    const VARIANTS: &[&str] = &[
        "Cyan", "Rgb(", "White", "Black", "Red", "Green", "Yellow", "Blue", "Magenta",
        "Gray", "DarkGray", "LightCyan", "LightRed", "LightGreen", "LightYellow",
        "LightBlue", "LightMagenta",
    ];

    fn visit(dir: &Path, hits: &mut Vec<String>) {
        let Ok(entries) = fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                visit(&p, hits);
                continue;
            }
            if p.extension().map_or(false, |e| e == "rs") && !is_allowlisted(&p) {
                let Ok(contents) = fs::read_to_string(&p) else { continue };
                for (idx, raw) in contents.lines().enumerate() {
                    let line = raw.trim_start();
                    if line.starts_with("//") || line.starts_with("///") {
                        continue;
                    }
                    let Some(pos) = raw.find("Color::") else { continue };
                    let tail = &raw[pos + "Color::".len()..];
                    if VARIANTS.iter().any(|v| tail.starts_with(v)) {
                        hits.push(format!("{}:{} {}", p.display(), idx + 1, raw.trim()));
                    }
                }
            }
        }
    }

    #[test]
    fn forbidden_color_literals_are_absent() {
        let mut hits = Vec::new();
        visit(Path::new("src/tui"), &mut hits);
        assert!(
            hits.is_empty(),
            "forbidden Color::<variant> literal in src/tui outside allowlisted files:\n{}",
            hits.join("\n")
        );
    }
}
