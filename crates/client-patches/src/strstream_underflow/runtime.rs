//! The probe, the vtable slot swap and the shim: everything that calls into
//! or writes to the loaded `msvcp80.dll`.

use core::ffi::{c_void, CStr};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use windows_sys::Win32::Foundation::HMODULE;
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows_sys::Win32::System::Memory::{VirtualProtect, PAGE_READWRITE};

use super::*;
use crate::counters::{bump, is_log_worthy};
use crate::log;
use crate::memory::{MemoryReader, ProcessMemory};

type Constructor = unsafe extern "thiscall-unwind" fn(*mut c_void, i32) -> *mut c_void;
type Destructor = unsafe extern "thiscall-unwind" fn(*mut c_void);
type PutN = unsafe extern "thiscall-unwind" fn(*mut c_void, *const u8, i32) -> i32;
type GetN = unsafe extern "thiscall-unwind" fn(*mut c_void, *mut u8, i32) -> i32;
type Underflow = unsafe extern "thiscall-unwind" fn(*mut c_void) -> i32;

const CONSTRUCTOR: &CStr = c"??0strstreambuf@std@@QAE@H@Z";
const DESTRUCTOR: &CStr = c"??1strstreambuf@std@@UAE@XZ";
const PUT_N: &CStr = c"?sputn@?$basic_streambuf@DU?$char_traits@D@std@@@std@@QAEHPBDH@Z";
const GET_N: &CStr = c"?sgetn@?$basic_streambuf@DU?$char_traits@D@std@@@std@@QAEHPADH@Z";
const UNDERFLOW: &CStr = c"?underflow@strstreambuf@std@@MAEHXZ";
const VTABLE: &CStr = c"??_7strstreambuf@std@@6B@";

/// The runtime's own `underflow`, set before the slot points at the shim.
static ORIGINAL: AtomicUsize = AtomicUsize::new(0);
/// Underflows where the shim had to raise the mark.
static RAISED: AtomicU64 = AtomicU64::new(0);

/// The exports the probe and the swap use.
struct Runtime {
    constructor: Constructor,
    destructor: Destructor,
    put_n: PutN,
    get_n: GetN,
    underflow: usize,
    vtable: usize,
}

fn module(name: &str) -> Option<HMODULE> {
    let wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
    // SAFETY: a NUL-terminated string that outlives the call.
    let handle = unsafe { GetModuleHandleW(wide.as_ptr()) };
    (!handle.is_null()).then_some(handle)
}

fn export(module: HMODULE, name: &CStr) -> Option<usize> {
    // SAFETY: a loaded module and a NUL-terminated name.
    unsafe { GetProcAddress(module, name.as_ptr().cast()) }.map(|f| f as usize)
}

/// Wine's `ntdll.dll` exports `wine_get_version`; Windows' does not.
fn under_wine() -> bool {
    module("ntdll.dll").is_some_and(|ntdll| export(ntdll, c"wine_get_version").is_some())
}

fn runtime() -> Result<Runtime, &'static str> {
    let dll = module("msvcp80.dll").ok_or("msvcp80.dll is not loaded")?;
    let need = |name| export(dll, name).ok_or("msvcp80.dll lacks a strstreambuf export");
    // SAFETY: each address is the export of that name, whose signature
    // the type states (thiscall member functions of MSVC 8's library).
    unsafe {
        Ok(Runtime {
            constructor: core::mem::transmute::<usize, Constructor>(need(CONSTRUCTOR)?),
            destructor: core::mem::transmute::<usize, Destructor>(need(DESTRUCTOR)?),
            put_n: core::mem::transmute::<usize, PutN>(need(PUT_N)?),
            get_n: core::mem::transmute::<usize, GetN>(need(GET_N)?),
            underflow: need(UNDERFLOW)?,
            vtable: need(VTABLE)?,
        })
    }
}

/// Write [`PROBE`] into a fresh dynamic `strstreambuf` and read it back.
/// `None` when a call faulted or raised.
fn round_trip(runtime: &Runtime) -> Option<RoundTrip> {
    // Room for the object several times over, zeroed and aligned.
    let mut object = [0u32; 64];
    let this = object.as_mut_ptr().cast::<c_void>();
    let field = |offset: usize| ProcessMemory.read_u32(this as usize + offset);
    let mut back = [0u8; PROBE.len()];
    let len = PROBE.len() as i32;
    // SAFETY: the object outlives the calls and is only reached through
    // `this`; a fault or a C++ exception in the runtime ends the guard.
    microseh::try_seh(|| unsafe {
        (runtime.constructor)(this, 0);
        let written = (runtime.put_n)(this, PROBE.as_ptr(), len);
        // The pointers are the runtime's; a layout that is not the one
        // expected must fail these reads, not fault.
        let first = field(layout::PUT_FIRST_PTR).and_then(|p| ProcessMemory.read_u32(p as usize));
        let next = field(layout::PUT_NEXT_PTR).and_then(|p| ProcessMemory.read_u32(p as usize));
        let mark = field(layout::SEEK_HIGH);
        let read = (runtime.get_n)(this, back.as_mut_ptr(), len);
        (runtime.destructor)(this);
        RoundTrip {
            written,
            read,
            intact: back == PROBE,
            put_span: first
                .zip(next)
                .filter(|(first, _)| *first != 0)
                .and_then(|(first, next)| next.checked_sub(first)),
            mark_at_start: first
                .zip(mark)
                .map(|(first, mark)| first != 0 && first == mark),
        }
    })
    .ok()
}

/// Point `slot` at `value`.
///
/// # Safety
///
/// `slot` must be a pointer-sized vtable entry, and `value` a function of
/// that entry's signature that stays loaded.
unsafe fn write_slot(slot: usize, value: usize) -> bool {
    let mut old = 0;
    // SAFETY: one pointer in a loaded module's read-only data, made
    // writable for the write and put back after.
    unsafe {
        if VirtualProtect(slot as *const c_void, 4, PAGE_READWRITE, &mut old) == 0 {
            return false;
        }
        (slot as *mut usize).write_volatile(value);
        let mut ignored = 0;
        VirtualProtect(slot as *const c_void, 4, old, &mut ignored);
    }
    true
}

/// Probe the runtime and install the shim if it has the fault. Called
/// once, from the bootstrap thread, before the client opens its cache.
pub(crate) fn repair() -> Outcome {
    if !under_wine() {
        return Outcome::NotWine;
    }
    let runtime = match runtime() {
        Ok(runtime) => runtime,
        Err(what) => return Outcome::RuntimeIncomplete(what),
    };
    let Some(before) = round_trip(&runtime) else {
        return Outcome::ProbeFaulted;
    };
    match verdict(&before) {
        Verdict::Healthy => return Outcome::Healthy,
        Verdict::LosesWrites => {}
        other => return Outcome::NotRepairable(other, before),
    }

    let slots: Option<Vec<u32>> = (0..VTABLE_SLOTS)
        .map(|i| ProcessMemory.read_u32(runtime.vtable + i * 4))
        .collect();
    let Some(index) = slots.and_then(|slots| find_slot(&slots, runtime.underflow as u32)) else {
        return Outcome::SlotUnavailable("was not found in the vtable");
    };
    let slot = runtime.vtable + index * 4;

    // Published before the swap: a call can arrive the moment it lands.
    ORIGINAL.store(runtime.underflow, Ordering::Release);
    // SAFETY: the slot holds `underflow`, and the shim has its signature.
    if !unsafe { write_slot(slot, underflow_shim as *const () as usize) } {
        return Outcome::SlotUnavailable("could not be made writable");
    }
    match round_trip(&runtime).filter(|after| verdict(after) == Verdict::Healthy) {
        Some(_) => Outcome::Repaired {
            before,
            slot: index,
        },
        None => {
            // SAFETY: restoring the value read from this slot.
            unsafe { write_slot(slot, runtime.underflow) };
            Outcome::StillBroken(before)
        }
    }
}

/// `int strstreambuf::underflow()`: raise `_Seekhigh` to the put pointer
/// when writes have passed it, then let the runtime's own function run.
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn underflow_shim(this: *mut c_void) -> i32 {
    let original = ORIGINAL.load(Ordering::Acquire);
    if original == 0 {
        // EOF. Unreachable: the slot points here only after the store.
        return -1;
    }
    // SAFETY: `this` is the live `strstreambuf` whose `underflow` was
    // called, with the fields the probe checked on this runtime.
    unsafe {
        let base = this.cast::<u8>();
        let put_next = base
            .add(layout::PUT_NEXT_PTR)
            .cast::<*const u32>()
            .read_unaligned();
        if !put_next.is_null() {
            let mark = base.add(layout::SEEK_HIGH).cast::<u32>();
            if let Some(value) = raised(mark.read_unaligned(), put_next.read_unaligned()) {
                mark.write_unaligned(value);
                let n = bump(&RAISED);
                if is_log_worthy(n) {
                    log::line(format_args!(
                        "strstream: repaired {n} read(s) after a write"
                    ));
                }
            }
        }
        let original: Underflow = core::mem::transmute::<usize, Underflow>(original);
        original(this)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Stands in for the runtime's `underflow`: reports the mark it was
    /// called with.
    unsafe extern "thiscall-unwind" fn report_mark(this: *mut c_void) -> i32 {
        unsafe {
            this.cast::<u8>()
                .add(layout::SEEK_HIGH)
                .cast::<i32>()
                .read_unaligned()
        }
    }

    /// A `strstreambuf`-shaped object with the put pointer at `put_next`
    /// and the mark at `mark`.
    fn object(put_next: &u32, mark: u32) -> [u32; 32] {
        let mut words = [0u32; 32];
        words[layout::PUT_NEXT_PTR / 4] = put_next as *const u32 as u32;
        words[layout::SEEK_HIGH / 4] = mark;
        words
    }

    /// The runtime's function runs with the mark where Microsoft's
    /// `underflow` would have put it, and only then.
    #[test]
    fn the_shim_raises_the_mark_before_the_runtime_runs() {
        ORIGINAL.store(report_mark as *const () as usize, Ordering::SeqCst);

        // Four bytes written past a mark still at the buffer's start.
        let put_next = 0x5004u32;
        let mut words = object(&put_next, 0x5000);
        let seen = unsafe { underflow_shim(words.as_mut_ptr().cast()) };
        assert_eq!(seen, 0x5004, "the runtime saw the raised mark");
        assert_eq!(words[layout::SEEK_HIGH / 4], 0x5004);

        // A mark already past the put pointer (after a seek) stays.
        let mut words = object(&put_next, 0x5010);
        assert_eq!(unsafe { underflow_shim(words.as_mut_ptr().cast()) }, 0x5010);

        // A read-only buffer has no put pointer.
        let none = 0u32;
        let mut words = object(&none, 0x5000);
        assert_eq!(unsafe { underflow_shim(words.as_mut_ptr().cast()) }, 0x5000);

        // A buffer whose put-pointer cell is not set up at all.
        let mut words = [0u32; 32];
        words[layout::SEEK_HIGH / 4] = 0x5000;
        assert_eq!(unsafe { underflow_shim(words.as_mut_ptr().cast()) }, 0x5000);
    }

    /// On Windows proper the runtime is never probed or written.
    #[test]
    fn windows_is_left_alone() {
        if !under_wine() {
            assert_eq!(repair(), Outcome::NotWine);
        }
    }
}
