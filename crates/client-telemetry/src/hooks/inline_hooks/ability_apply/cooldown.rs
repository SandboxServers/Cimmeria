//! Cooldowns: `CooldownManager`'s `onTimerUpdate` handler and its button
//! callback.
//!
//! `0x00ea6af0` (disassembled 2026-10-04) reads `SourceID` and returns
//! unless it equals the manager's owner (`[this]`, `cmp ecx, [ebp]` at
//! `0x00ea6ba6`); reads `Type`, `ID`, `TotalTime` and
//! `BigWorldTimeComplete`; looks `(Type, ID)` up in the button map at
//! `this+0x34` (`0x00ea7970`) and returns when no button registered it;
//! otherwise it queries the interval (`0x00ea6120`), updates the interval
//! tree at `this+0x14` and calls the button callback `0x00ea62b0` with
//! `(Type, ID, float, float)`. So a cooldown either reaches a button
//! (`applied`), has no button (`no_button`), or belongs to another being
//! (`other_source`). Effect timers (type 5) are the effect bar's, and a
//! `no_button` for one is not reported.

use std::cell::Cell;
use std::ffi::c_void;
use std::sync::OnceLock;

use super::super::entity_lifecycle::guarded;
use super::{i32_at, now, report, Restore};
use crate::hooks::ability_trace::{
    applied::{cooldown_fields, Timer, TIMER_TYPE_EFFECT},
    event_bag::LiveBag,
};
use crate::queue::Producer;

pub(in crate::hooks::inline_hooks) const ADDR_COOLDOWN_TIMER: usize = 0x00ea_6af0;
pub(in crate::hooks::inline_hooks) const ADDR_COOLDOWN_UI: usize = 0x00ea_62b0;

/// The manager's owner entity id.
const COOLDOWN_OWNER: u32 = 0x00;

static TIMER_TRAMPOLINE: OnceLock<usize> = OnceLock::new();
static UI_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

/// The two floats the handler passed the button callback.
type UiValues = Option<(f32, f32)>;

thread_local! {
    /// `Some` while the handler runs; the inner value once the callback
    /// ran.
    static UI: Cell<Option<UiValues>> = const { Cell::new(None) };
}

pub(super) unsafe fn install_all(producer: &Producer) {
    let hooks: [(&str, usize, *mut c_void, &OnceLock<usize>); 2] = [
        (
            "ability_cooldown_timer",
            ADDR_COOLDOWN_TIMER,
            timer_detour as *mut c_void,
            &TIMER_TRAMPOLINE,
        ),
        (
            "ability_cooldown_ui",
            ADDR_COOLDOWN_UI,
            ui_detour as *mut c_void,
            &UI_TRAMPOLINE,
        ),
    ];
    for (name, addr, detour, slot) in hooks {
        unsafe { super::super::install_one(producer, name, addr, detour, slot) };
    }
}

type HandlerFn = unsafe extern "thiscall-unwind" fn(*mut c_void, *mut c_void, *mut c_void);

/// `CooldownManager` timer handler `(event, subject)`.
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
    // The event getters are game code with a C++ EH frame (they copy the
    // property tree and can throw `bad_alloc`). They are called outside
    // `catch_unwind`: catching a foreign exception aborts or swallows it,
    // unspecified which, so a throw must unwind through this
    // `thiscall-unwind` frame to the game's own handler, as it would from
    // the handler's own call to the same getter. The Rust around them
    // cannot panic.
    let pre = Some((
        Timer::read(&LiveBag(event)),
        i32_at(this, COOLDOWN_OWNER),
        now(),
    ));
    let ui = {
        let _scope = Restore::set(&UI, Some(None));
        unsafe { original(this, event, subject) };
        UI.with(Cell::get).flatten()
    };
    guarded(|| {
        let Some((timer, owner, clock)) = pre else {
            return;
        };
        if timer.id.is_none() && timer.timer_type.is_none() {
            // Not a timer event the bag could read (or no bag at all).
            return;
        }
        let (level, f) = cooldown_fields(owner, &timer, ui, clock);
        let outcome = f
            .iter()
            .find(|(k, _)| *k == "outcome")
            .and_then(|(_, v)| v.as_str())
            .unwrap_or("unknown");
        if outcome == "no_button" && timer.timer_type == Some(TIMER_TYPE_EFFECT) {
            return;
        }
        report(&format!("applied:cooldown:{outcome}"), level, f);
    });
}

type UiFn = unsafe extern "thiscall-unwind" fn(*mut c_void, i32, i32, f32, f32) -> u32;

/// The button callback `(type, id, float, float)`: records that it ran.
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn ui_detour(
    this: *mut c_void,
    timer_type: i32,
    id: i32,
    a: f32,
    b: f32,
) -> u32 {
    let Some(&t) = UI_TRAMPOLINE.get() else {
        return 0;
    };
    let original: UiFn = unsafe { std::mem::transmute(t) };
    guarded(|| {
        UI.with(|c| {
            if c.get().is_some() {
                c.set(Some(Some((a, b))));
            }
        })
    });
    unsafe { original(this, timer_type, id, a, b) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// The callback detour hands all four stack arguments through
    /// (`ret 0x10`), floats included, and records them only inside the
    /// handler.
    #[test]
    fn the_callback_forwards_its_four_arguments_and_records_in_scope() {
        static SEEN: [AtomicU32; 5] = [const { AtomicU32::new(0) }; 5];
        unsafe extern "thiscall-unwind" fn original(
            this: *mut c_void,
            t: i32,
            id: i32,
            a: f32,
            b: f32,
        ) -> u32 {
            for (s, v) in
                SEEN.iter()
                    .zip([this as u32, t as u32, id as u32, a.to_bits(), b.to_bits()])
            {
                s.store(v, Ordering::SeqCst);
            }
            7
        }
        let _ = UI_TRAMPOLINE.set(original as *const () as usize);
        assert_eq!(
            unsafe { ui_detour(0x40 as *mut c_void, 1, 597, 2.5, 30.0) },
            7
        );
        let seen: Vec<u32> = SEEN.iter().map(|s| s.load(Ordering::SeqCst)).collect();
        assert_eq!(seen, [0x40, 1, 597, 2.5f32.to_bits(), 30.0f32.to_bits()]);
        assert_eq!(UI.with(Cell::get), None);
        let recorded = {
            let _scope = Restore::set(&UI, Some(None));
            unsafe { ui_detour(0x40 as *mut c_void, 1, 597, 2.5, 30.0) };
            UI.with(Cell::get)
        };
        assert_eq!(recorded, Some(Some((2.5, 30.0))));
        assert_eq!(UI.with(Cell::get), None);
    }

    /// The handler pops two stack words; both reach the original.
    #[test]
    fn the_handler_forwards_both_stack_arguments() {
        static SEEN: [AtomicU32; 3] = [const { AtomicU32::new(0) }; 3];
        unsafe extern "thiscall-unwind" fn original(
            this: *mut c_void,
            event: *mut c_void,
            subject: *mut c_void,
        ) {
            for (s, v) in SEEN.iter().zip([this as u32, event as u32, subject as u32]) {
                s.store(v, Ordering::SeqCst);
            }
        }
        let _ = TIMER_TRAMPOLINE.set(original as *const () as usize);
        unsafe {
            timer_detour(
                0x10 as *mut c_void,
                std::ptr::null_mut(),
                0x30 as *mut c_void,
            )
        };
        let seen: Vec<u32> = SEEN.iter().map(|s| s.load(Ordering::SeqCst)).collect();
        assert_eq!(seen, [0x10, 0, 0x30]);
        assert_eq!(UI.with(Cell::get), None);
    }
}
