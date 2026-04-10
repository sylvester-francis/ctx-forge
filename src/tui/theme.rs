//! Color grading for the token gauge and status elements.

use ratatui::style::Color;

/// Returns gauge color based on percentage of context window used.
/// Green (0-40%) → Yellow (40-75%) → Orange (75-90%) → Red (>90%).
pub fn gauge_color(pct: f64) -> Color {
    if pct > 90.0 {
        Color::Red
    } else if pct > 75.0 {
        Color::Rgb(255, 165, 0) // orange
    } else if pct > 40.0 {
        Color::Yellow
    } else {
        Color::Green
    }
}

/// Color for selected (bundled) items in the file tree.
pub fn selected_color() -> Color {
    Color::Cyan
}

/// Color for directory names in the file tree.
pub fn dir_color() -> Color {
    Color::Blue
}

/// Color for the hotspot warning.
pub fn hotspot_color() -> Color {
    Color::Rgb(255, 165, 0)
}
