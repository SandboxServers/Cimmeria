//! Real adoption paths emit scan workloads without changing the publication contract.
use super::*;
use crate::phase_timing::{Context, Phase as TimedPhase};
use crate::test_capture::capture;

#[test]
fn real_adoption_times_preview_confirmation_and_final_scans() {
    let f = in_place_tests::Fixture::new(None);
    let initial = f.scan();
    let bytes: u64 = initial.values().map(|e| e.size).sum();
    let (_, events) = capture(|| {
        let preview = f.preview().unwrap();
        in_place_tests::commit(preview).unwrap();
    });
    for phase in [
        "adoption_preview",
        "adoption_initial_scan",
        "adoption_index",
        "adoption_comparison",
        "adoption_review_scan",
        "adoption_confirmation",
        "adoption_confirmation_scan",
        "adoption_apply",
        "adoption_publication_lock",
        "adoption_setup",
        "adoption_final_scan",
    ] {
        let finished: Vec<_> = events
            .iter()
            .filter(|e| {
                e.field("phase") == Some(phase) && e.field("event") == Some("phase_finished")
            })
            .collect();
        assert!(!finished.is_empty(), "missing successful {phase}");
        // Publication may idempotently apply the reviewed files again. Each
        // real pass gets its own pair; do not hide it or require a single pass.
        if phase != "adoption_apply" {
            assert_eq!(finished.len(), 1, "exactly one {phase}");
        }
        assert!(
            finished
                .iter()
                .all(|e| e.field("outcome") == Some("succeeded")),
            "{phase}"
        );
        assert!(finished[0]
            .field("duration_ms")
            .unwrap()
            .parse::<u64>()
            .is_ok());
        if phase == "adoption_initial_scan" || phase == "adoption_confirmation_scan" {
            assert_eq!(
                finished[0].field("files"),
                Some(initial.len().to_string().as_str())
            );
            assert_eq!(finished[0].field("bytes"), Some(bytes.to_string().as_str()));
        }
    }
    in_place_tests::assert_player_files_untouched(&f);
}

#[test]
fn cancelled_scan_records_cancellation_and_no_completed_workload() {
    let f = in_place_tests::Fixture::new(None);
    let cancel = CancellationToken::new();
    cancel.cancel();
    let (result, events) = capture(|| {
        timing::scan(Context::preview(), TimedPhase::AdoptionInitialScan, || {
            inventory::scan(f.folder(), &cancel)
        })
    });
    assert_eq!(result.unwrap_err(), Error::Cancelled);
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].field("outcome"), Some("cancelled"));
    assert_eq!(events[1].field("files"), None);
    assert_eq!(events[1].field("bytes"), None);
}

#[test]
fn changed_source_failure_times_the_confirmation_rescan() {
    let f = in_place_tests::Fixture::new(None);
    let preview = f.preview().unwrap();
    std::fs::write(f.folder().join("new-player-file.txt"), b"new").unwrap();
    let (result, events) = capture(|| in_place_tests::commit(preview));
    assert!(matches!(result, Err(Error::SourceChanged)));
    let confirmation = events
        .iter()
        .find(|e| {
            e.field("phase") == Some("adoption_confirmation")
                && e.field("event") == Some("phase_finished")
        })
        .unwrap();
    assert_eq!(confirmation.field("outcome"), Some("failed"));
    assert!(events
        .iter()
        .any(|e| e.field("phase") == Some("adoption_confirmation_scan")
            && e.field("files").is_some()));
    assert!(!events
        .iter()
        .any(|e| e.field("phase") == Some("adoption_apply")));
}
