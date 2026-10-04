//! Patch, call, unpatch: every detour here is installed with MinHook on a
//! stand-in function of the same ABI (the mechanism `install_one` uses on
//! the client), called through the stand-in, then removed.
//!
//! The stand-ins are wired the way the game's chain is (the `useAction`
//! stand-in calls the slot stand-in, which calls the lookup stand-in, and
//! so on), so each test drives the real detours in their real order. A
//! trampoline slot is a `OnceLock`, set once per process, so each detour is
//! installed by exactly one test.

use std::ffi::c_void;
use std::hint::black_box;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use super::press_detours::*;
use super::route::test_access::{
    enter_route, leave_route, START_ENTITY_TRAMPOLINE, START_PROXY_TRAMPOLINE,
};
use super::route::{start_entity_detour, start_proxy_detour};
use super::seq::*;
use super::*;
use crate::hooks::ability_trace::capture;
use crate::hooks::ability_trace::seq_join::SentTag;
use crate::hooks::ability_trace::{field, Out, TARGET_DROPPED, TARGET_PRESS, TARGET_SENT_SEQ};
use serde_json::json;

/// The MinHook tests share the pending and join tables.
static SERIAL: Mutex<()> = Mutex::new(());

/// Install `detour` over `target` with MinHook, publish the trampoline
/// into `slot`, and return a guard that removes the hook.
struct Patched(usize);

unsafe fn patch(target: usize, detour: usize, slot: &OnceLock<usize>) -> Patched {
    unsafe {
        let s = minhook_sys::MH_Initialize();
        assert!(s == minhook_sys::MH_OK || s == minhook_sys::MH_ERROR_ALREADY_INITIALIZED);
        let mut tramp: *mut c_void = std::ptr::null_mut();
        let s =
            minhook_sys::MH_CreateHook(target as *mut c_void, detour as *mut c_void, &mut tramp);
        assert_eq!(s, minhook_sys::MH_OK, "create 0x{target:x}");
        slot.set(tramp as usize)
            .expect("each trampoline slot is set by one test only");
        assert_eq!(
            minhook_sys::MH_EnableHook(target as *mut c_void),
            minhook_sys::MH_OK
        );
    }
    Patched(target)
}

impl Drop for Patched {
    fn drop(&mut self) {
        unsafe {
            minhook_sys::MH_DisableHook(self.0 as *mut c_void);
            minhook_sys::MH_RemoveHook(self.0 as *mut c_void);
        }
    }
}

fn targets(outs: &[Out]) -> Vec<&'static str> {
    outs.iter().map(|o| o.target).collect()
}

fn reason(outs: &[Out]) -> Option<String> {
    outs.iter()
        .find(|o| o.target == TARGET_DROPPED)
        .and_then(|o| field(&o.fields, "reason"))
        .and_then(|v| v.as_str().map(str::to_owned))
}

mod press_chain;
mod router_exit;
mod router_probe;
mod sequence_join;
