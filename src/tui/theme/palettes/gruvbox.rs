//! Gruvbox dark — earthy browns + amber.

use crate::tui::theme::AppTheme;
use ratatui::style::Color;

pub const GRUVBOX: AppTheme = AppTheme {
    name: "gruvbox",
    bg: Color::Rgb(40, 40, 40),
    fg: Color::Rgb(235, 219, 178),
    accent: Color::Rgb(250, 189, 47),
    muted: Color::Rgb(146, 131, 116),
    highlight: Color::Rgb(250, 189, 47),
    success: Color::Rgb(184, 187, 38),
    warning: Color::Rgb(250, 189, 47),
    danger: Color::Rgb(251, 73, 52),
    border: Color::Rgb(80, 73, 69),
    border_focused: Color::Rgb(250, 189, 47),
    dir: Color::Rgb(131, 165, 152),
    hotspot: Color::Rgb(254, 128, 25),
    selected_fg: Color::Rgb(40, 40, 40),
    selected_bg: Color::Rgb(235, 219, 178),
    drag_selection_bg: Color::Rgb(80, 60, 30),
    focus_tree: Color::Rgb(131, 165, 152),
    focus_viewer: Color::Rgb(211, 134, 155),
    focus_bundle: Color::Rgb(254, 128, 25),
};
