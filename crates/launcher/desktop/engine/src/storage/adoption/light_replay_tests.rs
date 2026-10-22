use super::*;
use crate::install_progress::ProgressSink;
use cimmeria_launcher_core::patch_summary;
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};
use std::sync::{Mutex, MutexGuard};

static SERIAL: Mutex<()> = Mutex::new(());
fn serial() -> MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|e| e.into_inner())
}

const OVERLAY: &str = "Working/Data/Overlay.dat";
const SECOND: &str = "Working/Data/Second.dat";
const CHAIN: &str = "Working/Data/Chain.dat";
const COOKED_STOCK: &str = "Working/SGWGame/Cache.en-US/Cooked.pak";
const COOKED_PLACED: &str = "Working/SGWGame/SourceCache.en-us/Cooked.pak";

fn big(tag: &str) -> Vec<u8> {
    format!("{tag} line of a file the game ships\r\n")
        .repeat(60)
        .into_bytes()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// One recipe op: `target` rebuilt from `sources` into `result`.
struct OpFixture<'a> {
    target: &'a str,
    sources: &'a [(&'a str, &'a [u8])],
    result: &'a [u8],
}

/// A real patch zip, built with the patchset builder.
fn patch_zip(id: &str, ops: &[OpFixture], files: &[(&str, &[u8])]) -> Vec<u8> {
    let dir = tempfile::tempdir().unwrap();
    let (stock, patched, spec_dir) = (
        dir.path().join("stock"),
        dir.path().join("patched"),
        dir.path().join("spec"),
    );
    let write = |root: &Path, rel: &str, bytes: &[u8]| {
        let path = inventory::native(root, rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    };
    std::fs::create_dir_all(&spec_dir).unwrap();
    let mut spec_ops = Vec::new();
    for op in ops {
        for (path, bytes) in op.sources {
            write(&stock, path, bytes);
        }
        write(&patched, op.target, op.result);
        let sources: Vec<_> = op
            .sources
            .iter()
            .map(|(p, _)| serde_json::json!({ "path": p }))
            .collect();
        spec_ops.push(serde_json::json!({ "target": op.target, "sources": sources }));
    }
    let mut spec_files = Vec::new();
    for (i, (path, bytes)) in files.iter().enumerate() {
        write(&spec_dir, &format!("f{i}"), bytes);
        spec_files.push(serde_json::json!({ "path": path, "from": format!("f{i}") }));
    }
    let spec: cimmeria_patchset::Spec = serde_json::from_value(
        serde_json::json!({ "id": id, "ops": spec_ops, "files": spec_files }),
    )
    .unwrap();
    cimmeria_patchset::build(&spec, &spec_dir, &stock, &patched)
        .unwrap()
        .zip
}

/// A signed release naming `zips`, in order. The seed is signed, never used.
fn release_of(zips: &[Vec<u8>]) -> VerifiedRelease {
    release_with_roots(zips, &[])
}

/// Like [`release_of`], with `roots[i]` as patch `i`'s root (default when short).
fn release_with_roots(zips: &[Vec<u8>], roots: &[&str]) -> VerifiedRelease {
    let seed = b"seed".to_vec();
    let patches: Vec<_> = zips
        .iter()
        .enumerate()
        .map(|(i, zip)| {
            let mut patch = serde_json::json!({
                "id": format!("p{i}"), "blob": format!("p{i}.zip"),
                "size": zip.len(), "sha256": hex(&Sha256::digest(zip)),
            });
            if let Some(root) = roots.get(i) {
                patch["root"] = (*root).into();
            }
            patch
        })
        .collect();
    let body = serde_json::to_vec(&serde_json::json!({
        "schema": 1,
        "seed": {"blob": "seed.zip", "size": seed.len(), "sha256": hex(&Sha256::digest(&seed))},
        "patches": patches,
    }))
    .unwrap();
    let signature = SigningKey::from_bytes(&[0x2a; 32]).sign(&body);
    crate::catalog::verify_release(&body, hex(&signature.to_bytes()).as_bytes()).unwrap()
}

struct Fixture {
    release: VerifiedRelease,
    plan: LightPlan,
    folder: tempfile::TempDir,
    scan: inventory::Index,
    prep: tempfile::TempDir,
    _zips: tempfile::TempDir,
}

/// The light plan of `zips` over a folder holding `files`, computed the way
/// `light_plan::plan` does, without a transport.
fn fixture(zips: Vec<Vec<u8>>, files: &[(&str, &[u8])]) -> Fixture {
    fixture_rooted(zips, &[], files)
}

fn fixture_rooted(zips: Vec<Vec<u8>>, roots: &[&str], files: &[(&str, &[u8])]) -> Fixture {
    let release = release_with_roots(&zips, roots);
    let dir = tempfile::tempdir().unwrap();
    let mut paths = Vec::new();
    for (i, zip) in zips.iter().enumerate() {
        let path = dir.path().join(format!("p{i}.zip"));
        std::fs::write(&path, zip).unwrap();
        paths.push(path);
    }
    let folder = tempfile::tempdir().unwrap();
    for (rel, bytes) in files {
        let path = inventory::native(folder.path(), rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }
    let scan = inventory::scan(folder.path(), &CancellationToken::new()).unwrap();
    let summaries: Vec<_> = paths
        .iter()
        .zip(&release.manifest().patches)
        .map(|(zip, entry)| patch_summary::summarize(zip, entry).unwrap())
        .collect();
    let held: std::collections::BTreeMap<String, String> = scan
        .iter()
        .map(|(name, entry)| (name.to_ascii_lowercase(), hex(&entry.sha256)))
        .collect();
    let pending = patch_summary::pending(&summaries, |path| {
        held.get(&path.to_ascii_lowercase())
            .or_else(|| held.get(&light_plan::stock_spelling(path)?.to_ascii_lowercase()))
            .cloned()
    });
    Fixture {
        release,
        plan: LightPlan {
            summaries,
            zips: paths,
            pending,
            download_bytes: 0,
        },
        folder,
        scan,
        prep: tempfile::tempdir().unwrap(),
        _zips: dir,
    }
}

fn sink() -> UnpackSink {
    UnpackSink {
        progress: ProgressSink::latest().0,
        label: "Light replay fixture".into(),
        cancel: CancellationToken::new(),
    }
}

fn run(f: &Fixture) -> Result<LightReplay, Error> {
    replay(
        &f.plan,
        &f.release,
        f.folder.path(),
        &f.scan,
        f.prep.path(),
        &CancellationToken::new(),
        &sink(),
    )
}

fn stored(f: &Fixture, rel: &str) -> Vec<u8> {
    std::fs::read(inventory::native(&f.prep.path().join("replacements"), rel)).unwrap()
}

fn two_independent_patches() -> Vec<Vec<u8>> {
    vec![
        patch_zip(
            "p0",
            &[OpFixture {
                target: OVERLAY,
                sources: &[(OVERLAY, &big("stock overlay"))],
                result: &big("patched overlay"),
            }],
            &[],
        ),
        patch_zip(
            "p1",
            &[OpFixture {
                target: SECOND,
                sources: &[(SECOND, &big("stock second"))],
                result: &big("patched second"),
            }],
            &[],
        ),
    ]
}

#[test]
fn a_stock_folder_gets_every_patch_touched_file_as_a_replacement() {
    let f = fixture(
        two_independent_patches(),
        &[
            (OVERLAY, &big("stock overlay")),
            (SECOND, &big("stock second")),
        ],
    );
    assert_eq!(f.plan.pending, vec![0, 1]);
    let done = run(&f).unwrap();
    assert!(done.incomplete.is_empty());
    assert!(done.moves.is_empty());
    let mut paths: Vec<_> = done.replacements.iter().map(|r| r.path.as_str()).collect();
    paths.sort_unstable();
    assert_eq!(paths, [OVERLAY, SECOND]);
    assert!(done
        .replacements
        .iter()
        .all(|r| r.reason == Classification::NeedsPatching));
    assert_eq!(stored(&f, OVERLAY), big("patched overlay"));
    assert_eq!(stored(&f, SECOND), big("patched second"));
    let overlay = done
        .replacements
        .iter()
        .find(|r| r.path == OVERLAY)
        .unwrap();
    assert_eq!(
        overlay.sha256[..],
        Sha256::digest(big("patched overlay"))[..]
    );
    assert!(
        !f.prep.path().join("scratch").exists(),
        "the scratch tree is removed"
    );
}

#[test]
fn a_changed_source_is_skipped_and_recorded_not_refused() {
    let f = fixture(
        two_independent_patches(),
        &[
            (OVERLAY, b"modded by the player"),
            (SECOND, &big("stock second")),
        ],
    );
    let done = run(&f).expect("a changed input never blocks the adoption");
    let paths: Vec<_> = done.replacements.iter().map(|r| r.path.as_str()).collect();
    assert_eq!(paths, [SECOND], "the changed file keeps the player's bytes");
    assert_eq!(done.incomplete.len(), 1);
    assert_eq!(done.incomplete[0].state_key, f.plan.summaries[0].state_key);
    assert_eq!(
        done.incomplete[0].skipped,
        [SkippedTarget {
            path: OVERLAY.into(),
            reason: "source_mismatch".into()
        }]
    );
}

#[test]
fn a_patch_that_reads_a_skipped_target_is_recorded_too() {
    let p0 = patch_zip(
        "p0",
        &[OpFixture {
            target: OVERLAY,
            sources: &[(OVERLAY, &big("stock overlay"))],
            result: &big("patched overlay"),
        }],
        &[],
    );
    let p1 = patch_zip(
        "p1",
        &[OpFixture {
            target: CHAIN,
            sources: &[(OVERLAY, &big("patched overlay"))],
            result: &big("chain result"),
        }],
        &[],
    );
    let f = fixture(vec![p0, p1], &[(OVERLAY, b"modded by the player")]);
    let done = run(&f).unwrap();
    assert!(done.replacements.is_empty());
    let recorded: Vec<_> = done
        .incomplete
        .iter()
        .map(|p| {
            (
                p.state_key.as_str(),
                p.skipped[0].path.as_str(),
                p.skipped[0].reason.as_str(),
            )
        })
        .collect();
    assert_eq!(
        recorded,
        [
            (
                f.plan.summaries[0].state_key.as_str(),
                OVERLAY,
                "source_mismatch"
            ),
            (
                f.plan.summaries[1].state_key.as_str(),
                CHAIN,
                "source_mismatch"
            ),
        ]
    );
}

#[test]
fn an_unmodified_chain_replays_through_the_scratch_tree() {
    let p0 = patch_zip(
        "p0",
        &[OpFixture {
            target: OVERLAY,
            sources: &[(OVERLAY, &big("stock overlay"))],
            result: &big("patched overlay"),
        }],
        &[],
    );
    let p1 = patch_zip(
        "p1",
        &[OpFixture {
            target: CHAIN,
            sources: &[(OVERLAY, &big("patched overlay"))],
            result: &big("chain result"),
        }],
        &[],
    );
    let f = fixture(vec![p0, p1], &[(OVERLAY, &big("stock overlay"))]);
    let done = run(&f).unwrap();
    assert!(done.incomplete.is_empty());
    assert_eq!(
        stored(&f, CHAIN),
        big("chain result"),
        "patch 1 read patch 0's scratch output"
    );
    assert_eq!(stored(&f, OVERLAY), big("patched overlay"));
}

#[test]
fn an_already_patched_folder_yields_no_replacements() {
    let f = fixture(
        two_independent_patches(),
        &[
            (OVERLAY, &big("patched overlay")),
            (SECOND, &big("patched second")),
        ],
    );
    assert!(f.plan.pending.is_empty());
    let done = run(&f).unwrap();
    assert!(done.replacements.is_empty());
    assert!(done.incomplete.is_empty());
    assert!(done.moves.is_empty());
}

#[test]
fn a_cooked_file_held_under_the_stock_name_is_a_move_not_a_replacement() {
    let p0 = patch_zip(
        "p0",
        &[OpFixture {
            target: OVERLAY,
            sources: &[(OVERLAY, &big("stock overlay"))],
            result: &big("patched overlay"),
        }],
        &[(COOKED_PLACED, b"cooked")],
    );
    let f = fixture(
        vec![p0],
        &[(OVERLAY, &big("stock overlay")), (COOKED_STOCK, b"cooked")],
    );
    assert_eq!(f.plan.pending, vec![0]);
    let done = run(&f).unwrap();
    let paths: Vec<_> = done.replacements.iter().map(|r| r.path.as_str()).collect();
    assert_eq!(paths, [OVERLAY]);
    assert_eq!(
        done.moves,
        [(COOKED_STOCK.to_string(), COOKED_PLACED.to_string())]
    );
}

#[test]
fn an_unusable_patch_archive_is_refused_with_a_closed_code() {
    let mut f = fixture(
        two_independent_patches(),
        &[
            (OVERLAY, &big("stock overlay")),
            (SECOND, &big("stock second")),
        ],
    );
    std::fs::write(&f.plan.zips[1], b"not a zip").unwrap();
    f.scan = inventory::scan(f.folder.path(), &CancellationToken::new()).unwrap();
    let (result, recorded) = rejection::scope(|| run(&f));
    let error = result.expect_err("an unusable archive is refused");
    let recorded = recorded.expect("the refused archive is recorded");
    assert_eq!(recorded.check, "patch_archive_unsupported");
    assert_eq!(recorded.entry.as_deref(), Some("patch:p1"));
    assert_eq!(recorded.found.as_deref(), Some(rejection::code(&error)));
}

#[test]
fn the_replayed_event_carries_the_four_counts() {
    let f = fixture(
        two_independent_patches(),
        &[
            (OVERLAY, b"modded by the player"),
            (SECOND, &big("stock second")),
        ],
    );
    let _serial = serial();
    // A second live Dispatch keeps callsite interest per dispatcher, so a callsite
    // that another module's test hit first is not left disabled.
    let _second = tracing::Dispatch::new(tracing_subscriber::registry());
    let (done, events) = crate::test_capture::capture(|| {
        tracing::callsite::rebuild_interest_cache(); // first line in the closure
        run(&f)
    });
    done.unwrap();
    let event = events
        .iter()
        .find(|e| e.field("event") == Some("adoption_light_replayed"))
        .expect("the replay emits adoption_light_replayed");
    assert_eq!(event.target, "adoption");
    assert_eq!(event.field("patches_replayed"), Some("2"));
    assert_eq!(event.field("replacements"), Some("1"));
    assert_eq!(event.field("ops_skipped"), Some("1"));
    assert_eq!(event.field("patches_incomplete"), Some("1"));
}

/// R-LA 10: tolerance is a per-call option. The strict unpack that downloaded
/// installs and the full replay use still refuses the same changed source.
#[test]
fn the_strict_unpack_still_refuses_a_changed_source() {
    let f = fixture(
        two_independent_patches(),
        &[
            (OVERLAY, b"modded by the player"),
            (SECOND, &big("stock second")),
        ],
    );
    let dest = tempfile::tempdir().unwrap();
    let path = inventory::native(dest.path(), OVERLAY);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"modded by the player").unwrap();
    let result = crate::unpack::unpack(&f.plan.zips[0], dest.path(), &sink());
    assert!(
        matches!(
            result,
            Err(cimmeria_launcher_core::unpack::UnpackError::PatchsetHash {
                kind: "source_mismatch",
                ..
            })
        ),
        "{result:?}"
    );
}

#[test]
fn a_replacement_keeps_the_folders_spelling() {
    let lower = "working/data/overlay.dat";
    let p0 = patch_zip(
        "p0",
        &[OpFixture {
            target: lower,
            sources: &[(lower, &big("stock overlay"))],
            result: &big("patched overlay"),
        }],
        &[],
    );
    let f = fixture(vec![p0], &[(OVERLAY, &big("stock overlay"))]);
    let done = run(&f).unwrap();
    assert_eq!(done.replacements.len(), 1);
    assert_eq!(done.replacements[0].path, OVERLAY);
    assert_eq!(stored(&f, OVERLAY), big("patched overlay"));
}

#[test]
fn an_overlay_in_a_patch_with_a_skipped_op_still_lands() {
    let p0 = patch_zip(
        "p0",
        &[OpFixture {
            target: OVERLAY,
            sources: &[(OVERLAY, &big("stock overlay"))],
            result: &big("patched overlay"),
        }],
        &[("Working/Data/Added.dat", b"added")],
    );
    let f = fixture(vec![p0], &[(OVERLAY, b"modded by the player")]);
    let done = run(&f).unwrap();
    let paths: Vec<_> = done.replacements.iter().map(|r| r.path.as_str()).collect();
    assert_eq!(paths, ["Working/Data/Added.dat"]);
    assert_eq!(stored(&f, "Working/Data/Added.dat"), b"added");
    assert_eq!(done.incomplete.len(), 1);
    assert_eq!(
        done.incomplete[0].skipped,
        [SkippedTarget {
            path: OVERLAY.into(),
            reason: "source_mismatch".into()
        }]
    );
}

#[test]
fn a_sgw_game_patch_records_its_skips_under_the_client_prefix() {
    let dialog = "UI/Dialog.lua";
    let p0 = patch_zip(
        "p0",
        &[OpFixture {
            target: dialog,
            sources: &[(dialog, &big("stock dialog"))],
            result: &big("patched dialog"),
        }],
        &[],
    );
    let f = fixture_rooted(
        vec![p0],
        &["sgw_game"],
        &[("Working/SGWGame/UI/Dialog.lua", b"modded by the player")],
    );
    let done = run(&f).unwrap();
    assert_eq!(done.incomplete.len(), 1);
    assert_eq!(
        done.incomplete[0].skipped,
        [SkippedTarget {
            path: "Working/SGWGame/UI/Dialog.lua".into(),
            reason: "source_mismatch".into()
        }]
    );
}

#[test]
fn a_cooked_source_held_under_the_stock_name_is_patched() {
    let p0 = patch_zip(
        "p0",
        &[OpFixture {
            target: COOKED_PLACED,
            sources: &[(COOKED_PLACED, &big("stock cooked"))],
            result: &big("patched cooked"),
        }],
        &[],
    );
    let f = fixture(vec![p0], &[(COOKED_STOCK, &big("stock cooked"))]);
    assert_eq!(f.plan.pending, vec![0]);
    let done = run(&f).unwrap();
    assert!(done.incomplete.is_empty(), "{:?}", done.incomplete);
    assert!(done.moves.is_empty());
    assert_eq!(stored(&f, COOKED_PLACED), big("patched cooked"));
}
