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

use std::collections::BTreeMap;

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

/// Telemetry target of the per-entity viewport row.
pub const UNKNOWN_ENTITY_TARGET: &str = "client.viewport.unknown_entity";

/// The start of the client's format for a failed viewport lookup (the
/// `svidFollow` error). The entity id is its first argument.
pub const UNKNOWN_ENTITY_FORMAT: &[u8] = b"Viewport for entity %d";

/// Entities tracked at once. A message about a new entity past this bound
/// takes the free-text path, so the table cannot grow without limit.
pub const MAX_UNKNOWN_ENTITIES: usize = 256;

/// Milliseconds between rollups of the later counts.
pub const UNKNOWN_ROLLUP_MS: u64 = 10_000;

/// Whether `format` (the start of a message's format string) is the failed
/// viewport lookup.
pub fn is_unknown_entity_format(format: &[u8]) -> bool {
    format.starts_with(UNKNOWN_ENTITY_FORMAT)
}

/// The fields of one `client.viewport.unknown_entity` row.
pub fn entity_row(entity_id: u32, count: u64, first_ts: u64) -> Fields {
    vec![
        ("entity_id", json!(entity_id)),
        ("count", json!(count)),
        ("first_ts", json!(first_ts)),
    ]
}

/// What [`UnknownEntities::observe`] did with one message.
#[derive(Debug, PartialEq)]
pub enum Observed {
    /// The first message for this entity: its row, now.
    First(Fields),
    /// A later message: counted, and reported on the next rollup.
    Counted,
    /// The table is full: the message takes the free-text path.
    Untracked,
}

/// What the table knows about one entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Seen {
    first_ts: u64,
    count: u64,
    reported: u64,
}

/// Failed viewport lookups per entity: one row at first sight, the later
/// counts on the rollup.
#[derive(Debug)]
pub struct UnknownEntities {
    seen: BTreeMap<u32, Seen>,
    last_rollup: u64,
}

impl UnknownEntities {
    /// An empty table.
    pub const fn new() -> Self {
        Self {
            seen: BTreeMap::new(),
            last_rollup: 0,
        }
    }

    /// Count one failed lookup for `entity_id` at `now_ms`.
    pub fn observe(&mut self, entity_id: u32, now_ms: u64) -> Observed {
        if let Some(s) = self.seen.get_mut(&entity_id) {
            s.count += 1;
            return Observed::Counted;
        }
        if self.seen.len() >= MAX_UNKNOWN_ENTITIES {
            return Observed::Untracked;
        }
        self.seen.insert(
            entity_id,
            Seen {
                first_ts: now_ms,
                count: 1,
                reported: 1,
            },
        );
        Observed::First(entity_row(entity_id, 1, now_ms))
    }

    /// The rows of the entities whose count grew since the last rollup. Empty
    /// until [`UNKNOWN_ROLLUP_MS`] has passed since the last one.
    pub fn rollup(&mut self, now_ms: u64) -> Vec<Fields> {
        if now_ms.saturating_sub(self.last_rollup) < UNKNOWN_ROLLUP_MS {
            return Vec::new();
        }
        self.last_rollup = now_ms;
        let mut rows = Vec::new();
        for (id, s) in self.seen.iter_mut() {
            if s.count > s.reported {
                s.reported = s.count;
                rows.push(entity_row(*id, s.count, s.first_ts));
            }
        }
        rows
    }
}

impl Default for UnknownEntities {
    fn default() -> Self {
        Self::new()
    }
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

    fn field(row: &Fields, key: &str) -> Option<serde_json::Value> {
        row.iter().find(|(k, _)| *k == key).map(|(_, v)| v.clone())
    }

    #[test]
    fn a_failed_viewport_lookup_is_recognised_by_its_format() {
        assert!(is_unknown_entity_format(
            b"Viewport for entity %d (immed vehicle %d, top vehicle %d) is unknown"
        ));
        assert!(!is_unknown_entity_format(b"Entity %s has %d items"));
    }

    /// Three messages for one entity: one row at first sight, then the
    /// count of 3 on the rollup, once the interval has passed.
    #[test]
    fn three_messages_for_one_entity_give_one_row_then_count_three() {
        let mut t = UnknownEntities::new();
        let Observed::First(row) = t.observe(7, 0) else {
            panic!("the first message is reported at once");
        };
        assert_eq!(field(&row, "entity_id"), Some(json!(7)));
        assert_eq!(field(&row, "count"), Some(json!(1)));
        assert_eq!(t.observe(7, 1), Observed::Counted);
        assert_eq!(t.observe(7, 2), Observed::Counted);
        assert!(t.rollup(5_000).is_empty(), "no rollup before the interval");

        let rows = t.rollup(UNKNOWN_ROLLUP_MS);
        assert_eq!(rows.len(), 1);
        assert_eq!(field(&rows[0], "entity_id"), Some(json!(7)));
        assert_eq!(field(&rows[0], "count"), Some(json!(3)));
        assert_eq!(field(&rows[0], "first_ts"), Some(json!(0)));
        assert!(
            t.rollup(2 * UNKNOWN_ROLLUP_MS).is_empty(),
            "nothing new to roll up"
        );
    }

    /// A second entity has its own row and its own count.
    #[test]
    fn another_entity_is_independent() {
        let mut t = UnknownEntities::new();
        assert!(matches!(t.observe(7, 0), Observed::First(_)));
        assert!(matches!(t.observe(9, 0), Observed::First(_)));
        assert_eq!(t.observe(9, 1), Observed::Counted);
        let rows = t.rollup(UNKNOWN_ROLLUP_MS);
        assert_eq!(rows.len(), 1, "only the entity with later counts rolls up");
        assert_eq!(field(&rows[0], "entity_id"), Some(json!(9)));
        assert_eq!(field(&rows[0], "count"), Some(json!(2)));
    }

    /// The table stops at its cap: a new entity falls back, a tracked one
    /// still counts.
    #[test]
    fn the_table_is_bounded() {
        let mut t = UnknownEntities::new();
        for id in 0..MAX_UNKNOWN_ENTITIES as u32 {
            assert!(matches!(t.observe(id, 0), Observed::First(_)));
        }
        assert_eq!(t.observe(u32::MAX, 0), Observed::Untracked);
        assert_eq!(t.observe(0, 0), Observed::Counted);
    }

    #[test]
    fn the_hooked_address_is_the_ghidra_one() {
        assert_eq!(ADDR_BW_MESSAGE, 0x00a3_6460);
        assert_eq!(THRESHOLD_OFFSET, 0x3c);
    }
}
