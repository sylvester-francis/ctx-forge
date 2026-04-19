//! Full-screen welcome splash. Dismissed by any keypress.

use crate::tui::theme::Theme;
use iocraft::prelude::*;

pub fn render_splash(
    theme: &Theme,
    file_count: usize,
    term_w: u16,
    term_h: u16,
) -> Vec<AnyElement<'static>> {
    let card_w = if term_w < 100 {
        ((term_w as u32) * 92 / 100).max(40)
    } else {
        ((term_w as u32) * 70 / 100).clamp(60, 160)
    };
    let card_h = if term_h < 30 {
        ((term_h as u32) * 92 / 100).max(15)
    } else {
        ((term_h as u32) * 75 / 100).clamp(20, 44)
    };
    let offset_left = (term_w as u32).saturating_sub(card_w) / 2;
    let offset_top = (term_h as u32).saturating_sub(card_h) / 2;
    let compact = term_h < 30;

    let mut body: Vec<AnyElement<'static>> = Vec::new();

    if !compact {
        body.push(element! { Text(content: "") }.into_any());
    }

    // 3-row block-letter wordmark; "ctx" muted, "forge" accent.
    let logo_row1_ctx = " ▄▀▀  ▀█▀  █ █";
    let logo_row2_ctx = " █     █   ▀▄▀";
    let logo_row3_ctx = " ▀▄▄   █   █ █";

    let logo_row1_forge = "  █▀▀  ▄▀▀▄  █▀▄  ▄▀▀▄  █▀▀";
    let logo_row2_forge = "  █▀▀  █  █  ██▀  █ ▀█  █▀▀";
    let logo_row3_forge = "  █    ▀▄▄▀  █ ▀  ▀▄▄▀  █▄▄";

    for (ctx_row, forge_row) in [
        (logo_row1_ctx, logo_row1_forge),
        (logo_row2_ctx, logo_row2_forge),
        (logo_row3_ctx, logo_row3_forge),
    ] {
        body.push(
            element! {
                MixedText(align: TextAlign::Center, contents: vec![
                    MixedTextContent::new(ctx_row).color(theme.muted).weight(Weight::Bold),
                    MixedTextContent::new(forge_row).color(theme.accent).weight(Weight::Bold),
                ])
            }
            .into_any(),
        );
    }

    body.push(
        element! {
            Text(
                content: "the prompt engineer for AI coding",
                color: theme.muted,
                align: TextAlign::Center,
            )
        }
        .into_any(),
    );
    if !compact {
        body.push(element! { Text(content: "") }.into_any());
        let divider2 = "─".repeat((card_w as usize).saturating_sub(12).max(10));
        body.push(
            element! {
                Text(content: divider2.leak() as &str, color: theme.muted, weight: Weight::Light, align: TextAlign::Center)
            }
            .into_any(),
        );
    }
    body.push(element! { Text(content: "") }.into_any());

    let steps: &[(&str, &str)] = &[
        ("S", "pick a scenario"),
        ("space", "add files to bundle"),
        ("i", "write your task"),
        ("P", "preview composed prompt"),
        ("d", "deliver — copy · pipe · export"),
    ];

    // Fixed-width lines so center-align shifts the block as a unit.
    let max_action = steps.iter().map(|(_, a)| a.len()).max().unwrap_or(0);
    for (i, (key, action)) in steps.iter().enumerate() {
        body.push(
            element! {
                MixedText(align: TextAlign::Center, contents: vec![
                    MixedTextContent::new(format!("{}  ", i + 1)).color(theme.muted),
                    MixedTextContent::new(format!("{:<6}", key)).color(theme.accent).weight(Weight::Bold),
                    MixedTextContent::new(format!("{:<width$}", action, width = max_action)),
                ])
            }
            .into_any(),
        );
    }

    body.push(element! { Text(content: "") }.into_any());
    body.push(
        element! {
            MixedText(align: TextAlign::Center, contents: vec![
                MixedTextContent::new("/").color(theme.accent).weight(Weight::Bold),
                MixedTextContent::new(" commands   ").color(theme.muted),
                MixedTextContent::new("?").color(theme.accent).weight(Weight::Bold),
                MixedTextContent::new(" help   ").color(theme.muted),
                MixedTextContent::new(format!("{file_count} files")).color(theme.muted).weight(Weight::Light),
            ])
        }
        .into_any(),
    );
    body.push(element! { Text(content: "") }.into_any());
    body.push(
        element! {
            MixedText(align: TextAlign::Center, contents: vec![
                MixedTextContent::new("press any key to continue   ").color(theme.muted).weight(Weight::Light),
                MixedTextContent::new("q").color(theme.accent).weight(Weight::Bold),
                MixedTextContent::new(" quit").color(theme.muted).weight(Weight::Light),
            ])
        }
        .into_any(),
    );

    vec![
        element! {
            View(
                position: Position::Absolute,
                top: offset_top,
                left: offset_left,
                width: card_w,
                height: card_h,
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                border_style: BorderStyle::Round,
                border_color: theme.accent,
                background_color: theme.bg,
                padding_left: 4,
                padding_right: 4,
                overflow: Overflow::Hidden,
            ) {
                #(body)
            }
        }
        .into_any(),
    ]
}
