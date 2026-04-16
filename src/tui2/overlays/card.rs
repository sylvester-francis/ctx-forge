//! Reusable overlay card — dimmed full-screen backdrop + centered panel.

use crate::tui2::theme::Theme;
use iocraft::prelude::*;

pub fn render_card(
    title: &str,
    body: Vec<AnyElement<'static>>,
    theme: &Theme,
    term_w: u16,
    term_h: u16,
) -> AnyElement<'static> {
    // Scale with terminal: aim for 90% × 85% with sensible min/max. On narrow
    // terminals this uses most of the screen; on very wide terminals it's
    // capped so the card stays visually centered without stretching.
    let card_w = ((term_w as u32 * 90) / 100).clamp(30, 140);
    let card_h = ((term_h as u32 * 85) / 100).clamp(8, 50);
    let offset_left = (term_w as u32).saturating_sub(card_w) / 2;
    let offset_top = (term_h as u32).saturating_sub(card_h) / 2;

    let backdrop = element! {
        View(
            position: Position::Absolute,
            top: 0,
            left: 0,
            width: term_w as u32,
            height: term_h as u32,
            background_color: Color::Rgb { r: 0, g: 0, b: 0 },
        ) {}
    }
    .into_any();

    let title_owned = format!(" {} ", title);

    let card = element! {
        View(
            position: Position::Absolute,
            top: offset_top,
            left: offset_left,
            width: card_w,
            height: card_h,
            flex_direction: FlexDirection::Column,
            border_style: BorderStyle::Round,
            border_color: theme.accent,
            background_color: theme.bg,
            padding_left: 2,
            padding_right: 2,
            padding_top: 1,
            overflow: Overflow::Hidden,
        ) {
            MixedText(contents: vec![
                MixedTextContent::new("▍ ").color(theme.accent).weight(Weight::Bold),
                MixedTextContent::new(title_owned).color(theme.accent).weight(Weight::Bold),
            ])
            Text(content: "")
            #(body)
        }
    }
    .into_any();

    element! {
        View(
            position: Position::Absolute,
            top: 0,
            left: 0,
            width: term_w as u32,
            height: term_h as u32,
        ) {
            #(vec![backdrop, card])
        }
    }
    .into_any()
}
