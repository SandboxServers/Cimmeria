//! The last main-thread phase the client started, and when.
//!
//! A seam that begins a piece of main-thread work (a level-streaming step, for
//! one) calls [`set`]. A `client.engine.hitch` row then carries the phase and
//! its age, so a freeze is named by what the frame was doing before the gap
//! rather than by the gap alone.
//!
//! The store is two atomics and no lock: it is written on the game's main
//! thread, which must never wait. The phase is stored with `Release` after its
//! time, and read with `Acquire` before it. A reader that catches the two
//! mid-update reports an age off by one update, which is fine for a hitch.

use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};

use crate::hooks::sinks::throttle::now_ms;

/// What the main thread was doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Phase {
    /// One step of `UWorld::UpdateLevelStreamingInner`.
    LevelStreaming = 1,
}

impl Phase {
    /// The name a hitch row carries in `phase`.
    pub fn name(self) -> &'static str {
        match self {
            Self::LevelStreaming => "level_streaming",
        }
    }

    /// The phase with this stored code, or `None` for "none set yet".
    fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::LevelStreaming),
            _ => None,
        }
    }
}

/// The last phase's code; `0` means none has been set.
static PHASE: AtomicU8 = AtomicU8::new(0);
/// [`now_ms`] when the last phase was set.
static PHASE_AT_MS: AtomicU64 = AtomicU64::new(0);

/// Records that `phase` started now.
pub fn set(phase: Phase) {
    set_at(phase, now_ms());
}

/// Records that `phase` started at `at_ms` (a [`now_ms`] reading).
pub fn set_at(phase: Phase, at_ms: u64) {
    PHASE_AT_MS.store(at_ms, Ordering::Relaxed);
    PHASE.store(phase as u8, Ordering::Release);
}

/// The last phase set and how many milliseconds ago it was, measured at
/// `now` (a [`now_ms`] reading); `None` when no phase has been set.
pub fn last(now: u64) -> Option<(Phase, u64)> {
    let phase = Phase::from_code(PHASE.load(Ordering::Acquire))?;
    let at = PHASE_AT_MS.load(Ordering::Relaxed);
    Some((phase, now.saturating_sub(at)))
}
