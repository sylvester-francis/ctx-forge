//! Search input bar — shown above the file tree when Mode::Search is active.

use crate::tui2::theme::Theme;
use iocraft::prelude::*;

pub fn render_search_bar(query: &str, theme: &Theme) -> AnyElement<'static> {
    let display = format!("{query}▏");
    element! {
        View(
            flex_direction: FlexDirection::Row,
            border_style: BorderStyle::Round,
            border_color: theme.accent,
            background_color: theme.bg,
            width: 100pct,
            height: 3,
            padding_left: 1,
            padding_right: 1,
        ) {
            MixedText(contents: vec![
                MixedTextContent::new("▍ ").color(theme.accent).weight(Weight::Bold),
                MixedTextContent::new("SEARCH  ").color(theme.accent).weight(Weight::Bold),
                MixedTextContent::new(display).color(theme.fg),
            ])
        }
    }
    .into_any()
}
