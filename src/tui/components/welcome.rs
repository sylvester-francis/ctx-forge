//! Full-screen welcome splash — first thing users see on launch.
//! Dismissed by any keypress, transitions to the main TUI.

use crate::tui::theme::Theme;
use iocraft::prelude::*;

pub fn render_splash(
    theme: &Theme,
    file_count: usize,
    term_w: u16,
    term_h: u16,
) -> Vec<AnyElement<'static>> {
    // The splash is a centered card inside a full-screen backdrop.
    // Card fills ~70% width and uses vertical centering via top padding.
    let card_w = ((term_w as u32) * 70 / 100).clamp(50, 160);
    let card_h = ((term_h as u32) * 70 / 100).clamp(18, 40);
    let offset_left = (term_w as u32).saturating_sub(card_w) / 2;
    let offset_top = (term_h as u32).saturating_sub(card_h) / 2;

    let mut body: Vec<AnyElement<'static>> = Vec::new();

    // ─── Wordmark — large spaced letters ──────────────────────
    body.push(element! { Text(content: "") }.into_any());
    body.push(element! { Text(content: "") }.into_any());

    // Block-letter wordmark — 3 rows tall, OpenCode-style pixel font.
    // "ctx" in muted, "forge" in accent for visual contrast.
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

    body.push(element! { Text(content: "") }.into_any());
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
    body.push(element! { Text(content: "") }.into_any());

    let divider2 = "─".repeat((card_w as usize).saturating_sub(8).max(10));
    body.push(
        element! {
            Text(content: divider2.leak() as &str, color: theme.muted, weight: Weight::Light, align: TextAlign::Center)
        }
        .into_any(),
    );
    body.push(element! { Text(content: "") }.into_any());

    // ─── Quick start ──────────────────────────────────────────
    body.push(
        element! {
            Text(content: "QUICK START", color: theme.accent, weight: Weight::Bold, align: TextAlign::Center)
        }
        .into_any(),
    );
    body.push(element! { Text(content: "") }.into_any());

    let steps: &[(&str, &str)] = &[
        ("  S    ", "pick a scenario"),
        ("  space", "add files to bundle"),
        ("  i    ", "write your task"),
        ("  P    ", "preview the composed prompt"),
        ("  d    ", "deliver — copy · pipe · export"),
    ];

    for (i, (key, action)) in steps.iter().enumerate() {
        body.push(
            element! {
                MixedText(align: TextAlign::Center, contents: vec![
                    MixedTextContent::new(format!(" {}  ", i + 1)).color(theme.muted),
                    MixedTextContent::new(*key).color(theme.accent).weight(Weight::Bold),
                    MixedTextContent::new(format!("  {action}")),
                ])
            }
            .into_any(),
        );
    }

    body.push(element! { Text(content: "") }.into_any());

    let file_hint = format!("{file_count} files in project");
    body.push(
        element! {
            Text(content: file_hint.leak() as &str, color: theme.muted, weight: Weight::Light, align: TextAlign::Center)
        }
        .into_any(),
    );
    body.push(element! { Text(content: "") }.into_any());
    body.push(
        element! {
            MixedText(align: TextAlign::Center, contents: vec![
                MixedTextContent::new("/").color(theme.accent).weight(Weight::Bold),
                MixedTextContent::new(" commands     ").color(theme.muted),
                MixedTextContent::new("?").color(theme.accent).weight(Weight::Bold),
                MixedTextContent::new(" help").color(theme.muted),
            ])
        }
        .into_any(),
    );
    body.push(element! { Text(content: "") }.into_any());
    body.push(
        element! {
            Text(
                content: "press any key to continue",
                color: theme.muted,
                weight: Weight::Light,
                align: TextAlign::Center,
            )
        }
        .into_any(),
    );

    // Wrap body in a positioned card
    vec![
        element! {
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
                padding_left: 4,
                padding_right: 4,
                padding_top: 1,
                overflow: Overflow::Hidden,
            ) {
                #(body)
            }
        }
        .into_any(),
    ]
}
