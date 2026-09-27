//! The two network-thread detours. See the [module docs](super).

use core::ffi::c_void;
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::claim::{claim, Claim};
use super::stream::ClientStream;
use crate::counters::{bump, is_log_worthy};
use crate::memory::ProcessMemory;
use crate::{log, COUNTERS, EVENTS};

/// MinHook's trampoline to the original dispatcher. Set before the hook is
/// enabled, so the detour never runs without it.
pub(crate) static DISPATCH_ORIGINAL: AtomicUsize = AtomicUsize::new(0);

/// MinHook's trampoline to the original drop callee.
pub(crate) static LOOKUP_ORIGINAL: AtomicUsize = AtomicUsize::new(0);

/// The dispatcher. Its prologue sets up a C++ exception frame, so it may
/// throw (a failed allocation, a decoder that rejects its input). The
/// `-unwind` ABI lets such an exception pass through the detour to
/// whatever catches it in the game, running [`Scope`]'s drop on the way,
/// instead of aborting the process in the detour.
type DispatchFn = unsafe extern "thiscall-unwind" fn(
    this: *mut c_void,
    entity: *mut c_void,
    msg_id: u32,
    stream: *mut c_void,
);

/// The drop callee: a bounds-checked vector lookup that never throws.
type LookupFn = unsafe extern "thiscall" fn(method_table: *mut c_void, index: i32) -> *mut c_void;

/// The dispatcher call in progress on this thread.
#[derive(Debug, Clone, Copy, Default)]
struct Current {
    entity: usize,
    stream: usize,
}

thread_local! {
    static CURRENT: Cell<Current> = const {
        Cell::new(Current {
            entity: 0,
            stream: 0,
        })
    };
}

/// Records the dispatcher call for its duration and puts the previous
/// value back when it ends, by return or by unwind.
struct Scope {
    previous: Current,
}

impl Scope {
    fn enter(entity: usize, stream: usize) -> Self {
        let previous = CURRENT
            .try_with(|c| c.replace(Current { entity, stream }))
            .unwrap_or_default();
        Self { previous }
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        let _ = CURRENT.try_with(|c| c.set(self.previous));
    }
}

/// The recorded call, with its stream cleared so that one dispatch is
/// claimed at most once.
fn take_current() -> Current {
    CURRENT
        .try_with(|c| {
            let current = c.get();
            c.set(Current {
                entity: current.entity,
                stream: 0,
            });
            current
        })
        .unwrap_or_default()
}

/// Detour for `Client_NetIn_EntityMethodDispatch` (`0x00c6f8f0`):
/// `__thiscall(this, Entity* entity, u32 msgId, BinaryIStream* stream)`,
/// callee cleans 12 bytes. Runs for every inbound entity method on a
/// Mercury network thread, so it only records two pointers and calls
/// through. Nothing in it can panic.
pub(crate) unsafe extern "thiscall-unwind" fn dispatch_detour(
    this: *mut c_void,
    entity: *mut c_void,
    msg_id: u32,
    stream: *mut c_void,
) {
    let original = DISPATCH_ORIGINAL.load(Ordering::Acquire);
    if original == 0 {
        // Unreachable: the trampoline is stored before the hook is enabled.
        return;
    }
    let _scope = Scope::enter(entity as usize, stream as usize);
    // SAFETY: MinHook's trampoline for this exact function and signature.
    unsafe {
        let original = core::mem::transmute::<usize, DispatchFn>(original);
        original(this, entity, msg_id, stream);
    }
}

/// Detour for the drop callee (`0x01590f30`): `__thiscall(void*
/// methodTable, int index) -> MethodDescription*`, callee cleans 4 bytes.
/// Calls the original, then offers the result to [`claim`], and returns
/// the original result unchanged.
pub(crate) unsafe extern "thiscall" fn lookup_detour(
    method_table: *mut c_void,
    index: i32,
) -> *mut c_void {
    let original = LOOKUP_ORIGINAL.load(Ordering::Acquire);
    if original == 0 {
        // Unreachable, as above. Null is the callee's own "not found", and
        // its only caller ignores the result.
        return core::ptr::null_mut();
    }
    // SAFETY: MinHook's trampoline for this exact function and signature.
    let method = unsafe {
        let original = core::mem::transmute::<usize, LookupFn>(original);
        original(method_table, index)
    };
    if !method.is_null() {
        let _ = std::panic::catch_unwind(|| on_dropped_method(method as usize));
    }
    method
}

/// A method the client is about to drop: claim it if it is ours.
fn on_dropped_method(method: usize) {
    let current = take_current();
    if current.stream == 0 {
        return;
    }
    // SAFETY: `current.stream` is the stream of the dispatcher call running
    // on this thread right now (the drop callee is only called from it).
    let open = || unsafe { ClientStream::open(current.stream) };
    match claim(&ProcessMemory, method, current.entity, open) {
        Claim::NotOurs => {}
        Claim::NotLocalPlayer(m) => {
            let n = bump(&COUNTERS.not_local_player);
            if is_log_worthy(n) {
                log::line(format_args!(
                    "{} for another entity left to the client (#{n})",
                    m.name()
                ));
            }
        }
        Claim::NoStream(m) => {
            let n = bump(&COUNTERS.decode_failed);
            if is_log_worthy(n) {
                log::line(format_args!(
                    "{}: could not open the argument stream (#{n})",
                    m.name()
                ));
            }
        }
        Claim::Malformed(m, e) => {
            let n = bump(&COUNTERS.decode_failed);
            if is_log_worthy(n) {
                log::line(format_args!("{}: dropped, {e} (#{n})", m.name()));
            }
        }
        Claim::Decoded(call) => {
            let m = call.method();
            let n = bump(&COUNTERS.received);
            if is_log_worthy(n) {
                log::line(format_args!("claimed {} (#{n} received)", m.name()));
            }
            if EVENTS.push(call).is_err() {
                let n = bump(&COUNTERS.dropped_queue_full);
                if is_log_worthy(n) {
                    log::line(format_args!(
                        "{}: dropped, the queue is full (#{n})",
                        m.name()
                    ));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A nested dispatch restores the outer call's record when it ends, and
    /// a claim clears only the stream.
    #[test]
    fn scope_is_nest_safe_and_claims_once() {
        {
            let _outer = Scope::enter(1, 10);
            {
                let _inner = Scope::enter(2, 20);
                let taken = take_current();
                assert_eq!((taken.entity, taken.stream), (2, 20));
                assert_eq!(take_current().stream, 0, "claimed at most once");
            }
            let outer = take_current();
            assert_eq!((outer.entity, outer.stream), (1, 10));
        }
        assert_eq!(take_current().stream, 0, "nothing left after the call");
    }

    /// The record is restored even when the call unwinds.
    #[test]
    fn scope_restores_on_unwind() {
        let result = std::panic::catch_unwind(|| {
            let _scope = Scope::enter(3, 30);
            panic!("the original threw");
        });
        assert!(result.is_err());
        assert_eq!(take_current().stream, 0);
    }
}
