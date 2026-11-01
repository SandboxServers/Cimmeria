//! Per-frame work for lab features: called once per Tick from
//! `drain_main_thread`. Idle cost: two relaxed atomic loads (C13 item 3).

use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(test)]
use std::sync::atomic::AtomicU64;

/// Set while any key hold is live (AX-21).
pub(crate) static HOLDS_ACTIVE: AtomicBool = AtomicBool::new(false);
/// Set while a watch is running (AX-24).
pub(crate) static WATCH_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Counts calls into the four stubs; later packets keep the increment at
/// the top of their bodies so tests can see work happen.
#[cfg(test)]
pub(crate) static TICK_WORK: AtomicU64 = AtomicU64::new(0);

pub(crate) fn on_tick() {
    if HOLDS_ACTIVE.load(Ordering::Relaxed) {
        holds_tick();
    }
    if WATCH_ACTIVE.load(Ordering::Relaxed) {
        watch_tick();
    }
}

/// Called when the authenticated daemon connection ends.
pub(crate) fn on_disconnect() {
    holds_disconnect();
    watch_disconnect();
}

fn holds_tick() {
    #[cfg(test)]
    TICK_WORK.fetch_add(1, Ordering::SeqCst);
    // AX-21
}
fn holds_disconnect() {
    #[cfg(test)]
    TICK_WORK.fetch_add(1, Ordering::SeqCst);
    // AX-21
}
fn watch_tick() {
    #[cfg(test)]
    TICK_WORK.fetch_add(1, Ordering::SeqCst);
    super::watch::tick();
}
fn watch_disconnect() {
    #[cfg(test)]
    TICK_WORK.fetch_add(1, Ordering::SeqCst);
    super::watch::disconnect();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_tick_does_no_work() {
        let _g = crate::bridge::meta::tests::SERIAL
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        HOLDS_ACTIVE.store(false, Ordering::SeqCst);
        WATCH_ACTIVE.store(false, Ordering::SeqCst);
        let before = TICK_WORK.load(Ordering::SeqCst);
        for _ in 0..100 {
            on_tick();
        }
        assert_eq!(TICK_WORK.load(Ordering::SeqCst), before);
    }
}
