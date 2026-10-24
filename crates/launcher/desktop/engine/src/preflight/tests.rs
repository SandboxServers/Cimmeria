use super::*;
use PreflightOperation::{Install, Play, Repair};

const ALL_OPERATIONS: [PreflightOperation; 3] = [Play, Install, Repair];
const FINE_FOLDER: FolderFacts = FolderFacts {
    chosen: true,
    exists: true,
    writable: true,
};

/// The single check a report built from `facts` holds.
fn only(operation: PreflightOperation, facts: Facts) -> Check {
    let report = evaluate(operation, &facts);
    assert_eq!(report.checks.len(), 1, "{report:?}");
    report.checks[0].clone()
}

fn assert_ok(check: &Check, id: CheckId) {
    assert_eq!(check, &ok(id));
}

/// Pins id, status, action, failure and blocking.
fn assert_check(
    check: &Check,
    id: CheckId,
    status: CheckStatus,
    action: CheckAction,
    failure: FailureCode,
    blocking: bool,
) {
    assert_eq!(
        (
            check.id,
            check.status,
            check.action,
            check.failure,
            check.blocking
        ),
        (id, status, action, Some(failure), blocking),
        "{check:?}"
    );
}

#[test]
fn rosetta_present_is_ok() {
    for operation in ALL_OPERATIONS {
        let facts = Facts {
            rosetta: Some(RosettaProbe::Present),
            ..Facts::default()
        };
        assert_ok(&only(operation, facts), CheckId::Rosetta);
    }
}

#[test]
fn rosetta_missing_blocks_with_auto_install() {
    for operation in ALL_OPERATIONS {
        let facts = Facts {
            rosetta: Some(RosettaProbe::Missing),
            ..Facts::default()
        };
        assert_check(
            &only(operation, facts),
            CheckId::Rosetta,
            CheckStatus::Missing,
            CheckAction::AutoInstall,
            FailureCode::RosettaMissing,
            true,
        );
    }
}

#[test]
fn rosetta_timeout_or_error_is_unknown_and_does_not_block() {
    for probe in [RosettaProbe::TimedOut, RosettaProbe::Error] {
        let facts = Facts {
            rosetta: Some(probe),
            ..Facts::default()
        };
        assert_check(
            &only(Play, facts),
            CheckId::Rosetta,
            CheckStatus::Unknown,
            CheckAction::AskUser,
            FailureCode::RosettaCheckTimedOut,
            false,
        );
    }
}

#[test]
fn wine_runtime_cached_is_ok() {
    let facts = Facts {
        wine_runtime_cached: Some(true),
        ..Facts::default()
    };
    assert_ok(&only(Play, facts), CheckId::WineRuntime);
}

#[test]
fn wine_runtime_missing_downloads_and_blocks_play_and_repair_only() {
    for (operation, blocking) in [(Play, true), (Install, false), (Repair, true)] {
        let facts = Facts {
            wine_runtime_cached: Some(false),
            ..Facts::default()
        };
        let check = only(operation, facts);
        assert_check(
            &check,
            CheckId::WineRuntime,
            CheckStatus::Missing,
            CheckAction::Download,
            FailureCode::WineRuntimeMissing,
            blocking,
        );
        assert_eq!(check.download_bytes, Some(WINE_RUNTIME_BYTES));
    }
}

#[test]
fn helper_unavailable_blocks() {
    let facts = |available| Facts {
        helper_available: Some(available),
        ..Facts::default()
    };
    assert_ok(&only(Play, facts(true)), CheckId::CompatibilityHelper);
    assert_check(
        &only(Play, facts(false)),
        CheckId::CompatibilityHelper,
        CheckStatus::Missing,
        CheckAction::Manual,
        FailureCode::HelperUnavailable,
        true,
    );
}

#[test]
fn low_disk_blocks_install_and_repair_with_the_bytes() {
    for operation in [Install, Repair] {
        let free = INSTALL_BYTES - 1;
        let facts = Facts {
            free_bytes: Some(free),
            ..Facts::default()
        };
        let check = only(operation, facts);
        assert_check(
            &check,
            CheckId::DiskSpace,
            CheckStatus::Missing,
            CheckAction::Manual,
            FailureCode::DiskSpaceLow,
            true,
        );
        assert_eq!(
            (check.bytes_needed, check.bytes_free),
            (Some(INSTALL_BYTES), Some(free))
        );
        let enough = Facts {
            free_bytes: Some(INSTALL_BYTES),
            ..Facts::default()
        };
        assert_ok(&only(operation, enough), CheckId::DiskSpace);
    }
}

#[test]
fn low_disk_only_warns_for_play() {
    let facts = |free| Facts {
        free_bytes: Some(free),
        ..Facts::default()
    };
    let check = only(Play, facts(PLAY_WARN_BYTES - 1));
    assert_check(
        &check,
        CheckId::DiskSpace,
        CheckStatus::Warning,
        CheckAction::Manual,
        FailureCode::DiskSpaceLow,
        false,
    );
    assert_eq!(
        (check.bytes_needed, check.bytes_free),
        (Some(PLAY_WARN_BYTES), Some(PLAY_WARN_BYTES - 1))
    );
    // Play needs far less than an install: 1 GB free is enough.
    assert_ok(&only(Play, facts(PLAY_WARN_BYTES)), CheckId::DiskSpace);
}

#[test]
fn unchosen_folder_asks_the_user_and_blocks() {
    for operation in ALL_OPERATIONS {
        let facts = Facts {
            folder: Some(FolderFacts {
                chosen: false,
                exists: false,
                writable: false,
            }),
            ..Facts::default()
        };
        assert_check(
            &only(operation, facts),
            CheckId::GameFolder,
            CheckStatus::Missing,
            CheckAction::AskUser,
            FailureCode::GameFolderNotChosen,
            true,
        );
    }
}

#[test]
fn absent_folder_blocks_play_and_repair_but_not_install() {
    let absent = FolderFacts {
        exists: false,
        ..FINE_FOLDER
    };
    for operation in [Play, Repair] {
        let facts = Facts {
            // What folder_facts(.., true) returns for a missing folder.
            folder: Some(FolderFacts {
                chosen: true,
                exists: false,
                writable: false,
            }),
            ..Facts::default()
        };
        assert_check(
            &only(operation, facts),
            CheckId::GameFolder,
            CheckStatus::Missing,
            CheckAction::Manual,
            FailureCode::GameFolderMissing,
            true,
        );
    }
    let facts = Facts {
        folder: Some(absent),
        ..Facts::default()
    };
    assert_ok(&only(Install, facts), CheckId::GameFolder);
}

#[test]
fn unwritable_folder_blocks() {
    for operation in ALL_OPERATIONS {
        let facts = Facts {
            folder: Some(FolderFacts {
                writable: false,
                ..FINE_FOLDER
            }),
            ..Facts::default()
        };
        assert_check(
            &only(operation, facts),
            CheckId::GameFolder,
            CheckStatus::Missing,
            CheckAction::Manual,
            FailureCode::GameFolderNotWritable,
            true,
        );
        let fine = Facts {
            folder: Some(FINE_FOLDER),
            ..Facts::default()
        };
        assert_ok(&only(operation, fine), CheckId::GameFolder);
    }
}

#[test]
fn manifest_failures_block_with_their_codes() {
    for (probe, failure) in [
        (ManifestProbe::Unreachable, FailureCode::ManifestUnreachable),
        (ManifestProbe::Invalid, FailureCode::ManifestInvalid),
        (
            ManifestProbe::SigningKeyUnavailable,
            FailureCode::SigningKeyUnavailable,
        ),
    ] {
        let facts = Facts {
            manifest: Some(probe),
            ..Facts::default()
        };
        assert_check(
            &only(Install, facts),
            CheckId::ReleaseManifest,
            CheckStatus::Missing,
            CheckAction::Manual,
            failure,
            true,
        );
    }
    let fine = Facts {
        manifest: Some(ManifestProbe::Ok),
        ..Facts::default()
    };
    assert_ok(&only(Repair, fine), CheckId::ReleaseManifest);
}

#[test]
fn translocation_only_warns() {
    let facts = |translocated| Facts {
        translocated: Some(translocated),
        ..Facts::default()
    };
    assert_ok(&only(Play, facts(false)), CheckId::AppLocation);
    assert_check(
        &only(Play, facts(true)),
        CheckId::AppLocation,
        CheckStatus::Warning,
        CheckAction::Manual,
        FailureCode::AppTranslocated,
        false,
    );
}

#[test]
fn missing_physx_only_warns() {
    let facts = |installed| Facts {
        physx_installed: Some(installed),
        ..Facts::default()
    };
    assert_ok(&only(Play, facts(true)), CheckId::Physx);
    assert_check(
        &only(Play, facts(false)),
        CheckId::Physx,
        CheckStatus::Warning,
        CheckAction::AutoInstall,
        FailureCode::PhysxMissing,
        false,
    );
}

#[test]
fn ready_is_false_with_one_blocking_check_and_true_with_only_warnings() {
    let warnings = Facts {
        free_bytes: Some(0),
        translocated: Some(true),
        physx_installed: Some(false),
        rosetta: Some(RosettaProbe::TimedOut),
        ..Facts::default()
    };
    let report = evaluate(Play, &warnings);
    assert_eq!(report.checks.len(), 4);
    assert!(report.ready, "{report:?}");
    assert_eq!(report.schema_version, 1);
    assert_eq!(report.operation, Play);

    let blocked = Facts {
        helper_available: Some(false),
        ..warnings
    };
    assert!(!evaluate(Play, &blocked).ready);
}

#[test]
fn empty_facts_give_an_empty_ready_report() {
    let report = evaluate(Repair, &Facts::default());
    assert!(report.checks.is_empty());
    assert!(report.ready);
}

#[test]
fn checks_follow_check_id_order() {
    let facts = Facts {
        rosetta: Some(RosettaProbe::Present),
        wine_runtime_cached: Some(true),
        helper_available: Some(true),
        free_bytes: Some(u64::MAX),
        folder: Some(FINE_FOLDER),
        manifest: Some(ManifestProbe::Ok),
        translocated: Some(false),
        physx_installed: Some(true),
    };
    let ids: Vec<CheckId> = evaluate(Install, &facts)
        .checks
        .iter()
        .map(|check| check.id)
        .collect();
    assert_eq!(
        ids,
        [
            CheckId::Rosetta,
            CheckId::WineRuntime,
            CheckId::CompatibilityHelper,
            CheckId::DiskSpace,
            CheckId::GameFolder,
            CheckId::ReleaseManifest,
            CheckId::AppLocation,
            CheckId::Physx,
        ]
    );
}

#[test]
fn report_serializes_snake_case_with_null_optionals() {
    let facts = Facts {
        wine_runtime_cached: Some(false),
        ..Facts::default()
    };
    let json = serde_json::to_value(evaluate(Play, &facts)).unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "schema_version": 1,
            "operation": "play",
            "checks": [{
                "id": "wine_runtime",
                "status": "missing",
                "action": "download",
                "failure": "wine_runtime_missing",
                "blocking": true,
                "bytes_needed": null,
                "bytes_free": null,
                "download_bytes": WINE_RUNTIME_BYTES,
            }],
            "ready": false,
        })
    );
}

#[test]
fn free_bytes_is_positive_on_a_temp_dir() {
    let dir = tempfile::tempdir().unwrap();
    assert!(matches!(free_bytes(dir.path()), Some(n) if n > 0));
}

#[test]
fn free_bytes_on_a_missing_child_reads_the_existing_ancestor() {
    let dir = tempfile::tempdir().unwrap();
    let child = dir.path().join("absent").join("deeper");
    // Other tests write to the same volume, so allow a few tries for the two
    // reads to land between writes.
    let matched = (0..5).any(|_| {
        let parent = free_bytes(dir.path());
        let missing = free_bytes(&child);
        parent.is_some() && parent == missing
    });
    assert!(matched);
}

#[test]
fn folder_facts_on_a_temp_dir_is_writable_and_leaves_no_probe() {
    let dir = tempfile::tempdir().unwrap();
    let facts = folder_facts(Some(dir.path()), true);
    assert_eq!(facts, FINE_FOLDER);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn folder_facts_on_a_missing_path() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("game");
    let must_exist = folder_facts(Some(&missing), true);
    assert_eq!(
        must_exist,
        FolderFacts {
            chosen: true,
            exists: false,
            writable: false,
        }
    );
    // An install creates the folder, so the ancestor's writability counts.
    let install = folder_facts(Some(&missing), false);
    assert_eq!(
        install,
        FolderFacts {
            chosen: true,
            exists: false,
            writable: true,
        }
    );
}

#[test]
fn folder_facts_without_a_path_is_not_chosen() {
    assert!(!folder_facts(None, false).chosen);
}
