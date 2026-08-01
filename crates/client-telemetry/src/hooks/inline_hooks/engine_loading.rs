//! Asset/package loading + level streaming hooks:
//! `FArchiveAsync::Serialize`, `UWorld::UpdateLevelStreamingInner`,
//! `UObject::StaticLoadObject`, and the cooked-data PAK loader.
//! Together these turn "client froze during a load" into a SigNoz
//! trace with the offending package name attached.

#[cfg(all(target_os = "windows", target_arch = "x86"))]
use std::ffi::c_void;
#[cfg(all(target_os = "windows", target_arch = "x86"))]
use std::sync::OnceLock;

#[cfg(all(target_os = "windows", target_arch = "x86"))]
use crate::queue::Producer;

use crate::hooks::sampling::SamplingCounter;

/// Address of `FArchiveAsync::Serialize(void* V, INT Length)` —
/// vtable slot 1, the bulk byte-read path during async loads.
/// Resolved via RTTI walk from `.?AVFArchiveAsync@@` at
/// 0x01dafd0c → COL @ 0x01b54f94 → vtable @ 0x01814198.
/// Body 0x004c7ae0 - 0x004c7bd8.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) const ADDR_ARCHIVE_ASYNC_SERIALIZE: usize = 0x004c7ae0;

/// Address of `UWorld::UpdateLevelStreamingInner(ULevelStreaming*,
/// FVector*)` — the per-streaming-level helper.
///
/// Resolved 2026-06-04 by xref to the "StreamingLevel" check-macro
/// string @ 0x01837518; signature confirmed by decompile (param_1
/// is StreamingLevel pointer, param_2 is FVector pointer derefed
/// as 3 floats for distance check).
///
/// The OUTER `UWorld::UpdateLevelStreaming` is at 0x005527a0 —
/// it loops over streaming levels and invokes this inner per-
/// level. Hooking the inner gives per-level transition events.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) const ADDR_UPDATE_LEVEL_STREAMING_INNER: usize = 0x0054e9c0;

/// Address of `UObject::StaticLoadObject(UClass*, UObject*, TCHAR*,
/// TCHAR*, DWORD, UPackageMap*, UBOOL)` — the canonical generic
/// object loader (UnObj.cpp).
///
/// Resolved 2026-06-04 by xref to "FailedLoadPackage" wstring @
/// 0x0180f104 (in the SEH catch handler); signature confirmed by
/// decompile against UE3 leaked source. The 7 cdecl args match
/// the UE3 source exactly; the recursive self-call at 0x004a8f8d
/// is the redirector-handling pattern.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) const ADDR_STATIC_LOAD_OBJECT: usize = 0x004a8e10;

/// Cooked-data PAK load — fires once per PAK load (21 categories).
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) const ADDR_COOKED_DATA_LOAD: usize = 0x00420074;

/// Trampoline pointer for `FArchiveAsync::Serialize`.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
static ARCHIVE_SERIALIZE_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

/// Trampoline pointer for `UWorld::UpdateLevelStreamingInner`.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
static UPDATE_LEVEL_STREAMING_INNER_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

/// Trampoline pointer for `UObject::StaticLoadObject`.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
static STATIC_LOAD_OBJECT_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

#[cfg(all(target_os = "windows", target_arch = "x86"))]
static COOKED_DATA_LOAD_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

/// Sampling counter for `FArchiveAsync::Serialize`. Bulk byte-read
/// path during package loads; can fire thousands of times per
/// cold-load. 1/1000 sampling keeps it bounded while still showing
/// the load-storm signature in SigNoz.
#[cfg_attr(not(all(target_os = "windows", target_arch = "x86")), allow(dead_code))]
static ARCHIVE_SERIALIZE_SAMPLER: SamplingCounter = SamplingCounter::new(1000);

/// Sampling counter for `UWorld::UpdateLevelStreamingInner`. Fires
/// per streaming level per frame — typically 1-10 active streaming
/// levels at any time, so 1/10 sampling keeps the wire rate ≤ ~1
/// emit/sec at 30 fps with 10 streaming levels.
#[cfg_attr(not(all(target_os = "windows", target_arch = "x86")), allow(dead_code))]
static UPDATE_LEVEL_STREAMING_SAMPLER: SamplingCounter = SamplingCounter::new(10);

/// Sampling counter for `UObject::StaticLoadObject`. Called bursts
/// of dozens to thousands during a cold load; 1/10 sampling makes
/// the load storm visible without flooding SigNoz with every nested
/// import resolve.
#[cfg_attr(not(all(target_os = "windows", target_arch = "x86")), allow(dead_code))]
static STATIC_LOAD_OBJECT_SAMPLER: SamplingCounter = SamplingCounter::new(10);

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) unsafe fn install_archive_async_serialize(producer: &Producer) {
    super::install_one(
        producer,
        "farchiveasync_serialize",
        ADDR_ARCHIVE_ASYNC_SERIALIZE,
        archive_async_serialize_detour as *mut c_void,
        &ARCHIVE_SERIALIZE_TRAMPOLINE,
    );
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) unsafe fn install_update_level_streaming_inner(producer: &Producer) {
    super::install_one(
        producer,
        "uworld_update_level_streaming_inner",
        ADDR_UPDATE_LEVEL_STREAMING_INNER,
        update_level_streaming_inner_detour as *mut c_void,
        &UPDATE_LEVEL_STREAMING_INNER_TRAMPOLINE,
    );
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) unsafe fn install_static_load_object(producer: &Producer) {
    super::install_one(
        producer,
        "uobject_static_load_object",
        ADDR_STATIC_LOAD_OBJECT,
        static_load_object_detour as *mut c_void,
        &STATIC_LOAD_OBJECT_TRAMPOLINE,
    );
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) unsafe fn install_cooked_data_load(producer: &Producer) {
    super::install_one(
        producer,
        "cooked_data_load",
        ADDR_COOKED_DATA_LOAD,
        cooked_data_load_detour as *mut c_void,
        &COOKED_DATA_LOAD_TRAMPOLINE,
    );
}

/// Detour for `FArchiveAsync::Serialize(void* V, INT Length)` —
/// vtable slot 1.
///
/// Signature: `extern "thiscall" fn(*mut FArchiveAsync, *mut c_void,
/// c_int)`. ECX = this, stack = V + Length.
///
/// **Hot path discipline:** can fire thousands of times during a
/// cold package load. Sampled at 1/1000 via `ARCHIVE_SERIALIZE_SAMPLER`
/// so we surface the load storm without flooding.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall" fn archive_async_serialize_detour(
    this: *mut c_void,
    v: *mut c_void,
    length: i32,
) {
    let _ = std::panic::catch_unwind(|| {
        if ARCHIVE_SERIALIZE_SAMPLER.should_emit() {
            if let Some(p) = crate::boot::producer() {
                p.try_emit(
                    crate::events::ClientNativeEvent::builder(
                        "client.engine.async_archive_serialize",
                        "debug",
                    )
                    .field("length", serde_json::json!(length)),
                );
            }
        }
    });

    if let Some(t) = ARCHIVE_SERIALIZE_TRAMPOLINE.get() {
        let original: unsafe extern "thiscall" fn(*mut c_void, *mut c_void, i32) =
            unsafe { std::mem::transmute(*t) };
        original(this, v, length);
    }
}

/// Detour for `UWorld::UpdateLevelStreamingInner(ULevelStreaming*,
/// FVector*)`.
///
/// Signature: `extern "thiscall" fn(*mut UWorld, *mut ULevelStreaming,
/// *mut FVector)`. ECX = this, stack = (StreamingLevel, FVector*).
///
/// **Hot path discipline:** fires per active streaming level per
/// frame on the main game thread. Sampled at 1/10 via
/// `UPDATE_LEVEL_STREAMING_SAMPLER`.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall" fn update_level_streaming_inner_detour(
    this: *mut c_void,
    streaming_level: *mut c_void,
    delta_position: *mut c_void,
) {
    let _ = std::panic::catch_unwind(|| {
        if UPDATE_LEVEL_STREAMING_SAMPLER.should_emit() {
            if let Some(p) = crate::boot::producer() {
                p.try_emit(crate::events::ClientNativeEvent::builder(
                    "client.engine.update_level_streaming",
                    "debug",
                ));
            }
        }
    });

    if let Some(t) = UPDATE_LEVEL_STREAMING_INNER_TRAMPOLINE.get() {
        let original: unsafe extern "thiscall" fn(*mut c_void, *mut c_void, *mut c_void) =
            unsafe { std::mem::transmute(*t) };
        original(this, streaming_level, delta_position);
    }
}

/// Detour for `UObject::StaticLoadObject(UClass*, UObject*, TCHAR*,
/// TCHAR*, DWORD, UPackageMap*, UBOOL) -> UObject*`.
///
/// Signature: 7 `__cdecl` args matching UE3 leaked source. ALL
/// args must be forwarded to the trampoline or the original
/// function reads garbage off the stack.
///
/// **Hot path discipline:** fires in bursts of dozens to thousands
/// during cold loads. Sampled at 1/10 via
/// `STATIC_LOAD_OBJECT_SAMPLER`.
///
/// **Sampled emit captures `package_name`**: when we DO emit, we
/// read up to 256 wchar_t from `in_name` (bounded, null-terminated
/// safe) and ship it as a UTF-8 string field. This is the load-
/// freeze investigation gold: SigNoz can show "which package was
/// loading when the client froze."
///
/// **Safety:** `in_name` is a stack-passed `*const u16`. The caller
/// holds the buffer for the duration of the call; we only read
/// from it inside the detour (before invoking the trampoline) so
/// the buffer is still live. The 256-wchar bound prevents runaway
/// reads if the string isn't null-terminated.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "C" fn static_load_object_detour(
    object_class: *mut c_void,
    in_outer: *mut c_void,
    in_name: *const u16,
    filename: *const u16,
    load_flags: u32,
    sandbox: *mut c_void,
    allow_reconciliation: i32,
) -> *mut c_void {
    let _ = std::panic::catch_unwind(|| {
        if STATIC_LOAD_OBJECT_SAMPLER.should_emit() {
            if let Some(p) = crate::boot::producer() {
                let name = read_utf16_bounded(in_name, 256);
                p.try_emit(
                    crate::events::ClientNativeEvent::builder(
                        "client.engine.static_load_object",
                        "debug",
                    )
                    .field("package_name", serde_json::Value::String(name))
                    .field("load_flags", serde_json::json!(load_flags)),
                );
            }
        }
    });

    if let Some(t) = STATIC_LOAD_OBJECT_TRAMPOLINE.get() {
        let original: unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            *const u16,
            *const u16,
            u32,
            *mut c_void,
            i32,
        ) -> *mut c_void = unsafe { std::mem::transmute(*t) };
        original(
            object_class,
            in_outer,
            in_name,
            filename,
            load_flags,
            sandbox,
            allow_reconciliation,
        )
    } else {
        // Trampoline missing — return null. The caller will treat
        // it as "object not found" and likely log + recover. Better
        // than crashing.
        std::ptr::null_mut()
    }
}

/// Detour for the cooked-data PAK loader.
///
/// Signature: `extern "cdecl" fn(u32 category, void** out_a, void** out_b)`.
///
/// Fires once per PAK load (21 categories). Emit 1/1 — rare event.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "C" fn cooked_data_load_detour(
    category: u32,
    out_a: *mut *mut c_void,
    out_b: *mut *mut c_void,
) {
    let _ = std::panic::catch_unwind(|| {
        if let Some(p) = crate::boot::producer() {
            p.try_emit(
                crate::events::ClientNativeEvent::builder("client.engine.pak_load", "info")
                    .field("category", serde_json::json!(category)),
            );
        }
    });

    if let Some(t) = COOKED_DATA_LOAD_TRAMPOLINE.get() {
        let original: unsafe extern "C" fn(u32, *mut *mut c_void, *mut *mut c_void) =
            unsafe { std::mem::transmute(*t) };
        original(category, out_a, out_b);
    }
}

/// Read a UTF-16 C string up to `max_chars`. Stops at the first
/// NUL or at the bound, whichever comes first. Returns the UTF-8
/// representation with `U+FFFD` replacement for invalid surrogates.
///
/// Used by the `static_load_object_detour` to capture the package
/// name for the emit field. The bound exists because we don't
/// trust the caller's buffer to be NUL-terminated — a runaway read
/// inside the detour would risk an access violation that takes
/// the whole game with it.
///
/// Returns `"<null>"` on null pointer, `""` on empty string. Never
/// reads past `max_chars` wchars (= 2 × max_chars bytes).
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(in crate::hooks) fn read_utf16_bounded(ptr: *const u16, max_chars: usize) -> String {
    if ptr.is_null() {
        return "<null>".into();
    }
    let mut len = 0usize;
    while len < max_chars {
        // SAFETY: bounded by max_chars; caller's contract is the
        // buffer is valid for at least max_chars wchars OR
        // NUL-terminated sooner. We exit on first NUL.
        let ch = unsafe { *ptr.add(len) };
        if ch == 0 {
            break;
        }
        len += 1;
    }
    // SAFETY: `len` is the number of u16 elements before NUL,
    // bounded by max_chars. `from_raw_parts` on the same pointer
    // with that length is sound.
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf16_lossy(slice)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sampling rates are load-bearing — they bound the wire cost
    /// of the hot-path hooks. Pin them so a careless edit doesn't
    /// 100× the telemetry volume.
    #[test]
    fn sampler_rates_are_bounded() {
        // 1/1000 → 1 emit per 1000 calls. Pin by emitting 1000
        // times and counting.
        let archive_emits: usize = (0..1000)
            .filter(|_| ARCHIVE_SERIALIZE_SAMPLER.should_emit())
            .count();
        assert_eq!(archive_emits, 1, "archive sampler should emit 1/1000");

        // 1/10 for the two coarser hooks.
        let stream_emits: usize = (0..10)
            .filter(|_| UPDATE_LEVEL_STREAMING_SAMPLER.should_emit())
            .count();
        assert_eq!(stream_emits, 1, "stream sampler should emit 1/10");

        let load_emits: usize = (0..10)
            .filter(|_| STATIC_LOAD_OBJECT_SAMPLER.should_emit())
            .count();
        assert_eq!(load_emits, 1, "load-object sampler should emit 1/10");
    }

    /// `read_utf16_bounded` must:
    /// 1. Stop at the NUL terminator.
    /// 2. Cap at `max_chars` so a missing NUL can't cause a runaway
    ///    read (the safety contract for use inside the StaticLoadObject
    ///    detour — caller's buffer is foreign-allocated, not trusted).
    /// 3. Handle a null pointer without UB.
    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    #[test]
    fn read_utf16_bounded_stops_at_nul() {
        let buf: Vec<u16> = "Engine/EngineMaterials/DefaultMaterial\0extra"
            .encode_utf16()
            .collect();
        let got = read_utf16_bounded(buf.as_ptr(), 256);
        assert_eq!(got, "Engine/EngineMaterials/DefaultMaterial");
    }

    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    #[test]
    fn read_utf16_bounded_caps_at_max() {
        // 300 chars, no NUL — must stop at max_chars=8.
        let buf: Vec<u16> = (0..300).map(|_| 'A' as u16).collect();
        let got = read_utf16_bounded(buf.as_ptr(), 8);
        assert_eq!(got, "AAAAAAAA");
        assert_eq!(got.chars().count(), 8);
    }

    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    #[test]
    fn read_utf16_bounded_null_ptr() {
        assert_eq!(read_utf16_bounded(std::ptr::null(), 256), "<null>");
    }

    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    #[test]
    fn read_utf16_bounded_empty_string() {
        let buf: Vec<u16> = vec![0u16, 0u16];
        let got = read_utf16_bounded(buf.as_ptr(), 256);
        assert_eq!(got, "");
    }
}
