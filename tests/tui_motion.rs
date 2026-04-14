#![cfg(feature = "tui")]

use ctxforge::tui::motion::{
    Clock, Lerp, MockClock, SystemClock, blend, ease_in_cubic, ease_in_out_cubic, ease_out_cubic,
    ease_out_quad, linear,
};
use ratatui::style::Color;
use std::time::Duration;

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
