#![cfg(feature = "tui")]

use ctxforge::tui::motion::{Clock, Lerp, MockClock, SystemClock};
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
