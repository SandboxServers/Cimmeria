//! A folder adoption (AI-10 part b2) publishes and launches like a fresh install.
//! No legacy import is saved here, so nothing reviewed in one can bind the copy.
use super::in_place_tests::commit;
use super::*;
use crate::install_worker::fixtures;

#[test]
fn a_folder_adoption_publishes_and_follows_the_patch_preference() {
    let root = tempfile::tempdir().unwrap();
    let game = root.path().join("folder");
    std::fs::create_dir(&game).unwrap();
    let seed = root.path().join("seed.zip");
    let bytes = fixtures::archive(true);
    std::fs::write(&seed, &bytes).unwrap();
    let release = fixtures::verified(&bytes);
    let (progress, _) = crate::install_progress::ProgressSink::latest();
    crate::unpack::unpack(
        &seed,
        &game,
        &crate::unpack::UnpackSink {
            progress,
            label: "fixture".into(),
            cancel: CancellationToken::new(),
        },
    )
    .unwrap();
    // The folder holds what a fresh install writes, as the imported folders do.
    crate::client_setup::prepare(
        &game,
        &crate::client_setup::login_servers::default_servers(),
    )
    .unwrap();
    let game = game.canonicalize().unwrap();
    let state = Arc::new(Mutex::new(
        DesktopState::open(&root.path().join("state")).unwrap(),
    ));
    let request = {
        let owner = state.lock().unwrap();
        PreviewRequest {
            source: SourceRequest::Folder {
                selected: game.clone(),
            },
            destination: game.clone(),
            operation_revision: owner.operations().snapshot().revision,
            preferences_revision: owner.preferences().revision,
            release: crate::catalog::verify_release(release.evidence().0, release.evidence().1)
                .unwrap(),
            artifacts: Some(Artifacts {
                seed,
                patches: vec![],
            }),
        }
    };
    let previewed = preview(
        state.clone(),
        request,
        CancellationToken::new(),
        crate::install_progress::ProgressSink::latest().0,
    )
    .unwrap();
    assert!(matches!(previewed.source, AdoptionSource::Folder(_)));
    let provenance = commit(previewed).unwrap();
    assert_eq!(provenance.legacy_install_id, None);
    assert_eq!(provenance.import_digest, None);
    assert!(provenance.folder_scan_digest.is_some());

    // Both patch choices are followed: a folder has no reviewed choice to keep.
    for preference in [true, false] {
        let mut owner = state.lock().unwrap();
        let revision = owner.preferences().revision;
        owner
            .save_client_options(vec![], preference, revision)
            .unwrap();
        assert_eq!(owner.effective_launch_binding(), Ok(None));
        assert_eq!(
            owner.injection_policy().unwrap().client_patches,
            preference,
            "preference {preference}"
        );
    }
}
