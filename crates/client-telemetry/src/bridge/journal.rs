//! DLL-side crash evidence: the in-flight command flag and the crash
//! marker the unhandled-exception filter flushes next to the minidump.
//!
//! Phase 2 (issue #685) builds the *skeleton* the ADR §6 crash-recovery
//! table calls for. The rich "last N bridge commands" ring lives in the
//! supervisor (`cimmeria-lab`), which is the single bridge client and
//! therefore knows every command it sent and which one was in flight.
//! The DLL's job here is narrower and, crucially, **signal-safe**: it
//! records only whether *a* command was executing when the process
//! faulted, so the supervisor can mark that in-flight command
//! quarantined.
//!
//! # Why a bare `AtomicBool`, not a string ring
//!
//! [`crate::bridge::crash`]'s unhandled-exception filter runs from an
//! arbitrary faulting thread that may hold any lock (the #686 spike:
//! **never** touch a lock a faulted thread might own). Reading a method
//! *name* would mean touching heap-allocated `String` state under a
//! lock. Instead dispatch flips a lock-free flag around each command;
//! the crash filter reads the flag and builds a fixed-shape marker. The
//! quarantined command's *identity* is recovered supervisor-side by
//! correlating this flag with its own command journal.

use std::sync::atomic::{AtomicBool, Ordering};

/// True while a command is being dispatched on the main thread. Flipped
/// on either side of the dispatch body in
/// [`crate::bridge::dispatch::dispatch`]. Read (never written) by the
/// crash filter.
static IN_FLIGHT: AtomicBool = AtomicBool::new(false);

/// Mark a command as executing. Called on the main thread immediately
/// before the dispatch body runs.
pub fn mark_in_flight() {
    IN_FLIGHT.store(true, Ordering::SeqCst);
}

/// Clear the in-flight flag. Called on the main thread immediately after
/// the dispatch body returns (whether it succeeded or produced an
/// error response — an *error* is a normal outcome, only an actual
/// fault leaves the flag set).
pub fn clear_in_flight() {
    IN_FLIGHT.store(false, Ordering::SeqCst);
}

/// Whether a command was in flight. Read by the crash filter.
pub fn is_in_flight() -> bool {
    IN_FLIGHT.load(Ordering::SeqCst)
}

/// Fixed-shape crash evidence written next to the minidump by the
/// unhandled-exception filter. Deliberately flat and cheap to build so
/// it can be serialized from a crash context.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CrashMarker {
    /// STATUS_* exception code (e.g. `0xC0000005` access violation).
    pub exception_code: u32,
    /// Faulting instruction address.
    pub exception_address: u64,
    /// OS thread id that faulted.
    pub thread_id: u32,
    /// Wall-clock time the marker was written, ms since epoch.
    pub ts_ms: i64,
    /// Whether a bridge command was executing at fault time. When true,
    /// the supervisor marks its own in-flight command *quarantined* and
    /// refuses to replay it on recovery (ADR §6).
    pub in_flight: bool,
    /// Filename of the minidump this marker accompanies, if one was
    /// written successfully. Relative — it sits in the same directory.
    pub minidump: Option<String>,
}

impl CrashMarker {
    /// Build a marker from a faulting context. Pure — the native crash
    /// filter gathers the raw fields and calls this, so the shape is
    /// unit-testable off Windows.
    pub fn new(
        exception_code: u32,
        exception_address: u64,
        thread_id: u32,
        ts_ms: i64,
        in_flight: bool,
        minidump: Option<String>,
    ) -> Self {
        Self {
            exception_code,
            exception_address,
            thread_id,
            ts_ms,
            in_flight,
            minidump,
        }
    }

    /// Serialize to the bytes the crash filter writes to
    /// `lab-crash-marker.json`. Infallible in practice (flat POD
    /// fields); degrades to a minimal object rather than failing so the
    /// crash path always leaves *something*.
    pub fn to_json_bytes(&self) -> Vec<u8> {
        serde_json::to_vec_pretty(self).unwrap_or_else(|_| {
            format!(
                r#"{{"exception_code":{},"in_flight":{},"note":"marker serialize failed"}}"#,
                self.exception_code, self.in_flight
            )
            .into_bytes()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The in-flight flag round-trips: set on dispatch entry, cleared on
    /// exit. A command that faults leaves it set — that is what the
    /// crash filter reads as "quarantine the in-flight command."
    #[test]
    fn in_flight_flag_round_trips() {
        clear_in_flight();
        assert!(!is_in_flight());
        mark_in_flight();
        assert!(
            is_in_flight(),
            "a dispatched command marks itself in flight"
        );
        clear_in_flight();
        assert!(!is_in_flight(), "a returned command clears the flag");
    }

    /// A marker built from an in-flight fault says so — this is the
    /// quarantine signal the supervisor keys on.
    #[test]
    fn marker_records_in_flight_fault() {
        let m = CrashMarker::new(
            0xC000_0005,
            0x0141_6ec0,
            4242,
            1_700_000_000_000,
            true,
            Some("lab-minidump-1700000000000.dmp".to_string()),
        );
        assert!(
            m.in_flight,
            "in-flight fault must mark the command quarantined"
        );
        let json = String::from_utf8(m.to_json_bytes()).unwrap();
        assert!(json.contains("\"in_flight\": true"));
        // serde serializes the code as a decimal u32.
        assert!(json.contains(&0xC000_0005u32.to_string()));
        assert!(json.contains("lab-minidump-1700000000000.dmp"));
    }

    /// A fault outside any command (in_flight=false) is still captured,
    /// but nothing gets quarantined.
    #[test]
    fn marker_records_idle_fault() {
        let m = CrashMarker::new(0xC000_0005, 0, 1, 0, false, None);
        assert!(!m.in_flight);
        let json = String::from_utf8(m.to_json_bytes()).unwrap();
        assert!(json.contains("\"in_flight\": false"));
        // No minidump ⇒ the field serializes as null.
        assert!(json.contains("\"minidump\": null"));
    }
}
