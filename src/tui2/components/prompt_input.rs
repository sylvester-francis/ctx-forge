//! Prompt input stub — bordered box with placeholder text.
//!
//! Full multi-line editor, `@` picker, and slash-command handling come in
//! Phase 2. This Phase 1 version renders the frame and placeholder so the
//! layout looks complete.

use crate::tui2::theme::Theme;
use iocraft::prelude::*;

#[derive(Default, Props)]
pub struct PromptInputProps {
    pub focused: bool,
    pub border_color: Option<Color>,
    pub scenario: String,
    pub theme: Option<Theme>,
}

#[component]
pub fn PromptInput(hooks: &mut Hooks, props: &PromptInputProps) -> impl Into<AnyElement<'static>> {
    let _ = hooks;
    let theme = props.theme.unwrap_or_else(|| {
        Theme::from_app_theme(crate::tui::theme::registry::default_theme())
    });

    let title = if props.scenario.is_empty() {
        " prompt · (no scenario) ".to_string()
    } else {
        format!(" prompt · scenario: {} ", props.scenario)
    };

    let bc = if props.focused {
        props.border_color.unwrap_or(theme.border_focused)
    } else {
        theme.border
    };

    element! {
        View(
            flex_direction: FlexDirection::Column,
            border_style: BorderStyle::Round,
            border_color: bc,
            background_color: theme.bg,
            height: 3,
        ) {
            Text(content: title.leak() as &str, color: theme.muted, weight: Weight::Bold)
            Text(content: " (press i to edit — full editor in Phase 2)", weight: Weight::Light)
        }
    }
}
