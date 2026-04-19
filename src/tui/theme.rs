//! Bridge: expose ctxforge's `AppTheme` (crossterm colors) as an iocraft-
//! compatible `Theme` struct. `iocraft::Color == crossterm::style::Color`,
//! so this is a field-for-field copy.

use crate::theme::AppTheme;

pub type Color = crossterm::style::Color;

#[derive(Clone, Copy)]
pub struct Theme {
    pub name: &'static str,
    pub bg: Color,
    pub fg: Color,
    pub accent: Color,
    pub muted: Color,
    pub highlight: Color,
    pub border: Color,
    pub border_focused: Color,
    pub dir: Color,
    pub success: Color,
    pub warning: Color,
    pub danger: Color,
    pub selected_fg: Color,
    pub selected_bg: Color,
    pub focus_tree: Color,
    pub focus_viewer: Color,
    pub focus_bundle: Color,
    pub hotspot: Color,
    pub drag_selection_bg: Color,
}

impl Theme {
    pub fn from_app_theme(t: &AppTheme) -> Self {
        Self {
            name: t.name,
            bg: t.bg,
            fg: t.fg,
            accent: t.accent,
            muted: t.muted,
            highlight: t.highlight,
            border: t.border,
            border_focused: t.border_focused,
            dir: t.dir,
            success: t.success,
            warning: t.warning,
            danger: t.danger,
            selected_fg: t.selected_fg,
            selected_bg: t.selected_bg,
            focus_tree: t.focus_tree,
            focus_viewer: t.focus_viewer,
            focus_bundle: t.focus_bundle,
            hotspot: t.hotspot,
            drag_selection_bg: t.drag_selection_bg,
        }
    }
}
