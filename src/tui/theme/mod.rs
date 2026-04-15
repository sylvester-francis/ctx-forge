//! Color theme registry for the TUI.
//!
//! Every colored element in the TUI reads from an `AppTheme` via
//! `app.theme.<field>`. Hardcoded `Color::<variant>` usage in `src/tui/**`
//! is forbidden by design — route all colors through this module.

use ratatui::style::Color;

pub mod config;
pub mod palettes;
pub mod registry;

#[cfg(test)]
mod lint;

/// A colour palette used by every TUI widget. Each field has exactly one
/// semantic role; pick the field that matches your role, do not pick by colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppTheme {
    pub name: &'static str,
    pub bg: Color,
    pub fg: Color,
    pub accent: Color,
    pub muted: Color,
    pub highlight: Color,
    pub success: Color,
    pub warning: Color,
    pub danger: Color,
    pub border: Color,
    pub border_focused: Color,
    pub dir: Color,
    pub hotspot: Color,
    /// Foreground of a selected list row (command palette, model switch, etc.).
    pub selected_fg: Color,
    /// Background of a selected list row.
    pub selected_bg: Color,
    /// Background fill for viewer drag-selection.
    pub drag_selection_bg: Color,
    /// Animated focus-border tint for the file tree panel.
    pub focus_tree: Color,
    /// Animated focus-border tint for the viewer panel.
    pub focus_viewer: Color,
    /// Animated focus-border tint for the bundle list panel.
    pub focus_bundle: Color,
}

impl AppTheme {
    /// Pick the animated focus-border tint for a given panel. The prompt
    /// input borrows the theme's accent since it's the "always-available"
    /// text surface — users want it visually consistent across themes.
    pub fn focus_tint(&self, focus: crate::tui::app::Focus) -> Color {
        use crate::tui::app::Focus as F;
        match focus {
            F::FileTree => self.focus_tree,
            F::Viewer => self.focus_viewer,
            F::BundleList => self.focus_bundle,
            F::Prompt => self.accent,
        }
    }
}

use registry::default_theme;

/// Gauge colour for a context-window percentage. Preserved as a free
/// function so existing call sites that do not carry a theme handle still
/// compile; reads from the default theme under the hood.
pub fn gauge_color(pct: f64) -> Color {
    if pct > 90.0 {
        default_theme().danger
    } else if pct > 75.0 {
        default_theme().hotspot
    } else if pct > 40.0 {
        default_theme().warning
    } else {
        default_theme().success
    }
}

pub fn selected_color() -> Color {
    default_theme().accent
}

pub fn dir_color() -> Color {
    default_theme().dir
}

pub fn hotspot_color() -> Color {
    default_theme().hotspot
}
