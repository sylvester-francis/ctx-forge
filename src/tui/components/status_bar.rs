//! Footer — status message + keybindings hint.

use crate::tui::theme::Theme;
use iocraft::prelude::*;

pub fn render_footer(status: &str, theme: &Theme) -> AnyElement<'static> {
    let status_line = if status.is_empty() {
        " Ready".to_string()
    } else {
        format!(" {status}")
    };

    element! {
        View(
            flex_direction: FlexDirection::Column,
            background_color: theme.bg,
            height: 2,
        ) {
            Text(content: status_line.leak() as &str, color: theme.muted)
            MixedText(contents: vec![
                MixedTextContent::new(" Tab").color(theme.accent),
                MixedTextContent::new(" focus  "),
                MixedTextContent::new("j/k").color(theme.accent),
                MixedTextContent::new(" move  "),
                MixedTextContent::new("Space").color(theme.accent),
                MixedTextContent::new(" toggle  "),
                MixedTextContent::new("v").color(theme.accent),
                MixedTextContent::new(" viewer  "),
                MixedTextContent::new("d").color(theme.accent),
                MixedTextContent::new(" deliver  "),
                MixedTextContent::new("?").color(theme.accent),
                MixedTextContent::new(" help  "),
                MixedTextContent::new("q").color(theme.accent),
                MixedTextContent::new(" quit"),
            ])
        }
    }
    .into_any()
}
