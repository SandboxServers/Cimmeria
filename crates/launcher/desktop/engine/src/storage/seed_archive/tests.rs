use super::*;
use crate::install_progress::ProgressSink;
use crate::storage::install_worker::{
    dispatch_with,
    fixtures::{archive, verified},
    tests::{outcome, setup},
    Outcome,
};
use cimmeria_launcher_core::seed_source::LocalFileIdentity;

fn remembered_seed() -> VerifiedLocalSeed {
    VerifiedLocalSeed {
        path: PathBuf::from("player-seed.zip"),
        size: 4,
        sha256: "11".repeat(32),
        identity: LocalFileIdentity {
            volume: 1,
            file_id: 2,
            modified_ns: 3,
        },
    }
}

#[test]
fn seed_archive_for_another_release_is_not_used_after_reopen() {
    let root = tempfile::tempdir().unwrap();
    let release = verified(b"this release's seed");
    {
        let mut state = DesktopState::open(root.path()).unwrap();
        state
            .record_seed_archive(SeedArchiveRecord {
                seed: remembered_seed(),
                seed_sha256: "00".repeat(32),
            })
            .unwrap();
    }
    let state = DesktopState::open(root.path()).unwrap();
    assert!(state.seed_archive.is_some(), "the record survives a reopen");
    assert!(state.seed_source_for(&release).current().is_none());
}

/// A remembered archive for this release is what Install reads, so the install
/// succeeds with the seed blob URL unroutable: a download would fail.
#[tokio::test]
async fn install_uses_remembered_archive_without_downloading_seed() {
    let seed = archive(true);
    let release = verified(&seed);
    let (root, state, id) = setup(&release);
    let player_seed = root.path().join("player-seed.zip");
    std::fs::write(&player_seed, &seed).unwrap();
    let (progress, _observed) = ProgressSink::latest();
    verify_and_remember(&state, &release, &player_seed, &progress).unwrap();
    let worker = dispatch_with(
        state.clone(),
        id,
        release,
        "http://127.0.0.1:9/manifest.json".into(),
        reqwest::Client::new(),
    )
    .unwrap();
    assert_eq!(
        outcome(worker.result.clone()).await,
        Outcome::ContentPrepared
    );
    assert!(root
        .path()
        .join("install/game/Working/Binaries/SGW.exe")
        .is_file());
    assert!(
        player_seed.is_file(),
        "the player's archive is copied, not moved"
    );
}
