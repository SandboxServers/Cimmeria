//! Golden v1 bytes of an in-place adoption plan and its provenance. The
//! fixtures are what the schema-1 build serializes (compact, as
//! `publication.rs` writes them). Later model changes must keep reading them,
//! re-serializing them byte for byte and hashing the same plan digest.
//! Regenerate only deliberately: `CIMMERIA_WRITE_GOLDEN=1` with this test
//! filter rewrites the fixtures, and the digest then has to be updated by hand.
use super::inventory::Identity;
use super::model::{Choices, Classification, Difference, FileCheck, Plan, Provenance, Report};
use crate::storage::{
    install_intent::{ExtractionBackend, InstallIntent},
    migration::{
        self, LegacyConfig, LegacyIdentity, LegacyLedger, LegacyPatches, LegacySource,
        LegacyTelemetry, LoginServer,
    },
    Preferences,
};
use std::path::PathBuf;
use uuid::Uuid;

pub(super) const PLAN_BYTES: &[u8] = include_bytes!("fixtures/plan-v1-in-place.json");
const PROVENANCE_BYTES: &[u8] = include_bytes!("fixtures/provenance-v1.json");
/// Hex of `plan().digest()`, the schema-1 intent digest of the fixture.
pub(super) const PLAN_DIGEST: &str =
    "115c9e09bba351247498d457fccf64c1690fcdb237ab6f0aa9d46e3af75c9179";

const WORK: u128 = 0x6f1c2a4e_3b5d_4e7f_8a9b_0c1d2e3f4a5b;
const OWNER: u128 = 0x1a2b3c4d_5e6f_4a7b_8c9d_0e1f2a3b4c5d;
const LEGACY: u128 = 0x9e8d7c6b_5a4f_4e3d_8c2b_1a0f9e8d7c6b;
const PREVIEW: u128 = 0x0c1d2e3f_4a5b_4c6d_8e7f_9a0b1c2d3e4f;
/// Plain strings, not `Path` joins: the bytes are the same on every OS.
const GAME: &str = "C:\\Game";

pub(super) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn provenance() -> Provenance {
    Provenance {
        work_id: Uuid::from_u128(WORK),
        legacy_install_id: Some(Uuid::from_u128(LEGACY)),
        import_digest: Some("confirmation".into()),
        reference_digest: [8; 32],
        setup_policy_version: 1,
        folder_scan_digest: None,
        file_index_sha256: None,
        check: FileCheck::Full,
    }
}

/// `Identity`'s length and mtime fields are private to the inventory module,
/// so the fixed value comes from its serialized form.
fn identity(device: u64) -> Identity {
    serde_json::from_str(&format!(
        r#"{{"device":{device},"inode":2,"length":3,"modified_ns":4}}"#
    ))
    .unwrap()
}

fn preferences(revision: u64) -> Preferences {
    Preferences {
        schema_version: 2,
        revision,
        install_directory: Some(PathBuf::from(GAME)),
        launcher_summary_consent: true,
        login_servers: Vec::new(),
        client_patches_enabled: true,
        display: Default::default(),
        display_placements: Vec::new(),
    }
}

/// An in-place adoption: the reviewed folder is both source and destination.
fn plan() -> Plan {
    let legacy = LegacyIdentity {
        schema_version: 1,
        install_id: Uuid::from_u128(LEGACY),
        machine_id: "machine".into(),
        first_seen_ms: 1,
        created_by_launcher_version: "0.9.0".into(),
    };
    let config = LegacyConfig {
        schema_version: 1,
        install_path: PathBuf::from(GAME),
        manifest_url: "https://example.test/manifest.json".into(),
        login_servers: vec![LoginServer {
            name: "Cimmeria".into(),
            url: "http://login.example:8081".into(),
        }],
        telemetry: LegacyTelemetry {
            opted_in: false,
            prompt_answered: false,
            auth_url: "http://login.example:8081/api".into(),
        },
        client_patches: LegacyPatches {
            enabled: true,
            dll_override: None,
        },
    };
    Plan {
        schema_version: 1,
        installation: InstallIntent {
            schema_version: 1,
            operation_id: Uuid::from_u128(OWNER),
            preferences_revision: 3,
            destination: PathBuf::from(GAME),
            manifest_digest: [7; 32],
            login_servers: Vec::new(),
            backend: ExtractionBackend::Native,
        },
        provenance: provenance(),
        imported: Some(migration::LegacyImport {
            source: LegacySource {
                launcher_directory: PathBuf::from("C:\\Legacy"),
                game_directory: PathBuf::from(GAME),
            },
            identity: legacy.clone(),
            config: config.clone(),
            ledger: LegacyLedger {
                applied_patches: Vec::new(),
                seed_sha256: None,
                seed_adopted: false,
            },
            confirmation: "confirmation".into(),
        }),
        folder: None,
        before: preferences(3),
        after: preferences(4),
        report: Report {
            preview_handle: Uuid::from_u128(PREVIEW),
            source: PathBuf::from(GAME),
            destination: PathBuf::from(GAME),
            release_digest: [7; 32],
            imported_identity: Some(legacy),
            requested_config: Some(config),
            files: vec![Difference {
                path: "Working/Binaries/SGW.exe".into(),
                source_path: None,
                classification: Classification::Matched,
            }],
            user_data_remains_in_source: true,
            game_telemetry_available: false,
            index_fallback: None,
            download: None,
            backup: None,
            login_replaced: false,
        },
        choices: Choices {
            normalize_managed_files: true,
            accept_unavailable_game_telemetry: false,
            old_game_closed: true,
            replace_modified_files: false,
        },
        source_digest: [9; 32],
        stage_identity: identity(1),
        destination_identity: identity(5),
        replacements: Vec::new(),
        incomplete_patches: Vec::new(),
    }
}

#[test]
fn golden_fixtures_are_written_only_when_requested() {
    if std::env::var_os("CIMMERIA_WRITE_GOLDEN").is_none() {
        return;
    }
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/storage/adoption/fixtures");
    std::fs::write(
        format!("{dir}/plan-v1-in-place.json"),
        serde_json::to_vec(&plan()).unwrap(),
    )
    .unwrap();
    std::fs::write(
        format!("{dir}/provenance-v1.json"),
        serde_json::to_vec(&provenance()).unwrap(),
    )
    .unwrap();
}

#[test]
fn golden_plan_v1_serializes_to_the_fixture_bytes() {
    assert_eq!(serde_json::to_vec(&plan()).unwrap(), PLAN_BYTES);
}

#[test]
fn golden_plan_v1_fixture_reads_back_to_the_fixed_value() {
    let read: Plan = serde_json::from_slice(PLAN_BYTES).unwrap();
    assert_eq!(read, plan());
}

#[test]
fn golden_plan_v1_digest_matches_the_hard_coded_constant() {
    assert_eq!(hex(&plan().digest().unwrap()), PLAN_DIGEST);
}

#[test]
fn golden_provenance_v1_serializes_to_the_fixture_bytes() {
    assert_eq!(serde_json::to_vec(&provenance()).unwrap(), PROVENANCE_BYTES);
}

#[test]
fn golden_provenance_v1_fixture_reads_back_to_the_fixed_value() {
    let read: Provenance = serde_json::from_slice(PROVENANCE_BYTES).unwrap();
    assert_eq!(read, provenance());
}

#[test]
fn choices_without_replace_modified_files_read_false_and_stay_unwritten() {
    let old = r#"{"normalize_managed_files":true,"accept_unavailable_game_telemetry":false,"old_game_closed":true}"#;
    let read: Choices = serde_json::from_str(old).unwrap();
    assert!(!read.replace_modified_files);
    assert_eq!(serde_json::to_string(&read).unwrap(), old);
}

#[test]
fn plan_source_names_exactly_one_of_legacy_or_folder() {
    use super::model::{AdoptionSourceRef, FolderSource};
    let legacy = plan();
    assert!(matches!(legacy.source(), Ok(AdoptionSourceRef::Legacy(_))));
    let folder = FolderSource {
        selected: PathBuf::from(GAME),
        root: PathBuf::from(GAME),
        scan_digest: [3; 32],
    };
    let mut folder_plan = legacy.clone();
    folder_plan.imported = None;
    folder_plan.folder = Some(folder.clone());
    assert_eq!(
        folder_plan.source().unwrap(),
        AdoptionSourceRef::Folder(&folder)
    );
    let mut both = legacy.clone();
    both.folder = Some(folder);
    assert!(both.source().is_err());
    let mut neither = legacy;
    neither.imported = None;
    assert!(neither.source().is_err());
}
