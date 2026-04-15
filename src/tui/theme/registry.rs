//! Registry of available themes.

use crate::tui::theme::AppTheme;
use crate::tui::theme::palettes;

pub fn all_themes() -> &'static [AppTheme] {
    &[
        palettes::ctxforge::CTXFORGE,
        palettes::zinc::ZINC,
        palettes::tokyo_night::TOKYO_NIGHT,
        palettes::gruvbox::GRUVBOX,
    ]
}

pub fn by_name(name: &str) -> Option<&'static AppTheme> {
    all_themes().iter().find(|t| t.name == name)
}

pub fn default_theme() -> &'static AppTheme {
    &palettes::ctxforge::CTXFORGE
}
