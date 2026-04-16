//! Framework-agnostic animation primitives.
//!
//! Shared by both `tui` (ratatui v1) and `tui2` (iocraft v2).
//! No dependency on any TUI framework — only std types.

use std::env;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionLevel {
    Full,
    None,
}

pub fn detect_motion() -> MotionLevel {
    if env::var("NO_ANIMATIONS").is_ok() {
        return MotionLevel::None;
    }
    if env::var("PROMPT_NO_ANIMATIONS").is_ok() {
        return MotionLevel::None;
    }
    match env::var("COLORTERM").as_deref() {
        Ok("truecolor") | Ok("24bit") => MotionLevel::Full,
        _ => MotionLevel::None,
    }
}

pub trait Lerp: Copy {
    fn lerp(from: Self, to: Self, t: f32) -> Self;
}

impl Lerp for f32 {
    fn lerp(from: Self, to: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        from + (to - from) * t
    }
}

impl Lerp for u8 {
    fn lerp(from: Self, to: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        (from as f32 + (to as f32 - from as f32) * t) as u8
    }
}

pub type EasingFn = fn(f32) -> f32;

pub fn linear(t: f32) -> f32 {
    t
}

pub fn ease_out_cubic(t: f32) -> f32 {
    let inv = 1.0 - t;
    1.0 - inv * inv * inv
}

pub fn ease_in_cubic(t: f32) -> f32 {
    t * t * t
}

pub fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        let f = 2.0 * t - 2.0;
        0.5 * f * f * f + 1.0
    }
}

pub fn ease_out_quad(t: f32) -> f32 {
    1.0 - (1.0 - t) * (1.0 - t)
}

pub mod constants {
    use super::Duration;

    pub const MODAL_IN: Duration = Duration::from_millis(200);
    pub const MODAL_OUT: Duration = Duration::from_millis(180);
    pub const MODAL_CROSSFADE: Duration = Duration::from_millis(240);
    pub const GAUGE_FILL: Duration = Duration::from_millis(320);
    pub const STATUS_IN: Duration = Duration::from_millis(180);
    pub const STATUS_OUT: Duration = Duration::from_millis(280);
    pub const STATUS_HOLD: Duration = Duration::from_millis(2400);
    pub const HIGHLIGHT_MOVE: Duration = Duration::from_millis(120);
    pub const ROW_IN: Duration = Duration::from_millis(200);
    pub const ROW_OUT: Duration = Duration::from_millis(160);
    pub const FOCUS_BORDER: Duration = Duration::from_millis(160);
    pub const TREE_EXPAND: Duration = Duration::from_millis(180);
    pub const LIST_FILTER: Duration = Duration::from_millis(140);
    pub const STARTUP: Duration = Duration::from_millis(260);
    pub const LIST_STAGGER_STEP: Duration = Duration::from_millis(20);
    pub const LIST_STAGGER_CAP_ROWS: usize = 6;
    pub const BACKDROP_DIM: f32 = 0.35;
}
