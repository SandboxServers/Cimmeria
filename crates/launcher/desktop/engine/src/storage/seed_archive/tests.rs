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
    verify_and_remember(
        &state,
        &release,
        &player_seed,
        &CancellationToken::new(),
        &progress,
    )
    .unwrap();
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

/// A remembered archive the player moved or deleted: the install fails, and the
/// archive is forgotten, so the next attempt downloads instead of failing again.
#[tokio::test]
async fn a_moved_archive_fails_the_install_and_is_forgotten() {
    let seed = archive(true);
    let release = verified(&seed);
    let (root, state, id) = setup(&release);
    let player_seed = root.path().join("player-seed.zip");
    std::fs::write(&player_seed, &seed).unwrap();
    let (progress, _observed) = ProgressSink::latest();
    verify_and_remember(
        &state,
        &release,
        &player_seed,
        &CancellationToken::new(),
        &progress,
    )
    .unwrap();
    std::fs::remove_file(&player_seed).unwrap();
    let worker = dispatch_with(
        state.clone(),
        id,
        release,
        "http://127.0.0.1:9/manifest.json".into(),
        reqwest::Client::new(),
    )
    .unwrap();
    assert_eq!(outcome(worker.result.clone()).await, Outcome::InstallFailed);
    let owner = state.lock().unwrap();
    assert!(owner.seed_archive().is_none());
    assert!(owner.seed_source.current().is_none());
}

mod downloads_offer {
    use super::*;
    use cimmeria_launcher_core::manifest::SeedEntry;

    fn seed(size: u64) -> SeedEntry {
        SeedEntry {
            blob: "https://archive.org/download/sgw/Stargate%20Worlds%20Game.rar".to_string(),
            size,
            sha256: "22".repeat(32),
        }
    }

    #[test]
    fn archive_name_is_the_decoded_last_segment() {
        assert_eq!(
            archive_file_name("https://archive.org/download/sgw/Stargate%20Worlds%20Game.rar?x=1")
                .as_deref(),
            Some("Stargate Worlds Game.rar")
        );
        assert_eq!(
            archive_file_name("https://archive.org/download/sgw/%2e%2e%2f"),
            None
        );
    }

    #[test]
    fn exact_size_and_name_is_offered() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("Stargate Worlds Game.rar");
        std::fs::write(&file, b"0123456789").unwrap();
        assert_eq!(find_downloads_offer(dir.path(), &seed(10)), Some(file));
    }

    #[test]
    fn wrong_size_is_not_offered() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Stargate Worlds Game.rar"), b"0123456789").unwrap();
        assert_eq!(find_downloads_offer(dir.path(), &seed(11)), None);
    }

    #[test]
    fn subfolder_is_not_searched() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("old");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("Stargate Worlds Game.rar"), b"0123456789").unwrap();
        assert_eq!(find_downloads_offer(dir.path(), &seed(10)), None);
    }

    /// LT-15: a Downloads path that cannot be listed is one closed warn row, and
    /// no offer is made.
    #[test]
    fn unlistable_downloads_folder_is_logged_and_not_offered() {
        let dir = tempfile::tempdir().unwrap();
        let not_a_folder = dir.path().join("file");
        std::fs::write(&not_a_folder, b"0123456789").unwrap();
        let (offer, events) = crate::test_capture::capture(|| {
            tracing::callsite::rebuild_interest_cache();
            find_downloads_offer(&not_a_folder, &seed(10))
        });
        assert_eq!(offer, None);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].target, "launcher.install");
        assert_eq!(events[0].level, tracing::Level::WARN);
        assert_eq!(events[0].field("event"), Some("seed_archive_failed"));
        assert_eq!(events[0].field("reason"), Some("offer_unreadable"));
    }
}
