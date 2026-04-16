//! Bridge: convert ctxforge's `AppTheme` (ratatui colors) to iocraft colors.
//!
//! v2 components consume this `Theme` struct; the underlying palette registry
//! is shared with v1 via `crate::tui::theme`.

use crate::tui::theme::AppTheme;

pub fn to_iocraft_color(c: ratatui::style::Color) -> iocraft::Color {
    match c {
        ratatui::style::Color::Rgb(r, g, b) => iocraft::Color::Rgb { r, g, b },
        ratatui::style::Color::Black => iocraft::Color::Black,
        ratatui::style::Color::Red => iocraft::Color::Red,
        ratatui::style::Color::Green => iocraft::Color::Green,
        ratatui::style::Color::Yellow => iocraft::Color::Yellow,
        ratatui::style::Color::Blue => iocraft::Color::Blue,
        ratatui::style::Color::Magenta => iocraft::Color::Magenta,
        ratatui::style::Color::Cyan => iocraft::Color::Cyan,
        ratatui::style::Color::Gray => iocraft::Color::Grey,
        ratatui::style::Color::DarkGray => iocraft::Color::DarkGrey,
        ratatui::style::Color::White => iocraft::Color::White,
        ratatui::style::Color::LightRed => iocraft::Color::Red,
        ratatui::style::Color::LightGreen => iocraft::Color::Green,
        ratatui::style::Color::LightYellow => iocraft::Color::Yellow,
        ratatui::style::Color::LightBlue => iocraft::Color::Blue,
        ratatui::style::Color::LightMagenta => iocraft::Color::Magenta,
        ratatui::style::Color::LightCyan => iocraft::Color::Cyan,
        ratatui::style::Color::Indexed(n) => iocraft::Color::AnsiValue(n),
        ratatui::style::Color::Reset => iocraft::Color::Reset,
    }
}

#[derive(Clone, Copy)]
pub struct Theme {
    pub name: &'static str,
    pub bg: iocraft::Color,
    pub fg: iocraft::Color,
    pub accent: iocraft::Color,
    pub muted: iocraft::Color,
    pub highlight: iocraft::Color,
    pub border: iocraft::Color,
    pub border_focused: iocraft::Color,
    pub dir: iocraft::Color,
    pub success: iocraft::Color,
    pub warning: iocraft::Color,
    pub danger: iocraft::Color,
    pub selected_fg: iocraft::Color,
    pub selected_bg: iocraft::Color,
    pub focus_tree: iocraft::Color,
    pub focus_viewer: iocraft::Color,
    pub focus_bundle: iocraft::Color,
    pub hotspot: iocraft::Color,
}

impl Theme {
    pub fn from_app_theme(t: &AppTheme) -> Self {
        Self {
            name: t.name,
            bg: to_iocraft_color(t.bg),
            fg: to_iocraft_color(t.fg),
            accent: to_iocraft_color(t.accent),
            muted: to_iocraft_color(t.muted),
            highlight: to_iocraft_color(t.highlight),
            border: to_iocraft_color(t.border),
            border_focused: to_iocraft_color(t.border_focused),
            dir: to_iocraft_color(t.dir),
            success: to_iocraft_color(t.success),
            warning: to_iocraft_color(t.warning),
            danger: to_iocraft_color(t.danger),
            selected_fg: to_iocraft_color(t.selected_fg),
            selected_bg: to_iocraft_color(t.selected_bg),
            focus_tree: to_iocraft_color(t.focus_tree),
            focus_viewer: to_iocraft_color(t.focus_viewer),
            focus_bundle: to_iocraft_color(t.focus_bundle),
            hotspot: to_iocraft_color(t.hotspot),
        }
    }
}
