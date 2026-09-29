//! The BigWorld client message sink: every `DEBUG_MSG`, `INFO_MSG`,
//! `WARNING_MSG`, `ERROR_MSG`, `CRITICAL_MSG` and assertion the BigWorld
//! layer of the client raises, with its priority.
//!
//! # Where it hooks
//!
//! `DebugMsgHelper::message` at `0x00a36460`
//! (`__thiscall(this, const int* header, const char* fmt, va_list)`,
//! `ret 0xc`). It is the single choke point:
//!
//! - `0x00a35210` (a cdecl varargs wrapper with 30 callers, all in
//!   `ServerConnection`, `EntityManager` and `Mercury::Nub`) builds a
//!   `va_list` and calls it directly;
//! - `0x00a351d0` (the assertion wrapper, 14 callers) goes
//!   `0x00a36ac0` -> `0x00a36900` (which `vsprintf`s, then shows the
//!   "Do you want to enter debugger?" box for a critical one) ->
//!   `0x00a36650` -> `0x00a36460` with the already formatted text and the
//!   format `"%s"`.
//!
//! `0x00a36460` has exactly two xrefs, both those wrappers.
//!
//! # The header and the filter
//!
//! `header` points at two `int`s: the component priority and this
//! message's priority. The function takes a critical section (`*this` is the
//! implementation object, whose first field is it) and lets a message
//! through only when `header[0] + impl[0x3c] <= header[1]`; otherwise it
//! returns without calling any output. `impl[0x3c]` is the filter threshold.
//! Past the filter it runs the registered message callbacks (`impl + 0x30`)
//! and, if none handled it, the default output `0x00a353b0` ->
//! `0x00a352f0`, which prints `"<PRIORITY>: "` (table at `0x01922380`:
//! TRACE, DEBUG, INFO, NOTICE, WARNING, ERROR, CRITICAL, HACK) and the
//! formatted line through `OutputDebugStringA` (and to stderr when
//! `0x01ef0713` is set).
//!
//! # What the sink adds
//!
//! It runs *before* the filter, so it reports messages the client itself
//! drops, with a `filtered` flag saying so. It formats the message with the
//! C runtime's `_vsnprintf` over the same `va_list`, only for a message that
//! passed the rate limit. With `unfilter` on it also writes a very low
//! threshold into `impl[0x3c]`, so the client's own outputs (its callbacks,
//! `OutputDebugString`) see everything too.
//!
//! The rate limit key is the format string's address plus the priority, so
//! the limit needs no formatting: two calls from the same source line share a
//! bucket, two different lines never do.
//!
//! Static evidence only (2026-09-28, Ghidra: decompile of `0x00a36460`,
//! `0x00a36900`, `0x00a352f0`, `0x00a35210`, the priority-name table at
//! `0x01922380`); not yet seen from a live client.

use serde_json::json;

use super::emit::Fields;
use super::text;

/// Entry of `DebugMsgHelper::message`.
pub const ADDR_BW_MESSAGE: usize = 0x00a3_6460;

/// Offset of the filter threshold inside the implementation object.
pub const THRESHOLD_OFFSET: usize = 0x3c;

/// What `unfilter` writes into the threshold: low enough that
/// `component + threshold <= priority` holds for any real priority.
pub const FORCED_THRESHOLD: i32 = -1_000_000;

/// Telemetry target.
pub const TARGET: &str = "client.bw.message";

/// The BigWorld priority names, in table order (`0x01922380`).
const PRIORITY_NAMES: [&str; 8] = [
    "TRACE", "DEBUG", "INFO", "NOTICE", "WARNING", "ERROR", "CRITICAL", "HACK",
];

/// Name of a priority, `PRIORITY_<n>` past the table. The table has eight
/// names and a null ninth slot, so 8 and above have none.
pub fn priority_name(priority: i32) -> String {
    usize::try_from(priority)
        .ok()
        .and_then(|i| PRIORITY_NAMES.get(i))
        .map_or_else(|| format!("PRIORITY_{priority}"), |n| (*n).to_string())
}

/// Telemetry level for a priority: a warning is a warning, an error or worse
/// is an error, the rest are `debug` (trace, debug) or `info`.
pub fn telemetry_level(priority: i32) -> &'static str {
    match priority {
        i32::MIN..=1 => "debug",
        2 | 3 => "info",
        4 | 7 => "warn",
        _ => "error",
    }
}

/// Whether the client's own filter would drop a message: it passes when
/// `component + threshold <= priority` (`CMP [ebp+4], ecx; JGE` at
/// `0x00a364a9`).
pub fn would_filter(component: i32, priority: i32, threshold: i32) -> bool {
    component.saturating_add(threshold) > priority
}

/// Rate-limit key: the format string's address and the priority.
pub fn throttle_key(fmt_addr: usize, priority: i32) -> String {
    format!("{fmt_addr:08x}:{priority}")
}

/// The fields of one `client.bw.message`.
pub fn message_fields(
    message: &str,
    truncated: bool,
    component: i32,
    priority: i32,
    filtered: bool,
    fmt_addr: usize,
    suppressed: u64,
) -> Fields {
    let mut f: Fields = vec![
        ("message", json!(message)),
        ("priority", json!(priority)),
        ("priority_name", json!(priority_name(priority))),
        ("component_priority", json!(component)),
        ("fmt_addr", json!(text::hex32(fmt_addr))),
    ];
    if filtered {
        f.push(("filtered", json!(true)));
    }
    if suppressed > 0 {
        f.push(("suppressed", json!(suppressed)));
    }
    if truncated {
        f.push(("truncated", json!(true)));
    }
    f
}

/// Install the sink.
///
/// # Safety
///
/// MinHook is initialised and the address is fingerprinted.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) unsafe fn install(producer: &crate::queue::Producer) {
    use super::bw_message_detour::{detour, TRAMPOLINE};
    super::install::inline(
        producer,
        "bw_message",
        ADDR_BW_MESSAGE,
        detour as *mut std::ffi::c_void,
        &TRAMPOLINE,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priority_names_follow_the_clients_table() {
        // Table at 0x01922380: TRACE, DEBUG, INFO, NOTICE, WARNING, ERROR,
        // CRITICAL, HACK, then a null slot.
        assert_eq!(priority_name(0), "TRACE");
        assert_eq!(priority_name(1), "DEBUG");
        assert_eq!(priority_name(2), "INFO");
        assert_eq!(priority_name(4), "WARNING");
        assert_eq!(priority_name(5), "ERROR");
        assert_eq!(priority_name(6), "CRITICAL");
        assert_eq!(priority_name(7), "HACK");
        assert_eq!(priority_name(8), "PRIORITY_8");
        assert_eq!(priority_name(-1), "PRIORITY_-1");
    }

    #[test]
    fn levels_escalate_with_priority() {
        assert_eq!(telemetry_level(0), "debug");
        assert_eq!(telemetry_level(1), "debug");
        assert_eq!(telemetry_level(2), "info");
        assert_eq!(telemetry_level(3), "info");
        assert_eq!(telemetry_level(4), "warn");
        assert_eq!(telemetry_level(5), "error");
        assert_eq!(telemetry_level(6), "error");
        assert_eq!(telemetry_level(7), "warn");
        assert_eq!(telemetry_level(99), "error");
        assert_eq!(telemetry_level(-3), "debug");
    }

    /// `0x00a364a9`: the message passes when `component + threshold <=
    /// priority`. The boundary is on the passing side.
    #[test]
    fn the_filter_predicate_matches_the_clients_compare() {
        assert!(!would_filter(1, 1, 0), "equal passes");
        assert!(!would_filter(1, 2, 0));
        assert!(would_filter(2, 1, 0), "below the threshold is dropped");
        assert!(would_filter(1, 1, 1), "a raised threshold drops it");
        assert!(!would_filter(1, 1, FORCED_THRESHOLD), "unfilter passes all");
        // No overflow at the extremes.
        assert!(would_filter(i32::MAX, 0, 10));
        assert!(!would_filter(i32::MIN, 0, -10));
    }

    #[test]
    fn a_source_line_has_one_key_per_priority() {
        assert_eq!(throttle_key(0x019c_df80, 6), "019cdf80:6");
        assert_ne!(throttle_key(0x019c_df80, 6), throttle_key(0x019c_df84, 6));
        assert_ne!(throttle_key(0x019c_df80, 6), throttle_key(0x019c_df80, 5));
    }

    #[test]
    fn fields_carry_the_priority_and_only_set_flags() {
        let f = message_fields("hello", false, 1, 4, false, 0x1234, 0);
        let get = |k: &str| f.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone());
        assert_eq!(get("message"), Some(json!("hello")));
        assert_eq!(get("priority"), Some(json!(4)));
        assert_eq!(get("priority_name"), Some(json!("WARNING")));
        assert_eq!(get("component_priority"), Some(json!(1)));
        assert_eq!(get("fmt_addr"), Some(json!("0x00001234")));
        assert_eq!(get("filtered"), None);
        assert_eq!(get("suppressed"), None);
        assert_eq!(get("truncated"), None);

        let f = message_fields("hello", true, 1, 4, true, 0x1234, 9);
        let get = |k: &str| f.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone());
        assert_eq!(get("filtered"), Some(json!(true)));
        assert_eq!(get("suppressed"), Some(json!(9)));
        assert_eq!(get("truncated"), Some(json!(true)));
    }

    #[test]
    fn the_hooked_address_is_the_ghidra_one() {
        assert_eq!(ADDR_BW_MESSAGE, 0x00a3_6460);
        assert_eq!(THRESHOLD_OFFSET, 0x3c);
    }
}
