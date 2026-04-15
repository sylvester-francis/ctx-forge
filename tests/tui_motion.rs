#![cfg(feature = "tui")]

use ctxforge::tui::motion::{
    AnimCtx, Animated, Clock, Fade, Gauge, Highlight, Lerp, MockClock, MotionLevel, SystemClock,
    blend, constants, detect_motion, ease_in_cubic, ease_in_out_cubic, ease_out_cubic,
    ease_out_quad, linear,
};
use ratatui::style::Color;
use std::time::Duration;
use std::time::Instant;

#[test]
fn system_clock_returns_monotonic_instants() {
    let c = SystemClock;
    let t0 = c.now();
    std::thread::sleep(Duration::from_millis(1));
    let t1 = c.now();
    assert!(t1 > t0);
}

#[test]
fn mock_clock_starts_at_fixed_instant_and_advances() {
    let c = MockClock::new();
    let t0 = c.now();
    c.advance(Duration::from_millis(150));
    let t1 = c.now();
    assert_eq!(t1 - t0, Duration::from_millis(150));
}

#[test]
fn lerp_f32_interpolates_midpoint() {
    assert_eq!(f32::lerp(0.0, 100.0, 0.0), 0.0);
    assert_eq!(f32::lerp(0.0, 100.0, 0.5), 50.0);
    assert_eq!(f32::lerp(0.0, 100.0, 1.0), 100.0);
}

#[test]
fn lerp_f32_clamps_t_out_of_range() {
    assert_eq!(f32::lerp(0.0, 100.0, -0.5), 0.0);
    assert_eq!(f32::lerp(0.0, 100.0, 1.5), 100.0);
}

#[test]
fn lerp_rgb_color_linear_channel() {
    let from = Color::Rgb(0, 0, 0);
    let to = Color::Rgb(200, 100, 50);
    let mid = Color::lerp(from, to, 0.5);
    assert_eq!(mid, Color::Rgb(100, 50, 25));
}

#[test]
fn lerp_non_rgb_color_snaps_at_midpoint() {
    let from = Color::Cyan;
    let to = Color::Rgb(255, 255, 255);
    assert_eq!(Color::lerp(from, to, 0.3), Color::Cyan);
    assert_eq!(Color::lerp(from, to, 0.7), Color::Rgb(255, 255, 255));
}

#[test]
fn easings_are_identity_at_boundaries() {
    for ease in [
        linear,
        ease_out_cubic,
        ease_in_out_cubic,
        ease_out_quad,
        ease_in_cubic,
    ] {
        assert!((ease(0.0) - 0.0).abs() < 1e-6, "f(0) must be 0");
        assert!((ease(1.0) - 1.0).abs() < 1e-6, "f(1) must be 1");
    }
}

#[test]
fn ease_out_cubic_decelerates() {
    assert!(ease_out_cubic(0.5) > 0.5);
}

#[test]
fn blend_at_opacity_zero_returns_background() {
    let fg = Color::Rgb(255, 0, 0);
    let bg = Color::Rgb(0, 0, 0);
    assert_eq!(blend(0.0, fg, bg), bg);
}

#[test]
fn blend_at_opacity_one_returns_foreground() {
    let fg = Color::Rgb(255, 0, 0);
    let bg = Color::Rgb(0, 0, 0);
    assert_eq!(blend(1.0, fg, bg), fg);
}

#[test]
fn blend_at_opacity_half_is_midpoint() {
    let fg = Color::Rgb(200, 0, 0);
    let bg = Color::Rgb(0, 0, 100);
    assert_eq!(blend(0.5, fg, bg), Color::Rgb(100, 0, 50));
}

#[test]
fn detect_motion_respects_no_animations_env() {
    temp_env::with_vars(
        [
            ("NO_ANIMATIONS", Some("1")),
            ("COLORTERM", Some("truecolor")),
        ],
        || {
            assert_eq!(detect_motion(), MotionLevel::None);
        },
    );
}

#[test]
fn detect_motion_requires_truecolor_otherwise() {
    temp_env::with_vars(
        [
            ("NO_ANIMATIONS", None::<&str>),
            ("PROMPT_NO_ANIMATIONS", None::<&str>),
            ("COLORTERM", Some("truecolor")),
        ],
        || {
            assert_eq!(detect_motion(), MotionLevel::Full);
        },
    );
    temp_env::with_vars(
        [
            ("NO_ANIMATIONS", None::<&str>),
            ("PROMPT_NO_ANIMATIONS", None::<&str>),
            ("COLORTERM", Some("ansi")),
        ],
        || {
            assert_eq!(detect_motion(), MotionLevel::None);
        },
    );
}

#[test]
fn anim_ctx_carries_clock_and_motion() {
    let clock = MockClock::new();
    let now = clock.now();
    let ctx = AnimCtx {
        now,
        motion: MotionLevel::Full,
    };
    assert_eq!(ctx.now, now);
    assert_eq!(ctx.motion, MotionLevel::Full);
}

fn ctx_at(now: Instant, motion: MotionLevel) -> AnimCtx {
    AnimCtx { now, motion }
}

#[test]
fn animated_starts_at_initial_value() {
    let clock = MockClock::new();
    let a: Animated<f32> = Animated::new(0.0);
    assert_eq!(a.value(clock.now()), 0.0);
    assert!(!a.is_active(clock.now()));
}

#[test]
fn animated_tweens_linearly() {
    let clock = MockClock::new();
    let t0 = clock.now();
    let mut a = Animated::new(0.0_f32);
    a.set(
        100.0,
        &ctx_at(t0, MotionLevel::Full),
        Duration::from_millis(200),
        linear,
    );
    assert_eq!(a.value(t0), 0.0);
    assert!((a.value(t0 + Duration::from_millis(100)) - 50.0).abs() < 1e-3);
    assert_eq!(a.value(t0 + Duration::from_millis(200)), 100.0);
    assert_eq!(a.value(t0 + Duration::from_millis(300)), 100.0);
}

#[test]
fn animated_interrupt_tweens_from_current_value() {
    let clock = MockClock::new();
    let t0 = clock.now();
    let mut a = Animated::new(0.0_f32);

    a.set(
        100.0,
        &ctx_at(t0, MotionLevel::Full),
        Duration::from_millis(200),
        linear,
    );

    let t1 = t0 + Duration::from_millis(100);
    a.set(
        0.0,
        &ctx_at(t1, MotionLevel::Full),
        Duration::from_millis(100),
        linear,
    );

    let t_mid = t0 + Duration::from_millis(150);
    assert!((a.value(t_mid) - 25.0).abs() < 0.5);
}

#[test]
fn animated_is_active_while_tweening() {
    let clock = MockClock::new();
    let t0 = clock.now();
    let mut a = Animated::new(0.0_f32);
    a.set(
        100.0,
        &ctx_at(t0, MotionLevel::Full),
        Duration::from_millis(200),
        linear,
    );
    assert!(a.is_active(t0));
    assert!(a.is_active(t0 + Duration::from_millis(100)));
    assert!(!a.is_active(t0 + Duration::from_millis(200)));
    assert!(!a.is_active(t0 + Duration::from_millis(250)));
}

#[test]
fn animated_snap_skips_tween() {
    let clock = MockClock::new();
    let t0 = clock.now();
    let mut a = Animated::new(0.0_f32);
    a.snap(42.0);
    assert_eq!(a.value(t0), 42.0);
    assert!(!a.is_active(t0));
}

#[test]
fn animated_set_with_motion_none_snaps() {
    let clock = MockClock::new();
    let t0 = clock.now();
    let mut a = Animated::new(0.0_f32);
    a.set(
        100.0,
        &ctx_at(t0, MotionLevel::None),
        Duration::from_millis(200),
        linear,
    );
    assert_eq!(a.value(t0), 100.0);
    assert!(!a.is_active(t0));
}

#[test]
fn animated_set_with_zero_duration_snaps() {
    let clock = MockClock::new();
    let t0 = clock.now();
    let mut a = Animated::new(0.0_f32);
    a.set(
        100.0,
        &ctx_at(t0, MotionLevel::Full),
        Duration::ZERO,
        linear,
    );
    assert_eq!(a.value(t0), 100.0);
    assert!(!a.is_active(t0));
}

#[test]
fn motion_constants_are_defined() {
    assert_eq!(constants::MODAL_IN, Duration::from_millis(200));
    assert_eq!(constants::MODAL_OUT, Duration::from_millis(180));
    assert_eq!(constants::MODAL_CROSSFADE, Duration::from_millis(240));
    assert_eq!(constants::GAUGE_FILL, Duration::from_millis(320));
    assert_eq!(constants::STATUS_IN, Duration::from_millis(180));
    assert_eq!(constants::STATUS_OUT, Duration::from_millis(280));
    assert_eq!(constants::STATUS_HOLD, Duration::from_millis(2400));
    assert_eq!(constants::HIGHLIGHT_MOVE, Duration::from_millis(120));
    assert_eq!(constants::ROW_IN, Duration::from_millis(200));
    assert_eq!(constants::ROW_OUT, Duration::from_millis(160));
    assert_eq!(constants::FOCUS_BORDER, Duration::from_millis(160));
    assert_eq!(constants::TREE_EXPAND, Duration::from_millis(180));
    assert_eq!(constants::LIST_FILTER, Duration::from_millis(140));
    assert_eq!(constants::STARTUP, Duration::from_millis(260));
    assert_eq!(constants::LIST_STAGGER_STEP, Duration::from_millis(20));
    assert_eq!(constants::LIST_STAGGER_CAP_ROWS, 6);
    // Tuned down from 0.55 in v1.3 — the scenario-aware layout's unfocused
    // theme borders (DarkGray on the default palette) vanished under a
    // 0.55 dim. 0.35 still signals 'overlay active' without hiding the
    // preview / bundle summary.
    assert!((constants::BACKDROP_DIM - 0.35).abs() < 1e-6);
}

#[test]
fn fade_wrapper_shows_and_hides() {
    let clock = MockClock::new();
    let t0 = clock.now();
    let mut f = Fade::new_hidden();
    assert_eq!(f.opacity(t0), 0.0);

    let ctx = AnimCtx {
        now: t0,
        motion: MotionLevel::Full,
    };
    f.show(&ctx);
    let t_end = t0 + constants::MODAL_IN;
    assert_eq!(f.opacity(t_end), 1.0);

    let ctx_end = AnimCtx {
        now: t_end,
        motion: MotionLevel::Full,
    };
    f.hide(&ctx_end);
    let t_fin = t_end + constants::MODAL_OUT;
    assert_eq!(f.opacity(t_fin), 0.0);
}

#[test]
fn gauge_wrapper_tweens_with_gauge_fill_duration() {
    let clock = MockClock::new();
    let t0 = clock.now();
    let mut g = Gauge::new(0.0);
    let ctx = AnimCtx {
        now: t0,
        motion: MotionLevel::Full,
    };
    g.set(1000.0, &ctx);
    assert_eq!(g.current(t0), 0.0);
    assert_eq!(g.current(t0 + constants::GAUGE_FILL), 1000.0);
}

#[test]
fn highlight_wrapper_tweens_color() {
    let clock = MockClock::new();
    let t0 = clock.now();
    let from = Color::Rgb(0, 0, 0);
    let to = Color::Rgb(100, 100, 100);
    let mut h = Highlight::new(from);
    let ctx = AnimCtx {
        now: t0,
        motion: MotionLevel::Full,
    };
    h.transition_to(to, &ctx);
    assert_eq!(h.current(t0 + constants::HIGHLIGHT_MOVE), to);
}
