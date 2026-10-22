//! Setup originals and Abandon after `Promoted` (LF-03, D-LF2): a light adoption keeps
//! the setup files as they were, so a failed final check can still be backed out of.
use super::in_place_check::{read_expected, write_expected, SETUP_MANIFEST};
use super::in_place_tests::{commit_with, Fixture};
use super::light_replay::tests::{big, patch_zip, OpFixture, OVERLAY};
use super::light_tests::Light;
use super::*;
use crate::test_capture::capture;
use std::collections::BTreeMap;

const SGW: &str = "Working/Binaries/SGW.exe";
const LOGIN: &str = "Working/SGWGame/Content/UI/Startup/Login/LoginInternal.lua";

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

/// Every file under `folder` with its size and digest.
fn files_of(folder: &Path) -> BTreeMap<String, (u64, [u8; 32])> {
    inventory::scan(folder, &CancellationToken::new())
        .unwrap()
        .iter()
        .map(|(path, entry)| (path.clone(), (entry.size, entry.sha256)))
        .collect()
}

fn stage_of(folder: &Path) -> PathBuf {
    std::fs::read_dir(folder)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with(".cimmeria-adopt-"))
        })
        .expect("the work folder is there")
}

/// Fails the final check after setup ran, without touching a release file: the stored
/// tree gains a file nothing will ever provide, so the check reports it missing.
fn fail_final_check(point: publication::Point, folder: &Path) -> Result<(), Error> {
    if point == publication::Point::SetUp {
        let stage = stage_of(folder);
        let mut tree = read_expected(&stage).expect("admission stored the tree");
        tree.insert("Working/Never/There.bin".into(), (1, [7; 32]));
        write_expected(&stage, &tree).unwrap();
    }
    Ok(())
}

fn operation(state: &Arc<Mutex<DesktopState>>) -> (Uuid, u64) {
    let state = state.lock().unwrap();
    let snapshot = state.operations().snapshot();
    (snapshot.operation.clone().unwrap().id, snapshot.revision)
}

fn abandonable_now(state: &Arc<Mutex<DesktopState>>) -> bool {
    let (id, _) = operation(state);
    interrupted(&state.lock().unwrap(), id).unwrap().abandonable
}

/// A light adoption that finished setup and then failed its check.
async fn failed_after_setup() -> (Light, PathBuf) {
    let lt = Light::new(overlay_patch(), &[(OVERLAY, &big("stock overlay"))]).await;
    let folder = lt.folder.clone();
    let result = commit_with(lt.preview().unwrap(), |point| {
        fail_final_check(point, &folder)
    });
    assert!(matches!(result, Err(Error::Storage(StorageError::Corrupt))));
    (lt, folder)
}

/// The packet's headline: after setup rewrote `SGW.exe` and created `LoginInternal.lua`,
/// Abandon puts the folder back the way the review found it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_light_adoption_is_abandonable_after_setup_and_restores_the_folder() {
    let lt = Light::new(overlay_patch(), &[(OVERLAY, &big("stock overlay"))]).await;
    let folder = lt.folder.clone();
    let before = files_of(&folder);
    let exe_before = std::fs::read(inventory::native(&folder, SGW)).unwrap();
    assert!(!inventory::native(&folder, LOGIN).exists());
    let result = commit_with(lt.preview().unwrap(), |point| {
        fail_final_check(point, &folder)
    });
    assert!(matches!(result, Err(Error::Storage(StorageError::Corrupt))));
    let game = lt.game();
    // Setup really changed what the test must prove is restored.
    assert_ne!(
        std::fs::read(inventory::native(&game, SGW)).unwrap(),
        exe_before
    );
    assert!(inventory::native(&game, LOGIN).exists());

    assert!(abandonable_now(&lt.state));
    let (id, revision) = operation(&lt.state);
    // A second live Dispatch stops tracing's single-dispatcher shortcut from caching this
    // event's callsite as disabled when a sibling test hits it first without a subscriber.
    let _second = tracing::Dispatch::new(tracing_subscriber::Registry::default());
    let (result, events) = capture(|| abandon(&mut lt.state.lock().unwrap(), id, revision));
    result.expect("abandons");
    assert!(
        events.iter().all(|e| e.level != tracing::Level::WARN),
        "{events:?}"
    );
    let [event] = events
        .iter()
        .filter(|e| e.field("restored_setup").is_some())
        .collect::<Vec<_>>()[..]
    else {
        panic!("one abandon event: {events:?}");
    };
    assert_eq!(event.level, tracing::Level::INFO);
    assert_eq!(event.target, "adoption");
    // SGW.exe (copied back) and LoginInternal.lua (absent before, deleted); the ledger
    // did not exist either, so it counts as deleted too.
    assert_eq!(event.field("restored_setup"), Some("3"));

    assert_eq!(
        std::fs::read(inventory::native(&folder, SGW)).unwrap(),
        exe_before
    );
    assert!(!inventory::native(&folder, LOGIN).exists());
    assert_eq!(files_of(&folder), before);
    assert!(!game.exists(), "{:?}", walk(&game));
    assert!(!folder.join("content-ready.json").exists());
    assert!(
        std::fs::read_dir(&folder).unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".cimmeria-adopt-")),
        "the work folder is gone"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_promoted_light_record_without_the_manifest_is_not_abandonable() {
    let (lt, folder) = failed_after_setup().await;
    assert!(abandonable_now(&lt.state));
    std::fs::remove_file(stage_of(&folder).join(SETUP_MANIFEST)).unwrap();
    assert!(!abandonable_now(&lt.state));
    let (id, revision) = operation(&lt.state);
    assert!(abandon(&mut lt.state.lock().unwrap(), id, revision).is_err());
}

/// A manifest the copy did not finish is not one to restore from.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unreadable_manifest_is_not_abandonable() {
    let (lt, folder) = failed_after_setup().await;
    std::fs::write(
        stage_of(&folder).join(SETUP_MANIFEST),
        b"{\"version\":1,\"fi",
    )
    .unwrap();
    assert!(!abandonable_now(&lt.state));
}

/// R10: a verified adoption keeps no setup originals and stays recover-only once
/// it is `Promoted`.
#[test]
fn a_promoted_verified_record_is_not_abandonable() {
    let f = Fixture::new(None);
    let result = commit_with(f.preview().unwrap(), |point| {
        if point == publication::Point::SetUp {
            Err(StorageError::Io.into())
        } else {
            Ok(())
        }
    });
    assert!(result.is_err());
    let f = f.reopen();
    let (id, _) = f.operation();
    let state = f.state.lock().unwrap();
    assert!(!interrupted(&state, id).unwrap().abandonable);
    let plan: Plan =
        publication::read_large(&state.directory.root.join(publication::plan_name(id))).unwrap();
    assert!(!plan.stage().join(SETUP_MANIFEST).exists());
    // A valid manifest alone must not make a verified record abandonable: only the Light
    // check may do that. A verified record has no stage folder left, so make one.
    std::fs::create_dir_all(plan.stage()).unwrap();
    std::fs::write(
        plan.stage().join(SETUP_MANIFEST),
        br#"{"version":1,"files":[]}"#,
    )
    .unwrap();
    assert!(!interrupted(&state, id).unwrap().abandonable);
}

/// A rerun of the restore after an earlier run already moved `SGW.exe` home must not copy
/// it back into `game` (that would leave a stray game folder behind).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_restore_rerun_skips_a_stock_file_already_moved_home() {
    let (lt, folder) = failed_after_setup().await;
    let stage = stage_of(&folder);
    let game = lt.game();
    std::fs::rename(inventory::native(&game, SGW), folder.join("SGW.exe")).unwrap();
    in_place_check::restore_setup_originals(&stage, &game).unwrap();
    assert!(!inventory::native(&game, SGW).exists());
}

/// A manifest row that climbs out of `game` through a backslash is refused, and the file it
/// names outside `game` is left alone.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_manifest_row_that_climbs_out_of_the_game_folder_is_refused() {
    let (lt, folder) = failed_after_setup().await;
    let stage = stage_of(&folder);
    let game = lt.game();
    let outside = game.parent().unwrap().join("x");
    std::fs::write(&outside, b"keep").unwrap();
    std::fs::write(
        stage.join(SETUP_MANIFEST),
        br#"{"version":1,"files":[{"path":"..\\x","existed":false}]}"#,
    )
    .unwrap();
    assert!(in_place_check::restore_setup_originals(&stage, &game).is_err());
    assert!(outside.exists());
}

/// A rerun of the copy (recovery re-enters the `Staged` branch) replaces the first set.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn saving_the_originals_twice_replaces_the_first_set() {
    let (lt, folder) = failed_after_setup().await;
    let stage = stage_of(&folder);
    let (id, _) = operation(&lt.state);
    let root = lt.state.lock().unwrap().directory.root.clone();
    let plan: Plan = publication::read_large(&root.join(publication::plan_name(id))).unwrap();
    // The game folder now holds the rewritten SGW.exe; saving again copies that.
    let game = lt.game();
    in_place_check::save_setup_originals(&plan, &game).unwrap();
    assert_eq!(
        std::fs::read(inventory::native(&stage.join("setup-originals"), SGW)).unwrap(),
        std::fs::read(inventory::native(&game, SGW)).unwrap()
    );
    assert!(in_place_check::has_setup_originals(&stage));
}

fn walk(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries {
            let path = entry.unwrap().path();
            out.push(path.display().to_string());
            out.extend(walk(&path));
        }
    }
    out
}
