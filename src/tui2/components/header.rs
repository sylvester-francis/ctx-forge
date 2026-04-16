//! Header bar — token gauge, scenario badge, model name.
//!
//! Uses `use_animated` to smoothly fill the gauge when token totals change.

use crate::motion_core::{constants, ease_out_quad};
use crate::tui2::motion::use_animated;
use crate::tui2::theme::Theme;
use iocraft::prelude::*;

fn format_tokens(n: usize) -> String {
    if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        format!("{n}")
    }
}

fn gauge_color(pct: f64, theme: &Theme) -> Color {
    if pct > 90.0 {
        theme.danger
    } else if pct > 75.0 {
        theme.hotspot
    } else if pct > 40.0 {
        theme.warning
    } else {
        theme.success
    }
}

#[derive(Default, Props)]
pub struct HeaderProps {
    pub total_tokens: usize,
    pub model_window: usize,
    pub model_name: String,
    pub scenario: String,
    pub theme: Option<Theme>,
}

#[component]
pub fn Header(hooks: &mut Hooks, props: &HeaderProps) -> impl Into<AnyElement<'static>> {
    let theme = props.theme.unwrap_or_else(|| {
        Theme::from_app_theme(crate::tui::theme::registry::default_theme())
    });

    let pct = if props.model_window == 0 {
        0.0
    } else {
        (props.total_tokens as f64 / props.model_window as f64) * 100.0
    };

    let ratio = (props.total_tokens as f32) / (props.model_window.max(1) as f32);
    let animated_ratio = use_animated(
        hooks,
        ratio.clamp(0.0, 1.0),
        constants::GAUGE_FILL,
        ease_out_quad,
    );

    let label = format!(
        " ctxforge │ scenario: {} │ {} │ ~{} / {} ({:.1}%) ",
        if props.scenario.is_empty() {
            "(none)"
        } else {
            props.scenario.as_str()
        },
        props.model_name,
        format_tokens(props.total_tokens),
        format_tokens(props.model_window),
        pct,
    );

    let bar_width = 40u32;
    let filled = ((animated_ratio * bar_width as f32) as u32).min(bar_width);
    let empty = bar_width - filled;
    let bar = format!(
        "{}{}",
        "█".repeat(filled as usize),
        "░".repeat(empty as usize)
    );

    let color = gauge_color(pct, &theme);

    element! {
        View(
            border_style: BorderStyle::Single,
            border_color: theme.border,
            background_color: theme.bg,
        ) {
            MixedText(contents: vec![
                MixedTextContent::new(&label),
                MixedTextContent::new(&bar).color(color),
            ])
        }
    }
}
