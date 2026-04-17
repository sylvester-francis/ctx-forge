//! Full-screen welcome splash — first thing users see on launch.
//! Dismissed by any keypress, transitions to the main TUI.

use crate::tui::theme::Theme;
use iocraft::prelude::*;

pub fn render_splash(theme: &Theme, file_count: usize, term_w: u16, term_h: u16) -> AnyElement<'static> {
    let mut rows: Vec<AnyElement<'static>> = Vec::new();

    // Vertical centering: push empty rows to center the content block
    let content_height = 22; // approximate rows of content
    let top_pad = (term_h as usize).saturating_sub(content_height) / 2;
    for _ in 0..top_pad {
        rows.push(element! { Text(content: "") }.into_any());
    }

    // ─── Wordmark (large, centered) ───────────────────────────
    rows.push(
        element! {
            MixedText(align: TextAlign::Center, contents: vec![
                MixedTextContent::new("───────────── ").color(theme.muted).weight(Weight::Light),
                MixedTextContent::new("⚒").color(theme.accent).weight(Weight::Bold),
                MixedTextContent::new(" ─────────────").color(theme.muted).weight(Weight::Light),
            ])
        }
        .into_any(),
    );
    rows.push(element! { Text(content: "") }.into_any());
    rows.push(
        element! {
            Text(content: "c  t  x  f  o  r  g  e", color: theme.accent, weight: Weight::Bold, align: TextAlign::Center)
        }
        .into_any(),
    );
    rows.push(element! { Text(content: "") }.into_any());
    rows.push(
        element! {
            Text(content: "the prompt engineer for AI coding", color: theme.muted, align: TextAlign::Center)
        }
        .into_any(),
    );
    rows.push(element! { Text(content: "") }.into_any());
    rows.push(
        element! {
            MixedText(align: TextAlign::Center, contents: vec![
                MixedTextContent::new("───────────────────────────────").color(theme.muted).weight(Weight::Light),
            ])
        }
        .into_any(),
    );
    rows.push(element! { Text(content: "") }.into_any());
    rows.push(element! { Text(content: "") }.into_any());

    // ─── Quick start steps ────────────────────────────────────
    let steps: &[(&str, &str)] = &[
        ("S", "pick a scenario"),
        ("space", "add files to bundle"),
        ("i", "write your task"),
        ("P", "preview the composed prompt"),
        ("d", "deliver — copy · pipe · export"),
    ];

    for (i, (key, action)) in steps.iter().enumerate() {
        rows.push(
            element! {
                MixedText(align: TextAlign::Center, contents: vec![
                    MixedTextContent::new(format!("{}  ", i + 1)).color(theme.muted),
                    MixedTextContent::new(format!("{key:<6}")).color(theme.accent).weight(Weight::Bold),
                    MixedTextContent::new(*action),
                ])
            }
            .into_any(),
        );
    }

    rows.push(element! { Text(content: "") }.into_any());
    rows.push(element! { Text(content: "") }.into_any());

    // ─── Footer hint ──────────────────────────────────────────
    let file_hint = format!("{file_count} files in project");
    rows.push(
        element! {
            Text(content: file_hint.leak() as &str, color: theme.muted, weight: Weight::Light, align: TextAlign::Center)
        }
        .into_any(),
    );
    rows.push(element! { Text(content: "") }.into_any());
    rows.push(
        element! {
            MixedText(align: TextAlign::Center, contents: vec![
                MixedTextContent::new("/").color(theme.accent).weight(Weight::Bold),
                MixedTextContent::new(" commands   ").color(theme.muted),
                MixedTextContent::new("?").color(theme.accent).weight(Weight::Bold),
                MixedTextContent::new(" help").color(theme.muted),
            ])
        }
        .into_any(),
    );
    rows.push(element! { Text(content: "") }.into_any());
    rows.push(
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

    element! {
        View(
            flex_direction: FlexDirection::Column,
            background_color: theme.bg,
            width: term_w as u32,
            height: term_h as u32,
        ) {
            #(rows)
        }
    }
    .into_any()
}
