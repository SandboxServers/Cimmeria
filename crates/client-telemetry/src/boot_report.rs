//! DLL boot failures as events (TG-PIPE-14).
//!
//! Before this module, four boot steps that can fail wrote only to the
//! DLL's local log, so SigNoz could not tell "the DLL loaded but the lab
//! bridge never listened" from "the lab bridge was never asked for". Each
//! now also queues a `warn` row. The builders live here, apart from the
//! `cfg(windows)` boot code, so they are unit-tested on every target.
//!
//! All four are must-keep in the governor (`client.dll.*` and
//! `client.hooks.*`) and replayed over budget by the ingest.

use std::io;

use crate::events::{ClientNativeEvent, EventBuilder};

/// The uploader thread could not be spawned.
pub const UPLOADER_SPAWN_FAILED: &str = "client.dll.uploader_spawn_failed";
/// The lab bridge was asked for (a `lab` block in the session) but did
/// not start.
pub const BRIDGE_FAILED: &str = "client.dll.bridge_failed";
/// The lab bridge started but its DirectInput hook did not go in, so the
/// lab cannot drive input without focus.
pub const INPUT_UNHOOKED: &str = "client.dll.input_unhooked";
/// The install lock shared with the client-patches DLL was not taken, so
/// no hooks went in.
pub const INSTALL_LOCK_UNAVAILABLE: &str = "client.hooks.install_lock_unavailable";

/// `client.dll.uploader_spawn_failed {error}`. With no uploader nothing
/// ships this row; it is queued so a ring dump shows it, and the local log
/// line next to it is the record that survives.
pub fn uploader_spawn_failed(e: &io::Error) -> EventBuilder {
    ClientNativeEvent::builder(UPLOADER_SPAWN_FAILED, "warn").field("error", e.to_string())
}

/// The `reason` of `client.dll.bridge_failed`: a closed list.
///
/// - `invalid_token`: the session's lab token is not 64 hex characters
///   (`bridge::start` refuses it with `InvalidInput`).
/// - `addr_in_use`: another process holds the lab port.
/// - `addr_not_available`: the bind address is not on this machine.
/// - `permission_denied`: the OS refused the bind.
/// - `io_error`: anything else (the listener's `local_addr`, the IO
///   thread's spawn).
pub fn bridge_reason(kind: io::ErrorKind) -> &'static str {
    match kind {
        io::ErrorKind::InvalidInput => "invalid_token",
        io::ErrorKind::AddrInUse => "addr_in_use",
        io::ErrorKind::AddrNotAvailable => "addr_not_available",
        io::ErrorKind::PermissionDenied => "permission_denied",
        _ => "io_error",
    }
}

/// The event for a lab bridge start, if it needs one: `None` when no
/// bridge was asked for (`None`) or it started (`Some(Ok(_))`), and
/// `client.dll.bridge_failed {reason, error}` when it failed. Generic over
/// the handle so it compiles, and is tested, without the `lab-bridge`
/// feature.
pub fn report_bridge_result<H>(result: &Option<io::Result<H>>) -> Option<EventBuilder> {
    let Some(Err(e)) = result else {
        return None;
    };
    Some(
        ClientNativeEvent::builder(BRIDGE_FAILED, "warn")
            .field("reason", bridge_reason(e.kind()))
            .field("error", e.to_string()),
    )
}

/// `client.dll.input_unhooked {error}`.
pub fn input_unhooked(error: &str) -> EventBuilder {
    ClientNativeEvent::builder(INPUT_UNHOOKED, "warn").field("error", error)
}

/// `client.hooks.install_lock_unavailable {wait_ms}`: the lock was not
/// taken within `wait` (the client-patches DLL is still hooking, or the
/// mutex could not be created), so this DLL installed no hooks.
pub fn install_lock_unavailable(wait: std::time::Duration) -> EventBuilder {
    let wait_ms = u64::try_from(wait.as_millis()).unwrap_or(u64::MAX);
    ClientNativeEvent::builder(INSTALL_LOCK_UNAVAILABLE, "warn").field("wait_ms", wait_ms)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::queue::channel;
    use serde_json::json;

    /// Ship a builder through a plain channel and read back the event.
    fn emitted(ev: EventBuilder) -> ClientNativeEvent {
        let (p, c) = channel();
        assert!(p.try_emit(ev));
        c.try_recv().unwrap()
    }

    /// No bridge asked for, or a bridge that started: no row. A bridge that
    /// failed: one warn row naming why. Reverting the call in `boot.rs` to a
    /// bare log line would leave SigNoz without it; this pins the row.
    #[test]
    fn report_bridge_result_reports_only_a_failed_start() {
        assert!(report_bridge_result::<()>(&None).is_none());
        assert!(report_bridge_result(&Some(Ok(()))).is_none());

        let failed: Option<io::Result<()>> =
            Some(Err(io::Error::new(io::ErrorKind::AddrInUse, "port taken")));
        let ev = emitted(report_bridge_result(&failed).expect("a failed start is reported"));
        assert_eq!(ev.target, "client.dll.bridge_failed");
        assert_eq!(ev.level, "warn");
        assert_eq!(ev.fields["reason"], json!("addr_in_use"));
        assert_eq!(ev.fields["error"], json!("port taken"));
    }

    #[test]
    fn bridge_reasons_are_a_closed_list() {
        assert_eq!(bridge_reason(io::ErrorKind::InvalidInput), "invalid_token");
        assert_eq!(bridge_reason(io::ErrorKind::AddrInUse), "addr_in_use");
        assert_eq!(
            bridge_reason(io::ErrorKind::AddrNotAvailable),
            "addr_not_available"
        );
        assert_eq!(
            bridge_reason(io::ErrorKind::PermissionDenied),
            "permission_denied"
        );
        assert_eq!(bridge_reason(io::ErrorKind::Other), "io_error");
        assert_eq!(bridge_reason(io::ErrorKind::OutOfMemory), "io_error");
    }

    #[test]
    fn the_other_boot_failures_are_warn_rows_with_their_cause() {
        let spawn = emitted(uploader_spawn_failed(&io::Error::other("no threads")));
        assert_eq!(
            (spawn.target.as_str(), spawn.level.as_str()),
            ("client.dll.uploader_spawn_failed", "warn")
        );
        assert_eq!(spawn.fields["error"], json!("no threads"));

        let input = emitted(input_unhooked("DirectInput8Create not imported"));
        assert_eq!(input.target, "client.dll.input_unhooked");
        assert_eq!(input.level, "warn");
        assert_eq!(
            input.fields["error"],
            json!("DirectInput8Create not imported")
        );

        let lock = emitted(install_lock_unavailable(std::time::Duration::from_secs(10)));
        assert_eq!(lock.target, "client.hooks.install_lock_unavailable");
        assert_eq!(lock.level, "warn");
        assert_eq!(lock.fields["wait_ms"], json!(10_000));
    }

    /// All four are must-keep in the governor, so a flood never summarizes
    /// one away.
    #[test]
    fn every_boot_failure_is_must_keep() {
        use crate::governor::{classify, Class};
        for target in [
            UPLOADER_SPAWN_FAILED,
            BRIDGE_FAILED,
            INPUT_UNHOOKED,
            INSTALL_LOCK_UNAVAILABLE,
        ] {
            let class = classify(target, "info", &Default::default()).class;
            assert!(matches!(class, Class::MustKeep(_)), "{target}: {class:?}");
        }
    }
}
