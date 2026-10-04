//! Stats: the `GameBeing` stat handlers and the functors they call per
//! stat.
//!
//! `0x00e01f40` (current) and `0x00e02060` (base) read the `Stats` list
//! from the event and hand it to `0x00e00e60` with a functor, `0x00e004e0`
//! or `0x00e005b0`. The iterator calls the functor once per element as
//! `thiscall(being, StatId, Max, Min, Current)` (`ret 0x10`), and the
//! functor stores the values in the being's stat map at `this+0x160`.
//! The handler hook opens a scope; the functor hooks collect what was
//! stored; the handler emits one event per message.

use std::cell::RefCell;
use std::ffi::c_void;
use std::sync::OnceLock;

use super::super::entity_lifecycle::guarded;
use super::{i32_at, is_local_player, report};
use crate::hooks::ability_trace::applied::{stat_fields, StatCall, MAX_STATS};
use crate::hooks::ability_trace::whose;
use crate::queue::Producer;

pub(in crate::hooks::inline_hooks) const ADDR_STAT_HANDLER: usize = 0x00e0_1f40;
pub(in crate::hooks::inline_hooks) const ADDR_STAT_BASE_HANDLER: usize = 0x00e0_2060;
pub(in crate::hooks::inline_hooks) const ADDR_STAT_FUNCTOR: usize = 0x00e0_04e0;
pub(in crate::hooks::inline_hooks) const ADDR_STAT_BASE_FUNCTOR: usize = 0x00e0_05b0;

/// `GameBeing`'s entity id.
const BEING_ID: u32 = 0x0c;

static HANDLER_TRAMPOLINE: OnceLock<usize> = OnceLock::new();
static BASE_HANDLER_TRAMPOLINE: OnceLock<usize> = OnceLock::new();
static FUNCTOR_TRAMPOLINE: OnceLock<usize> = OnceLock::new();
static BASE_FUNCTOR_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

/// The stats one handler call stored, the first [`MAX_STATS`] kept.
#[derive(Debug, Default)]
struct Collected {
    calls: Vec<StatCall>,
    total: usize,
}

thread_local! {
    static COLLECTED: RefCell<Option<Collected>> = const { RefCell::new(None) };
}

/// Opens a collection for one handler call and restores the previous
/// state when dropped, including on an unwind.
struct Scope {
    prev: Option<Option<Collected>>,
}

impl Scope {
    fn enter() -> Self {
        Self {
            prev: Some(COLLECTED.with(|c| c.replace(Some(Collected::default())))),
        }
    }

    /// Close the scope and return what it collected.
    fn finish(mut self) -> Option<Collected> {
        let prev = self.prev.take().flatten();
        COLLECTED.with(|c| c.replace(prev))
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        if let Some(prev) = self.prev.take() {
            COLLECTED.with(|c| *c.borrow_mut() = prev);
        }
    }
}

pub(super) unsafe fn install_all(producer: &Producer) {
    let hooks: [(&str, usize, *mut c_void, &OnceLock<usize>); 4] = [
        (
            "ability_stat_handler",
            ADDR_STAT_HANDLER,
            handler_detour as *mut c_void,
            &HANDLER_TRAMPOLINE,
        ),
        (
            "ability_stat_base_handler",
            ADDR_STAT_BASE_HANDLER,
            base_handler_detour as *mut c_void,
            &BASE_HANDLER_TRAMPOLINE,
        ),
        (
            "ability_stat_functor",
            ADDR_STAT_FUNCTOR,
            functor_detour as *mut c_void,
            &FUNCTOR_TRAMPOLINE,
        ),
        (
            "ability_stat_base_functor",
            ADDR_STAT_BASE_FUNCTOR,
            base_functor_detour as *mut c_void,
            &BASE_FUNCTOR_TRAMPOLINE,
        ),
    ];
    for (name, addr, detour, slot) in hooks {
        unsafe { super::super::install_one(producer, name, addr, detour, slot) };
    }
}

type HandlerFn = unsafe extern "thiscall-unwind" fn(*mut c_void, *mut c_void, *mut c_void);

unsafe fn handle(
    trampoline: &OnceLock<usize>,
    base: bool,
    this: *mut c_void,
    event: *mut c_void,
    subject: *mut c_void,
) {
    let Some(&t) = trampoline.get() else {
        return;
    };
    let original: HandlerFn = unsafe { std::mem::transmute(t) };
    let collected = {
        let scope = Scope::enter();
        unsafe { original(this, event, subject) };
        scope.finish()
    };
    guarded(|| {
        let Some(c) = collected else { return };
        let id = i32_at(this, BEING_ID);
        let f = stat_fields(id, base, &c.calls, c.total);
        let kind = if base { "stat_base" } else { "stat" };
        let key = format!("applied:{kind}:{}", whose(is_local_player(id)));
        report(&key, "info", f);
    });
}

/// `GameBeing` stat handler `(event, subject)`.
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn handler_detour(
    this: *mut c_void,
    event: *mut c_void,
    subject: *mut c_void,
) {
    unsafe { handle(&HANDLER_TRAMPOLINE, false, this, event, subject) }
}

/// `GameBeing` base-stat handler `(event, subject)`.
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn base_handler_detour(
    this: *mut c_void,
    event: *mut c_void,
    subject: *mut c_void,
) {
    unsafe { handle(&BASE_HANDLER_TRAMPOLINE, true, this, event, subject) }
}

type FunctorFn = unsafe extern "thiscall-unwind" fn(*mut c_void, i32, i32, i32, i32) -> u32;

fn collect(call: StatCall) {
    let _ = COLLECTED.try_with(|c| {
        if let Some(c) = c.borrow_mut().as_mut() {
            c.total += 1;
            if c.calls.len() < MAX_STATS {
                c.calls.push(call);
            }
        }
    });
}

unsafe fn store(
    trampoline: &OnceLock<usize>,
    this: *mut c_void,
    stat_id: i32,
    max: i32,
    min: i32,
    current: i32,
) -> u32 {
    let Some(&t) = trampoline.get() else {
        return 0;
    };
    let original: FunctorFn = unsafe { std::mem::transmute(t) };
    let r = unsafe { original(this, stat_id, max, min, current) };
    guarded(|| {
        collect(StatCall {
            stat_id,
            max,
            min,
            current,
        })
    });
    r
}

/// Current-stat functor `(StatId, Max, Min, Current)`.
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn functor_detour(
    this: *mut c_void,
    stat_id: i32,
    max: i32,
    min: i32,
    current: i32,
) -> u32 {
    unsafe { store(&FUNCTOR_TRAMPOLINE, this, stat_id, max, min, current) }
}

/// Base-stat functor `(StatId, Max, Min, Current)`.
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn base_functor_detour(
    this: *mut c_void,
    stat_id: i32,
    max: i32,
    min: i32,
    current: i32,
) -> u32 {
    unsafe { store(&BASE_FUNCTOR_TRAMPOLINE, this, stat_id, max, min, current) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// The functor sees exactly the arguments the iterator passed
    /// (`ret 0x10`: four stack words), and inside a handler scope every
    /// call is collected; the scope is gone afterwards, even if the
    /// handler throws.
    #[test]
    fn the_handler_collects_each_functor_call() {
        static SEEN: [AtomicU32; 5] = [const { AtomicU32::new(0) }; 5];
        unsafe extern "thiscall-unwind" fn functor(
            this: *mut c_void,
            id: i32,
            max: i32,
            min: i32,
            cur: i32,
        ) -> u32 {
            for (s, v) in
                SEEN.iter()
                    .zip([this as u32, id as u32, max as u32, min as u32, cur as u32])
            {
                s.store(v, Ordering::SeqCst);
            }
            1
        }
        unsafe extern "thiscall-unwind" fn handler(
            this: *mut c_void,
            _event: *mut c_void,
            subject: *mut c_void,
        ) {
            // The iterator, for two list elements.
            unsafe {
                functor_detour(this, 6, 1000, 0, 840);
                functor_detour(this, 7, 100, 0, 55);
            }
            if subject as usize == 0xdead {
                panic!("engine error");
            }
        }
        let _ = FUNCTOR_TRAMPOLINE.set(functor as *const () as usize);
        let _ = HANDLER_TRAMPOLINE.set(handler as *const () as usize);

        // Outside a handler: forwarded, not collected.
        assert_eq!(
            unsafe { functor_detour(0x50 as *mut c_void, 1, 2, 3, 4) },
            1
        );
        let seen: Vec<u32> = SEEN.iter().map(|s| s.load(Ordering::SeqCst)).collect();
        assert_eq!(seen, [0x50, 1, 2, 3, 4]);
        assert!(COLLECTED.with(|c| c.borrow().is_none()));

        let scope = Scope::enter();
        unsafe {
            handler(
                0x50 as *mut c_void,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        let c = scope.finish().unwrap();
        assert_eq!(c.total, 2);
        assert_eq!(
            c.calls[0],
            StatCall {
                stat_id: 6,
                max: 1000,
                min: 0,
                current: 840
            }
        );
        assert!(COLLECTED.with(|c| c.borrow().is_none()));

        // Through the detour, with a throwing handler.
        let caught = std::panic::catch_unwind(|| unsafe {
            handler_detour(
                0x50 as *mut c_void,
                std::ptr::null_mut(),
                0xdead as *mut c_void,
            )
        });
        assert!(caught.is_err());
        assert!(COLLECTED.with(|c| c.borrow().is_none()));
    }
}
