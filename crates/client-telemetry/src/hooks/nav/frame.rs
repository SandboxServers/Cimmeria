//! The per-tick frame: the monotonic clock, the controller pointer cache, and
//! the snapshot the input and pose steps read.
//!
//! The scan for the controller is the costly part, so a failed scan is not
//! repeated for [`PC_RESCAN_MS`]. A cached pointer is reused while its vtable
//! still matches.

use std::sync::OnceLock;
use std::time::Instant;

use super::controller::ControllerState;
use crate::hooks::camera::layout::PLAYER_CONTROLLER_VTABLE_VA;
use crate::hooks::camera::read::find_player_controller;
use crate::hooks::entity_trace::map::Mem;

/// How long a failed controller scan waits before it scans again.
pub const PC_RESCAN_MS: u64 = 1000;
/// The longest wait between scans: the wait doubles from [`PC_RESCAN_MS`] to this.
pub const PC_RESCAN_MAX_MS: u64 = 5000;

/// What one tick knows about the input: the clock and the controller's words.
/// Whether the UI owns the mouse is read lazily, only for a button edge.
pub struct NavFrame {
    /// Milliseconds since the first tick ([`now_ms`]).
    pub now_ms: u64,
    /// The controller's input words for this tick.
    pub ctrl: ControllerState,
}

/// The cached controller pointer (`0` when none), and the earliest time the
/// next scan may run.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PcCache {
    /// The controller's address, or `0`.
    pub ptr: u32,
    /// The earliest `now_ms` at which a scan runs again.
    pub next_scan_ms: u64,
    /// The current wait after a failed scan; doubles up to [`PC_RESCAN_MAX_MS`].
    pub backoff_ms: u64,
}

/// The process's monotonic clock, in milliseconds since the first call.
static START: OnceLock<Instant> = OnceLock::new();

/// Milliseconds since the first call to this function.
pub fn now_ms() -> u64 {
    START.get_or_init(Instant::now).elapsed().as_millis() as u64
}

/// Is `ptr` non-null and does the word at `ptr` equal the controller vtable?
fn has_controller_vtable(mem: &impl Mem, ptr: u32) -> bool {
    ptr != 0 && mem.u32_at(ptr) == Some(PLAYER_CONTROLLER_VTABLE_VA)
}

/// The controller's address: the cached one while its vtable still matches,
/// else a fresh scan. A failed scan is stored as `0` and holds off the next
/// scan for [`PC_RESCAN_MS`].
pub fn controller_ptr(mem: &impl Mem, now_ms: u64, cache: &mut PcCache) -> Option<u32> {
    if has_controller_vtable(mem, cache.ptr) {
        return Some(cache.ptr);
    }
    if now_ms < cache.next_scan_ms {
        return None;
    }
    match find_player_controller(mem) {
        Some(pc) => {
            cache.ptr = pc;
            cache.backoff_ms = 0;
            Some(pc)
        }
        None => {
            cache.ptr = 0;
            // Back off while there is no controller (loading, character
            // select): the scan reads the whole Actors array.
            cache.backoff_ms = (cache.backoff_ms * 2).clamp(PC_RESCAN_MS, PC_RESCAN_MAX_MS);
            cache.next_scan_ms = now_ms + cache.backoff_ms;
            None
        }
    }
}

/// Remembers the controller pointer seen on the last tick, so the per-session
/// state (button tracker, mode/hover baselines, pose clock) can be re-baselined
/// when the controller goes away or is replaced (zone change, character switch).
#[derive(Debug, Default)]
pub struct PcWatch {
    last: u32,
}

impl PcWatch {
    /// A watch that has seen no controller. `const`, so it can seed a `static`.
    pub const fn new() -> Self {
        Self { last: 0 }
    }

    /// Notes this tick's controller (`None` for none). True when it differs
    /// from last tick's, which includes the controller going away.
    pub fn changed(&mut self, pc: Option<u32>) -> bool {
        let now = pc.unwrap_or(0);
        let changed = now != self.last;
        self.last = now;
        changed
    }
}
