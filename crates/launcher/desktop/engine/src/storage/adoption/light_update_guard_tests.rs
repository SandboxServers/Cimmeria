//! Guards for LA-13 (D-LA14, R-LA 10): an install that was not adopted with the
//! light check keeps the seed-rebuild Update. Each case below has a ledger the
//! seedless path would accept, so only the adoption provenance keeps it off that
//! path; loosening the discriminator makes them fail.
use super::in_place_tests::{commit, Fixture, SeedKind};
use super::light_replay::tests::{big, patch_zip, OpFixture};
use super::light_update_tests::{read, run_update};
use super::*;
use crate::install_worker::fixtures;
use crate::storage::install_worker::tests::prepared_fixture;
use cimmeria_launcher_core::install::{self, InstallContext};
use cimmeria_launcher_core::seed_source::SeedSourceHandle;
use cimmeria_launcher_core::state::InstalledState;
use ed25519_dalek::{Signer, SigningKey};
use std::collections::BTreeMap;
use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// A signed release over `seed`, naming `zips` as patches `p0..`.
fn release_over(seed: &[u8], zips: &[Vec<u8>]) -> VerifiedRelease {
    let patches: Vec<_> = zips
        .iter()
        .enumerate()
        .map(|(i, zip)| {
            serde_json::json!({
                "id": format!("p{i}"), "blob": format!("p{i}.zip"),
                "size": zip.len(), "sha256": hex(&Sha256::digest(zip)),
            })
        })
        .collect();
    let body = serde_json::to_vec(&serde_json::json!({
        "schema": 1,
        "seed": {"blob": "seed.zip", "size": seed.len(), "sha256": hex(&Sha256::digest(seed))},
        "patches": patches,
    }))
    .unwrap();
    let signature = SigningKey::from_bytes(&[0x2a; 32]).sign(&body);
    crate::catalog::verify_release(&body, hex(&signature.to_bytes()).as_bytes()).unwrap()
}

/// A patch that adds `later.txt` to the seed fixture, whose own copy differs.
fn later_patch() -> Vec<u8> {
    patch_zip(
        "p0",
        &[OpFixture {
            target: "later.txt",
            sources: &[("later.txt", b"later entry")],
            result: &big("patched later"),
        }],
        &[],
    )
}

/// Every file below `dir`, by `/` path, except the launcher's bookkeeping.
fn tree(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            // Per-operation markers carry the operation id, so they never match.
            if path
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with(".cimmeria-"))
            {
                continue;
            }
            if path.is_dir() {
                walk(base, &path, out);
            } else {
                let rel = path
                    .strip_prefix(base)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                out.insert(rel, std::fs::read(&path).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

async fn serve(server: &MockServer, route: &str, body: Vec<u8>) {
    Mock::given(matchers::path(route))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(body))
        .mount(server)
        .await;
}

/// A downloaded install, whose ledger the seedless path would accept, updates
/// exactly as it does today: from the seed, into the same tree a strict install
/// of the target produces.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_downloaded_install_still_rebuilds_from_the_seed() {
    let (root, state, _id, server) = prepared_fixture().await;
    let seed = fixtures::archive(true);
    let patches = vec![later_patch()];
    let target = release_over(&seed, &patches);
    serve(&server, "/p0.zip", patches[0].clone()).await;
    assert_eq!(state.lock().unwrap().adoption_check().unwrap(), None);
    assert!(InstalledState::load(&root.path().join("install/game"))
        .applied_patches
        .is_empty());

    let up = {
        let (state, server) = (state.clone(), &server);
        let target = &target;
        // The Update runs on its own current-thread runtime so its rows are captured.
        std::thread::scope(|scope| {
            scope
                .spawn(move || run_update(&state, target, server, || {}))
                .join()
                .unwrap()
        })
    };
    assert_eq!(up.result, Ok(()), "{:?}", up.events);
    assert!(up.requested("/seed.zip"), "{:?}", up.paths);
    let row = up.staged();
    assert_eq!(row.field("mode"), Some("full"));
    assert_eq!(row.field("stage_linked"), Some("0"));
    assert_eq!(row.field("stage_copied"), Some("0"));
    assert!(InstalledState::load(&root.path().join("install/game"))
        .incomplete_patches
        .is_empty());

    // The strict reference: today's install of the same release.
    let reference = tempfile::tempdir().unwrap();
    let live = tree(&root.path().join("install/game"));
    let cancel = tokio_util::sync::CancellationToken::new();
    let (progress, _) = crate::install_progress::ProgressSink::latest();
    let ctx = InstallContext {
        manifest_url: &format!("{}/manifest.json", server.uri()),
        install_dir: reference.path(),
        manifest: target.manifest(),
        login_servers: &up.plan.owner.login_servers,
        cancel,
        progress,
        http: &reqwest::Client::new(),
        seed_source: &SeedSourceHandle::download(),
    };
    let (result, _) = install::install_all(ctx).await;
    result.unwrap();
    assert_eq!(live, tree(reference.path()));
}

/// A failed fetch of the seed still fails the Update and leaves the live tree
/// alone: the downloaded install has no seedless fallback.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_downloaded_install_cannot_update_without_the_seed() {
    let (root, state, _id, server) = prepared_fixture().await;
    let seed = fixtures::archive(true);
    let patches = vec![later_patch()];
    let target = release_over(&seed, &patches);
    let before = tree(&root.path().join("install/game"));
    server.reset().await;
    serve(&server, "/p0.zip", patches[0].clone()).await;
    let up = {
        let (state, server) = (state.clone(), &server);
        let target = &target;
        std::thread::scope(|scope| {
            scope
                .spawn(move || run_update(&state, target, server, || {}))
                .join()
                .unwrap()
        })
    };
    assert!(up.result.is_err(), "{:?}", up.events);
    assert!(
        up.events
            .iter()
            .all(|row| row.field("event") != Some("update_staged")),
        "{:?}",
        up.events
    );
    assert_eq!(tree(&root.path().join("install/game")), before);
}

/// A copy-adopted install (`check = full`) with a ledger the seedless path would
/// take still rebuilds from the seed.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_copy_adopted_install_still_rebuilds_from_the_seed() {
    let archive = fixtures::archive(true);
    let fixture = Fixture::with_archive(None, SeedKind::Zip, archive.clone());
    let provenance = commit(fixture.preview().unwrap()).unwrap();
    assert_eq!(provenance.check, FileCheck::Full);
    let game = fixture.folder().join("game");
    let seed_sha = fixture.release.manifest().seed.sha256.clone();
    // The copy-adoption ledger carries `seed_adopted`, which alone would refuse the
    // seedless path; rewrite it so only the provenance is left to decide.
    InstalledState {
        applied_patches: Vec::new(),
        seed_sha256: Some(seed_sha),
        seed_adopted: false,
        incomplete_patches: Vec::new(),
    }
    .save(&game)
    .unwrap();
    let patches = vec![later_patch()];
    let target = release_over(&archive, &patches);
    let server = MockServer::start().await;
    serve(&server, "/seed.zip", archive).await;
    serve(&server, "/p0.zip", patches[0].clone()).await;
    let state = fixture.state.clone();
    assert_eq!(
        state.lock().unwrap().adoption_check().unwrap(),
        Some(FileCheck::Full)
    );
    let up = {
        let (state, server) = (state.clone(), &server);
        let target = &target;
        std::thread::scope(|scope| {
            scope
                .spawn(move || run_update(&state, target, server, || {}))
                .join()
                .unwrap()
        })
    };
    assert_eq!(up.result, Ok(()), "{:?}", up.events);
    assert!(up.requested("/seed.zip"), "{:?}", up.paths);
    let row = up.staged();
    assert_eq!(row.field("mode"), Some("full"));
    assert_eq!(row.field("stage_linked"), Some("0"));
    // The rebuild's result is the strict one: the patched file, not the player's.
    assert_eq!(read(&game, "later.txt"), big("patched later"));
}
