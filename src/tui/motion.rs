//! Animation primitives for the TUI.
//!
//! See `docs/superpowers/specs/2026-04-14-fluid-tui-design.md` for the design.
//! Gated behind the `tui` feature because it uses `ratatui::style::Color`.

use ratatui::style::Color;
use std::cell::Cell;
use std::env;
use std::rc::Rc;
use std::time::{Duration, Instant};

/// Whether animations are allowed. Detected once at app startup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionLevel {
    /// Full animation support (truecolor terminal, user hasn't opted out).
    Full,
    /// Animations snap to target. No tweening.
    None,
}

/// Context passed to animation calls. Threads the clock and motion level
/// through the app without globals.
#[derive(Clone, Copy)]
pub struct AnimCtx {
    pub now: Instant,
    pub motion: MotionLevel,
}

/// Detect the motion level from env vars.
///
/// Rules:
/// - `NO_ANIMATIONS` or `PROMPT_NO_ANIMATIONS` set → `None`.
/// - `COLORTERM = truecolor` or `24bit` → `Full`.
/// - Everything else → `None`.
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

/// A tween in progress.
struct Tween<T> {
    from: T,
    to: T,
    start: Instant,
    duration: Duration,
    easing: EasingFn,
}

/// A value that can animate between targets over time.
///
/// Invariant: calling `set()` while a tween is in flight restarts the tween
/// from the current interpolated value, not from the previous target. This
/// makes interruption feel continuous rather than jumpy.
pub struct Animated<T: Lerp> {
    current: T,
    tween: Option<Tween<T>>,
}

impl<T: Lerp> Animated<T> {
    pub fn new(initial: T) -> Self {
        Self {
            current: initial,
            tween: None,
        }
    }

    /// Return the interpolated value at `now`. If no tween is active, returns
    /// the stored `current` value. If the tween has finished, returns the
    /// target.
    pub fn value(&self, now: Instant) -> T {
        match &self.tween {
            None => self.current,
            Some(tw) => {
                let elapsed = now.saturating_duration_since(tw.start);
                if elapsed >= tw.duration {
                    tw.to
                } else {
                    let t = elapsed.as_secs_f32() / tw.duration.as_secs_f32();
                    T::lerp(tw.from, tw.to, (tw.easing)(t))
                }
            }
        }
    }

    /// Start a new tween toward `target`. Snaps instantly if motion is None
    /// or duration is zero. If a tween is in flight, the new tween starts
    /// from the current interpolated value (the interrupt invariant).
    pub fn set(&mut self, target: T, ctx: &AnimCtx, duration: Duration, easing: EasingFn) {
        if ctx.motion == MotionLevel::None || duration.is_zero() {
            self.snap(target);
            return;
        }
        let from = self.value(ctx.now);
        self.current = from;
        self.tween = Some(Tween {
            from,
            to: target,
            start: ctx.now,
            duration,
            easing,
        });
    }

    /// Snap to target with no animation. Clears any in-flight tween.
    pub fn snap(&mut self, target: T) {
        self.current = target;
        self.tween = None;
    }

    /// True iff a tween is in flight at `now`.
    pub fn is_active(&self, now: Instant) -> bool {
        match &self.tween {
            None => false,
            Some(tw) => now.saturating_duration_since(tw.start) < tw.duration,
        }
    }
}

/// Signature for easing functions. Maps `t` in `[0, 1]` to eased output.
pub type EasingFn = fn(f32) -> f32;

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
    pub const BACKDROP_DIM: f32 = 0.55;
}

/// Opacity animation, 0.0..1.0.
pub struct Fade(Animated<f32>);

impl Fade {
    pub fn new_hidden() -> Self {
        Self(Animated::new(0.0))
    }
    pub fn new_shown() -> Self {
        Self(Animated::new(1.0))
    }
    pub fn opacity(&self, now: Instant) -> f32 {
        self.0.value(now)
    }
    pub fn show(&mut self, ctx: &AnimCtx) {
        self.0.set(1.0, ctx, constants::MODAL_IN, ease_out_cubic);
    }
    pub fn hide(&mut self, ctx: &AnimCtx) {
        self.0.set(0.0, ctx, constants::MODAL_OUT, ease_in_cubic);
    }
    pub fn set_over(&mut self, target: f32, duration: Duration, easing: EasingFn, ctx: &AnimCtx) {
        self.0.set(target, ctx, duration, easing);
    }
    pub fn snap(&mut self, target: f32) {
        self.0.snap(target);
    }
    pub fn is_active(&self, now: Instant) -> bool {
        self.0.is_active(now)
    }
}

/// Numeric fill animation (e.g. token gauge).
pub struct Gauge(Animated<f32>);

impl Gauge {
    pub fn new(initial: f32) -> Self {
        Self(Animated::new(initial))
    }
    pub fn current(&self, now: Instant) -> f32 {
        self.0.value(now)
    }
    pub fn set(&mut self, target: f32, ctx: &AnimCtx) {
        self.0
            .set(target, ctx, constants::GAUGE_FILL, ease_out_quad);
    }
    pub fn snap(&mut self, target: f32) {
        self.0.snap(target);
    }
    pub fn is_active(&self, now: Instant) -> bool {
        self.0.is_active(now)
    }
}

/// Color highlight animation (cursor bg, focus border).
pub struct Highlight(Animated<Color>);

impl Highlight {
    pub fn new(initial: Color) -> Self {
        Self(Animated::new(initial))
    }
    pub fn current(&self, now: Instant) -> Color {
        self.0.value(now)
    }
    pub fn transition_to(&mut self, target: Color, ctx: &AnimCtx) {
        self.0
            .set(target, ctx, constants::HIGHLIGHT_MOVE, ease_out_cubic);
    }
    pub fn transition_to_over(
        &mut self,
        target: Color,
        duration: Duration,
        easing: EasingFn,
        ctx: &AnimCtx,
    ) {
        self.0.set(target, ctx, duration, easing);
    }
    pub fn snap(&mut self, target: Color) {
        self.0.snap(target);
    }
    pub fn is_active(&self, now: Instant) -> bool {
        self.0.is_active(now)
    }
}

/// Cell-offset slide animation. Truncated to whole cells at read time.
pub struct Slide(Animated<f32>);

impl Slide {
    pub fn new(initial: f32) -> Self {
        Self(Animated::new(initial))
    }
    pub fn current_cells(&self, now: Instant) -> i16 {
        self.0.value(now) as i16
    }
    pub fn current(&self, now: Instant) -> f32 {
        self.0.value(now)
    }
    pub fn set(&mut self, target: f32, duration: Duration, easing: EasingFn, ctx: &AnimCtx) {
        self.0.set(target, ctx, duration, easing);
    }
    pub fn snap(&mut self, target: f32) {
        self.0.snap(target);
    }
    pub fn is_active(&self, now: Instant) -> bool {
        self.0.is_active(now)
    }
}

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
