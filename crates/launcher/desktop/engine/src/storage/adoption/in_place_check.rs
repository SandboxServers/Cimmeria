//! The final check of an in-place adoption, before the receipt. A full check compares
//! one digest of the owned tree; a light check compares the tree file by file against
//! the one stored at admission, so a mismatch names what differs and a spelling-only
//! difference (the directory's case) no longer fails an otherwise correct adoption.
use super::in_place::SETUP_FILES;
use super::publication::{read_large, write_large};
use super::tree_diff::{diff, log_tree_checked, TreeOutcome};
use super::verification::Tree;
use super::*;
use std::collections::BTreeMap;

/// Where admission stores a light adoption's expected tree, inside the work folder
/// (C-LF3). It is not part of `Plan`, which is digested and rejects unknown fields.
pub(super) const EXPECTED_TREE: &str = "expected-tree.json";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Stored {
    version: u32,
    tree: BTreeMap<String, (u64, String)>,
}

/// Writes `<stage>/expected-tree.json` atomically. The work folder may not exist
/// yet: it is created by the first replacement copy, and a light adoption can have none.
pub(super) fn write_expected(stage: &Path, tree: &Tree) -> Result<(), Error> {
    std::fs::create_dir_all(stage)?;
    let stored = Stored {
        version: 1,
        tree: tree
            .iter()
            .map(|(path, (size, sha))| (path.clone(), (*size, to_hex(sha))))
            .collect(),
    };
    write_large(stage, EXPECTED_TREE, &stored)
}

/// The stored tree, or `None` when the file is absent, unreadable or not what
/// `write_expected` writes: an adoption an earlier build began has none, and the
/// check then has nothing to compare (it reports `unrecorded`).
pub(super) fn read_expected(stage: &Path) -> Option<Tree> {
    let stored: Stored = read_large(&stage.join(EXPECTED_TREE)).ok()?;
    if stored.version != 1 {
        return None;
    }
    stored
        .tree
        .into_iter()
        .map(|(path, (size, hex))| Some((path, (size, from_hex(&hex)?))))
        .collect()
}

pub(super) fn validate(game: &Path, plan: &Plan) -> Result<(), Error> {
    inventory::same_directory(game, &plan.stage_identity)?;
    let scanned = inventory::scan(game, &CancellationToken::new())?;
    if !matches!(plan.provenance.check, FileCheck::Light) {
        if inventory::content_digest(&scanned)? != plan.provenance.reference_digest {
            log_tree_checked("verified", TreeOutcome::Mismatch, None);
            return Err(StorageError::Corrupt.into());
        }
        return Ok(());
    }
    let Some(expected) = read_expected(&plan.stage()) else {
        log_tree_checked("light", TreeOutcome::Unrecorded, None);
        return Ok(());
    };
    // The ledger is synthesized after the tree was predicted and is not in it.
    let [_, _, ledger] = SETUP_FILES;
    let actual: Tree = scanned
        .iter()
        .filter(|(path, _)| !path.eq_ignore_ascii_case(ledger))
        .map(|(path, entry)| (path.clone(), (entry.size, entry.sha256)))
        .collect();
    let found = diff(&expected, &actual, &SETUP_FILES);
    let outcome = if found.is_empty() {
        TreeOutcome::Matched
    } else if found.blocking() {
        TreeOutcome::Mismatch
    } else {
        TreeOutcome::Accepted
    };
    log_tree_checked("light", outcome, Some(&found));
    if found.blocking() {
        return Err(StorageError::Corrupt.into());
    }
    Ok(())
}

fn to_hex(sha: &[u8; 32]) -> String {
    sha.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn from_hex(text: &str) -> Option<[u8; 32]> {
    if text.len() != 64 || !text.is_ascii() {
        return None;
    }
    let mut out = [0u8; 32];
    for (slot, pair) in out.iter_mut().zip(text.as_bytes().chunks(2)) {
        *slot = u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()?;
    }
    Some(out)
}
