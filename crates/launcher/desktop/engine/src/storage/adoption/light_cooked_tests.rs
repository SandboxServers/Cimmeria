//! Light adoption and the bundled cooked-data tier (LA-11b): the 2009 cabinets keep the
//! stock PAKs under `Cache.en-US`, a fresh install places them under
//! `SourceCache.en-us`, and a light adoption must leave the same single tier.
use super::in_place_tests::{commit, commit_with, crash_after_first_move};
use super::light_replay::tests::{big, patch_zip, OpFixture};
use super::light_tests::Light;
use super::*;
use crate::storage::install_worker::content_valid;

const STOCK_A: &str = "Working/SGWGame/Cache.en-US/A.pak";
const STOCK_B: &str = "Working/SGWGame/Cache.en-US/B.pak";
const PLACED_A: &str = "Working/SGWGame/SourceCache.en-us/A.pak";
const PLACED_B: &str = "Working/SGWGame/SourceCache.en-us/B.pak";
const PLACED_C: &str = "Working/SGWGame/SourceCache.en-us/C.pak";

/// One patch that edits `A.pak` at its placed path.
fn pak_patch() -> Vec<Vec<u8>> {
    vec![patch_zip(
        "p0",
        &[OpFixture {
            target: PLACED_A,
            sources: &[(PLACED_A, &big("stock a"))],
            result: &big("patched a"),
        }],
        &[],
    )]
}

async fn stock_tier(extra: &[(&str, &[u8])]) -> Light {
    let (a, b) = (big("stock a"), big("stock b"));
    let mut files: Vec<(&str, &[u8])> = vec![(STOCK_A, &a), (STOCK_B, &b)];
    files.extend_from_slice(extra);
    Light::new(pak_patch(), &files).await
}

fn read(root: &Path, rel: &str) -> Vec<u8> {
    std::fs::read(inventory::native(root, rel)).unwrap()
}

fn exists(root: &Path, rel: &str) -> bool {
    inventory::native(root, rel).exists()
}

fn assert_placed_tree(lt: &Light) {
    let game = lt.game();
    assert_eq!(read(&game, PLACED_A), big("patched a"));
    assert_eq!(read(&game, PLACED_B), big("stock b"));
    assert!(!exists(&game, "Working/SGWGame/Cache.en-US"));
    assert!(!exists(&lt.folder, "Working/SGWGame/Cache.en-US"));
    assert!(content_valid(&game, &lt.verified()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stock_tier_publishes_as_one_placed_tier() {
    let lt = stock_tier(&[]).await;
    commit(lt.preview().unwrap()).unwrap();
    assert_placed_tree(&lt);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_review_lists_placed_rows_and_copies_no_renamed_file() {
    let lt = stock_tier(&[]).await;
    let preview = lt.preview().unwrap();
    let files = &preview.report().files;
    assert!(files
        .iter()
        .all(|d| !d.path.to_ascii_lowercase().contains("/cache.en-us/")));
    let b = files.iter().find(|d| d.path == PLACED_B).unwrap();
    assert_eq!(b.source_path.as_deref(), Some(STOCK_B));
    assert_eq!(b.classification, Classification::KnownTransform);
    let a = files.iter().find(|d| d.path == PLACED_A).unwrap();
    assert_eq!(a.source_path.as_deref(), Some(STOCK_A));
    assert_eq!(a.classification, Classification::NeedsPatching);
    let store = preview.preparation.record.directory.join("replacements");
    assert!(exists(&store, PLACED_A));
    assert!(!exists(&store, PLACED_B), "a renamed file is never copied");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_folder_with_both_tiers_is_left_as_it_is() {
    let lt = stock_tier(&[(PLACED_C, b"c")]).await;
    commit(lt.preview().unwrap()).unwrap();
    let game = lt.game();
    assert_eq!(read(&game, STOCK_B), big("stock b"));
    assert_eq!(read(&game, PLACED_C), b"c");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn abandon_puts_the_stock_tier_back() {
    let lt = stock_tier(&[]).await;
    assert!(commit_with(lt.preview().unwrap(), crash_after_first_move).is_err());
    let (id, revision) = {
        let state = lt.state.lock().unwrap();
        let snapshot = state.operations().snapshot();
        (snapshot.operation.clone().unwrap().id, snapshot.revision)
    };
    abandon(&mut lt.state.lock().unwrap(), id, revision).unwrap();
    assert_eq!(read(&lt.folder, STOCK_A), big("stock a"));
    assert_eq!(read(&lt.folder, STOCK_B), big("stock b"));
    assert!(!exists(&lt.folder, "Working/SGWGame/SourceCache.en-us"));
    assert!(lt.game().exists());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn recovery_publishes_the_placed_tree_from_either_crash_point() {
    for crash in [
        publication::Point::BeforePromotion,
        publication::Point::Moved,
    ] {
        let lt = stock_tier(&[]).await;
        let failed = commit_with(lt.preview().unwrap(), |point| {
            if point == crash {
                Err(StorageError::Io.into())
            } else {
                Ok(())
            }
        });
        assert!(failed.is_err(), "{crash:?}");
        let state_dir = lt.root.path().join("state");
        let id = lt
            .state
            .lock()
            .unwrap()
            .operations()
            .snapshot()
            .operation
            .clone()
            .unwrap()
            .id;
        let (game, folder, release) = (lt.game(), lt.folder.clone(), lt.verified());
        drop(lt.state);
        let mut reopened = DesktopState::open(&state_dir).unwrap();
        let revision = reopened.operations().snapshot().revision;
        recover(&mut reopened, id, revision).unwrap();
        assert_eq!(read(&game, PLACED_A), big("patched a"), "{crash:?}");
        assert_eq!(read(&game, PLACED_B), big("stock b"), "{crash:?}");
        assert!(!exists(&game, "Working/SGWGame/Cache.en-US"), "{crash:?}");
        assert!(!exists(&folder, "Working/SGWGame/Cache.en-US"), "{crash:?}");
        assert!(content_valid(&game, &release), "{crash:?}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_empty_placed_directory_does_not_stop_the_rename() {
    let lt = stock_tier(&[]).await;
    std::fs::create_dir_all(inventory::native(
        &lt.folder,
        "Working/SGWGame/SourceCache.en-us",
    ))
    .unwrap();
    commit(lt.preview().unwrap()).unwrap();
    assert_placed_tree(&lt);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn only_a_light_plan_places_the_cooked_tier() {
    use super::in_place_replace::cooked_tier;
    use super::model::FileCheck;
    use super::publication::{plan_name, read_large};
    let lt = stock_tier(&[]).await;
    // A stopped run leaves its plan behind, which is the only way to hold one.
    assert!(commit_with(lt.preview().unwrap(), crash_after_first_move).is_err());
    let id = lt
        .state
        .lock()
        .unwrap()
        .operations()
        .snapshot()
        .operation
        .clone()
        .unwrap()
        .id;
    let root = lt.state.lock().unwrap().directory.root.clone();
    let mut plan: Plan = read_large(&root.join(plan_name(id))).unwrap();
    assert_eq!(
        cooked_tier(&plan),
        Some((
            "Working/SGWGame/Cache.en-US".to_string(),
            "Working/SGWGame/SourceCache.en-us".to_string()
        ))
    );
    // Full-check and copy adoption never rename a directory.
    plan.provenance.check = FileCheck::Full;
    assert!(cooked_tier(&plan).is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_source_shorter_than_the_stock_prefix_places_no_tier_and_does_not_panic() {
    use super::in_place_replace::cooked_tier;
    use super::publication::{plan_name, read_large};
    let lt = stock_tier(&[]).await;
    assert!(commit_with(lt.preview().unwrap(), crash_after_first_move).is_err());
    let id = lt
        .state
        .lock()
        .unwrap()
        .operations()
        .snapshot()
        .operation
        .clone()
        .unwrap()
        .id;
    let root = lt.state.lock().unwrap().directory.root.clone();
    let mut plan: Plan = read_large(&root.join(plan_name(id))).unwrap();
    for source in [
        "Working/SGWGame/Cache.en-US",
        "Working/SGWGame/Cache.en-U",
        "Working/SGWGame/Cach\u{e9}.en-US/a.pak",
        "W",
    ] {
        for file in &mut plan.report.files {
            file.source_path = Some(source.to_string());
        }
        assert_eq!(cooked_tier(&plan), None, "{source}");
    }
}
