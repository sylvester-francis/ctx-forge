#![cfg(feature = "tui")]

use ctxforge::tui::motion::{Clock, MockClock, SystemClock};
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
