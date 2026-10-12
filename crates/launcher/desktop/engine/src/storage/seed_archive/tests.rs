use super::*;
use crate::storage::install_worker::fixtures::verified;
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
