//! The x86-only half of the [D3D9 seam](super::d3d9): the `Direct3DCreate9` IAT
//! detour and the COM vtable detours (`CreateDevice`, `Reset`,
//! `TestCooperativeLevel`). Split out of `d3d9.rs` to keep both under the
//! size cap; the layouts, the event shapes and the evidence live there.

use std::ffi::c_void;
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicUsize, Ordering};
use std::time::Instant;

use super::d3d9::*;
use crate::hooks::sinks::emit::emit;
use crate::hooks::sinks::install::SlotImport;
use crate::hooks::sinks::mem::{self, process_reader};

pub(in crate::hooks) static ORIG_CREATE9: AtomicUsize = AtomicUsize::new(0);
static ORIG_CREATE_DEVICE: AtomicUsize = AtomicUsize::new(0);
static ORIG_RESET: AtomicUsize = AtomicUsize::new(0);
static ORIG_TEST: AtomicUsize = AtomicUsize::new(0);
static D3D_PATCHED: AtomicBool = AtomicBool::new(false);
static DEVICE_PATCHED: AtomicBool = AtomicBool::new(false);
/// The last `TestCooperativeLevel` answer; `i64::MIN` = none yet.
static LAST_STATE: AtomicI64 = AtomicI64::new(i64::MIN);

pub(in crate::hooks) const IMPORT_CREATE9: SlotImport = SlotImport {
    slot: IAT_DIRECT3D_CREATE9,
    module: "d3d9.dll",
    symbol: c"Direct3DCreate9",
};

/// The slot address of vtable entry `index` of the COM object at `obj`.
fn slot_of(obj: usize, index: usize) -> Option<usize> {
    let vtable = mem::read_u32(&process_reader, obj)? as usize;
    (vtable != 0).then_some(vtable + index * 4)
}

/// Replace one COM vtable slot, once, publishing the original in `orig`.
/// A slot that already holds our detour (a second device from the same
/// vtable) is left alone.
fn patch(obj: usize, index: usize, detour: usize, orig: &AtomicUsize) -> bool {
    let Some(slot) = slot_of(obj, index) else {
        return false;
    };
    let Some(current) = mem::read_u32(&process_reader, slot) else {
        return false;
    };
    if current as usize == detour {
        return true;
    }
    orig.store(current as usize, Ordering::Release);
    // SAFETY: `slot` was just read, so it is mapped; the detour has the
    // exact signature of the method.
    match unsafe { crate::hooks::swap_vtable_slot(slot, detour) } {
        Ok(_) => true,
        Err(_) => {
            orig.store(0, Ordering::Release);
            false
        }
    }
}

/// `IDirect3D9* __stdcall Direct3DCreate9(UINT SDKVersion)`.
#[allow(improper_ctypes_definitions)]
pub(in crate::hooks) unsafe extern "stdcall-unwind" fn create9_detour(
    sdk_version: u32,
) -> *mut c_void {
    let orig = ORIG_CREATE9.load(Ordering::Acquire);
    if orig == 0 {
        return std::ptr::null_mut();
    }
    let original: unsafe extern "stdcall-unwind" fn(u32) -> *mut c_void =
        unsafe { std::mem::transmute(orig) };
    let d3d = original(sdk_version);
    if !d3d.is_null() && !D3D_PATCHED.load(Ordering::Acquire) {
        let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
            if patch(
                d3d as usize,
                VT_CREATE_DEVICE,
                create_device_detour as *const () as usize,
                &ORIG_CREATE_DEVICE,
            ) {
                D3D_PATCHED.store(true, Ordering::Release);
            }
        }));
    }
    d3d
}

type CreateDeviceFn = unsafe extern "stdcall-unwind" fn(
    *mut c_void,
    u32,
    u32,
    *mut c_void,
    u32,
    *mut c_void,
    *mut *mut c_void,
) -> i32;

/// `HRESULT IDirect3D9::CreateDevice(UINT, D3DDEVTYPE, HWND, DWORD,
/// D3DPRESENT_PARAMETERS*, IDirect3DDevice9**)`.
#[allow(improper_ctypes_definitions)]
unsafe extern "stdcall-unwind" fn create_device_detour(
    this: *mut c_void,
    adapter: u32,
    device_type: u32,
    focus_window: *mut c_void,
    behaviour: u32,
    params: *mut c_void,
    device_out: *mut *mut c_void,
) -> i32 {
    let orig = ORIG_CREATE_DEVICE.load(Ordering::Acquire);
    if orig == 0 {
        // 0x8876086C: D3DERR_INVALIDCALL, the API's own refusal.
        return 0x8876_086C_u32 as i32;
    }
    // The parameters are read before the call: CreateDevice may adjust
    // them (and the client owns the struct).
    let requested = std::panic::catch_unwind(AssertUnwindSafe(|| {
        read_params(&process_reader, params as usize)
    }))
    .ok()
    .flatten();
    let original: CreateDeviceFn = unsafe { std::mem::transmute(orig) };
    let hr = original(
        this,
        adapter,
        device_type,
        focus_window,
        behaviour,
        params,
        device_out,
    );
    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
        emit(
            CREATED_TARGET,
            state_level(hr),
            "gfx.device_created",
            created_fields(hr, adapter, behaviour, requested),
        );
        if hr >= 0 && !DEVICE_PATCHED.load(Ordering::Acquire) {
            if let Some(dev) = mem::read_u32(&process_reader, device_out as usize) {
                let dev = dev as usize;
                let ok_test = patch(
                    dev,
                    VT_TEST_COOPERATIVE_LEVEL,
                    test_detour as *const () as usize,
                    &ORIG_TEST,
                );
                let ok_reset = patch(
                    dev,
                    VT_RESET,
                    reset_detour as *const () as usize,
                    &ORIG_RESET,
                );
                DEVICE_PATCHED.store(ok_test && ok_reset, Ordering::Release);
            }
        }
    }));
    hr
}

/// `HRESULT IDirect3DDevice9::Reset(D3DPRESENT_PARAMETERS*)`.
#[allow(improper_ctypes_definitions)]
unsafe extern "stdcall-unwind" fn reset_detour(this: *mut c_void, params: *mut c_void) -> i32 {
    let orig = ORIG_RESET.load(Ordering::Acquire);
    if orig == 0 {
        return 0x8876_086C_u32 as i32;
    }
    let requested = std::panic::catch_unwind(AssertUnwindSafe(|| {
        read_params(&process_reader, params as usize)
    }))
    .ok()
    .flatten();
    let original: unsafe extern "stdcall-unwind" fn(*mut c_void, *mut c_void) -> i32 =
        unsafe { std::mem::transmute(orig) };
    let started = Instant::now();
    let hr = original(this, params);
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    // A reset changes the answer the cooperative level will give next.
    LAST_STATE.store(i64::MIN, Ordering::Relaxed);
    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
        emit(
            RESET_TARGET,
            state_level(hr),
            "gfx.device_reset",
            reset_fields(hr, elapsed_ms, requested),
        );
    }));
    hr
}

/// `HRESULT IDirect3DDevice9::TestCooperativeLevel()`.
#[allow(improper_ctypes_definitions)]
unsafe extern "stdcall-unwind" fn test_detour(this: *mut c_void) -> i32 {
    let orig = ORIG_TEST.load(Ordering::Acquire);
    if orig == 0 {
        return 0;
    }
    let original: unsafe extern "stdcall-unwind" fn(*mut c_void) -> i32 =
        unsafe { std::mem::transmute(orig) };
    let hr = original(this);
    let previous = LAST_STATE.swap(i64::from(hr), Ordering::Relaxed);
    let previous = (previous != i64::MIN).then_some(previous as i32);
    if state_changed(previous, hr) {
        let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
            emit(
                STATE_TARGET,
                state_level(hr),
                "gfx.device_state",
                state_fields(hr, previous),
            );
        }));
    }
    hr
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::sinks::emit::take_captured;
    use serde_json::json;

    static SEEN: AtomicUsize = AtomicUsize::new(0);
    static TEST_ANSWER: AtomicI64 = AtomicI64::new(0);

    unsafe extern "stdcall-unwind" fn fake_test(this: *mut c_void) -> i32 {
        SEEN.store(this as usize, Ordering::SeqCst);
        TEST_ANSWER.load(Ordering::SeqCst) as i32
    }
    unsafe extern "stdcall-unwind" fn fake_reset(_this: *mut c_void, params: *mut c_void) -> i32 {
        SEEN.store(params as usize, Ordering::SeqCst);
        0
    }

    fn params_bytes() -> Vec<u8> {
        let mut b = vec![0u8; 0x40];
        for (off, v) in [
            (0x00usize, 1920u32),
            (0x04, 1080),
            (0x08, 22),
            (0x0c, 1),
            (0x10, 4),
            (0x18, 1),
            (0x20, 0),
            (0x30, 60),
            (0x34, 1),
        ] {
            b[off..off + 4].copy_from_slice(&v.to_le_bytes());
        }
        b
    }

    /// One test owns the originals and the last-state cell. It walks the
    /// loss and recovery: OK, lost, lost again (silent), needs reset,
    /// a reset, OK again.
    #[test]
    fn loss_and_recovery_are_reported_once_each_and_reset_reports_the_mode() {
        ORIG_TEST.store(fake_test as *const () as usize, Ordering::SeqCst);
        ORIG_RESET.store(fake_reset as *const () as usize, Ordering::SeqCst);
        LAST_STATE.store(i64::MIN, Ordering::SeqCst);
        let _ = take_captured();

        let dev = 0x5000 as *mut c_void;
        let states = [
            (0, true),
            (0, false),
            (D3DERR_DEVICELOST, true),
            (D3DERR_DEVICELOST, false),
            (D3DERR_DEVICENOTRESET, true),
        ];
        for (answer, reported) in states {
            TEST_ANSWER.store(i64::from(answer), Ordering::SeqCst);
            let hr = unsafe { test_detour(dev) };
            assert_eq!(hr, answer, "the answer is passed through");
            let events = take_captured();
            assert_eq!(events.len(), usize::from(reported), "answer {answer:#x}");
            if reported {
                assert_eq!(events[0].target, STATE_TARGET);
            }
        }

        let bytes = params_bytes();
        let hr = unsafe { reset_detour(dev, bytes.as_ptr() as *mut c_void) };
        assert_eq!(hr, 0);
        assert_eq!(SEEN.load(Ordering::SeqCst), bytes.as_ptr() as usize);
        let events = take_captured();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].target, RESET_TARGET);
        assert_eq!(events[0].get("width"), Some(&json!(1920)));
        assert_eq!(events[0].get("windowed"), Some(&json!(false)));
        assert_eq!(events[0].get("format"), Some(&json!("X8R8G8B8")));

        // The reset forgot the last answer: OK is reported again.
        TEST_ANSWER.store(0, Ordering::SeqCst);
        unsafe { test_detour(dev) };
        let events = take_captured();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].get("hresult_name"), Some(&json!("S_OK")));
    }
}
