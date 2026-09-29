//! `UObject::StaticLoadObject` failures: an object the engine was asked to
//! load and could not.
//!
//! `client.engine.static_load_object` (in `inline_hooks`) samples the calls
//! at 1/10 and reports the requested name; it never looks at the answer.
//! `StaticLoadObject` returns `NULL` when the object or its package cannot be
//! found or loaded. That path (decompile of `0x004a8e10`, 2026-09-28) ends in
//! the localised `Core.ObjectNotFound` error report (`FUN_0049ee60`) and
//! returns the `NULL`; a caller that does not check it goes on to
//! use a missing mesh, texture or material, which is the shape of most "it
//! renders as a grey box" reports.
//!
//! This reports every `NULL` return, unsampled and rate-limited per name
//! shape, with the requested name, the file name the caller supplied (often
//! empty) and the raw load flags. The flags' meaning in this build is not
//! recovered (UE3's `LOAD_NoWarn` and `LOAD_Quiet` bits would mark an optional
//! lookup that is expected to fail), so they are reported as the integer and
//! the level is always `warn`.
//!
//! The `NULL` check needs the result, so the detour in `inline_hooks` now
//! keeps the original's return value and calls [`observe_failure`] when it is
//! null; everything else about that hook is unchanged.

use serde_json::json;

use crate::hooks::sinks::emit::Fields;
use crate::hooks::sinks::text;

/// Longest name kept.
pub const MAX_NAME_CHARS: usize = 256;

/// Telemetry target.
pub const TARGET: &str = "client.engine.load_failed";

/// Rate-limit key.
pub fn throttle_key(name: &str) -> String {
    text::message_shape(name)
}

/// The fields of one failed load.
pub fn failure_fields(name: &str, filename: Option<&str>, flags: u32, suppressed: u64) -> Fields {
    let mut f: Fields = vec![
        ("name", json!(name)),
        ("flags", json!(format!("0x{flags:08x}"))),
    ];
    if let Some(file) = filename.filter(|s| !s.is_empty()) {
        f.push(("filename", json!(file)));
    }
    if suppressed > 0 {
        f.push(("suppressed", json!(suppressed)));
    }
    f
}

/// Report one `NULL` return.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(in crate::hooks) fn observe_failure(name: *const u16, filename: *const u16, flags: u32) {
    use crate::hooks::name_throttle::Decision;
    use crate::hooks::sinks::emit::emit;
    use crate::hooks::sinks::mem;
    use crate::hooks::sinks::throttle::SinkThrottle;

    static THROTTLE: SinkThrottle = SinkThrottle::new();

    let read = |p: *const u16| {
        mem::wide_at(p as usize, MAX_NAME_CHARS).map(|s| text::decode_wide(&s.units))
    };
    let name = read(name).unwrap_or_else(|| "<unreadable>".to_string());
    let Decision::Emit { suppressed } = THROTTLE.check(&throttle_key(&name)) else {
        return;
    };
    let filename = read(filename);
    emit(
        TARGET,
        "warn",
        "engine.load_failed",
        failure_fields(&name, filename.as_deref(), flags, suppressed),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failed_load_reports_the_name_and_raw_flags() {
        let f = failure_fields("SGWGame.Materials.Missing", Some("C:\\x.upk"), 0x2001, 0);
        let get = |k: &str| f.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone());
        assert_eq!(get("name"), Some(json!("SGWGame.Materials.Missing")));
        assert_eq!(get("flags"), Some(json!("0x00002001")));
        assert_eq!(get("filename"), Some(json!("C:\\x.upk")));
        assert_eq!(get("suppressed"), None);
    }

    /// An empty file name (the usual case) is left out, not reported empty.
    #[test]
    fn an_empty_or_absent_filename_is_omitted() {
        for name in [None, Some("")] {
            let f = failure_fields("A.B", name, 0, 3);
            assert!(!f.iter().any(|(k, _)| *k == "filename"));
            assert!(f.contains(&("suppressed", json!(3))));
        }
    }

    #[test]
    fn numbered_variants_of_one_missing_object_share_a_bucket() {
        assert_eq!(
            throttle_key("SGWGame.Meshes.Pawn_12"),
            throttle_key("SGWGame.Meshes.Pawn_47")
        );
        assert_ne!(throttle_key("A.B"), throttle_key("A.C"));
    }
}
