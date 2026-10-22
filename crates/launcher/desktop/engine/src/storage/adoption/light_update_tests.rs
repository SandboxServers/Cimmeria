//! The seedless Update of a light-adopted install, end to end (LA-13, D-LA14). The
//! install is adopted with the light check, then updated to a release that adds
//! patches. The guard tests for every other kind of install are in
//! `light_update_guard_tests`.
//!
//! The adoption needs a multi-thread runtime (`block_in_place`); the Update runs on
//! a current-thread one, so the `update_staged` row it logs is captured on the
//! test thread.
use super::in_place_tests::commit;
use super::light_replay::tests::{big, patch_zip, release_with_roots, OpFixture, OVERLAY};
use super::light_tests::Light;
use super::*;
use crate::install_worker::fixtures;
use crate::storage::install_worker::content_valid;
use crate::storage::update::{commit as update_commit, preparation, Plan, Request};
use crate::test_capture::{capture, Captured};
use cimmeria_launcher_core::state::InstalledState;
use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

pub(super) const SECOND: &str = "Working/Data/Second.dat";
pub(super) const THIRD: &str = "Working/Data/Third.dat";

/// What one Update did: its plan, how it ended, the rows it logged and the paths
/// the origin was asked for.
pub(super) struct Updated {
    pub(super) plan: Plan,
    pub(super) result: Result<(), preparation::Failure>,
    pub(super) events: Vec<Captured>,
    pub(super) paths: Vec<String>,
}

impl Updated {
    /// The `update_staged` row. Panics when the Update never staged.
    pub(super) fn staged(&self) -> &Captured {
        self.events
            .iter()
            .find(|row| row.field("event") == Some("update_staged"))
            .unwrap_or_else(|| panic!("no update_staged row in {:?}", self.events))
    }

    pub(super) fn requested(&self, path: &str) -> bool {
        self.paths.iter().any(|seen| seen == path)
    }
}

/// Admits and runs one Update of the installed content to `target`. `between` runs
/// after staging and before the commit, so a test can look at the live tree then.
pub(super) fn run_update(
    state: &Arc<Mutex<DesktopState>>,
    target: &VerifiedRelease,
    server: &MockServer,
    between: impl FnOnce(),
) -> Updated {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let (out, events) = capture(|| {
        rt.block_on(async {
            let before = server.received_requests().await.unwrap().len();
            let plan = {
                let mut owner = state.lock().unwrap();
                let installed = owner.installed_content().unwrap().unwrap();
                let revision = owner.operations().snapshot().revision;
                owner
                    .admit_update(Request {
                        id: Uuid::new_v4(),
                        operation_revision: revision,
                        installation_id: installed.intent.operation_id,
                        expected_current: installed.current_release,
                        target,
                        confirmed: true,
                    })
                    .unwrap()
                    .plan
            };
            let prepared = preparation::start(
                state.clone(),
                plan.id,
                reqwest::Client::new(),
                format!("{}/manifest.json", server.uri()),
            )
            .unwrap()
            .result
            .await
            .unwrap();
            let result = match prepared {
                Ok(prepared) => {
                    between();
                    update_commit::start(state.clone(), prepared)
                        .unwrap()
                        .await
                        .unwrap()
                }
                Err(failure) => Err(failure),
            };
            let paths: Vec<String> = server
                .received_requests()
                .await
                .unwrap()
                .iter()
                .skip(before)
                .map(|request| request.url.path().to_owned())
                .collect();
            (plan, result, paths)
        })
    });
    let (plan, result, paths) = out;
    Updated {
        plan,
        result,
        events,
        paths,
    }
}

/// A light-adopted install and the origin it was adopted from.
pub(super) struct World {
    pub(super) rt: tokio::runtime::Runtime,
    pub(super) lt: Light,
    zips: Vec<Vec<u8>>,
}

impl World {
    /// Adopts `files` as of the first `adopted` patches of `zips`. The origin serves
    /// every zip, so a later release can add the rest.
    pub(super) fn adopt(zips: Vec<Vec<u8>>, adopted: usize, files: &[(&str, &[u8])]) -> Self {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        let lt = rt.block_on(async {
            let release = release_with_roots(&zips[..adopted], &[]);
            let lt = Light::with_release(release, zips.clone(), files).await;
            let provenance = commit(lt.preview().unwrap()).unwrap();
            assert_eq!(provenance.check, FileCheck::Light);
            lt
        });
        Self { rt, lt, zips }
    }

    /// The release naming the first `patches` zips. It shares the seed with the
    /// adopted one and differs in its manifest digest once the count differs.
    pub(super) fn target(&self, patches: usize) -> VerifiedRelease {
        release_with_roots(&self.zips[..patches], &[])
    }

    pub(super) fn update(&self, target: &VerifiedRelease) -> Updated {
        run_update(&self.lt.state, target, &self.lt.server, || {})
    }

    pub(super) fn serve(&self, route: &str, body: Vec<u8>) {
        self.rt.block_on(
            Mock::given(matchers::path(route))
                .respond_with(ResponseTemplate::new(200).set_body_bytes(body))
                .mount(&self.lt.server),
        );
    }

    pub(super) fn check(&self) -> Option<FileCheck> {
        self.lt.state.lock().unwrap().adoption_check().unwrap()
    }
}

/// Three independent patches, one per file.
pub(super) fn three_patches() -> Vec<Vec<u8>> {
    [(OVERLAY, "overlay"), (SECOND, "second"), (THIRD, "third")]
        .into_iter()
        .enumerate()
        .map(|(i, (target, tag))| {
            patch_zip(
                &format!("p{i}"),
                &[OpFixture {
                    target,
                    sources: &[(target, &big(&format!("stock {tag}")))],
                    result: &big(&format!("patched {tag}")),
                }],
                &[],
            )
        })
        .collect()
}

/// The stock bytes of the three files, with the third one replaceable.
fn stock_files(third: Vec<u8>) -> Vec<(&'static str, Vec<u8>)> {
    vec![
        (OVERLAY, big("stock overlay")),
        (SECOND, big("stock second")),
        (THIRD, third),
    ]
}

fn borrowed<'a>(files: &'a [(&'static str, Vec<u8>)]) -> Vec<(&'static str, &'a [u8])> {
    files
        .iter()
        .map(|(path, bytes)| (*path, bytes.as_slice()))
        .collect()
}

pub(super) fn read(root: &Path, rel: &str) -> Vec<u8> {
    std::fs::read(inventory::native(root, rel)).unwrap()
}

fn number(row: &Captured, key: &str) -> u64 {
    row.field(key).unwrap().parse().unwrap()
}

/// The point of LA-13: a light-adopted install updates by applying only the new
/// patch to its own tree. The seed is never requested and neither are the patches
/// the install already holds.
#[test]
fn a_light_adopted_install_updates_without_the_seed() {
    let files = stock_files(big("stock third"));
    let world = World::adopt(three_patches(), 2, &borrowed(&files));
    let game = world.lt.game();
    let target = world.target(3);
    let up = world.update(&target);
    assert_eq!(up.result, Ok(()), "{:?}", up.events);
    assert!(!up.requested("/seed.zip"), "{:?}", up.paths);
    assert!(
        !up.paths.is_empty() && up.paths.iter().all(|path| path == "/p2.zip"),
        "only the new patch is fetched: {:?}",
        up.paths
    );
    let row = up.staged();
    assert_eq!(row.field("mode"), Some("incremental"));
    assert_eq!(row.field("ops_skipped"), Some("0"));
    let (linked, copied) = (number(row, "stage_linked"), number(row, "stage_copied"));
    assert!(linked + copied >= 5, "every live file reaches the stage");
    if cfg!(windows) {
        assert!(linked > 0, "unchanged files are linked, not copied");
    }
    assert_eq!(read(&game, OVERLAY), big("patched overlay"));
    assert_eq!(read(&game, SECOND), big("patched second"));
    assert_eq!(read(&game, THIRD), big("patched third"));
    let ledger = InstalledState::load(&game);
    assert_eq!(ledger.applied_patches.len(), 3);
    assert!(ledger.incomplete_patches.is_empty());
    assert!(content_valid(&game, &target));
    // The update keeps the adoption's provenance, so the next one is seedless too.
    assert_eq!(world.check(), Some(FileCheck::Light));
    let installed = world.lt.state.lock().unwrap().installed_content().unwrap();
    assert_eq!(installed.unwrap().current_release, up.plan.target);
}

/// A release with another seed cannot be applied on top of the live tree: it
/// rebuilds from the new seed, exactly as an install without a light adoption.
#[test]
fn a_target_with_another_seed_rebuilds_from_the_seed() {
    let files = stock_files(big("stock third"));
    let world = World::adopt(three_patches(), 2, &borrowed(&files));
    let seed = fixtures::archive(true);
    let target = fixtures::verified(&seed);
    world.serve("/seed.zip", seed);
    let up = world.update(&target);
    assert_eq!(up.result, Ok(()), "{:?}", up.events);
    assert!(up.requested("/seed.zip"), "{:?}", up.paths);
    let row = up.staged();
    assert_eq!(row.field("mode"), Some("full"));
    assert_eq!(row.field("stage_linked"), Some("0"));
    assert_eq!(row.field("stage_copied"), Some("0"));
}

/// D-LA9: a file the player changed never stops the Update. The new patch's op on
/// it is skipped and recorded, like at adoption, and the player's bytes stay.
#[test]
fn a_changed_source_of_the_new_patch_is_skipped_not_refused() {
    let files = stock_files(b"modded by the player".to_vec());
    let world = World::adopt(three_patches(), 2, &borrowed(&files));
    let game = world.lt.game();
    let target = world.target(3);
    let up = world.update(&target);
    assert_eq!(up.result, Ok(()), "{:?}", up.events);
    assert!(!up.requested("/seed.zip"), "{:?}", up.paths);
    let row = up.staged();
    assert_eq!(row.field("mode"), Some("incremental"));
    assert_eq!(row.field("ops_skipped"), Some("1"));
    assert_eq!(read(&game, THIRD), b"modded by the player");
    assert_eq!(read(&game, SECOND), big("patched second"));
    let ledger = InstalledState::load(&game);
    let [incomplete] = ledger.incomplete_patches.as_slice() else {
        panic!("one incomplete patch: {:?}", ledger.incomplete_patches);
    };
    assert!(ledger.applied_patches.contains(&incomplete.state_key));
    assert_eq!(incomplete.skipped.len(), 1);
    assert_eq!(incomplete.skipped[0].path, THIRD);
    assert!(content_valid(&game, &target));
}

/// The live tree is not touched before the commit: the files the update rewrites
/// in place (the ledger, `SGW.exe`) and the patch outputs are copies in the stage,
/// and the rest are links nothing writes through.
#[test]
fn the_live_tree_is_untouched_until_the_commit() {
    let files = stock_files(big("stock third"));
    let world = World::adopt(three_patches(), 2, &borrowed(&files));
    let game = world.lt.game();
    let tracked = [
        "launcher-installed.json",
        "Working/Binaries/SGW.exe",
        OVERLAY,
        SECOND,
        THIRD,
    ];
    let before: Vec<Vec<u8>> = tracked.iter().map(|rel| read(&game, rel)).collect();
    let target = world.target(3);
    let up = run_update(&world.lt.state, &target, &world.lt.server, || {
        for (rel, bytes) in tracked.iter().zip(&before) {
            assert_eq!(&read(&game, rel), bytes, "{rel} changed before the commit");
        }
    });
    assert_eq!(up.result, Ok(()), "{:?}", up.events);
    assert_ne!(read(&game, "launcher-installed.json"), before[0]);
    assert_ne!(read(&game, THIRD), before[4]);
    assert_eq!(read(&game, OVERLAY), before[2]);
}

/// Repair is outside LA-13: it still rebuilds from the seed, so it asks for it.
/// (The same flow, on a downloaded install, is
/// `signed_update_reopen_repair_current_and_uninstall_keep_permanent_owner`.)
#[test]
fn repair_of_a_light_adopted_install_still_asks_for_the_seed() {
    let files = stock_files(big("stock third"));
    let world = World::adopt(three_patches(), 2, &borrowed(&files));
    world.serve("/seed.zip", b"seed".to_vec());
    let before = world.rt.block_on(world.lt.server.received_requests());
    let before = before.unwrap().len();
    let plan = {
        let mut owner = world.lt.state.lock().unwrap();
        let installed = owner.installed_content().unwrap().unwrap();
        let revision = owner.operations().snapshot().revision;
        owner
            .admit_repair(
                Uuid::new_v4(),
                revision,
                installed.intent.operation_id,
                true,
            )
            .unwrap()
            .plan
    };
    world.rt.block_on(async {
        let prepared = crate::storage::repair::preparation::start(
            world.lt.state.clone(),
            plan.id,
            reqwest::Client::new(),
            format!("{}/manifest.json", world.lt.server.uri()),
        )
        .unwrap()
        .result
        .await
        .unwrap();
        // The served "seed" is not an archive, so the rebuild ends here; the request
        // is what this test is about.
        assert!(prepared.is_err());
    });
    let seen = world
        .rt
        .block_on(world.lt.server.received_requests())
        .unwrap();
    assert!(
        seen.iter()
            .skip(before)
            .any(|request| request.url.path() == "/seed.zip"),
        "repair rebuilt without the seed"
    );
}

/// The cooked layout of LA-11b: the stock PAKs sit under `SourceCache.en-us` after
/// adoption, and the update edits one of them in place.
#[test]
fn the_cooked_tier_updates_incrementally() {
    const STOCK_A: &str = "Working/SGWGame/Cache.en-US/A.pak";
    const STOCK_B: &str = "Working/SGWGame/Cache.en-US/B.pak";
    const PLACED_A: &str = "Working/SGWGame/SourceCache.en-us/A.pak";
    const PLACED_B: &str = "Working/SGWGame/SourceCache.en-us/B.pak";
    let edit = |id: &str, from: &str, to: &str| {
        patch_zip(
            id,
            &[OpFixture {
                target: PLACED_A,
                sources: &[(PLACED_A, &big(from))],
                result: &big(to),
            }],
            &[],
        )
    };
    let zips = vec![
        edit("p0", "stock a", "patched a"),
        edit("p1", "patched a", "patched a again"),
    ];
    let (a, b) = (big("stock a"), big("stock b"));
    let world = World::adopt(zips, 1, &[(STOCK_A, &a), (STOCK_B, &b)]);
    let game = world.lt.game();
    assert_eq!(read(&game, PLACED_A), big("patched a"));
    let target = world.target(2);
    let up = world.update(&target);
    assert_eq!(up.result, Ok(()), "{:?}", up.events);
    assert!(!up.requested("/seed.zip"), "{:?}", up.paths);
    assert_eq!(up.staged().field("mode"), Some("incremental"));
    assert_eq!(read(&game, PLACED_A), big("patched a again"));
    assert_eq!(read(&game, PLACED_B), big("stock b"));
    assert!(!inventory::native(&game, "Working/SGWGame/Cache.en-US").exists());
    let ledger = InstalledState::load(&game);
    assert_eq!(ledger.applied_patches.len(), 2);
    assert!(ledger.incomplete_patches.is_empty());
    assert!(content_valid(&game, &target));
}
