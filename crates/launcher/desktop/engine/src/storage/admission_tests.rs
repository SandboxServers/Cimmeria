//! Mutation admission for the LX-02 commands: what each refuses with `Busy`,
//! and that a refusal writes nothing.
use super::client_maintenance::{ClientMaintenanceCommand, ClientMaintenanceError};
use super::log_upload::{LogUploadCommand, LogUploadError, LogUploader};
use super::*;
use crate::OperationKind;
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
enum Blocker {
    Operation(OperationKind),
    Updater,
}

/// A fresh state directory with `blocker` nonterminal. The updater ticket is
/// returned so the check stays busy for the whole test.
fn blocked(blocker: Blocker) -> (tempfile::TempDir, DesktopState, Option<updater::Ticket>) {
    let root = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(root.path()).unwrap();
    let ticket = match blocker {
        Blocker::Operation(kind) => {
            let revision = state.operations().snapshot().revision;
            state
                .operations_mut()
                .unwrap()
                .begin(Uuid::new_v4(), kind, [1; 32], revision)
                .unwrap();
            None
        }
        Blocker::Updater => {
            use base64::Engine;
            let keys = minisign::KeyPair::generate_unencrypted_keypair().unwrap();
            let config = updater::Config::new(
                "https://updates.example.test/feed",
                &base64::engine::general_purpose::STANDARD
                    .encode(keys.pk.to_box().unwrap().to_string()),
                "1.0.0",
                "darwin-aarch64",
                vec!["updates.example.test".into()],
            )
            .unwrap();
            Some(
                state
                    .begin_launcher_update_check(Some(&config), 0, 0)
                    .unwrap(),
            )
        }
    };
    (root, state, ticket)
}

fn files(root: &Path) -> BTreeMap<String, Vec<u8>> {
    std::fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap())
        // Held locked by the open state, and never written after open.
        .filter(|entry| entry.file_name() != "launcher.lock")
        .map(|entry| {
            (
                entry.file_name().to_string_lossy().into_owned(),
                std::fs::read(entry.path()).unwrap(),
            )
        })
        .collect()
}

const BLOCKERS: [Blocker; 4] = [
    Blocker::Operation(OperationKind::Launch),
    Blocker::Operation(OperationKind::Install),
    Blocker::Operation(OperationKind::Repair),
    Blocker::Updater,
];

#[test]
fn resets_and_upload_are_busy_while_any_operation_or_the_updater_runs() {
    for blocker in BLOCKERS {
        let (root, mut state, _ticket) = blocked(blocker);
        let before = files(root.path());
        // A temp profile, never the player's: the refusal is what is under
        // test, and a broken gate must not reach a real deletion.
        let profile = tempfile::tempdir().unwrap();
        let firesky = profile.path().join("Documents/My Games/Firesky");
        let marker = firesky.join("SGWGame/Cache.en-US/marker.pak");
        std::fs::create_dir_all(marker.parent().unwrap()).unwrap();
        std::fs::write(&marker, b"keep").unwrap();
        for request in [
            ClientMaintenanceCommand::ResetCache { schema_version: 1 },
            ClientMaintenanceCommand::ResetAllClientState {
                schema_version: 1,
                confirmed: true,
            },
        ] {
            assert_eq!(
                state.client_maintenance_in(request, &firesky),
                Err(ClientMaintenanceError::Storage(StorageError::Busy)),
                "{request:?} during {blocker:?}"
            );
            assert!(marker.exists(), "{request:?} during {blocker:?}");
        }
        assert_eq!(
            state
                .begin_log_upload(
                    LogUploadCommand::Upload { schema_version: 1 },
                    &LogUploader::new(),
                )
                .err(),
            Some(LogUploadError::Storage(StorageError::Busy)),
            "upload during {blocker:?}"
        );
        assert_eq!(files(root.path()), before, "{blocker:?}");
    }
}

#[test]
fn client_options_save_during_play_only() {
    let servers = || {
        vec![LoginServer {
            name: "Test".into(),
            url: "http://login.example:8081".into(),
        }]
    };
    for blocker in BLOCKERS {
        let (root, mut state, _ticket) = blocked(blocker);
        let before = files(root.path());
        let saved = state.save_client_options(servers(), false, 0);
        if matches!(blocker, Blocker::Operation(OperationKind::Launch)) {
            // Play reads the options only at launch, so the save lands now and
            // applies to the next Play.
            let saved = saved.unwrap();
            assert_eq!(saved.revision, 1);
            assert_eq!(saved.login_servers, servers());
            assert!(!saved.client_patches_enabled);
        } else {
            assert_eq!(saved, Err(StorageError::Busy), "{blocker:?}");
            assert_eq!(state.preferences(), &Preferences::default());
            assert_eq!(files(root.path()), before, "{blocker:?}");
        }
    }
}
