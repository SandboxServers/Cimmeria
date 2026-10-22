//! The discriminator and the ledger gate of the seedless Update (LA-13).
use super::*;
use crate::manifest::PatchRoot;
use crate::state::{InstalledState, SkippedTarget};

fn patch(id: &str, sha: &str) -> PatchEntry {
    PatchEntry {
        id: id.to_owned(),
        blob: format!("{id}.zip"),
        size: 1,
        sha256: sha.to_owned(),
        after: None,
        root: PatchRoot::InstallDir,
        title: None,
        description: None,
    }
}

fn ledger(game: &Path, seed: &str, applied: &[&str], adopted: bool) {
    InstalledState {
        applied_patches: applied.iter().map(|key| (*key).to_owned()).collect(),
        seed_sha256: Some(seed.to_owned()),
        seed_adopted: adopted,
        incomplete_patches: Vec::new(),
    }
    .save(game)
    .unwrap();
}

/// D-LA14: only the light check opens the seedless path. Loosening this to
/// `Some(_)` or `true` sends copy-adopted and downloaded installs down it.
#[test]
fn only_a_light_adoption_may_skip_the_seed() {
    assert!(!seedless_allowed(None));
    assert!(!seedless_allowed(Some(FileCheck::Full)));
    assert!(seedless_allowed(Some(FileCheck::Light)));
}

#[test]
fn the_plan_lists_the_patches_after_the_applied_prefix() {
    let game = tempfile::tempdir().unwrap();
    ledger(game.path(), "AA", &["p0", "p1"], false);
    let all = [patch("p0", "a"), patch("p1", "b"), patch("p2", "c")];
    let planned = plan(game.path(), &all[..2], "aa", &all).unwrap();
    assert_eq!(planned.pending.len(), 1);
    assert_eq!(planned.pending[0].id, "p2");
}

#[test]
fn the_plan_refuses_a_ledger_that_does_not_match_the_target() {
    let game = tempfile::tempdir().unwrap();
    let all = [patch("p0", "a"), patch("p1", "b")];
    // No ledger at all.
    assert!(plan(game.path(), &all, "aa", &all).is_none());
    // A different seed.
    ledger(game.path(), "bb", &["p0"], false);
    assert!(plan(game.path(), &all, "aa", &all).is_none());
    // A seed recorded by an adoption flag, not a real extract.
    ledger(game.path(), "aa", &["p0"], true);
    assert!(plan(game.path(), &all, "aa", &all).is_none());
    // Applied patches that are not a prefix of the target's.
    ledger(game.path(), "aa", &["p1"], false);
    assert!(plan(game.path(), &all, "aa", &all).is_none());
    // More applied patches than the target has.
    ledger(game.path(), "aa", &["p0", "p1", "p2"], false);
    assert!(plan(game.path(), &all, "aa", &all).is_none());
}

#[test]
fn the_plan_refuses_a_republished_patch_under_the_same_id() {
    let game = tempfile::tempdir().unwrap();
    ledger(game.path(), "aa", &["p0"], false);
    let before = [patch("p0", "old")];
    let after = [patch("p0", "new"), patch("p1", "b")];
    assert!(plan(game.path(), &before, "aa", &after).is_none());
    // Without the republish it is accepted.
    let same = [patch("p0", "old"), patch("p1", "b")];
    assert!(plan(game.path(), &before, "aa", &same).is_some());
}

#[test]
fn the_write_set_names_every_file_the_update_rewrites_in_place() {
    let game = tempfile::tempdir().unwrap();
    let mut state = InstalledState::default();
    state.record_incomplete(
        "p0",
        vec![SkippedTarget {
            path: "Working/Data/Skipped.dat".to_owned(),
            reason: "source_missing".to_owned(),
        }],
    );
    state.save(game.path()).unwrap();
    let set = write_set(game.path(), &[]).unwrap();
    for expected in [
        "launcher-installed.json",
        "working/binaries/sgw.exe",
        "working/sgwgame/content/ui/startup/login/logininternal.lua",
        "working/data/skipped.dat",
    ] {
        assert!(set.contains(expected), "{expected} missing from {set:?}");
    }
}
