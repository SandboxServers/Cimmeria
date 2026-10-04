//! The client `SequenceManager`'s silent drops of a server `onSequence`.
//!
//! | Function | Address | Signature |
//! |---|---|---|
//! | `SequenceManager` `Event_Cache_ElementReady` handler | `0x00d06f30` | `thiscall(this, evt, arg)`, `ret 8` |
//! | play step (view-distance cull, then instantiate) | `0x00d06dd0` | `thiscall(this, data, request, source_entity)`, `ret 0xc` |
//! | Kismet sequence instantiate | `0x00d067e0` | `thiscall(this, out_instance*, name, pawn)`, `ret 0xc` |
//!
//! The ready handler's drops are worked out from memory before the
//! original runs (`entity_trace::sequences::plan_ready`), because the
//! original erases the requests it walks. The play step's cull cannot be
//! recomputed without calling game code (the distance is
//! `BW__unknown_00d00c10` against a camera cvar), so it is observed
//! instead: the play step's only early return skips the instantiate call,
//! and the instantiate hook records, for the play step that called it,
//! whether it ran and what it produced. The play step is also called from
//! the `Event_AppearanceJob_Completed` path (`0x00d055f0`), which the
//! event reports as `stage = appearance_ready`.
//!
//! The first drop site, the `Event_NetIn_onSequence` handler
//! (`0x00d05790`), is hooked in [`super::sequence_net_in`] (AB-C5): it
//! reads the event's fields through the game's own getters instead of the
//! property tree.
//!
//! A play step that instantiates is reported as `client.ability.shown`
//! `kind = sequence_played`.
//!
//! Event: `client.sequence.dropped` (`info`; `debug` for the view-distance
//! cull), throttled per (path, Source entity): burst 8, then 4 a second.

use std::cell::Cell;
use std::ffi::c_void;
use std::sync::OnceLock;

use super::entity_lifecycle::guarded;
use crate::hooks::emit::emit;
use crate::hooks::entity_trace::{
    self as trace,
    map::{LiveMem, Mem},
    sequences::{self, layout, SequenceDrop},
};

pub(super) const ADDR_CACHE_READY: usize = 0x00d0_6f30;
pub(super) const ADDR_PLAY: usize = 0x00d0_6dd0;
pub(super) const ADDR_INSTANTIATE: usize = 0x00d0_67e0;

static CACHE_READY_TRAMPOLINE: OnceLock<usize> = OnceLock::new();
static PLAY_TRAMPOLINE: OnceLock<usize> = OnceLock::new();
static INSTANTIATE_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

const TARGET: &str = "client.sequence.dropped";

pub(super) unsafe fn install_all(producer: &crate::queue::Producer) {
    let hooks: [(&str, usize, *mut c_void, &OnceLock<usize>); 3] = [
        (
            "sequence_cache_ready",
            ADDR_CACHE_READY,
            cache_ready_detour as *mut c_void,
            &CACHE_READY_TRAMPOLINE,
        ),
        (
            "sequence_play",
            ADDR_PLAY,
            play_detour as *mut c_void,
            &PLAY_TRAMPOLINE,
        ),
        (
            "sequence_instantiate",
            ADDR_INSTANTIATE,
            instantiate_detour as *mut c_void,
            &INSTANTIATE_TRAMPOLINE,
        ),
    ];
    for (name, addr, detour, slot) in hooks {
        super::install_one(producer, name, addr, detour, slot);
    }
}

/// What the instantiate call did inside the current play step.
#[derive(Debug, Clone, Copy, Default)]
struct PlayProbe {
    requested: bool,
    instance: u32,
}

thread_local! {
    /// Set while the ready handler runs, so a nested play step knows its
    /// stage.
    static IN_CACHE_READY: Cell<bool> = const { Cell::new(false) };
    /// Set while a play step runs.
    static PLAY: Cell<Option<PlayProbe>> = const { Cell::new(None) };
}

/// Restores a thread-local on drop, including when a C++ exception
/// unwinds through the detour.
struct Restore<T: Copy + 'static> {
    key: &'static std::thread::LocalKey<Cell<T>>,
    prev: T,
}

impl<T: Copy + 'static> Restore<T> {
    fn set(key: &'static std::thread::LocalKey<Cell<T>>, v: T) -> Self {
        Self {
            key,
            prev: key.with(|c| c.replace(v)),
        }
    }
}

impl<T: Copy + 'static> Drop for Restore<T> {
    fn drop(&mut self) {
        let prev = self.prev;
        self.key.with(|c| c.set(prev));
    }
}

/// The current `_time64`, low word, as the game stamps its requests.
fn now_secs() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as u32)
        .unwrap_or(0)
}

/// Emit one drop through the per-(path, Source) throttle.
pub(super) fn report(drop: &SequenceDrop, stage: &'static str) {
    let key = format!("{TARGET}:{}", drop.path.as_str());
    let decision = trace::throttle(&key, drop.ids.source_id.unwrap_or(-1));
    if let Some(f) = trace::with_suppressed(drop.fields(stage), decision) {
        emit(TARGET, drop.path.level(), f);
    }
}

// ---------------------------------------------------------------------

type CacheReadyFn = unsafe extern "thiscall-unwind" fn(*mut c_void, *mut c_void, u32);

/// `Event_Cache_ElementReady(evt, arg)`.
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn cache_ready_detour(
    this: *mut c_void,
    evt: *mut c_void,
    arg: u32,
) {
    let Some(&t) = CACHE_READY_TRAMPOLINE.get() else {
        return;
    };
    let original: CacheReadyFn = unsafe { std::mem::transmute(t) };
    let drops = guarded(|| sequences::plan_ready(&LiveMem, this as u32, evt as u32, now_secs()))
        .unwrap_or_default();
    {
        let _stage = Restore::set(&IN_CACHE_READY, true);
        original(this, evt, arg);
    }
    guarded(|| {
        for d in &drops {
            report(d, "cache_ready");
        }
    });
}

type PlayFn =
    unsafe extern "thiscall-unwind" fn(*mut c_void, *mut c_void, *mut c_void, *mut c_void);

/// Play step `(data, request, source_entity)`.
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn play_detour(
    this: *mut c_void,
    data: *mut c_void,
    request: *mut c_void,
    source: *mut c_void,
) {
    let Some(&t) = PLAY_TRAMPOLINE.get() else {
        return;
    };
    let original: PlayFn = unsafe { std::mem::transmute(t) };
    // Read before the call: the request belongs to the caller's map.
    let ids = guarded(|| sequences::read_request(&LiveMem, request as u32));
    let event_id =
        guarded(|| LiveMem.u32_at((data as u32).wrapping_add(layout::DATA_EVENT_ID))).flatten();
    let stage = if IN_CACHE_READY.with(Cell::get) {
        "cache_ready"
    } else {
        "appearance_ready"
    };
    let probe = {
        let _probe = Restore::set(&PLAY, Some(PlayProbe::default()));
        original(this, data, request, source);
        PLAY.with(Cell::get)
    };
    guarded(|| {
        let (Some(ids), Some(p)) = (ids, probe) else {
            return;
        };
        if let Some(path) = sequences::play_outcome(p.requested, p.instance) {
            report(
                &SequenceDrop {
                    path,
                    ids,
                    event_id,
                    age_secs: None,
                },
                stage,
            );
        } else if p.requested {
            // Played: `client.ability.shown` `kind = sequence_played`
            // (AB-C5), with the cooked event id (1002 is the interrupt).
            let f =
                crate::hooks::ability_trace::shown::sequence_played_fields(&ids, event_id, stage);
            if let Some(f) = crate::hooks::ability_trace::admit("shown:sequence_played", || f) {
                emit(crate::hooks::ability_trace::shown::TARGET_SHOWN, "info", f);
            }
        }
    });
}

type InstantiateFn =
    unsafe extern "thiscall-unwind" fn(*mut c_void, *mut u32, *mut c_void, *mut c_void);

/// Instantiate `(out_instance, name, pawn)`. Records its result for the
/// play step that called it; other callers (the test-sequence slash
/// command, the editor) are passed straight through.
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn instantiate_detour(
    this: *mut c_void,
    out: *mut u32,
    name: *mut c_void,
    pawn: *mut c_void,
) {
    let Some(&t) = INSTANTIATE_TRAMPOLINE.get() else {
        return;
    };
    let original: InstantiateFn = unsafe { std::mem::transmute(t) };
    original(this, out, name, pawn);
    guarded(|| {
        if PLAY.with(Cell::get).is_some() {
            let instance = LiveMem.u32_at(out as u32).unwrap_or(0);
            PLAY.with(|c| {
                c.set(Some(PlayProbe {
                    requested: true,
                    instance,
                }))
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static SERIAL: Mutex<()> = Mutex::new(());

    /// A play step that returns without instantiating (the cull) leaves no
    /// probe result; one that instantiates records it; the probe is gone
    /// afterwards either way.
    #[test]
    fn the_play_probe_sees_the_nested_instantiate_and_is_cleared() {
        let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        unsafe extern "thiscall-unwind" fn inst(
            _: *mut c_void,
            out: *mut u32,
            _: *mut c_void,
            _: *mut c_void,
        ) {
            unsafe { *out = 0x1234 };
        }
        let _ = INSTANTIATE_TRAMPOLINE.set(inst as *const () as usize);

        let seen = {
            let _p = Restore::set(&PLAY, Some(PlayProbe::default()));
            let mut slot = 0u32;
            unsafe {
                instantiate_detour(
                    std::ptr::null_mut(),
                    &mut slot,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            };
            assert_eq!(slot, 0x1234, "the original's result is untouched");
            PLAY.with(Cell::get)
        };
        let p = seen.expect("probe set");
        assert!(p.requested);
        assert_eq!(p.instance, 0x1234);
        assert!(PLAY.with(Cell::get).is_none(), "restored after the step");

        // Outside a play step the instantiate hook records nothing.
        let mut slot = 0u32;
        unsafe {
            instantiate_detour(
                std::ptr::null_mut(),
                &mut slot,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        assert!(PLAY.with(Cell::get).is_none());
    }

    /// A C++ exception out of the ready handler must not leave the stage
    /// flag set on the thread.
    #[test]
    fn a_throwing_ready_handler_leaves_no_stage_behind() {
        let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        unsafe extern "thiscall-unwind" fn original(_: *mut c_void, _: *mut c_void, _: u32) {
            assert!(IN_CACHE_READY.with(Cell::get));
            panic!("engine error");
        }
        let _ = CACHE_READY_TRAMPOLINE.set(original as *const () as usize);
        let caught = std::panic::catch_unwind(|| unsafe {
            cache_ready_detour(0x10 as *mut c_void, 0x20 as *mut c_void, 0)
        });
        assert!(caught.is_err());
        assert!(!IN_CACHE_READY.with(Cell::get));
    }
}
