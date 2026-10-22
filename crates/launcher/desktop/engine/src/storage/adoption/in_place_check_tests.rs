//! The final check of an in-place adoption (LF-02): a light adoption compares the tree
//! file by file against the one stored at admission, a verified one still compares a
//! digest, and an adoption an older build began (no stored tree) is finished anyway.
use super::in_place_check::{read_expected, write_expected, EXPECTED_TREE};
use super::in_place_tests::{commit, commit_with, Fixture};
use super::light_replay::tests::{big, patch_zip, OpFixture, OVERLAY};
use super::light_tests::Light;
use super::publication::{name, plan_name, read_large};
use super::*;
use crate::test_capture::{capture, Captured};

const STOCK_A: &str = "Working/SGWGame/Cache.en-US/A.pak";
const PLACED_A: &str = "Working/SGWGame/SourceCache.en-us/A.pak";

fn overlay_patch() -> Vec<Vec<u8>> {
    vec![patch_zip(
        "p0",
        &[OpFixture {
            target: OVERLAY,
            sources: &[(OVERLAY, &big("stock overlay"))],
            result: &big("patched overlay"),
        }],
        &[],
    )]
}

fn checked(events: &[Captured]) -> Vec<&Captured> {
    events
        .iter()
        .filter(|e| e.field("event") == Some("adoption_tree_checked"))
        .collect()
}

fn stop_at(point: publication::Point) -> impl FnMut(publication::Point) -> Result<(), Error> {
    move |at| {
        if at == point {
            Err(StorageError::Io.into())
        } else {
            Ok(())
        }
    }
}

fn operation_id(state: &Arc<Mutex<DesktopState>>) -> Uuid {
    state
        .lock()
        .unwrap()
        .operations()
        .snapshot()
        .operation
        .clone()
        .unwrap()
        .id
}

/// Reopens the state a stopped run left and recovers it. `tamper` sees the plan first,
/// while the work folder is still there.
fn recover_after_stop(
    lt: Light,
    tamper: impl FnOnce(&Plan),
) -> (Result<(), Error>, Vec<Captured>, Uuid, PathBuf) {
    let state_dir = lt.root.path().join("state");
    let id = operation_id(&lt.state);
    let plan: Plan = read_large(&state_dir.join(plan_name(id))).unwrap();
    tamper(&plan);
    drop(lt.state);
    let mut reopened = DesktopState::open(&state_dir).unwrap();
    let revision = reopened.operations().snapshot().revision;
    let (result, events) = capture(|| recover(&mut reopened, id, revision));
    (result, events, id, state_dir)
}

/// A file system that folds case, which is what puts one directory under two spellings.
fn folds_case(dir: &Path) -> bool {
    let probe = dir.join("CaseProbe");
    std::fs::create_dir_all(&probe).unwrap();
    let folds = dir.join("caseprobe").exists();
    std::fs::remove_dir_all(&probe).unwrap();
    folds
}

/// F1a. The cooked tier is renamed to `SourceCache.en-us`, but the folder already holds
/// that directory as `SourceCache.en-US`: on a case-folding disk the files land under
/// the existing spelling, so the tree on disk and the predicted tree differ in case only.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_tree_that_differs_only_in_case_is_accepted() {
    let (a, other) = (big("stock a"), big("other"));
    let patch = vec![patch_zip(
        "p0",
        &[OpFixture {
            target: PLACED_A,
            sources: &[(PLACED_A, &a)],
            result: &big("patched a"),
        }],
        &[],
    )];
    let lt = Light::new(
        patch,
        &[
            (STOCK_A, &a),
            ("Working/SGWGame/SourceCache.en-US/Other.pak", &other),
        ],
    )
    .await;
    if !folds_case(&lt.folder) {
        eprintln!("skipped: this file system keeps case, so the spelling cannot split");
        return;
    }
    let preview = lt.preview().unwrap();
    let (result, events) = capture(|| commit(preview));
    result.expect("a spelling-only difference no longer fails the check");
    let events = checked(&events);
    let [event] = events.as_slice() else {
        panic!("one tree check: {events:?}");
    };
    assert_eq!(event.field("check"), Some("light"));
    assert_eq!(event.field("outcome"), Some("accepted"));
    let case_only: u32 = event.field("case_only").unwrap().parse().unwrap();
    assert!(case_only >= 1, "{event:?}");
}

/// The same verdict from a hand-made tree, which does not depend on the disk folding
/// case: the stored tree names the overlay in other letters than the disk does.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stored_tree_in_other_letters_is_accepted() {
    let lt = Light::new(overlay_patch(), &[(OVERLAY, &big("stock overlay"))]).await;
    let failed = commit_with(lt.preview().unwrap(), stop_at(publication::Point::Receipt));
    assert!(failed.is_err());
    let (result, events, _, _) = recover_after_stop(lt, |plan| {
        let mut tree = read_expected(&plan.stage()).expect("admission stored the tree");
        let (size, sha) = tree.remove(OVERLAY).expect("the overlay is expected");
        tree.insert(OVERLAY.to_lowercase(), (size, sha));
        write_expected(&plan.stage(), &tree).unwrap();
    });
    result.expect("recovery finishes");
    let events = checked(&events);
    let [event] = events.as_slice() else {
        panic!("one tree check: {events:?}");
    };
    assert_eq!(event.field("outcome"), Some("accepted"));
    assert_eq!(event.field("case_only"), Some("1"));
}

/// 2b. A folder that never had `LoginInternal.lua` publishes: setup writes it and the
/// stored tree already expects it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_folder_without_the_login_file_publishes() {
    let lt = Light::new(overlay_patch(), &[(OVERLAY, &big("stock overlay"))]).await;
    let login = "Working/SGWGame/Content/UI/Startup/Login/LoginInternal.lua";
    assert!(!inventory::native(&lt.folder, login).exists());
    let (result, events) = capture(|| commit(lt.preview().unwrap()));
    result.expect("publishes");
    assert!(inventory::native(&lt.game(), login).exists());
    let events = checked(&events);
    let [event] = events.as_slice() else {
        panic!("one tree check: {events:?}");
    };
    assert_ne!(event.field("outcome"), Some("mismatch"));
}

/// A real loss still fails, names the file, and leaves the record `Promoted` so
/// Recover stays offered.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_missing_release_file_still_fails_the_check() {
    let lt = Light::new(overlay_patch(), &[(OVERLAY, &big("stock overlay"))]).await;
    let game = lt.game();
    let (result, events) = capture(|| {
        commit_with(lt.preview().unwrap(), |point| {
            if point == publication::Point::SetUp {
                std::fs::remove_file(inventory::native(&game, OVERLAY)).unwrap();
            }
            Ok(())
        })
    });
    assert!(matches!(result, Err(Error::Storage(StorageError::Corrupt))));
    let events = checked(&events);
    let [event] = events.as_slice() else {
        panic!("one tree check: {events:?}");
    };
    assert_eq!(event.field("outcome"), Some("mismatch"));
    assert_eq!(event.level, tracing::Level::ERROR);
    assert!(
        event
            .field("missing_paths")
            .unwrap()
            .contains("Overlay.dat"),
        "{event:?}"
    );
    let id = operation_id(&lt.state);
    let root = lt.root.path().join("state");
    let record: Record = read_large(&root.join(name(id))).unwrap();
    assert_eq!(record.phase, Phase::Promoted);
}

/// An adoption an earlier build began has no stored tree: it finishes, and the log
/// says nothing was compared.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_promoted_adoption_without_a_stored_tree_is_finished() {
    let lt = Light::new(overlay_patch(), &[(OVERLAY, &big("stock overlay"))]).await;
    let failed = commit_with(lt.preview().unwrap(), stop_at(publication::Point::Receipt));
    assert!(failed.is_err());
    let (result, events, id, state_dir) = recover_after_stop(lt, |plan| {
        let stored = plan.stage().join(EXPECTED_TREE);
        assert!(stored.exists(), "admission stored the tree");
        std::fs::remove_file(stored).unwrap();
    });
    result.expect("recovery finishes");
    let events = checked(&events);
    let [event] = events.as_slice() else {
        panic!("one tree check: {events:?}");
    };
    assert_eq!(event.field("check"), Some("light"));
    assert_eq!(event.field("outcome"), Some("unrecorded"));
    // Finishing removes the record with the work folder, so the end state is the
    // operation's `Succeeded` that `recover` only reaches after `Published`.
    let state = DesktopState::open(&state_dir).unwrap();
    assert!(state
        .operations()
        .snapshot()
        .operation
        .as_ref()
        .is_none_or(|op| op.state == OperationState::Succeeded));
    assert!(read_large::<Record>(&state_dir.join(name(id))).is_err());
}

/// A verified adoption still compares one digest and still fails on any difference;
/// the log now says which check it was.
#[test]
fn a_verified_adoption_still_fails_on_a_digest_miss() {
    let f = Fixture::new(None);
    let game = f.folder().join("game");
    let (result, events) = capture(|| {
        commit_with(f.preview().unwrap(), |point| {
            if point == publication::Point::SetUp {
                std::fs::write(game.join("extra.bin"), b"not in the release").unwrap();
            }
            Ok(())
        })
    });
    assert!(matches!(result, Err(Error::Storage(StorageError::Corrupt))));
    let events = checked(&events);
    let [event] = events.as_slice() else {
        panic!("one tree check: {events:?}");
    };
    assert_eq!(event.field("check"), Some("verified"));
    assert_eq!(event.field("outcome"), Some("mismatch"));
}
