use super::*;
use crate::test_capture::capture;

#[test]
fn picker_cancellation_is_explicit_on_one_reply_and_not_persisted() {
    let root = tempfile::tempdir().unwrap();
    let host = NativeHost::new(root.path().join("state"));
    let cancelled = host.seed_archive_picker_cancelled().unwrap();
    assert_eq!(
        serde_json::to_value(cancelled).unwrap()["picker_cancelled"],
        true
    );
    let normal = host
        .seed_archive_command(SeedArchiveCommand::Inspect { schema_version: 1 }, None)
        .unwrap();
    assert!(serde_json::to_value(normal)
        .unwrap()
        .get("picker_cancelled")
        .is_none());
}

#[test]
fn catalog_build_key_and_verification_failures_are_not_network_failures() {
    use cimmeria_launcher_engine::catalog::CatalogError;
    for (error, expected) in [
        (CatalogError::Network, "release_unavailable"),
        (
            CatalogError::DevelopmentSigningKey,
            "signing_key_unavailable",
        ),
        (
            CatalogError::SigningKeyUnavailable,
            "signing_key_unavailable",
        ),
        (CatalogError::Signature, "release_verification_failed"),
        (CatalogError::InvalidManifest, "release_verification_failed"),
        (CatalogError::TooLarge, "release_verification_failed"),
    ] {
        assert_eq!(
            serde_json::to_value(SeedArchiveError::from(error)).unwrap(),
            expected
        );
    }
    assert_eq!(
        serde_json::to_value(crate::host::JobError::from(
            CatalogError::DevelopmentSigningKey
        ))
        .unwrap(),
        "signing_key_unavailable"
    );
}

/// An offer whose file cannot be read is logged once and not offered.
#[test]
fn unreadable_offer_is_logged_and_not_offered() {
    let (offer, records) = capture(|| {
        tracing::callsite::rebuild_interest_cache();
        offered_archive(Path::new("missing-seed.zip"))
    });
    assert!(offer.is_none());
    let failed: Vec<_> = records
        .iter()
        .filter(|r| r.field("event") == Some("seed_archive_failed"))
        .collect();
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0].target, "launcher.install");
    assert_eq!(failed[0].level, tracing::Level::WARN);
    assert_eq!(failed[0].field("reason"), Some("offer_unreadable"));
    assert_eq!(failed[0].field("error_kind"), Some("not_found"));
}

/// An offer that vanished after the scan is logged once, then dropped, so
/// the 500 ms Inspect poll does not repeat the warning.
#[test]
fn vanished_offer_is_logged_once_then_dropped() {
    let root = tempfile::tempdir().unwrap();
    let host = NativeHost::new(root.path().join("state"));
    let mut archive = SeedArchive {
        offer: Some(root.path().join("missing-seed.zip")),
        ..SeedArchive::default()
    };
    let ((), records) = capture(|| {
        tracing::callsite::rebuild_interest_cache();
        host.seed_archive_status(&mut archive).unwrap();
        host.seed_archive_status(&mut archive).unwrap();
    });
    let failed = records
        .iter()
        .filter(|r| r.field("event") == Some("seed_archive_failed"))
        .count();
    assert_eq!(failed, 1);
    assert!(archive.offer.is_none());
}

/// Inspect is polled and logs no started or finished row, while Cancel logs both.
#[test]
fn polled_inspect_logs_nothing_and_cancel_logs_started() {
    let root = tempfile::tempdir().unwrap();
    let host = NativeHost::new(root.path().join("state"));
    let (_, inspect) = capture(|| {
        tracing::callsite::rebuild_interest_cache();
        host.seed_archive_command(SeedArchiveCommand::Inspect { schema_version: 1 }, None)
    });
    assert!(inspect.iter().all(|r| !matches!(
        r.field("event"),
        Some("seed_archive_started" | "seed_archive_finished")
    )));
    let (_, cancel) = capture(|| {
        tracing::callsite::rebuild_interest_cache();
        host.seed_archive_command(SeedArchiveCommand::Cancel { schema_version: 1 }, None)
    });
    let events: Vec<_> = cancel.iter().filter_map(|r| r.field("event")).collect();
    assert_eq!(events, ["seed_archive_started", "seed_archive_finished"]);
}

#[test]
fn command_with_unknown_field_is_refused() {
    let json = r#"{"command":"inspect","schema_version":1,"extra":true}"#;
    assert!(serde_json::from_str::<SeedArchiveCommand>(json).is_err());
}

#[test]
fn command_with_a_path_is_refused() {
    let json = r#"{"command":"forget","schema_version":1,"path":"C:\\archive.rar"}"#;
    assert!(serde_json::from_str::<SeedArchiveCommand>(json).is_err());
}

#[test]
fn accept_offer_with_a_path_is_refused() {
    let json = r#"{"command":"accept_offer","schema_version":1,"path":"C:\\archive.rar"}"#;
    assert!(serde_json::from_str::<SeedArchiveCommand>(json).is_err());
}

#[test]
fn plain_command_is_accepted() {
    let json = r#"{"command":"cancel","schema_version":1}"#;
    assert!(matches!(
        serde_json::from_str::<SeedArchiveCommand>(json),
        Ok(SeedArchiveCommand::Cancel { schema_version: 1 })
    ));
}

#[test]
fn error_codes_are_snake_case() {
    let code = |error| serde_json::to_string(&error).unwrap();
    assert_eq!(code(SeedArchiveError::NotAFile), "\"not_a_file\"");
    assert_eq!(code(SeedArchiveError::SizeMismatch), "\"size_mismatch\"");
    assert_eq!(code(SeedArchiveError::HashMismatch), "\"hash_mismatch\"");
    assert_eq!(
        code(SeedArchiveError::ReleaseUnavailable),
        "\"release_unavailable\""
    );
    assert_eq!(code(SeedArchiveError::Busy), "\"busy\"");
}

#[test]
fn failed_release_fetch_waits_before_retry() {
    let mut archive = SeedArchive::default();
    assert!(archive.release_wanted());
    archive.release_failed_at = Some(Instant::now());
    assert!(!archive.release_wanted());
    archive.release_failed_at = Instant::now().checked_sub(RELEASE_RETRY);
    assert!(archive.release_wanted());
}

#[test]
fn one_release_fetch_runs_at_a_time() {
    let mut archive = SeedArchive::default();
    assert!(archive.claim_release());
    // A poll while the fetch runs must not start a second one.
    assert!(!archive.claim_release());
    assert!(!archive.release_wanted());
}

fn seed_entry(sha256: &str, size: u64) -> SeedEntry {
    SeedEntry {
        blob: String::new(),
        size,
        sha256: sha256.to_string(),
    }
}

#[test]
fn stale_archive_is_not_the_chosen_one() {
    let seed = seed_entry(&"AB".repeat(32), 1);
    assert!(matches_release(&"ab".repeat(32), Some(&seed)));
    assert!(!matches_release(&"cd".repeat(32), Some(&seed)));
    // Before a release is cached, a remembered archive still counts as chosen.
    assert!(matches_release(&"cd".repeat(32), None));
}

#[test]
fn check_that_just_started_reports_a_placeholder() {
    let (_progress_sender, progress) = tokio::sync::watch::channel(None);
    let (_result_sender, result) = tokio::sync::watch::channel(None);
    let archive = SeedArchive {
        worker: Some(VerifyWorker {
            progress,
            result,
            cancel: tokio_util::sync::CancellationToken::new(),
        }),
        seed: Some(seed_entry(&"ab".repeat(32), 4096)),
        ..SeedArchive::default()
    };
    assert!(matches!(
        verifying_progress(&archive),
        Some(JobProgress::Download {
            current: 0,
            total: 4096
        })
    ));
    assert!(verifying_progress(&SeedArchive::default()).is_none());
}
