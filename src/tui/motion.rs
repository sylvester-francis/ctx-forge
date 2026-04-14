//! Animation primitives for the TUI.
//!
//! See `docs/superpowers/specs/2026-04-14-fluid-tui-design.md` for the design.
//! Gated behind the `tui` feature because it uses `ratatui::style::Color`.

use ratatui::style::Color;
use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

/// Interpolation between two values. `t` is clamped to `[0.0, 1.0]`.
pub trait Lerp: Copy {
    fn lerp(from: Self, to: Self, t: f32) -> Self;
}

impl Lerp for f32 {
    fn lerp(from: Self, to: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        from + (to - from) * t
    }
}

/// Signature for easing functions. Maps `t` in `[0, 1]` to eased output.
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

/// Blend `fg` toward `bg` by `(1 - opacity)`. At opacity = 1.0 returns `fg`,
/// at 0.0 returns `bg`. For non-RGB colors the blend snaps at midpoint
/// (same rule as `Color::lerp`).
pub fn blend(opacity: f32, fg: Color, bg: Color) -> Color {
    Color::lerp(bg, fg, opacity.clamp(0.0, 1.0))
}

impl Lerp for Color {
    /// RGB-space linear interpolation. Non-RGB endpoints (named palette
    /// colors like `Color::Cyan`) snap at t = 0.5 — there's no sensible
    /// interpolation into a palette color.
    fn lerp(from: Self, to: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        match (from, to) {
            (Color::Rgb(fr, fg, fb), Color::Rgb(tr, tg, tb)) => Color::Rgb(
                (fr as f32 + (tr as f32 - fr as f32) * t) as u8,
                (fg as f32 + (tg as f32 - fg as f32) * t) as u8,
                (fb as f32 + (tb as f32 - fb as f32) * t) as u8,
            ),
            _ => {
                if t < 0.5 {
                    from
                } else {
                    to
                }
            }
        }
    }
}

/// Abstracted time source. Production uses `SystemClock`; tests use `MockClock`
/// to advance time deterministically.
pub trait Clock {
    fn now(&self) -> Instant;
}

/// Real wall-clock time.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

/// Controllable clock for tests. Starts at a fixed `Instant` captured at
/// construction and only advances when `advance()` is called. Cloneable so
/// test helpers can hold a reference while the app owns a boxed trait object.
#[derive(Clone)]
pub struct MockClock {
    now: Rc<Cell<Instant>>,
}

impl MockClock {
    pub fn new() -> Self {
        Self {
            now: Rc::new(Cell::new(Instant::now())),
        }
    }

    pub fn advance(&self, dur: Duration) {
        self.now.set(self.now.get() + dur);
    }
}

impl Default for MockClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for MockClock {
    fn now(&self) -> Instant {
        self.now.get()
    }
}
