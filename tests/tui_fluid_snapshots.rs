//! Snapshot tests for fluid-TUI rendering at key animation frames.
//!
//! These exercise the render pipeline end-to-end with a `TestBackend`, at
//! mocked clock positions (start / mid-animation / settled) for overlay
//! transitions and the backdrop dim.
#![cfg(feature = "tui")]

use ctxforge::paths::CtxforgeRoot;
use ctxforge::tui::app::App;
use ctxforge::tui::mode::Mode;
use ctxforge::tui::motion::{Clock, MockClock, MotionLevel};
use ctxforge::tui::ui;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use std::time::Duration;
use tempfile::TempDir;

fn test_app_with_clock() -> (App, MockClock, TempDir) {
    let tmp = TempDir::new().unwrap();
    let root = CtxforgeRoot::find_or_create(tmp.path()).unwrap();
    let clock = MockClock::new();
    let mut app = App::new(root);
    app.clock = Box::new(clock.clone());
    app.motion = MotionLevel::Full;
    // Snap any animations that were kicked off in `new` (e.g. startup fade)
    // so test snapshots don't depend on startup timing.
    app.startup_fade.snap(1.0);
    (app, clock, tmp)
}

#[test]
fn mode_transition_lifecycle_advances_cleanly() {
    let (mut app, clock, _tmp) = test_app_with_clock();
    let backend = TestBackend::new(120, 40);
    let mut term = Terminal::new(backend).unwrap();

    // t=0: Normal, no dim.
    term.draw(|f| ui::draw(f, &app)).unwrap();
    assert_eq!(app.backdrop_dim.opacity(clock.now()), 0.0);

    // Open palette — triggers backdrop dim fade-in and modal fade-in.
    app.set_mode(Mode::CommandPalette {
        query: String::new(),
        cursor: 0,
    });
    assert!(app.has_active_animations());

    // Mid-fade: backdrop opacity should be rising but not yet at full.
    clock.advance(Duration::from_millis(100));
    let mid_dim = app.backdrop_dim.opacity(clock.now());
    assert!(mid_dim > 0.0 && mid_dim < 0.55);
    term.draw(|f| ui::draw(f, &app)).unwrap();

    // Past the transition duration — animations settled.
    clock.advance(Duration::from_millis(300));
    app.cleanup_finished_animations();
    assert_eq!(
        app.backdrop_dim.opacity(clock.now()),
        ctxforge::tui::motion::constants::BACKDROP_DIM
    );
    assert!(!app.has_active_animations());
    term.draw(|f| ui::draw(f, &app)).unwrap();
}

#[test]
fn overlay_to_overlay_triggers_crossfade() {
    let (mut app, clock, _tmp) = test_app_with_clock();
    let backend = TestBackend::new(120, 40);
    let mut term = Terminal::new(backend).unwrap();

    // Open first overlay, let it settle.
    app.set_mode(Mode::CommandPalette {
        query: String::new(),
        cursor: 0,
    });
    clock.advance(Duration::from_millis(300));
    app.cleanup_finished_animations();

    // Switch to second overlay — cross-fade starts.
    app.set_mode(Mode::Help);
    let t = app.mode_transition.as_ref().unwrap();
    assert_eq!(
        t.duration,
        ctxforge::tui::motion::constants::MODAL_CROSSFADE
    );

    // Mid cross-fade: both outgoing and incoming overlays have partial opacity.
    clock.advance(Duration::from_millis(120));
    let outgoing = app.outgoing_overlay_opacity();
    let incoming = app.incoming_overlay_opacity();
    assert!(outgoing > 0.0 && outgoing < 1.0);
    assert!(incoming > 0.0 && incoming < 1.0);
    term.draw(|f| ui::draw(f, &app)).unwrap();

    // Past the crossfade: outgoing is gone.
    clock.advance(Duration::from_millis(200));
    app.cleanup_finished_animations();
    assert_eq!(app.outgoing_overlay_opacity(), 0.0);
    assert_eq!(app.incoming_overlay_opacity(), 1.0);
}

#[test]
fn token_gauge_smooths_to_new_total() {
    let (mut app, clock, _tmp) = test_app_with_clock();
    let start = app.token_gauge.current(clock.now());
    app.token_gauge.set(5000.0, &app.anim_ctx());

    // Mid-tween.
    clock.advance(Duration::from_millis(160));
    let mid = app.token_gauge.current(clock.now());
    assert!(mid > start && mid < 5000.0);

    // Past duration.
    clock.advance(Duration::from_millis(400));
    assert_eq!(app.token_gauge.current(clock.now()), 5000.0);
}

#[test]
fn status_fade_cycles_in_hold_out() {
    use ctxforge::tui::motion::constants;
    let (mut app, clock, _tmp) = test_app_with_clock();

    app.set_status("hello");
    assert_eq!(app.status_fade.opacity(clock.now()), 0.0);

    // After fade-in duration: opacity at 1.0.
    clock.advance(constants::STATUS_IN);
    assert_eq!(app.status_fade.opacity(clock.now()), 1.0);

    // Still visible during hold.
    clock.advance(constants::STATUS_HOLD - Duration::from_millis(100));
    app.tick_status_fade();
    assert!(app.status_fade.opacity(clock.now()) > 0.9);

    // Past fade-out: back to 0.
    clock.advance(Duration::from_millis(100));
    app.tick_status_fade();
    clock.advance(constants::STATUS_OUT);
    assert_eq!(app.status_fade.opacity(clock.now()), 0.0);
}

#[test]
fn has_active_animations_returns_false_after_everything_settles() {
    let (mut app, clock, _tmp) = test_app_with_clock();

    app.set_mode(Mode::Help);
    app.set_status("something");
    app.token_gauge.set(1000.0, &app.anim_ctx());

    assert!(app.has_active_animations());

    // Advance past the longest single animation (status fade-in + hold = 2580ms)
    // and call tick to trigger the fade-out.
    clock.advance(Duration::from_millis(2600));
    app.cleanup_finished_animations();
    app.tick_status_fade();

    // Now advance past the fade-out + a safety margin.
    clock.advance(Duration::from_millis(500));
    app.cleanup_finished_animations();
    app.tick_status_fade();

    assert!(!app.has_active_animations());
}
