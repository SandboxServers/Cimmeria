//! The cooked-data cache: what version the client reads out of each cache
//! PAK, whether that read worked, and what it holds when it asks the server.
//!
//! A `ZipStorageBase` (one per cooked category, `LibCategory<..., ZipStorage,
//! ServerSource>`) keeps the category's version at `+0x24`. It is filled from
//! the PAK's `MetaData` entry when the archive is opened, replaced by every
//! `onVersionInfo`, and sent back in `versionInfoRequest` at login. A client
//! whose read fails keeps 0 there and is resynced in full on every login,
//! with a valid cache on disk the whole time (seen 2026-10-04 under Wine on
//! macOS). The server only sees the 0. These hooks say which step failed.
//!
//! | Function | Address | Signature |
//! |---|---|---|
//! | `ZipStorageBase` read version | `0x00478f00` | `thiscall(this, u32* out, CZipArchive*)`, `ret 8` |
//! | `ZipStorageBase` read entry | `0x00478e10` | `thiscall(this, stream*, CZipArchive*, const wchar_t* name)`, `ret 0xc`, `bool` |
//! | `CZipArchive::FindFile` | `0x01396900` | `thiscall(archive, const wchar_t* name, int case, bool name_only)`, `ret 0xc`, `u16` (`0xffff` = not found) |
//! | `CZipArchive::ExtractFile` (to memory) | `0x01398af0` | `thiscall(archive, index, CZipMemFile*, bool, u32 buffer)`, `ret 0x10`, `bool` |
//! | `ServerSource_SetVersion` | `0x00479e90` | `thiscall(this, const u32* version)`, `ret 4` |
//!
//! The read (`0x00478f00`) builds a stream, calls read entry with the name
//! `MetaData`, and copies four bytes to `out` only if that returns true.
//! Read entry is `FindFile(name, 0, true)`, then `ExtractFile` into a memory
//! file, then a length check. Its three callers are the tail of
//! `ZipStorageBase::OpenArchive` (`0x004798e6`, with `out = this + 0x24`),
//! `0x00479336` and the source-archive update (`0x0047a36e`). `FindFile` has
//! two call sites and `ExtractFile` one, all in this code, so hooking the
//! library functions observes nothing else. Checked against the QA `SGW.exe`
//! on 2026-10-04 (function entries, `ret N`, the call sites above).
//!
//! `OpenArchive` itself (`0x00479340`) is not hooked: every entry read and
//! write calls it, and a checked memory read there would cost a full resync
//! tens of thousands of them.
//!
//! Events:
//!
//! - `client.cooked.version_read`: one per read. `outcome` is `read`,
//!   `metadata_entry_not_found`, `metadata_extract_failed`, `metadata_empty`,
//!   `entry_read_failed` or `entry_read_not_attempted`. `warn` unless `read`.
//! - `client.cooked.version_set`: one per `onVersionInfo` stamp.
//! - `client.cooked.versions_held`: once per login, when the first
//!   `versionInfoRequest` leaves (see `net_out`): the version every known
//!   storage holds at that moment.
//!
//! The PAK name is the first `std::wstring` of the vector at `this + 0`
//! (element size `0x1c`, begin at `+0x04`, end at `+0x08`).

use serde_json::json;

use crate::hooks::entity_trace::Fields;

pub(super) const ADDR_READ_VERSION: usize = 0x0047_8f00;
pub(super) const ADDR_READ_ENTRY: usize = 0x0047_8e10;
pub(super) const ADDR_FIND_FILE: usize = 0x0139_6900;
pub(super) const ADDR_EXTRACT_FILE: usize = 0x0139_8af0;
pub(super) const ADDR_SET_VERSION: usize = 0x0047_9e90;

pub(crate) const TARGET_READ: &str = "client.cooked.version_read";
pub(crate) const TARGET_SET: &str = "client.cooked.version_set";
pub(crate) const TARGET_HELD: &str = "client.cooked.versions_held";

/// `ZipStorageBase` field offsets.
pub(crate) mod layout {
    /// `std::vector<std::wstring>` of PAK names: begin pointer.
    pub const NAMES_BEGIN: u32 = 0x04;
    /// The same vector's end pointer.
    pub const NAMES_END: u32 = 0x08;
    /// The category's version, as sent in `versionInfoRequest`.
    pub const VERSION: u32 = 0x24;
}

/// `CZipArchive::FindFile`'s "no such entry".
pub(crate) const NOT_FOUND: u16 = 0xffff;

/// What the calls under one version read did. `None` means the call was not
/// seen: it did not happen, or its hook is not installed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ReadProbe {
    pub find_index: Option<u16>,
    pub extract_ok: Option<bool>,
    pub entry_ok: Option<bool>,
}

/// A version as the cache protocol uses it. The server stamps the bitwise
/// NOT of its version while a resync is in flight, which has the top bit set
/// for every real version.
pub(crate) fn version_kind(version: u32) -> &'static str {
    match version {
        0 => "zero",
        v if v >= 0x8000_0000 => "resync_pending",
        _ => "real",
    }
}

/// Why a version read ended as it did.
pub(crate) fn read_outcome(probe: ReadProbe) -> &'static str {
    match (probe.find_index, probe.extract_ok, probe.entry_ok) {
        (_, _, Some(true)) => "read",
        (Some(NOT_FOUND), _, _) => "metadata_entry_not_found",
        (Some(_), Some(false), _) => "metadata_extract_failed",
        // Found and extracted, and still refused: the memory file was empty.
        (Some(_), Some(true), Some(false)) => "metadata_empty",
        (None, None, None) => "entry_read_not_attempted",
        _ => "entry_read_failed",
    }
}

/// `info` for a version that was read, `warn` for one that was not: the
/// client keeps whatever it had, usually 0, and says nothing.
pub(crate) fn read_level(probe: ReadProbe) -> &'static str {
    if probe.entry_ok == Some(true) {
        "info"
    } else {
        "warn"
    }
}

/// The fields of one `client.cooked.version_read`.
pub(crate) fn version_read_fields(
    pak: Option<&str>,
    before: Option<u32>,
    after: Option<u32>,
    probe: ReadProbe,
) -> Fields {
    let mut f: Fields = vec![
        ("pak", json!(pak)),
        ("outcome", json!(read_outcome(probe))),
        ("version", json!(after)),
        ("version_kind", json!(after.map(version_kind))),
    ];
    if before != after {
        f.push(("previous", json!(before)));
    }
    if let Some(index) = probe.find_index.filter(|i| *i != NOT_FOUND) {
        f.push(("entry_index", json!(index)));
    }
    f
}

/// The fields of one `client.cooked.version_set`.
pub(crate) fn version_set_fields(pak: Option<&str>, previous: Option<u32>, version: u32) -> Fields {
    vec![
        ("pak", json!(pak)),
        ("version", json!(version)),
        ("version_kind", json!(version_kind(version))),
        ("previous", json!(previous)),
    ]
}

/// The fields of one `client.cooked.versions_held`: every storage's version
/// when the client asks the server, and how many of them are 0.
pub(crate) fn versions_held_fields(held: &[(String, Option<u32>)]) -> Fields {
    let count = |kind: &str| {
        held.iter()
            .filter(|(_, v)| v.map(version_kind) == Some(kind))
            .count()
    };
    let versions: serde_json::Map<String, serde_json::Value> = held
        .iter()
        .map(|(pak, version)| (pak.clone(), json!(version)))
        .collect();
    vec![
        ("storages", json!(held.len())),
        ("zero", json!(count("zero"))),
        ("resync_pending", json!(count("resync_pending"))),
        ("real", json!(count("real"))),
        ("versions", serde_json::Value::Object(versions)),
    ]
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) use x86::{install_all, note_version_request};

#[cfg(all(target_os = "windows", target_arch = "x86"))]
mod x86 {
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

    fn note(update: impl FnOnce(&mut ReadProbe)) {
        PROBE.with(|c| {
            if let Some(mut probe) = c.get() {
                update(&mut probe);
                c.set(Some(probe));
            }
        });
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
            PROBE.with(|c| c.set(Some(ReadProbe::default())));
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

    type ReadEntryFn = unsafe extern "thiscall-unwind" fn(
        *mut c_void,
        *mut c_void,
        *mut c_void,
        *const u16,
    ) -> u32;

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
        // An exception out of the extraction leaves `extract_ok` unset, which
        // the outcome reports as a failed entry read.
        let result = original(archive, index, memory_file, flag, buffer_size);
        note(|probe| probe.extract_ok = Some(result & 0xff != 0));
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get(f: &Fields, key: &str) -> Option<serde_json::Value> {
        f.iter().find(|(k, _)| *k == key).map(|(_, v)| v.clone())
    }

    #[test]
    fn a_version_is_zero_real_or_the_servers_resync_placeholder() {
        assert_eq!(version_kind(0), "zero");
        assert_eq!(version_kind(5802), "real");
        assert_eq!(version_kind(0x7fff_ffff), "real");
        // The server stamps `!version` while it pushes: `!5802`.
        assert_eq!(version_kind(!5802), "resync_pending");
        assert_eq!(version_kind(4_294_961_493), "resync_pending");
    }

    #[test]
    fn the_outcome_names_the_step_that_failed() {
        let probe = |find_index, extract_ok, entry_ok| ReadProbe {
            find_index,
            extract_ok,
            entry_ok,
        };
        assert_eq!(read_outcome(probe(Some(4), Some(true), Some(true))), "read");
        assert_eq!(
            read_outcome(probe(Some(NOT_FOUND), None, Some(false))),
            "metadata_entry_not_found"
        );
        assert_eq!(
            read_outcome(probe(Some(4), Some(false), Some(false))),
            "metadata_extract_failed"
        );
        assert_eq!(
            read_outcome(probe(Some(4), Some(true), Some(false))),
            "metadata_empty"
        );
        // The entry read never ran, or none of the hooks under it is in.
        assert_eq!(
            read_outcome(ReadProbe::default()),
            "entry_read_not_attempted"
        );
        // Found, then an exception out of the extraction.
        assert_eq!(
            read_outcome(probe(Some(4), None, None)),
            "entry_read_failed"
        );
        // The library hooks are off; the entry read still reports itself.
        assert_eq!(read_outcome(probe(None, None, Some(true))), "read");
        assert_eq!(
            read_outcome(probe(None, None, Some(false))),
            "entry_read_failed"
        );
    }

    #[test]
    fn a_failed_read_is_a_warning_that_keeps_the_stale_version_visible() {
        let failed = ReadProbe {
            find_index: Some(NOT_FOUND),
            extract_ok: None,
            entry_ok: Some(false),
        };
        assert_eq!(read_level(failed), "warn");
        let f = version_read_fields(Some("TextStrings.pak"), Some(0), Some(0), failed);
        assert_eq!(get(&f, "pak"), Some(json!("TextStrings.pak")));
        assert_eq!(get(&f, "outcome"), Some(json!("metadata_entry_not_found")));
        assert_eq!(get(&f, "version"), Some(json!(0)));
        assert_eq!(get(&f, "version_kind"), Some(json!("zero")));
        assert_eq!(get(&f, "previous"), None, "nothing changed");
        assert_eq!(get(&f, "entry_index"), None);
    }

    #[test]
    fn a_good_read_reports_the_version_and_where_the_entry_was() {
        let read = ReadProbe {
            find_index: Some(29_126),
            extract_ok: Some(true),
            entry_ok: Some(true),
        };
        assert_eq!(read_level(read), "info");
        let f = version_read_fields(Some("TextStrings.pak"), Some(0), Some(5802), read);
        assert_eq!(get(&f, "outcome"), Some(json!("read")));
        assert_eq!(get(&f, "version"), Some(json!(5802)));
        assert_eq!(get(&f, "version_kind"), Some(json!("real")));
        assert_eq!(get(&f, "previous"), Some(json!(0)));
        assert_eq!(get(&f, "entry_index"), Some(json!(29_126)));
        // An unreadable `out` pointer is reported as null, not as zero.
        let f = version_read_fields(None, None, None, read);
        assert_eq!(get(&f, "version"), Some(serde_json::Value::Null));
        assert_eq!(get(&f, "pak"), Some(serde_json::Value::Null));
    }

    #[test]
    fn a_stamp_carries_the_old_and_new_version() {
        let f = version_set_fields(Some("CookedDataItems.pak"), Some(0), !44_303);
        assert_eq!(get(&f, "version"), Some(json!(!44_303u32)));
        assert_eq!(get(&f, "version_kind"), Some(json!("resync_pending")));
        assert_eq!(get(&f, "previous"), Some(json!(0)));
    }

    #[test]
    fn the_login_snapshot_counts_what_the_client_holds() {
        let held = vec![
            ("TextStrings.pak".to_string(), Some(0)),
            ("CookedDataItems.pak".to_string(), Some(44_303)),
            ("CookedSciences.pak".to_string(), Some(!2202)),
            ("0x0badf00d".to_string(), None),
        ];
        let f = versions_held_fields(&held);
        assert_eq!(get(&f, "storages"), Some(json!(4)));
        assert_eq!(get(&f, "zero"), Some(json!(1)));
        assert_eq!(get(&f, "real"), Some(json!(1)));
        assert_eq!(get(&f, "resync_pending"), Some(json!(1)));
        let versions = get(&f, "versions").unwrap();
        assert_eq!(versions["TextStrings.pak"], json!(0));
        assert_eq!(versions["CookedDataItems.pak"], json!(44_303));
        assert_eq!(versions["0x0badf00d"], serde_json::Value::Null);
    }
}
