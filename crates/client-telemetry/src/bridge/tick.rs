//! Per-frame work for lab features: called once per Tick from
//! `drain_main_thread`. Idle cost: two relaxed atomic loads (C13 item 3).

use std::sync::atomic::{AtomicBool, Ordering};

use super::input::hold::{HOLDS_EXPIRED, HOLDS_RELEASED_ON_DISCONNECT};
use super::input::post::post_all;
use super::input::{hold_table, input_state};

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
    // try_lock: never block the game thread. The IO thread holds the table
    // only for a disconnect release, so a miss just waits one frame.
    let mut t = match hold_table().try_lock() {
        Ok(g) => g,
        Err(std::sync::TryLockError::Poisoned(e)) => e.into_inner(),
        Err(std::sync::TryLockError::WouldBlock) => return,
    };
    let posts = t.expire(crate::bridge::meta::mono_ms());
    if !posts.is_empty() {
        post_all(&posts);
        HOLDS_EXPIRED.fetch_add(posts.len() as u64, Ordering::Relaxed);
    }
    if t.is_empty() {
        HOLDS_ACTIVE.store(false, Ordering::Relaxed);
    }
}

/// Runs on the bridge IO thread while `holds_tick` runs on the main thread:
/// the blocking lock serialises them, and `HOLDS_ACTIVE` is cleared under it
/// so a concurrent press cannot be left active-but-forgotten.
fn holds_disconnect() {
    #[cfg(test)]
    TICK_WORK.fetch_add(1, Ordering::SeqCst);
    let mut t = hold_table().lock().unwrap_or_else(|e| e.into_inner());
    let posts = t.release_all();
    post_all(&posts);
    HOLDS_RELEASED_ON_DISCONNECT.fetch_add(posts.len() as u64, Ordering::Relaxed);
    HOLDS_ACTIVE.store(false, Ordering::Relaxed);
    drop(t);
    // The DirectInput state goes too (C5); virtual focus stays as it is.
    input_state()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .release_all();
}
fn watch_tick() {
    #[cfg(test)]
    TICK_WORK.fetch_add(1, Ordering::SeqCst);
    // AX-24
}
fn watch_disconnect() {
    #[cfg(test)]
    TICK_WORK.fetch_add(1, Ordering::SeqCst);
    // AX-24
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
