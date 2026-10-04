//! The effect bar: `EffectSet`'s `onTimerUpdate` handler and three probes.
//!
//! `0x00e09160` (disassembled 2026-10-04) reads `Type` and returns unless
//! it is 5; reads `SecondaryId` and `BigWorldTimeComplete`; looks the
//! entry up by `SecondaryId` (`0x00e08570`). Found: it moves the entry's
//! interval (`0x00c6d1c0` on `this+0x38`) and returns. Not found, and the
//! complete time is after the game clock: it reads `TotalTime`, `SourceID`
//! and `ID`, builds a 0x20-byte entry, files it, and announces it to the
//! effect UI (`0x00e0a9e0` on the singleton from `0x004786f0`, with a
//! pointer to `ID`). The announcement looks the effect id up in the UI's
//! display cache (`0x00e0a6f0`) and posts the add (`0x00e0a2d0`) when it is
//! there; when it is not, it calls `0x00e0a810`, which (unless a request is
//! already outstanding, `[this+0x48]`) posts an
//! `Event_NetOut_elementDataRequest` with `CategoryId` 9 and `Key` = the
//! effect id: a request to the server for the effect's cooked data, not a
//! removal.

use std::cell::Cell;
use std::ffi::c_void;
use std::sync::OnceLock;

use super::super::entity_lifecycle::guarded;
use super::{i32_at, now, report, Restore};
use crate::hooks::ability_trace::{
    applied::{effect_bar_fields, EffectProbe, Timer, TIMER_TYPE_EFFECT},
    event_bag::{Bag, LiveBag},
};
use crate::queue::Producer;

pub(in crate::hooks::inline_hooks) const ADDR_EFFECT_TIMER: usize = 0x00e0_9160;
pub(in crate::hooks::inline_hooks) const ADDR_EFFECT_LOOKUP: usize = 0x00e0_8570;
pub(in crate::hooks::inline_hooks) const ADDR_EFFECT_ANNOUNCE: usize = 0x00e0_a9e0;
pub(in crate::hooks::inline_hooks) const ADDR_EFFECT_DATA_REQUEST: usize = 0x00e0_a810;

/// The `EffectSet`'s owner entity id (the subject it subscribed with,
/// stored by its constructor `0x00e094d0`).
const EFFECT_SET_OWNER: u32 = 0x2c;
/// Non-zero while a display-data request is outstanding (`0x00e0a810`
/// returns at once when it is set).
const UI_REQUEST_PENDING: u32 = 0x48;

static TIMER_TRAMPOLINE: OnceLock<usize> = OnceLock::new();
static LOOKUP_TRAMPOLINE: OnceLock<usize> = OnceLock::new();
static ANNOUNCE_TRAMPOLINE: OnceLock<usize> = OnceLock::new();
static DATA_REQUEST_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

thread_local! {
    /// Set while the handler runs for a type-5 timer.
    static PROBE: Cell<Option<EffectProbe>> = const { Cell::new(None) };
}

pub(super) unsafe fn install_all(producer: &Producer) {
    let hooks: [(&str, usize, *mut c_void, &OnceLock<usize>); 4] = [
        (
            "ability_effect_timer",
            ADDR_EFFECT_TIMER,
            timer_detour as *mut c_void,
            &TIMER_TRAMPOLINE,
        ),
        (
            "ability_effect_lookup",
            ADDR_EFFECT_LOOKUP,
            lookup_detour as *mut c_void,
            &LOOKUP_TRAMPOLINE,
        ),
        (
            "ability_effect_announce",
            ADDR_EFFECT_ANNOUNCE,
            announce_detour as *mut c_void,
            &ANNOUNCE_TRAMPOLINE,
        ),
        (
            "ability_effect_data_request",
            ADDR_EFFECT_DATA_REQUEST,
            data_request_detour as *mut c_void,
            &DATA_REQUEST_TRAMPOLINE,
        ),
    ];
    for (name, addr, detour, slot) in hooks {
        unsafe { super::super::install_one(producer, name, addr, detour, slot) };
    }
}

fn update_probe(f: impl FnOnce(&mut EffectProbe)) {
    PROBE.with(|c| {
        if let Some(mut p) = c.get() {
            f(&mut p);
            c.set(Some(p));
        }
    });
}

type HandlerFn = unsafe extern "thiscall-unwind" fn(*mut c_void, *mut c_void, *mut c_void);

/// `EffectSet` timer handler `(event, subject)`.
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn timer_detour(
    this: *mut c_void,
    event: *mut c_void,
    subject: *mut c_void,
) {
    let Some(&t) = TIMER_TRAMPOLINE.get() else {
        return;
    };
    let original: HandlerFn = unsafe { std::mem::transmute(t) };
    // Only a type-5 timer reaches the bar; read the rest only then.
    let pre = guarded(|| {
        let bag = LiveBag(event);
        (bag.byte(b"Type\0") == Some(TIMER_TYPE_EFFECT))
            .then(|| (Timer::read(&bag), i32_at(this, EFFECT_SET_OWNER), now()))
    })
    .flatten();
    let Some((timer, owner, clock)) = pre else {
        unsafe { original(this, event, subject) };
        return;
    };
    let probe = {
        let _scope = Restore::set(&PROBE, Some(EffectProbe::default()));
        unsafe { original(this, event, subject) };
        PROBE.with(Cell::get)
    };
    guarded(|| {
        let Some(probe) = probe else { return };
        if let Some((kind, f)) = effect_bar_fields(owner, &timer, &probe, clock) {
            report(&format!("applied:{kind}"), "info", f);
        }
    });
}

type LookupFn = unsafe extern "thiscall-unwind" fn(*mut c_void, i32) -> u32;

/// Entry lookup by `SecondaryId`: records whether the handler found one.
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn lookup_detour(this: *mut c_void, id: i32) -> u32 {
    let Some(&t) = LOOKUP_TRAMPOLINE.get() else {
        return 0;
    };
    let original: LookupFn = unsafe { std::mem::transmute(t) };
    let entry = unsafe { original(this, id) };
    guarded(|| {
        update_probe(|p| {
            p.lookup_hit.get_or_insert(entry != 0);
        })
    });
    entry
}

type IdPtrFn = unsafe extern "thiscall-unwind" fn(*mut c_void, *mut i32) -> u32;

/// The new entry's announcement to the effect UI.
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn announce_detour(this: *mut c_void, id: *mut i32) -> u32 {
    let Some(&t) = ANNOUNCE_TRAMPOLINE.get() else {
        return 0;
    };
    let original: IdPtrFn = unsafe { std::mem::transmute(t) };
    guarded(|| update_probe(|p| p.announced = true));
    unsafe { original(this, id) }
}

/// The display-data request the announcement makes when the effect's
/// cooked data is not cached.
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn data_request_detour(this: *mut c_void, id: *mut i32) -> u32 {
    let Some(&t) = DATA_REQUEST_TRAMPOLINE.get() else {
        return 0;
    };
    let original: IdPtrFn = unsafe { std::mem::transmute(t) };
    guarded(|| {
        let pending = i32_at(this, UI_REQUEST_PENDING).is_some_and(|v| v != 0);
        update_probe(|p| p.data_request = Some(pending));
    });
    unsafe { original(this, id) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    static SERIAL: Mutex<()> = Mutex::new(());

    /// The probes pass every argument and result through, and record only
    /// inside a handler scope.
    #[test]
    fn probes_forward_and_record_only_in_scope() {
        let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        static SEEN: AtomicUsize = AtomicUsize::new(0);
        unsafe extern "thiscall-unwind" fn lookup(this: *mut c_void, id: i32) -> u32 {
            SEEN.store(this as usize ^ id as usize, Ordering::SeqCst);
            0xABCD
        }
        unsafe extern "thiscall-unwind" fn announce(_: *mut c_void, id: *mut i32) -> u32 {
            unsafe { *id }.unsigned_abs()
        }
        let _ = LOOKUP_TRAMPOLINE.set(lookup as *const () as usize);
        let _ = ANNOUNCE_TRAMPOLINE.set(announce as *const () as usize);
        let _ = DATA_REQUEST_TRAMPOLINE.set(announce as *const () as usize);

        // Outside a handler: forwarded, nothing recorded.
        assert_eq!(unsafe { lookup_detour(0x100 as *mut c_void, 7) }, 0xABCD);
        assert_eq!(SEEN.load(Ordering::SeqCst), 0x100 ^ 7);
        assert_eq!(PROBE.with(Cell::get), None);

        let mut id = 1201;
        let probe = {
            let _scope = Restore::set(&PROBE, Some(EffectProbe::default()));
            unsafe { lookup_detour(0x100 as *mut c_void, 7) };
            assert_eq!(
                unsafe { announce_detour(0x200 as *mut c_void, &mut id) },
                1201
            );
            // `this` is not readable here, so the pending flag reads false.
            assert_eq!(
                unsafe { data_request_detour(0x300 as *mut c_void, &mut id) },
                1201
            );
            PROBE.with(Cell::get)
        };
        assert_eq!(
            probe,
            Some(EffectProbe {
                lookup_hit: Some(true),
                announced: true,
                data_request: Some(false),
            })
        );
        assert_eq!(PROBE.with(Cell::get), None, "restored after the handler");
    }

    /// The handler forwards its three arguments (it pops two stack words;
    /// the finding's one-argument signature would unbalance the stack),
    /// leaves no probe behind when the original throws, and a non-effect
    /// event (unreadable here) goes straight through.
    #[test]
    fn the_handler_forwards_and_unwinds_cleanly() {
        let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        static ARGS: [AtomicUsize; 3] = [const { AtomicUsize::new(0) }; 3];
        unsafe extern "thiscall-unwind" fn original(
            this: *mut c_void,
            event: *mut c_void,
            subject: *mut c_void,
        ) {
            for (slot, v) in ARGS
                .iter()
                .zip([this as usize, event as usize, subject as usize])
            {
                slot.store(v, Ordering::SeqCst);
            }
            if subject as usize == 0xdead {
                panic!("engine error");
            }
        }
        let _ = TIMER_TRAMPOLINE.set(original as *const () as usize);
        unsafe {
            timer_detour(
                0x11 as *mut c_void,
                std::ptr::null_mut(),
                0x33 as *mut c_void,
            )
        };
        let seen: Vec<usize> = ARGS.iter().map(|a| a.load(Ordering::SeqCst)).collect();
        assert_eq!(seen, [0x11, 0, 0x33]);
        let caught = std::panic::catch_unwind(|| unsafe {
            timer_detour(
                0x11 as *mut c_void,
                std::ptr::null_mut(),
                0xdead as *mut c_void,
            )
        });
        assert!(caught.is_err());
        assert_eq!(PROBE.with(Cell::get), None);
    }
}
