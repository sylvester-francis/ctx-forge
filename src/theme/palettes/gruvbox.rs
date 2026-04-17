//! Gruvbox dark — earthy browns + amber.

use crate::theme::AppTheme;
use crossterm::style::Color;

pub const GRUVBOX: AppTheme = AppTheme {
    name: "gruvbox",
    bg: Color::Rgb { r: 40, g: 40, b: 40 },
    fg: Color::Rgb { r: 235, g: 219, b: 178 },
    accent: Color::Rgb { r: 250, g: 189, b: 47 },
    muted: Color::Rgb { r: 146, g: 131, b: 116 },
    highlight: Color::Rgb { r: 250, g: 189, b: 47 },
    success: Color::Rgb { r: 184, g: 187, b: 38 },
    warning: Color::Rgb { r: 250, g: 189, b: 47 },
    danger: Color::Rgb { r: 251, g: 73, b: 52 },
    border: Color::Rgb { r: 80, g: 73, b: 69 },
    border_focused: Color::Rgb { r: 250, g: 189, b: 47 },
    dir: Color::Rgb { r: 131, g: 165, b: 152 },
    hotspot: Color::Rgb { r: 254, g: 128, b: 25 },
    selected_fg: Color::Rgb { r: 40, g: 40, b: 40 },
    selected_bg: Color::Rgb { r: 235, g: 219, b: 178 },
    drag_selection_bg: Color::Rgb { r: 80, g: 60, b: 30 },
    focus_tree: Color::Rgb { r: 131, g: 165, b: 152 },
    focus_viewer: Color::Rgb { r: 211, g: 134, b: 155 },
    focus_bundle: Color::Rgb { r: 254, g: 128, b: 25 },
};
