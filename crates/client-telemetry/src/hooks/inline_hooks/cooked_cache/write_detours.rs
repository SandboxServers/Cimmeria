//! The inline detours on the cache's write path; see [`write`](super::write).

use std::ffi::c_void;
use std::sync::{Mutex, OnceLock};

use super::detours::{pak_name, remember, word};
use super::layout;
use super::write::*;
use crate::hooks::emit::emit;
use crate::hooks::inline_hooks::entity_lifecycle::guarded;
use crate::hooks::sinks::{mem, text};

static WRITE_STREAM_TRAMPOLINE: OnceLock<usize> = OnceLock::new();
static COPY_SOURCE_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

/// Longest entry name or path read.
const MAX_NAME_CHARS: usize = 260;
/// Storages tracked for the once-per-version element report.
const MAX_STORAGES: usize = 64;

/// The version already reported for the elements of each storage.
static REPORTED: Mutex<Vec<(u32, Option<u32>)>> = Mutex::new(Vec::new());

pub(super) unsafe fn install(producer: &crate::queue::Producer) {
    super::super::install_one(
        producer,
        "cooked_write_stream",
        ADDR_WRITE_STREAM,
        write_stream_detour as *mut c_void,
        &WRITE_STREAM_TRAMPOLINE,
    );
    super::super::install_one(
        producer,
        "cooked_copy_source",
        ADDR_COPY_SOURCE,
        copy_source_detour as *mut c_void,
        &COPY_SOURCE_TRAMPOLINE,
    );
}

fn wide_string(addr: *const u16) -> Option<String> {
    mem::wide_at(addr as usize, MAX_NAME_CHARS).map(|s| text::decode_wide(&s.units))
}

/// Whether to report an element write of `storage` at `version`.
fn report_element(storage: u32, kind: EntryKind, ok: bool, version: Option<u32>) -> bool {
    let Ok(mut reported) = REPORTED.lock() else {
        return true;
    };
    let index = match reported.iter().position(|(s, _)| *s == storage) {
        Some(i) => i,
        None if reported.len() < MAX_STORAGES => {
            reported.push((storage, None));
            reported.len() - 1
        }
        None => return true,
    };
    should_report(kind, ok, &mut reported[index].1, version)
}

type WriteStreamFn =
    unsafe extern "thiscall-unwind" fn(*mut c_void, *const u16, *mut c_void) -> u32;

/// `ZipStorageBase::WriteStreamToFile(entry, stream)`. Only the low byte of
/// the result is the `bool`; the whole register goes back untouched.
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn write_stream_detour(
    this: *mut c_void,
    entry: *const u16,
    stream: *mut c_void,
) -> u32 {
    let Some(&t) = WRITE_STREAM_TRAMPOLINE.get() else {
        return 0;
    };
    let original: WriteStreamFn = unsafe { std::mem::transmute(t) };
    let result = original(this, entry, stream);
    guarded(|| {
        let storage = this as u32;
        remember(storage);
        let ok = result & 0xff != 0;
        let kind = entry_kind(wide_string(entry).as_deref().unwrap_or(""));
        // After the call: a stamp has just set the version it wrote.
        let version = word(storage.wrapping_add(layout::VERSION));
        if report_element(storage, kind, ok, version) {
            emit(
                TARGET_WRITE,
                write_level(ok),
                write_fields(pak_name(storage).as_deref(), kind, Tier::Cache, version, ok),
            );
        }
    });
    result
}

type CopySourceFn = unsafe extern "C-unwind" fn(*const u16, *const u16) -> u32;

/// The source-archive copy `(source path, cache path)`.
unsafe extern "C-unwind" fn copy_source_detour(source: *const u16, cache: *const u16) -> u32 {
    let Some(&t) = COPY_SOURCE_TRAMPOLINE.get() else {
        return 0;
    };
    let original: CopySourceFn = unsafe { std::mem::transmute(t) };
    let result = original(source, cache);
    guarded(|| {
        let ok = result & 0xff != 0;
        let path = wide_string(source).or_else(|| wide_string(cache));
        emit(
            TARGET_WRITE,
            write_level(ok),
            // The storage is not in hand, and the version the copy installs
            // is a local of the caller.
            write_fields(
                path.as_deref(),
                EntryKind::SourceArchive,
                Tier::SourceCache,
                None,
                ok,
            ),
        );
    });
    result
}
