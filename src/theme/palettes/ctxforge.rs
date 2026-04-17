//! Default palette — matches the colours used before the theme refactor.

use crate::theme::AppTheme;
use crossterm::style::Color;

pub const CTXFORGE: AppTheme = AppTheme {
    name: "ctxforge",
    bg: Color::Rgb {
        r: 10,
        g: 14,
        b: 22,
    },
    fg: Color::Reset,
    accent: Color::Cyan,
    muted: Color::DarkGrey,
    highlight: Color::Cyan,
    success: Color::Green,
    warning: Color::Yellow,
    danger: Color::Red,
    border: Color::DarkGrey,
    border_focused: Color::Cyan,
    dir: Color::Rgb {
        r: 121,
        g: 192,
        b: 255,
    },
    hotspot: Color::Rgb {
        r: 255,
        g: 165,
        b: 0,
    },
    selected_fg: Color::Black,
    selected_bg: Color::White,
    drag_selection_bg: Color::Rgb {
        r: 60,
        g: 40,
        b: 80,
    },
    focus_tree: Color::Rgb {
        r: 88,
        g: 166,
        b: 255,
    },
    focus_viewer: Color::Rgb {
        r: 163,
        g: 113,
        b: 247,
    },
    focus_bundle: Color::Rgb {
        r: 255,
        g: 165,
        b: 0,
    },
};
