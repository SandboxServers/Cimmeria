use super::*;
use std::io::Error;

#[test]
fn each_io_kind_has_its_closed_cause() {
    for (kind, cause) in [
        (ErrorKind::PermissionDenied, "permission_denied"),
        (ErrorKind::NotFound, "not_found"),
        (ErrorKind::AlreadyExists, "already_exists"),
        (ErrorKind::DirectoryNotEmpty, "directory_not_empty"),
        (ErrorKind::ReadOnlyFilesystem, "read_only"),
        (ErrorKind::StorageFull, "storage_full"),
        (ErrorKind::Interrupted, "other"),
        (ErrorKind::Other, "other"),
    ] {
        assert_eq!(io_cause(&Error::from(kind)), cause, "{kind:?}");
    }
}

#[test]
fn a_cause_never_carries_the_error_text() {
    let error = Error::new(ErrorKind::PermissionDenied, "/Users/someone/Games");
    assert_eq!(io_cause(&error), "permission_denied");
}

#[test]
fn each_claim_error_has_its_closed_cause() {
    for (error, cause) in [
        (StorageError::InvalidDirectory, "invalid_directory"),
        (StorageError::UnsafeFile, "unsafe_file"),
        (StorageError::InUse, "in_use"),
        (StorageError::Corrupt, "corrupt"),
        (StorageError::Io, "io"),
        (StorageError::Busy, "other"),
    ] {
        assert_eq!(storage_cause(&error), cause, "{error:?}");
    }
}

// The worker call sites (`install_worker/mod.rs`): each refusal logs its stage
// and a closed cause, and the outcome is unchanged (R10).
mod worker {
    use super::super::super::tests::{archive, setup, verified};
    use super::super::super::*;
    use crate::test_capture::{capture, Captured};
    use cimmeria_launcher_core::seed_source::SeedSourceHandle;

    fn intent_of(state: &Mutex<DesktopState>) -> InstallIntent {
        state.lock().unwrap().install_intent().unwrap().unwrap()
    }

    fn block_on<F: std::future::Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(future)
    }

    /// The single `destination_unavailable` row of a capture.
    fn refusal(events: &[Captured]) -> &Captured {
        let rows: Vec<_> = events
            .iter()
            .filter(|e| e.field("event") == Some("destination_unavailable"))
            .collect();
        assert_eq!(rows.len(), 1, "{events:?}");
        assert_eq!(rows[0].target, "launcher.install");
        assert_eq!(rows[0].level, tracing::Level::WARN);
        assert_eq!(
            rows[0].field("failure_code"),
            Some("game_folder_not_writable")
        );
        rows[0]
    }

    #[test]
    fn a_destination_that_is_a_file_logs_the_check_and_its_cause() {
        let release = verified(&archive(true));
        let (root, state, _) = setup(&release);
        std::fs::write(root.path().join("install"), b"not a folder").unwrap();
        let intent = intent_of(&state);
        let ((outcome, report), events) = capture(|| {
            block_on(install_owned(
                &intent,
                &root.path().join("state"),
                &release,
                "https://example.invalid/manifest.json",
                &reqwest::Client::new(),
                SeedSourceHandle::download(),
                CancellationToken::new(),
                ProgressSink::latest().0,
            ))
        });
        assert_eq!(outcome, Outcome::DestinationUnavailable);
        assert!(report.is_none());
        let row = refusal(&events);
        assert_eq!(row.field("stage"), Some("check"));
        assert_eq!(row.field("cause"), Some("invalid_directory"));
        let folder = root.path().to_string_lossy().into_owned();
        assert!(row.fields.values().all(|v| !v.contains(&folder)));
    }

    #[test]
    fn a_staging_folder_that_cannot_be_created_logs_create() {
        let release = verified(&archive(true));
        let (_root, state, _) = setup(&release);
        // The destination was never claimed, so the staging folder has no parent.
        let intent = intent_of(&state);
        let ((outcome, _), events) = capture(|| {
            block_on(install_stage(
                &intent,
                &release,
                "https://example.invalid/manifest.json",
                &reqwest::Client::new(),
                SeedSourceHandle::download(),
                CancellationToken::new(),
                ProgressSink::latest().0,
            ))
        });
        assert_eq!(outcome, Outcome::DestinationUnavailable);
        let row = refusal(&events);
        assert_eq!(row.field("stage"), Some("create"));
        assert_eq!(row.field("cause"), Some("not_found"));
    }

    #[test]
    fn existing_game_content_refuses_promotion_as_already_exists() {
        let release = verified(&archive(true));
        let (root, state, _) = setup(&release);
        std::fs::create_dir_all(root.path().join("install/game")).unwrap();
        let stage = root.path().join("install/stage");
        std::fs::create_dir(&stage).unwrap();
        let (outcome, events) = capture(|| promote(&intent_of(&state), &stage));
        assert_eq!(outcome, Outcome::DestinationUnavailable);
        let row = refusal(&events);
        assert_eq!(row.field("stage"), Some("promote"));
        assert_eq!(row.field("cause"), Some("already_exists"));
    }

    #[test]
    fn a_failed_promote_rename_logs_its_cause_and_stays_install_failed() {
        let release = verified(&archive(true));
        let (root, state, _) = setup(&release);
        // A stage that is gone makes the rename itself fail.
        let stage = root.path().join("install/stage-gone");
        let (outcome, events) = capture(|| promote(&intent_of(&state), &stage));
        assert_eq!(outcome, Outcome::InstallFailed);
        let row = refusal(&events);
        assert_eq!(row.field("stage"), Some("promote"));
        assert_eq!(row.field("cause"), Some("not_found"));
    }
}
