//! Client maintenance and client-changes events (launcher-telemetry LT-16b;
//! C5, C6, C9). Target `launcher.repair`. Maintenance is not a journal
//! operation, so no event carries an operation number.
use super::{ClientMaintenanceCommand, ClientMaintenanceError};
use std::time::Duration;

impl ClientMaintenanceError {
    /// The closed code for a refused request; `Storage` gives the storage code.
    pub fn code(self) -> &'static str {
        match self {
            Self::ConfirmationRequired => "confirmation_required",
            Self::ProfileUnavailable => "profile_unavailable",
            Self::UnsafePath => "unsafe_path",
            Self::Storage(error) => error.code(),
        }
    }
}

/// The closed action name of a maintenance request.
pub(crate) fn action(request: &ClientMaintenanceCommand) -> &'static str {
    match request {
        ClientMaintenanceCommand::Inspect { .. } => "inspect",
        ClientMaintenanceCommand::ResetCache { .. } => "reset_cache",
        ClientMaintenanceCommand::ResetAllClientState { .. } => "reset_all_client_state",
    }
}

/// A reset finished. `removed` is whether it deleted anything.
pub(crate) fn client_maintenance_finished(action: &'static str, removed: bool, took: Duration) {
    tracing::info!(
        target: "launcher.repair",
        event = "client_maintenance_finished",
        action,
        outcome = "ok",
        removed,
        duration_ms = u64::try_from(took.as_millis()).unwrap_or(u64::MAX),
        "launcher client maintenance: reset finished; expected the client folder to be reset; the reset is done"
    );
}

/// A request was refused or could not run; `reason` is a closed code.
pub(crate) fn client_maintenance_failed(action: &'static str, reason: &'static str) {
    tracing::warn!(
        target: "launcher.repair",
        event = "client_maintenance_failed",
        action,
        reason,
        "launcher client maintenance: request refused; expected a confirmed reset on a known profile; nothing was deleted"
    );
}

/// A reset stopped part-way; the next reset finishes it. `error_kind` is the I/O error kind.
pub(crate) fn client_maintenance_removal_failed(error_kind: &'static str) {
    tracing::warn!(
        target: "launcher.repair",
        event = "client_maintenance_removal_failed",
        error_kind,
        "launcher client maintenance: removal stopped part-way; expected every client file to be removable; the rest stays until the next reset"
    );
}

/// The install record of the client could not be read, so every patch reads
/// as pending in the changes list.
pub(crate) fn client_changes_ledger_unreadable(reason: &'static str) {
    tracing::warn!(
        target: "launcher.repair",
        event = "client_changes_ledger_unreadable",
        reason,
        "launcher client changes: install record unreadable; expected a readable record of applied patches; every patch is listed as pending"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_capture::capture;
    use crate::{DesktopState, StorageError};
    use std::path::Path;

    /// The launcher.repair events a capture saw, in order.
    fn repair_events(
        events: &[crate::test_capture::Captured],
    ) -> Vec<&crate::test_capture::Captured> {
        events
            .iter()
            .filter(|e| e.target == "launcher.repair")
            .collect()
    }

    fn open(root: &Path) -> DesktopState {
        DesktopState::open(&root.join("state")).unwrap()
    }

    #[test]
    fn an_unconfirmed_reset_logs_client_maintenance_failed() {
        let root = tempfile::tempdir().unwrap();
        let firesky = root.path().join("Firesky");
        let mut state = open(root.path());
        let (result, events) = capture(|| {
            tracing::callsite::rebuild_interest_cache();
            state.client_maintenance_fixture(
                ClientMaintenanceCommand::ResetAllClientState {
                    schema_version: 1,
                    confirmed: false,
                },
                &firesky,
            )
        });
        assert_eq!(result, Err(ClientMaintenanceError::ConfirmationRequired));
        let repair = repair_events(&events);
        assert_eq!(repair.len(), 1);
        assert_eq!(repair[0].level, tracing::Level::WARN);
        assert_eq!(repair[0].field("event"), Some("client_maintenance_failed"));
        assert_eq!(repair[0].field("action"), Some("reset_all_client_state"));
        assert_eq!(repair[0].field("reason"), Some("confirmation_required"));
    }

    #[test]
    fn an_empty_cache_reset_logs_removed_false() {
        let root = tempfile::tempdir().unwrap();
        let firesky = root.path().join("Firesky");
        let mut state = open(root.path());
        let (result, events) = capture(|| {
            tracing::callsite::rebuild_interest_cache();
            state.client_maintenance_fixture(
                ClientMaintenanceCommand::ResetCache { schema_version: 1 },
                &firesky,
            )
        });
        assert_eq!(result.unwrap().removed, Some(false));
        let repair = repair_events(&events);
        assert_eq!(repair.len(), 1);
        assert_eq!(repair[0].level, tracing::Level::INFO);
        assert_eq!(
            repair[0].field("event"),
            Some("client_maintenance_finished")
        );
        assert_eq!(repair[0].field("action"), Some("reset_cache"));
        assert_eq!(repair[0].field("removed"), Some("false"));
        assert_eq!(repair[0].field("outcome"), Some("ok"));
        assert!(repair[0].field("duration_ms").is_some());
    }

    #[test]
    fn an_inspect_logs_nothing() {
        let root = tempfile::tempdir().unwrap();
        let firesky = root.path().join("Firesky");
        let mut state = open(root.path());
        let (result, events) = capture(|| {
            tracing::callsite::rebuild_interest_cache();
            state.client_maintenance_fixture(
                ClientMaintenanceCommand::Inspect { schema_version: 1 },
                &firesky,
            )
        });
        assert!(result.unwrap().removed.is_none());
        assert!(repair_events(&events).is_empty());
    }

    #[test]
    fn storage_errors_use_the_storage_code() {
        assert_eq!(
            ClientMaintenanceError::Storage(StorageError::Busy).code(),
            "busy"
        );
        assert_eq!(ClientMaintenanceError::UnsafePath.code(), "unsafe_path");
    }

    #[test]
    fn ledger_unreadable_carries_its_reason() {
        let ((), events) = capture(|| {
            tracing::callsite::rebuild_interest_cache();
            client_changes_ledger_unreadable("unreadable");
        });
        let repair = repair_events(&events);
        assert_eq!(repair.len(), 1);
        assert_eq!(repair[0].level, tracing::Level::WARN);
        assert_eq!(
            repair[0].field("event"),
            Some("client_changes_ledger_unreadable")
        );
        assert_eq!(repair[0].field("reason"), Some("unreadable"));
    }

    #[test]
    fn removal_failure_carries_the_error_kind() {
        let ((), events) = capture(|| {
            tracing::callsite::rebuild_interest_cache();
            client_maintenance_removal_failed("permission_denied");
        });
        let repair = repair_events(&events);
        assert_eq!(repair[0].level, tracing::Level::WARN);
        assert_eq!(
            repair[0].field("event"),
            Some("client_maintenance_removal_failed")
        );
        assert_eq!(repair[0].field("error_kind"), Some("permission_denied"));
    }
}
