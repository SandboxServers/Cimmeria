//! Real adoption publication followed by installed-content consumer lifecycles.
use super::*;
use crate::storage::adoption;
use std::sync::Mutex;
use tokio_util::sync::CancellationToken;

pub(crate) fn fixture() -> (
    tempfile::TempDir,
    Arc<Mutex<DesktopState>>,
    InstallIntent,
    Vec<u8>,
) {
    let temp = tempfile::tempdir().unwrap();
    let game = temp.path().join("selected");
    std::fs::create_dir(&game).unwrap();
    let seed = temp.path().join("seed.zip");
    let bytes = install_worker::fixtures::archive(true);
    let release = install_worker::fixtures::verified(&bytes);
    std::fs::write(&seed, &bytes).unwrap();
    crate::unpack::unpack(
        &seed,
        &game,
        &crate::unpack::UnpackSink {
            progress: crate::install_progress::ProgressSink::latest().0,
            label: "fixture".into(),
            cancel: CancellationToken::new(),
        },
    )
    .unwrap();
    crate::client_setup::prepare(
        &game,
        &crate::client_setup::login_servers::default_servers(),
    )
    .unwrap();
    crate::state::InstalledState {
        seed_sha256: Some(release.manifest().seed.sha256.clone()),
        ..Default::default()
    }
    .save(&game)
    .unwrap();
    std::fs::write(
        game.join("Working/Binaries/player-notes.txt"),
        b"keep nested notes",
    )
    .unwrap();
    let game = game.canonicalize().unwrap();
    let state = Arc::new(Mutex::new(
        DesktopState::open(&temp.path().join("state")).unwrap(),
    ));
    let request = adoption::PreviewRequest {
        source: adoption::SourceRequest::Folder {
            selected: game.clone(),
        },
        mode: adoption::AdoptionMode::InPlace,
        destination: game,
        operation_revision: 0,
        preferences_revision: 0,
        release,
        artifacts: Some(adoption::Artifacts {
            seed,
            patches: vec![],
        }),
    };
    let preview = adoption::preview(
        state.clone(),
        request,
        CancellationToken::new(),
        crate::install_progress::ProgressSink::latest().0,
    )
    .unwrap();
    let handle = preview.report().preview_handle;
    adoption::confirm(
        preview,
        uuid::Uuid::new_v4(),
        handle,
        adoption::Choices {
            normalize_managed_files: true,
            accept_unavailable_game_telemetry: true,
            old_game_closed: true,
            replace_modified_files: false,
        },
        CancellationToken::new(),
    )
    .unwrap();
    let intent = state
        .lock()
        .unwrap()
        .installed_content()
        .unwrap()
        .unwrap()
        .intent;
    (temp, state, intent, bytes)
}

#[test]
fn direct_validity_survives_play_and_reopen_without_runtime_admission() {
    let (temp, state, intent, _) = fixture();
    assert_eq!(intent.schema_version, 2);
    assert!(!intent.destination.join("game").exists());
    assert!(state.lock().unwrap().has_valid_install().unwrap());
    let mut owner = state.lock().unwrap();
    let revision = owner.operations().snapshot().revision;
    owner
        .operations_mut()
        .unwrap()
        .begin(
            uuid::Uuid::new_v4(),
            OperationKind::Launch,
            [9; 32],
            revision,
        )
        .unwrap();
    assert!(owner.has_valid_install().unwrap());
    drop(owner);
    drop(state);
    let owner = DesktopState::open(&temp.path().join("state")).unwrap();
    assert!(owner.has_valid_install().unwrap());
    std::fs::remove_file(crate::install_layout::sgw_exe(&intent.content_root())).unwrap();
    assert!(!owner.has_valid_install().unwrap());
}

#[test]
fn direct_uninstall_removes_registered_files_and_keeps_selected_root_and_nested_extras() {
    let (_temp, state, intent, _) = fixture();
    let mut owner = state.lock().unwrap();
    let revision = owner.operations().snapshot().revision;
    owner
        .uninstall(uuid::Uuid::new_v4(), revision, intent.operation_id, true)
        .unwrap();
    assert!(intent.destination.is_dir());
    assert!(!crate::install_layout::sgw_exe(&intent.content_root()).exists());
    assert_eq!(
        std::fs::read(intent.destination.join("Working/Binaries/player-notes.txt")).unwrap(),
        b"keep nested notes"
    );
    assert!(!intent.destination.join(".cimmeria-install.json").exists());
    assert!(owner.installed_content().unwrap().is_none());
    assert!(!owner.has_valid_install().unwrap());
}
