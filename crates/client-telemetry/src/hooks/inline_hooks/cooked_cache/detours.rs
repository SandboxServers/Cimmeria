//! The inline detours on the cache's version read and what it calls, and the
//! thread-local probe the calls under one read report into.

use std::cell::Cell;
use std::ffi::c_void;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use super::super::entity_lifecycle::guarded;
use super::*;
use crate::hooks::emit::emit;
use crate::hooks::entity_trace::map::{LiveMem, Mem};

static READ_VERSION_TRAMPOLINE: OnceLock<usize> = OnceLock::new();
static READ_ENTRY_TRAMPOLINE: OnceLock<usize> = OnceLock::new();
static FIND_FILE_TRAMPOLINE: OnceLock<usize> = OnceLock::new();
static EXTRACT_FILE_TRAMPOLINE: OnceLock<usize> = OnceLock::new();
static SET_VERSION_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

/// Longest PAK name read.
const MAX_NAME_CHARS: usize = 96;
/// There are 21 categories; anything far past that is not a storage.
const MAX_STORAGES: usize = 64;
/// The 21 requests of one login leave within milliseconds of each other.
const HELD_BURST_MS: u64 = 5_000;

/// Every storage a version was read from or set on.
static STORAGES: Mutex<Vec<u32>> = Mutex::new(Vec::new());
static STARTED: OnceLock<Instant> = OnceLock::new();
/// Milliseconds since `STARTED` of the last `versions_held`, plus one.
static LAST_HELD: AtomicU64 = AtomicU64::new(0);

thread_local! {
    /// Set while a version read runs, for the calls under it.
    static PROBE: Cell<Option<ReadProbe>> = const { Cell::new(None) };
}

/// Clears the probe on drop, including when a C++ exception unwinds
/// through the detour.
struct ProbeScope;

impl Drop for ProbeScope {
    fn drop(&mut self) {
        PROBE.with(|c| c.set(None));
    }
}

/// Record into the running version read's probe; nothing outside one.
pub(super) fn note(update: impl FnOnce(&mut ReadProbe)) {
    PROBE.with(|c| {
        if let Some(mut probe) = c.get() {
            update(&mut probe);
            c.set(Some(probe));
        }
    });
}

/// Whether this thread is inside a version read.
pub(super) fn probing() -> bool {
    PROBE.with(|c| c.get().is_some())
}

pub(in crate::hooks::inline_hooks) unsafe fn install_all(producer: &crate::queue::Producer) {
    let hooks: [(&str, usize, *mut c_void, &OnceLock<usize>); 5] = [
        (
            "cooked_read_version",
            ADDR_READ_VERSION,
            read_version_detour as *mut c_void,
            &READ_VERSION_TRAMPOLINE,
        ),
        (
            "cooked_read_entry",
            ADDR_READ_ENTRY,
            read_entry_detour as *mut c_void,
            &READ_ENTRY_TRAMPOLINE,
        ),
        (
            "cooked_zip_find_file",
            ADDR_FIND_FILE,
            find_file_detour as *mut c_void,
            &FIND_FILE_TRAMPOLINE,
        ),
        (
            "cooked_zip_extract_file",
            ADDR_EXTRACT_FILE,
            extract_file_detour as *mut c_void,
            &EXTRACT_FILE_TRAMPOLINE,
        ),
        (
            "cooked_set_version",
            ADDR_SET_VERSION,
            set_version_detour as *mut c_void,
            &SET_VERSION_TRAMPOLINE,
        ),
    ];
    for (name, addr, detour, slot) in hooks {
        super::super::install_one(producer, name, addr, detour, slot);
    }
    import_detours::install(producer);
}

/// Run `f` as if under a version read, and return what it recorded.
#[cfg(test)]
pub(super) fn probed<R>(f: impl FnOnce() -> R) -> (R, ReadProbe) {
    PROBE.with(|c| {
        c.set(Some(ReadProbe {
            crt: Some(CrtIo::default()),
            ..ReadProbe::default()
        }));
    });
    let _scope = ProbeScope;
    let result = f();
    (result, PROBE.with(Cell::get).unwrap_or_default())
}

fn word(addr: u32) -> Option<u32> {
    LiveMem.u32_at(addr)
}

/// The storage's first PAK name.
fn pak_name(this: u32) -> Option<String> {
    let begin = word(this.wrapping_add(layout::NAMES_BEGIN))?;
    let end = word(this.wrapping_add(layout::NAMES_END))?;
    if begin == 0 || end <= begin {
        return None;
    }
    crate::msvc_string::read_checked(
        begin as usize,
        crate::msvc_string::Width::Wide,
        MAX_NAME_CHARS,
    )
    .map(|d| d.text)
}

fn remember(this: u32) {
    if let Ok(mut storages) = STORAGES.lock() {
        if !storages.contains(&this) && storages.len() < MAX_STORAGES {
            storages.push(this);
        }
    }
}

type ReadVersionFn = unsafe extern "thiscall-unwind" fn(*mut c_void, *mut u32, *mut c_void);

/// `read version(out, archive)`.
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn read_version_detour(
    this: *mut c_void,
    out: *mut u32,
    archive: *mut c_void,
) {
    let Some(&t) = READ_VERSION_TRAMPOLINE.get() else {
        return;
    };
    let original: ReadVersionFn = unsafe { std::mem::transmute(t) };
    let before = guarded(|| word(out as u32)).flatten();
    let probe = {
        PROBE.with(|c| {
            c.set(Some(ReadProbe {
                // Zero calls is a finding only when the calls are watched.
                crt: import_detours::counting().then(CrtIo::default),
                ..ReadProbe::default()
            }));
        });
        let _scope = ProbeScope;
        original(this, out, archive);
        PROBE.with(Cell::get).unwrap_or_default()
    };
    guarded(|| {
        let storage = this as u32;
        remember(storage);
        let after = word(out as u32);
        emit(
            TARGET_READ,
            read_level(probe),
            version_read_fields(pak_name(storage).as_deref(), before, after, probe),
        );
    });
}

type ReadEntryFn =
    unsafe extern "thiscall-unwind" fn(*mut c_void, *mut c_void, *mut c_void, *const u16) -> u32;

/// `read entry(stream, archive, name)`. Only the low byte of the result
/// is the `bool`; the whole register goes back untouched.
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn read_entry_detour(
    this: *mut c_void,
    stream: *mut c_void,
    archive: *mut c_void,
    name: *const u16,
) -> u32 {
    let Some(&t) = READ_ENTRY_TRAMPOLINE.get() else {
        return 0;
    };
    let original: ReadEntryFn = unsafe { std::mem::transmute(t) };
    let result = original(this, stream, archive, name);
    note(|probe| probe.entry_ok = Some(result & 0xff != 0));
    result
}

type FindFileFn = unsafe extern "thiscall-unwind" fn(*mut c_void, *const u16, i32, u32) -> u32;

/// `CZipArchive::FindFile(name, case, name_only)`.
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn find_file_detour(
    archive: *mut c_void,
    name: *const u16,
    case_sensitivity: i32,
    name_only: u32,
) -> u32 {
    let Some(&t) = FIND_FILE_TRAMPOLINE.get() else {
        return u32::from(NOT_FOUND);
    };
    let original: FindFileFn = unsafe { std::mem::transmute(t) };
    let result = original(archive, name, case_sensitivity, name_only);
    note(|probe| probe.find_index = Some(result as u16));
    result
}

type ExtractFileFn =
    unsafe extern "thiscall-unwind" fn(*mut c_void, u32, *mut c_void, u32, u32) -> u32;

/// `CZipArchive::ExtractFile(index, memory file, flag, buffer size)`.
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn extract_file_detour(
    archive: *mut c_void,
    index: u32,
    memory_file: *mut c_void,
    flag: u32,
    buffer_size: u32,
) -> u32 {
    let Some(&t) = EXTRACT_FILE_TRAMPOLINE.get() else {
        return 0;
    };
    let original: ExtractFileFn = unsafe { std::mem::transmute(t) };
    if !probing() {
        return original(archive, index, memory_file, flag, buffer_size);
    }
    // The directory record is there before the extraction and says what the
    // extraction should produce.
    if let Some(zip) = guarded(|| zip_entry(&LiveMem, archive as u32, index & 0xffff)) {
        note(|probe| probe.zip = zip);
    }
    // An exception out of the extraction leaves `extract_ok` unset, which
    // the outcome reports as a failed entry read.
    let result = original(archive, index, memory_file, flag, buffer_size);
    let held = guarded(|| extracted(&LiveMem, memory_file as u32)).flatten();
    note(|probe| {
        probe.extract_ok = Some(result & 0xff != 0);
        probe.extracted = held;
    });
    result
}

type SetVersionFn = unsafe extern "thiscall-unwind" fn(*mut c_void, *const u32);

/// `ServerSource_SetVersion(&version)`.
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn set_version_detour(this: *mut c_void, version: *const u32) {
    let Some(&t) = SET_VERSION_TRAMPOLINE.get() else {
        return;
    };
    let original: SetVersionFn = unsafe { std::mem::transmute(t) };
    // Read before the call: the original overwrites the old version.
    let seen = guarded(|| {
        let storage = this as u32;
        (
            word(storage.wrapping_add(layout::VERSION)),
            word(version as u32),
        )
    });
    original(this, version);
    guarded(|| {
        let storage = this as u32;
        remember(storage);
        if let Some((previous, Some(new))) = seen {
            emit(
                TARGET_SET,
                "info",
                version_set_fields(pak_name(storage).as_deref(), previous, new),
            );
        }
    });
}

/// Called when a `versionInfoRequest` leaves. Reports, once per login,
/// the version every known storage holds: what the 21 requests carry.
pub(in crate::hooks::inline_hooks) fn note_version_request() {
    let now = STARTED.get_or_init(Instant::now).elapsed().as_millis() as u64 + 1;
    let last = LAST_HELD.load(Ordering::Relaxed);
    if last != 0 && now.saturating_sub(last) < HELD_BURST_MS {
        return;
    }
    LAST_HELD.store(now, Ordering::Relaxed);
    let storages = match STORAGES.lock() {
        Ok(storages) => storages.clone(),
        Err(_) => return,
    };
    let held: Vec<(String, Option<u32>)> = storages
        .iter()
        .map(|&storage| {
            (
                pak_name(storage).unwrap_or_else(|| format!("0x{storage:08x}")),
                word(storage.wrapping_add(layout::VERSION)),
            )
        })
        .collect();
    let zero = held.iter().filter(|(_, v)| *v == Some(0)).count();
    // A client holding nothing asks for everything: worth keeping.
    let level = if zero > 0 { "warn" } else { "info" };
    emit(TARGET_HELD, level, versions_held_fields(&held));
}
