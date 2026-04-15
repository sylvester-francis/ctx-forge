//! Warm dark — zinc background, terracotta accent.

use crate::tui::theme::AppTheme;
use ratatui::style::Color;

pub const ZINC: AppTheme = AppTheme {
    name: "zinc",
    bg: Color::Rgb(24, 24, 27),
    fg: Color::Rgb(228, 228, 231),
    accent: Color::Rgb(251, 146, 60),
    muted: Color::Rgb(113, 113, 122),
    highlight: Color::Rgb(251, 146, 60),
    success: Color::Rgb(134, 239, 172),
    warning: Color::Rgb(253, 224, 71),
    danger: Color::Rgb(248, 113, 113),
    border: Color::Rgb(63, 63, 70),
    border_focused: Color::Rgb(251, 146, 60),
    dir: Color::Rgb(147, 197, 253),
    hotspot: Color::Rgb(251, 146, 60),
    selected_fg: Color::Rgb(24, 24, 27),
    selected_bg: Color::Rgb(228, 228, 231),
    drag_selection_bg: Color::Rgb(75, 45, 30),
    focus_tree: Color::Rgb(147, 197, 253),
    focus_viewer: Color::Rgb(196, 181, 253),
    focus_bundle: Color::Rgb(251, 146, 60),
};
