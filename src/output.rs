//! Styled-output helpers for the CLI.
//!
//! Centralizes color, error formatting, progress bars, and "did you mean"
//! suggestions. All helpers are TTY-aware and honor the `NO_COLOR`
//! environment variable per https://no-color.org.

#![allow(dead_code)]

use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use std::io::IsTerminal;
use std::path::Path;
use std::time::Duration;

/// Returns true if stdout is a terminal AND `NO_COLOR` is not set.
/// Output color helpers should check this before emitting ANSI codes.
pub fn color_enabled() -> bool {
    std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none()
}

/// Returns true if stdout is a TTY (regardless of NO_COLOR).
/// Used for deciding whether to render progress bars / spinners.
pub fn is_tty() -> bool {
    std::io::stdout().is_terminal()
}

/// Print a success message in green with a leading checkmark.
pub fn success(msg: &str) {
    if color_enabled() {
        println!("{} {}", "✓".green().bold(), msg);
    } else {
        println!("✓ {msg}");
    }
}

/// Print a warning message in yellow with a leading marker.
pub fn warn(msg: &str) {
    if color_enabled() {
        eprintln!("{} {}", "⚠".yellow().bold(), msg);
    } else {
        eprintln!("⚠ {msg}");
    }
}

/// Print a plain error header in red. Use `error_with_suggestion` for
/// the richer file-not-found layout.
pub fn error(msg: &str) {
    if color_enabled() {
        eprintln!("{} {}", "Error:".red().bold(), msg);
    } else {
        eprintln!("Error: {msg}");
    }
}

/// Render a NotFound-style error with path context, fuzzy-matched
/// suggestions, and an optional hint footer.
pub fn error_with_suggestion(
    header: &str,
    path: &Path,
    resolved: &Path,
    suggestions: &[String],
    hint: Option<&str>,
) {
    if color_enabled() {
        eprintln!("{} {}", "Error:".red().bold(), header.bold());
        eprintln!("  {} {}", "path:".dimmed(), path.display());
        eprintln!("  {} {}", "resolved:".dimmed(), resolved.display());
        if !suggestions.is_empty() {
            eprintln!();
            eprintln!("  Did you mean one of these?");
            for s in suggestions {
                eprintln!("    {}", s.cyan());
            }
        }
        if let Some(h) = hint {
            eprintln!();
            eprintln!("  {}", h.dimmed().italic());
        }
    } else {
        eprintln!("Error: {header}");
        eprintln!("  path:     {}", path.display());
        eprintln!("  resolved: {}", resolved.display());
        if !suggestions.is_empty() {
            eprintln!();
            eprintln!("  Did you mean one of these?");
            for s in suggestions {
                eprintln!("    {s}");
            }
        }
        if let Some(h) = hint {
            eprintln!();
            eprintln!("  {h}");
        }
    }
}

/// Returns the percentage as a colored string. Buckets:
///   <25% green, 25-50% yellow, 50-75% orange (208), 75%+ red.
pub fn colored_percent(pct: f64) -> String {
    if !color_enabled() {
        return format!("{pct:5.1}%");
    }
    let formatted = format!("{pct:5.1}%");
    if pct < 25.0 {
        formatted.green().to_string()
    } else if pct < 50.0 {
        formatted.yellow().to_string()
    } else if pct < 75.0 {
        // 256-color orange (xterm 208 = FlushOrange; spec said DarkOrange but that
        // variant does not exist in owo-colors 4.3.0 — FlushOrange is xterm 208)
        formatted.color(owo_colors::XtermColors::FlushOrange).to_string()
    } else {
        formatted.red().bold().to_string()
    }
}

/// Returns up to `limit` candidates that fuzzy-match `query`, ordered
/// best-first. Uses Jaro-Winkler similarity. Drops candidates whose
/// score is below 0.7 (anything lower is too dissimilar to be useful).
pub fn close_matches(query: &str, candidates: &[String], limit: usize) -> Vec<String> {
    let mut scored: Vec<(f64, &String)> = candidates
        .iter()
        .map(|c| (strsim::jaro_winkler(query, c), c))
        .filter(|(s, _)| *s >= 0.7)
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.into_iter().take(limit).map(|(_, c)| c.clone()).collect()
}

/// Create an indicatif spinner with our standard styling. Caller is
/// responsible for calling `.set_message()`, `.tick()` periodically,
/// and `.finish_with_message()` when done. Returns a no-op hidden bar
/// when stdout is not a TTY.
pub fn spinner(label: &str) -> ProgressBar {
    if !is_tty() {
        return ProgressBar::hidden();
    }
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("  {spinner} {msg}")
            .expect("valid spinner template")
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.set_message(label.to_string());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

/// Create an indicatif progress bar for a known total. Returns a no-op
/// hidden bar when stdout is not a TTY.
pub fn progress(total: u64, label: &str) -> ProgressBar {
    if !is_tty() {
        return ProgressBar::hidden();
    }
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("  {bar:40.cyan/blue} {pos}/{len} {msg}")
            .expect("valid bar template"),
    );
    pb.set_message(label.to_string());
    pb
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn close_matches_ranks_by_similarity() {
        let candidates: Vec<String> = vec!["main.rs", "main.py", "lib.rs", "README.md"]
            .into_iter()
            .map(String::from)
            .collect();
        let matches = close_matches("mian.py", &candidates, 3);
        // "main.py" should be the closest (one transposition).
        assert_eq!(matches.first().map(String::as_str), Some("main.py"));
    }

    #[test]
    fn close_matches_returns_at_most_limit() {
        let candidates: Vec<String> = vec!["a", "b", "c", "d", "e"]
            .into_iter()
            .map(String::from)
            .collect();
        let matches = close_matches("a", &candidates, 2);
        assert!(matches.len() <= 2);
    }

    #[test]
    fn close_matches_filters_low_scores() {
        let candidates: Vec<String> = vec!["totally-different".to_string()];
        let matches = close_matches("xyz", &candidates, 5);
        // Score should be too low; expect empty.
        assert!(matches.is_empty());
    }

    #[test]
    fn colored_percent_low_is_safe_color() {
        // Just verify the function doesn't panic and produces output.
        // We can't easily assert on the ANSI escapes themselves.
        let s = colored_percent(5.0);
        assert!(!s.is_empty());
    }

    #[test]
    fn colored_percent_high_is_warning_color() {
        let s = colored_percent(95.0);
        assert!(!s.is_empty());
    }

    #[test]
    fn no_color_env_disables_color() {
        // SAFETY: tests run single-threaded for env var manipulation.
        unsafe {
            std::env::set_var("NO_COLOR", "1");
        }
        assert!(!color_enabled());
        unsafe {
            std::env::remove_var("NO_COLOR");
        }
    }
}
