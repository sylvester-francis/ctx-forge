//! Tokyo-night inspired — cool blues + purple.

use crate::tui::theme::AppTheme;
use ratatui::style::Color;

pub const TOKYO_NIGHT: AppTheme = AppTheme {
    name: "tokyo-night",
    bg: Color::Rgb(26, 27, 38),
    fg: Color::Rgb(192, 202, 245),
    accent: Color::Rgb(187, 154, 247),
    muted: Color::Rgb(86, 95, 137),
    highlight: Color::Rgb(125, 207, 255),
    success: Color::Rgb(158, 206, 106),
    warning: Color::Rgb(224, 175, 104),
    danger: Color::Rgb(247, 118, 142),
    border: Color::Rgb(65, 72, 104),
    border_focused: Color::Rgb(187, 154, 247),
    dir: Color::Rgb(125, 207, 255),
    hotspot: Color::Rgb(224, 175, 104),
    selected_fg: Color::Rgb(26, 27, 38),
    selected_bg: Color::Rgb(192, 202, 245),
    drag_selection_bg: Color::Rgb(50, 45, 80),
    focus_tree: Color::Rgb(125, 207, 255),
    focus_viewer: Color::Rgb(187, 154, 247),
    focus_bundle: Color::Rgb(224, 175, 104),
};
