//! Prompt input renderer — multi-line text with inline cursor.
//!
//! Cursor rendering: since iocraft doesn't expose a terminal cursor API,
//! we render a visible `▏` glyph at the cursor position by splitting the
//! text there and using MixedText spans. The cursor only appears when the
//! input has focus.

use crate::tui::prompt_input::PromptInput;
use crate::tui2::theme::Theme;
use iocraft::prelude::*;

pub fn render_prompt_input(
    input: &PromptInput,
    focused: bool,
    border_color: Color,
    scenario: &str,
    theme: &Theme,
) -> AnyElement<'static> {
    let title = if scenario.is_empty() {
        " ⌥ prompt ".to_string()
    } else {
        format!(" ⌥ prompt · {} ", scenario)
    };

    let bc = if focused { border_color } else { theme.border };

    // Auto-grow 4..=10 rows: 2 borders + 1 title + N content lines.
    let content_lines = input.line_count().max(1);
    let height = (3 + content_lines as u32).clamp(4, 10);

    let body = render_lines(input, focused, theme);

    element! {
        View(
            flex_direction: FlexDirection::Column,
            border_style: BorderStyle::Round,
            border_color: bc,
            background_color: theme.bg,
            width: 100pct,
            height: height,
            padding_left: 1,
            padding_right: 1,
        ) {
            MixedText(contents: vec![
                MixedTextContent::new(title).color(theme.muted).weight(Weight::Bold),
            ])
            #(body)
        }
    }
    .into_any()
}

fn render_lines(input: &PromptInput, focused: bool, theme: &Theme) -> Vec<AnyElement<'static>> {
    let text = input.text();

    if text.is_empty() && !focused {
        return vec![
            element! {
                Text(
                    content: " press i to edit",
                    color: theme.muted,
                    weight: Weight::Light,
                )
            }
            .into_any(),
        ];
    }

    let (cursor_col, cursor_row) = input.cursor_line_column();
    let lines: Vec<&str> = text.split('\n').collect();

    lines
        .iter()
        .enumerate()
        .map(|(row, line)| {
            let owned = line.to_string();
            if focused && row == cursor_row as usize {
                let col = cursor_col as usize;
                let chars: Vec<char> = owned.chars().collect();
                let before: String = chars.iter().take(col).collect();
                let after: String = chars.iter().skip(col).collect();
                element! {
                    MixedText(contents: vec![
                        MixedTextContent::new(before),
                        MixedTextContent::new("▏").color(theme.accent).weight(Weight::Bold),
                        MixedTextContent::new(after),
                    ])
                }
                .into_any()
            } else {
                element! {
                    Text(content: owned.leak() as &str)
                }
                .into_any()
            }
        })
        .collect()
}
