//! Warm dark — zinc background, terracotta accent.

use crate::theme::AppTheme;
use crossterm::style::Color;

pub const ZINC: AppTheme = AppTheme {
    name: "zinc",
    bg: Color::Rgb {
        r: 24,
        g: 24,
        b: 27,
    },
    fg: Color::Rgb {
        r: 228,
        g: 228,
        b: 231,
    },
    accent: Color::Rgb {
        r: 251,
        g: 146,
        b: 60,
    },
    muted: Color::Rgb {
        r: 113,
        g: 113,
        b: 122,
    },
    highlight: Color::Rgb {
        r: 251,
        g: 146,
        b: 60,
    },
    success: Color::Rgb {
        r: 134,
        g: 239,
        b: 172,
    },
    warning: Color::Rgb {
        r: 253,
        g: 224,
        b: 71,
    },
    danger: Color::Rgb {
        r: 248,
        g: 113,
        b: 113,
    },
    border: Color::Rgb {
        r: 63,
        g: 63,
        b: 70,
    },
    border_focused: Color::Rgb {
        r: 251,
        g: 146,
        b: 60,
    },
    dir: Color::Rgb {
        r: 147,
        g: 197,
        b: 253,
    },
    hotspot: Color::Rgb {
        r: 251,
        g: 146,
        b: 60,
    },
    selected_fg: Color::Rgb {
        r: 24,
        g: 24,
        b: 27,
    },
    selected_bg: Color::Rgb {
        r: 228,
        g: 228,
        b: 231,
    },
    drag_selection_bg: Color::Rgb {
        r: 75,
        g: 45,
        b: 30,
    },
    focus_tree: Color::Rgb {
        r: 147,
        g: 197,
        b: 253,
    },
    focus_viewer: Color::Rgb {
        r: 196,
        g: 181,
        b: 253,
    },
    focus_bundle: Color::Rgb {
        r: 251,
        g: 146,
        b: 60,
    },
};
