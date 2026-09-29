//! DirectInput hooks that feed the lab's synthetic input to the game.
//!
//! SGW.exe reads its keyboard and mouse through DINPUT8 (CEGUI is linked
//! into the exe and fed from that input), so posted window messages never
//! reach the UI. The lab injects at the device instead:
//!
//! 1. `DirectInput8Create` is swapped in SGW.exe's IAT (slot
//!    `0x017EF024`). Our detour calls the real one, then swaps
//!    `IDirectInput8::CreateDevice` (vtable index 3) on the object it
//!    returns.
//! 2. `CreateDevice` records which device is the system keyboard or mouse
//!    (by GUID) and swaps `GetDeviceState` (index 9) and `GetDeviceData`
//!    (index 10) on that device's vtable.
//! 3. Those two merge [`super::state::InputState`] into what the game
//!    reads, and report success even when the device is not acquired
//!    because the window is in the background, so the lab can drive a
//!    client that does not have focus.
//!
//! The game may create its devices before the DLL's boot thread gets
//! here, so step 1 can miss them. [`install`] therefore also *primes* the
//! device vtables: it creates its own throwaway keyboard and mouse (both
//! the ANSI and wide interfaces) and swaps methods 9 and 10 on their
//! vtables, which every device of that class shares. A device that was
//! never seen at creation is classified on its first read by
//! `GetCapabilities` (`DIDEVCAPS::dwDevType`).
//!
//! Every original is looked up by the object's vtable, so devices that
//! share a vtable class share one swap and one original.

use core::ffi::c_void;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use super::state::{
    classify_dev_type, classify_guid, DeviceKind, GUID_SYS_KEYBOARD, GUID_SYS_MOUSE,
};

/// `DirectInput8Create` import slot in the QA SGW.exe (image base
/// `0x00400000`, ASLR off), from its import directory.
pub const IAT_DIRECT_INPUT8_CREATE: usize = 0x017E_F024;

const VT_CREATE_DEVICE: usize = 3;
const VT_GET_CAPABILITIES: usize = 3;
const VT_ACQUIRE: usize = 7;
const VT_GET_DEVICE_STATE: usize = 9;
const VT_GET_DEVICE_DATA: usize = 10;

const DI_OK: i32 = 0;

/// Calls seen, for `input_status`: proves the game reads the hooked path.
pub static CALLS_STATE: AtomicU64 = AtomicU64::new(0);
/// `Acquire` calls that failed (background window) and were reported as
/// success because the lab has virtual focus on.
pub static ACQUIRES_FAKED: AtomicU64 = AtomicU64::new(0);
pub static CALLS_DATA: AtomicU64 = AtomicU64::new(0);
pub static DEVICES_SEEN: AtomicU64 = AtomicU64::new(0);
/// Buffered reads per device kind, and synthetic records handed over.
pub static DATA_READS_KEYBOARD: AtomicU64 = AtomicU64::new(0);
pub static DATA_READS_MOUSE: AtomicU64 = AtomicU64::new(0);
pub static EVENTS_DELIVERED: AtomicU64 = AtomicU64::new(0);
/// The last buffered read's arguments: record size, capacity, flags.
pub static LAST_DATA_CALL: AtomicU64 = AtomicU64::new(0);

static ORIG_DI8_CREATE: AtomicUsize = AtomicUsize::new(0);

#[derive(Default)]
struct Registry {
    /// Device object -> kind (`None`: probed, not a keyboard or mouse).
    devices: HashMap<usize, Option<DeviceKind>>,
    /// (vtable, index) -> original method.
    originals: HashMap<(usize, usize), usize>,
}

fn registry() -> &'static Mutex<Registry> {
    static R: OnceLock<Mutex<Registry>> = OnceLock::new();
    R.get_or_init(|| Mutex::new(Registry::default()))
}

/// Which device (if any) an object is: recorded at creation, else asked
/// once through `GetCapabilities` and cached.
pub fn device_kind(obj: usize) -> Option<DeviceKind> {
    if let Some(known) = registry().lock().ok()?.devices.get(&obj).copied() {
        return known;
    }
    // SAFETY: `obj` is the live device whose hooked method is running.
    let kind = unsafe { probe_kind(obj) };
    if let Ok(mut r) = registry().lock() {
        r.devices.insert(obj, kind);
    }
    if kind.is_some() {
        DEVICES_SEEN.fetch_add(1, Ordering::Relaxed);
    }
    kind
}

type FnGetCapabilities = unsafe extern "stdcall" fn(this: usize, caps: *mut u32) -> i32;

/// # Safety
/// `obj` must be a live IDirectInputDevice8.
unsafe fn probe_kind(obj: usize) -> Option<DeviceKind> {
    // DIDEVCAPS: dwSize, dwFlags, dwDevType, ... (11 DWORDs, A and W alike).
    let mut caps = [0u32; 11];
    caps[0] = 44;
    // SAFETY: vtable[3] of a device is GetCapabilities (not hooked).
    let f: FnGetCapabilities = unsafe {
        core::mem::transmute(*((vtable_of(obj) + VT_GET_CAPABILITIES * 4) as *const usize))
    };
    // SAFETY: `caps` is a correctly sized DIDEVCAPS.
    if unsafe { f(obj, caps.as_mut_ptr()) } < 0 {
        return None;
    }
    classify_dev_type(caps[2])
}

/// Devices recorded so far, for `input_status`.
pub fn devices() -> Vec<(usize, DeviceKind)> {
    registry()
        .lock()
        .map(|r| {
            r.devices
                .iter()
                .filter_map(|(&o, &k)| k.map(|k| (o, k)))
                .collect()
        })
        .unwrap_or_default()
}

/// # Safety
/// `obj` must be a live COM object pointer.
unsafe fn vtable_of(obj: usize) -> usize {
    unsafe { *(obj as *const usize) }
}

fn original(vtable: usize, index: usize) -> usize {
    registry()
        .lock()
        .ok()
        .and_then(|r| r.originals.get(&(vtable, index)).copied())
        .unwrap_or(0)
}

/// Swap `vtable[index]` once per vtable.
///
/// # Safety
/// `vtable` must be a live COM vtable with at least `index + 1` entries,
/// and `detour` must match that method's stdcall signature.
unsafe fn hook_vtable_once(vtable: usize, index: usize, detour: usize) {
    let Ok(mut r) = registry().lock() else {
        return;
    };
    if r.originals.contains_key(&(vtable, index)) {
        return;
    }
    let slot = vtable + index * core::mem::size_of::<usize>();
    // SAFETY: forwarded contract.
    if let Ok(orig) = unsafe { crate::hooks::primitives::swap_vtable_slot(slot, detour) } {
        r.originals.insert((vtable, index), orig);
    }
}

/// Install the `DirectInput8Create` IAT swap. Must run before the game
/// creates its devices (the DLL is injected into a suspended process).
///
/// # Safety
/// Main-thread or pre-resume only; the slot must be SGW.exe's IAT entry.
pub unsafe fn install() -> Result<(), String> {
    use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
    let wide: Vec<u16> = "dinput8.dll".encode_utf16().chain(Some(0)).collect();
    // SAFETY: NUL-terminated strings; no reference kept.
    let expected = unsafe {
        let m = GetModuleHandleW(wide.as_ptr());
        if m.is_null() {
            return Err("dinput8.dll not loaded".into());
        }
        GetProcAddress(m, c"DirectInput8Create".as_ptr().cast()).map(|f| f as usize)
    }
    .ok_or("DirectInput8Create not exported")?;
    let current = cimmeria_client_hookgate::os::read_bytes(IAT_DIRECT_INPUT8_CREATE, 4)
        .and_then(|b| b.try_into().ok())
        .map(|b: [u8; 4]| u32::from_le_bytes(b) as usize);
    if current != Some(expected) {
        return Err(format!(
            "IAT slot 0x{IAT_DIRECT_INPUT8_CREATE:08x} holds {current:x?}, expected 0x{expected:08x}"
        ));
    }
    ORIG_DI8_CREATE.store(expected, Ordering::Release);
    // SAFETY: slot verified above; detour is stdcall like the import.
    unsafe {
        crate::hooks::primitives::replace_iat_slot(
            IAT_DIRECT_INPUT8_CREATE,
            di8_create_detour as *const c_void as usize,
        )
    }
    .map_err(|e| format!("IAT swap failed: {e:?}"))?;
    // SAFETY: `expected` is the real DirectInput8Create.
    let primed = unsafe { prime_device_vtables(expected) };
    if primed == 0 {
        return Err("could not create a probe device to find DirectInput's vtables".into());
    }
    Ok(())
}

/// `IID_IDirectInput8A` / `IID_IDirectInput8W` in memory order.
const IID_DI8A: [u8; 16] = [
    0x30, 0x80, 0x79, 0xBF, 0x3A, 0x48, 0xA2, 0x4D, 0xAA, 0x99, 0x5D, 0x64, 0xED, 0x36, 0x97, 0x00,
];
const IID_DI8W: [u8; 16] = [
    0x31, 0x80, 0x79, 0xBF, 0x3A, 0x48, 0xA2, 0x4D, 0xAA, 0x99, 0x5D, 0x64, 0xED, 0x36, 0x97, 0x00,
];

/// Create our own DirectInput objects and a keyboard and mouse on each
/// (ANSI and wide), and swap the read methods on their vtables. The
/// objects are kept alive for the process lifetime. Returns how many
/// devices were primed.
///
/// # Safety
/// `di8_create` must be the real `DirectInput8Create`.
unsafe fn prime_device_vtables(di8_create: usize) -> usize {
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    let create: FnDi8Create = unsafe { core::mem::transmute(di8_create) };
    // SAFETY: NULL = this process's exe module.
    let hinst = unsafe { GetModuleHandleW(core::ptr::null()) };
    let mut primed = 0;
    for iid in [&IID_DI8A, &IID_DI8W] {
        let mut di: *mut c_void = core::ptr::null_mut();
        // SAFETY: documented DirectInput8Create call.
        let hr = unsafe {
            create(
                hinst as *mut c_void,
                0x0800,
                iid.as_ptr().cast(),
                &mut di,
                core::ptr::null_mut(),
            )
        };
        if hr < 0 || di.is_null() {
            continue;
        }
        let di = di as usize;
        // SAFETY: `di` is a live IDirectInput8; vtable[3] is CreateDevice.
        let create_device: FnCreateDevice = unsafe {
            core::mem::transmute(*((vtable_of(di) + VT_CREATE_DEVICE * 4) as *const usize))
        };
        for (guid, kind) in [
            (&GUID_SYS_KEYBOARD, DeviceKind::Keyboard),
            (&GUID_SYS_MOUSE, DeviceKind::Mouse),
        ] {
            let mut dev: usize = 0;
            // SAFETY: documented CreateDevice call.
            if unsafe { create_device(di, guid, &mut dev, core::ptr::null_mut()) } < 0 || dev == 0 {
                continue;
            }
            if let Ok(mut r) = registry().lock() {
                // Our probe device is never read by the game; record it so
                // no probe runs on it.
                r.devices.insert(dev, Some(kind));
            }
            // SAFETY: `dev` is a live IDirectInputDevice8.
            unsafe {
                let vt = vtable_of(dev);
                hook_vtable_once(vt, VT_ACQUIRE, acquire_detour as *const c_void as usize);
                hook_vtable_once(
                    vt,
                    VT_GET_DEVICE_STATE,
                    get_device_state_detour as *const c_void as usize,
                );
                hook_vtable_once(
                    vt,
                    VT_GET_DEVICE_DATA,
                    get_device_data_detour as *const c_void as usize,
                );
            }
            primed += 1;
        }
    }
    primed
}

type FnDi8Create = unsafe extern "stdcall" fn(
    hinst: *mut c_void,
    version: u32,
    riid: *const c_void,
    out: *mut *mut c_void,
    outer: *mut c_void,
) -> i32;

unsafe extern "stdcall" fn di8_create_detour(
    hinst: *mut c_void,
    version: u32,
    riid: *const c_void,
    out: *mut *mut c_void,
    outer: *mut c_void,
) -> i32 {
    let orig = ORIG_DI8_CREATE.load(Ordering::Acquire);
    // SAFETY: `orig` is the real DirectInput8Create.
    let hr = unsafe {
        core::mem::transmute::<usize, FnDi8Create>(orig)(hinst, version, riid, out, outer)
    };
    if hr >= 0 && !out.is_null() {
        // SAFETY: on success `*out` is a live IDirectInput8 object.
        unsafe {
            let obj = *out as usize;
            if obj != 0 {
                hook_vtable_once(
                    vtable_of(obj),
                    VT_CREATE_DEVICE,
                    create_device_detour as *const c_void as usize,
                );
            }
        }
    }
    hr
}

type FnCreateDevice = unsafe extern "stdcall" fn(
    this: usize,
    guid: *const [u8; 16],
    out: *mut usize,
    outer: *mut c_void,
) -> i32;

unsafe extern "stdcall" fn create_device_detour(
    this: usize,
    guid: *const [u8; 16],
    out: *mut usize,
    outer: *mut c_void,
) -> i32 {
    // SAFETY: `this` is the live IDirectInput8 our hook was installed on.
    let orig = original(unsafe { vtable_of(this) }, VT_CREATE_DEVICE);
    // SAFETY: `orig` is the real CreateDevice.
    let hr = unsafe { core::mem::transmute::<usize, FnCreateDevice>(orig)(this, guid, out, outer) };
    if hr < 0 || out.is_null() || guid.is_null() {
        return hr;
    }
    // SAFETY: success leaves a live device in `*out`; `guid` is the REFGUID.
    let (dev, kind) = unsafe { (*out, classify_guid(&*guid)) };
    if let (Some(kind), true) = (kind, dev != 0) {
        if let Ok(mut r) = registry().lock() {
            r.devices.insert(dev, Some(kind));
        }
        DEVICES_SEEN.fetch_add(1, Ordering::Relaxed);
        // SAFETY: `dev` is a live IDirectInputDevice8.
        unsafe {
            let vt = vtable_of(dev);
            hook_vtable_once(vt, VT_ACQUIRE, acquire_detour as *const c_void as usize);
            hook_vtable_once(
                vt,
                VT_GET_DEVICE_STATE,
                get_device_state_detour as *const c_void as usize,
            );
            hook_vtable_once(
                vt,
                VT_GET_DEVICE_DATA,
                get_device_data_detour as *const c_void as usize,
            );
        }
    }
    hr
}

type FnAcquire = unsafe extern "stdcall" fn(this: usize) -> i32;

/// `Acquire` fails for a foreground-only device while the window is in the
/// background, and the game then never reads it. Under virtual focus the
/// failure is reported as success; the read hooks stand in for the device.
unsafe extern "stdcall" fn acquire_detour(this: usize) -> i32 {
    // SAFETY: `this` is a live device whose vtable we swapped.
    let orig = original(unsafe { vtable_of(this) }, VT_ACQUIRE);
    // SAFETY: `orig` is the real Acquire.
    let hr = unsafe { core::mem::transmute::<usize, FnAcquire>(orig)(this) };
    if hr < 0 && super::virtual_focus_hwnd().is_some() && device_kind(this).is_some() {
        ACQUIRES_FAKED.fetch_add(1, Ordering::Relaxed);
        return DI_OK;
    }
    hr
}

type FnGetDeviceState = unsafe extern "stdcall" fn(this: usize, cb: u32, data: *mut u8) -> i32;

unsafe extern "stdcall" fn get_device_state_detour(this: usize, cb: u32, data: *mut u8) -> i32 {
    CALLS_STATE.fetch_add(1, Ordering::Relaxed);
    // SAFETY: `this` is a live device whose vtable we swapped.
    let orig = original(unsafe { vtable_of(this) }, VT_GET_DEVICE_STATE);
    // SAFETY: `orig` is the real GetDeviceState.
    let hr = unsafe { core::mem::transmute::<usize, FnGetDeviceState>(orig)(this, cb, data) };
    let Some(kind) = device_kind(this) else {
        return hr;
    };
    let Ok(mut st) = super::input_state().lock() else {
        return hr;
    };
    if !(st.is_active() || st.virtual_focus) || data.is_null() {
        return hr;
    }
    // SAFETY: the caller owns `cb` bytes at `data`.
    let buf = unsafe { core::slice::from_raw_parts_mut(data, cb as usize) };
    if hr < 0 {
        // Not acquired (background window): report only the lab's input.
        buf.fill(0);
    }
    match kind {
        DeviceKind::Keyboard => st.overlay_keyboard_state(buf),
        DeviceKind::Mouse => st.overlay_mouse_state(buf),
    }
    DI_OK
}

type FnGetDeviceData = unsafe extern "stdcall" fn(
    this: usize,
    cb: u32,
    rgdod: *mut u8,
    inout: *mut u32,
    flags: u32,
) -> i32;

unsafe extern "stdcall" fn get_device_data_detour(
    this: usize,
    cb: u32,
    rgdod: *mut u8,
    inout: *mut u32,
    flags: u32,
) -> i32 {
    CALLS_DATA.fetch_add(1, Ordering::Relaxed);
    // SAFETY: `this` is a live device whose vtable we swapped.
    let orig = original(unsafe { vtable_of(this) }, VT_GET_DEVICE_DATA);
    // SAFETY: `inout` is the caller's capacity/count.
    let capacity = if inout.is_null() {
        0
    } else {
        unsafe { *inout }
    };
    // SAFETY: `orig` is the real GetDeviceData.
    let mut hr = unsafe {
        core::mem::transmute::<usize, FnGetDeviceData>(orig)(this, cb, rgdod, inout, flags)
    };
    // DIGDD_PEEK (1) must not consume; a null buffer is a flush/count call.
    let Some(kind) = device_kind(this) else {
        return hr;
    };
    match kind {
        DeviceKind::Keyboard => DATA_READS_KEYBOARD.fetch_add(1, Ordering::Relaxed),
        DeviceKind::Mouse => DATA_READS_MOUSE.fetch_add(1, Ordering::Relaxed),
    };
    LAST_DATA_CALL.store(
        (u64::from(cb) << 48) | (u64::from(capacity & 0xFFFF) << 32) | u64::from(flags),
        Ordering::Relaxed,
    );
    if flags & 1 != 0 || rgdod.is_null() || inout.is_null() || cb < 16 {
        return hr;
    }
    let Ok(mut st) = super::input_state().lock() else {
        return hr;
    };
    if !(st.is_active() || st.virtual_focus) {
        return hr;
    }
    // SAFETY: on failure the count is meaningless; on success it is the
    // number of records the device wrote.
    let used = if hr < 0 { 0 } else { unsafe { *inout } };
    let room = capacity.saturating_sub(used) as usize;
    let events = st.take_buffered(kind, room);
    let seq_base = used;
    for (i, (ofs, value)) in events.iter().enumerate() {
        // DIDEVICEOBJECTDATA: dwOfs, dwData, dwTimeStamp, dwSequence[, uAppData].
        let at = (used as usize + i) * cb as usize;
        // SAFETY: `at + cb` is within the caller's `capacity * cb` buffer.
        unsafe {
            let rec = core::slice::from_raw_parts_mut(rgdod.add(at), cb as usize);
            rec.fill(0);
            rec[0..4].copy_from_slice(&ofs.to_le_bytes());
            rec[4..8].copy_from_slice(&value.to_le_bytes());
            rec[8..12].copy_from_slice(&tick_ms().to_le_bytes());
            rec[12..16].copy_from_slice(&(seq_base + i as u32 + 1).to_le_bytes());
        }
    }
    EVENTS_DELIVERED.fetch_add(events.len() as u64, Ordering::Relaxed);
    // SAFETY: `inout` is valid (checked above).
    unsafe { *inout = used + events.len() as u32 };
    if hr < 0 {
        hr = DI_OK;
    }
    hr
}

/// Milliseconds since the first synthetic event: DirectInput timestamps
/// only need to be ordered.
fn tick_ms() -> u32 {
    static START: OnceLock<std::time::Instant> = OnceLock::new();
    START
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_millis() as u32
}
