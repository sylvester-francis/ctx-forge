//! Tokyo-night inspired — cool blues + purple.

use crate::theme::AppTheme;
use crossterm::style::Color;

pub const TOKYO_NIGHT: AppTheme = AppTheme {
    name: "tokyo-night",
    bg: Color::Rgb {
        r: 26,
        g: 27,
        b: 38,
    },
    fg: Color::Rgb {
        r: 192,
        g: 202,
        b: 245,
    },
    accent: Color::Rgb {
        r: 187,
        g: 154,
        b: 247,
    },
    muted: Color::Rgb {
        r: 86,
        g: 95,
        b: 137,
    },
    highlight: Color::Rgb {
        r: 125,
        g: 207,
        b: 255,
    },
    success: Color::Rgb {
        r: 158,
        g: 206,
        b: 106,
    },
    warning: Color::Rgb {
        r: 224,
        g: 175,
        b: 104,
    },
    danger: Color::Rgb {
        r: 247,
        g: 118,
        b: 142,
    },
    border: Color::Rgb {
        r: 65,
        g: 72,
        b: 104,
    },
    border_focused: Color::Rgb {
        r: 187,
        g: 154,
        b: 247,
    },
    dir: Color::Rgb {
        r: 125,
        g: 207,
        b: 255,
    },
    hotspot: Color::Rgb {
        r: 224,
        g: 175,
        b: 104,
    },
    selected_fg: Color::Rgb {
        r: 26,
        g: 27,
        b: 38,
    },
    selected_bg: Color::Rgb {
        r: 192,
        g: 202,
        b: 245,
    },
    drag_selection_bg: Color::Rgb {
        r: 50,
        g: 45,
        b: 80,
    },
    focus_tree: Color::Rgb {
        r: 125,
        g: 207,
        b: 255,
    },
    focus_viewer: Color::Rgb {
        r: 187,
        g: 154,
        b: 247,
    },
    focus_bundle: Color::Rgb {
        r: 224,
        g: 175,
        b: 104,
    },
};
