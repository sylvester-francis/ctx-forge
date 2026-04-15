//! Default palette — matches the colours used before the theme refactor.

use crate::tui::theme::AppTheme;
use ratatui::style::Color;

pub const CTXFORGE: AppTheme = AppTheme {
    name: "ctxforge",
    bg: Color::Rgb(10, 14, 22),
    fg: Color::Reset,
    accent: Color::Cyan,
    muted: Color::DarkGray,
    highlight: Color::Cyan,
    success: Color::Green,
    warning: Color::Yellow,
    danger: Color::Red,
    border: Color::DarkGray,
    border_focused: Color::Cyan,
    dir: Color::Rgb(121, 192, 255),
    hotspot: Color::Rgb(255, 165, 0),
    selected_fg: Color::Black,
    selected_bg: Color::White,
    drag_selection_bg: Color::Rgb(60, 40, 80),
};
