//! The incomplete patches of the installed copy, read from its own ledger.
use cimmeria_launcher_engine::state::InstalledState;
use std::path::Path;

/// How many patches the copy's `launcher-installed.json` lists as incomplete,
/// and how many ops they skipped. No copy, or no readable ledger, is (0, 0).
pub(super) fn incomplete_counts(game: Option<&Path>) -> (u32, u32) {
    let Some(game) = game else { return (0, 0) };
    let ledger = InstalledState::load(game);
    let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
    (
        count(ledger.incomplete_patches.len()),
        count(ledger.skipped_op_count()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_launcher_engine::state::{IncompletePatch, SkippedTarget};

    fn skip(path: &str) -> SkippedTarget {
        SkippedTarget {
            path: path.into(),
            reason: "source_mismatch".into(),
        }
    }

    #[test]
    fn no_copy_or_no_ledger_counts_nothing() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(incomplete_counts(None), (0, 0));
        assert_eq!(incomplete_counts(Some(dir.path())), (0, 0));
    }

    #[test]
    fn the_ledger_counts_its_incomplete_patches_and_skipped_ops() {
        let dir = tempfile::tempdir().unwrap();
        InstalledState {
            applied_patches: vec!["p1".into(), "p2".into()],
            incomplete_patches: vec![
                IncompletePatch {
                    state_key: "p1".into(),
                    skipped: vec![skip("Working/a.upk"), skip("Working/b.upk")],
                },
                IncompletePatch {
                    state_key: "p2".into(),
                    skipped: vec![skip("Working/c.upk")],
                },
            ],
            ..Default::default()
        }
        .save(dir.path())
        .unwrap();
        assert_eq!(incomplete_counts(Some(dir.path())), (2, 3));
    }
}
