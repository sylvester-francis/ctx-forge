//! iocraft animation hook wrapping `motion_core` primitives.
//!
//! `use_animated<T>` tweens a value toward a target over a duration using the
//! provided easing function. Backed by `use_state` + `use_future` with a 16ms
//! frame timer. Idle wake-up uses an `Arc<AtomicBool>` + 50ms poll; this can
//! be replaced with `event-listener` for zero-cost idle in a later cleanup.

use crate::motion_core::{EasingFn, Lerp};
use iocraft::hooks::{UseFuture, UseState};
use iocraft::Hooks;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const FRAME_INTERVAL: Duration = Duration::from_millis(16);
const IDLE_POLL: Duration = Duration::from_millis(50);

pub fn use_animated<T: Lerp + PartialEq + Unpin + Send + Sync + 'static>(
    hooks: &mut Hooks,
    target: T,
    duration: Duration,
    easing: EasingFn,
) -> T {
    let current = hooks.use_state(move || target);
    let mut tween = hooks.use_state(|| Option::<(T, T, Instant)>::None);
    let wake = hooks.use_state(|| Arc::new(AtomicBool::new(false)));

    let needs_start = match *tween.read() {
        Some((_, to, _)) => to != target,
        None => *current.read() != target,
    };
    if needs_start {
        *tween.write() = Some((*current.read(), target, Instant::now()));
        wake.read().store(true, Ordering::Release);
    }

    hooks.use_future({
        let mut current = current;
        let mut tween = tween;
        let wake = Arc::clone(&wake.read());
        async move {
            loop {
                let active = *tween.read();
                if let Some((from, to, start)) = active {
                    let elapsed = start.elapsed();
                    if elapsed >= duration {
                        *current.write() = to;
                        *tween.write() = None;
                    } else {
                        let t = elapsed.as_secs_f32() / duration.as_secs_f32();
                        *current.write() = T::lerp(from, to, easing(t));
                    }
                    smol::Timer::after(FRAME_INTERVAL).await;
                } else if wake.swap(false, Ordering::Acquire) {
                    continue;
                } else {
                    smol::Timer::after(IDLE_POLL).await;
                }
            }
        }
    });

    *current.read()
}
