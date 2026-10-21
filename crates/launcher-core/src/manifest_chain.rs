//! The append-only rule for signed manifests (LT-35).
//!
//! A release may only add to the manifest it replaces: new patches after the
//! last one, and a `file_index` entry. The seed and every patch already
//! listed keep their order, id, `after` link, `root`, size, sha256 and blob,
//! because an installed client keys its state on them. [`append_only`] is the
//! check `verify-file-index` runs before anyone signs.

use crate::manifest::{Manifest, PatchEntry};

/// One way a `next` manifest breaks the chain from `previous`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainBreak {
    /// The seed's blob, size or sha256 changed.
    SeedChanged,
    /// A patch from `previous` is gone.
    PatchRemoved(String),
    /// A patch from `previous` is still there, but at another position.
    PatchMoved(String),
    /// A field of a patch from `previous` changed (`size`, `sha256`, `blob`
    /// or `root`).
    PatchChanged { id: String, field: &'static str },
    /// The `after` link of a patch from `previous` changed.
    AfterChanged(String),
}

/// `next` may only append patches after `previous`'s last one and add or
/// replace `file_index`. Seed and every previous patch (order, id, after,
/// root, size, sha256, blob) unchanged. Returns every break found.
pub fn append_only(previous: &Manifest, next: &Manifest) -> Result<(), Vec<ChainBreak>> {
    let mut breaks = Vec::new();
    if previous.seed.blob != next.seed.blob
        || previous.seed.size != next.seed.size
        || !previous.seed.sha256.eq_ignore_ascii_case(&next.seed.sha256)
    {
        breaks.push(ChainBreak::SeedChanged);
    }
    for (index, old) in previous.patches.iter().enumerate() {
        match next.patches.get(index) {
            Some(new) if new.id == old.id => compare_patch(old, new, &mut breaks),
            _ if next.patches.iter().any(|p| p.id == old.id) => {
                breaks.push(ChainBreak::PatchMoved(old.id.clone()))
            }
            _ => breaks.push(ChainBreak::PatchRemoved(old.id.clone())),
        }
    }
    if breaks.is_empty() {
        Ok(())
    } else {
        Err(breaks)
    }
}

/// Pushes a break for each field of `old` that `new` changed.
fn compare_patch(old: &PatchEntry, new: &PatchEntry, breaks: &mut Vec<ChainBreak>) {
    let id = &old.id;
    if old.size != new.size {
        breaks.push(ChainBreak::PatchChanged {
            id: id.clone(),
            field: "size",
        });
    }
    if !old.sha256.eq_ignore_ascii_case(&new.sha256) {
        breaks.push(ChainBreak::PatchChanged {
            id: id.clone(),
            field: "sha256",
        });
    }
    if old.blob != new.blob {
        breaks.push(ChainBreak::PatchChanged {
            id: id.clone(),
            field: "blob",
        });
    }
    if old.root != new.root {
        breaks.push(ChainBreak::PatchChanged {
            id: id.clone(),
            field: "root",
        });
    }
    if old.after != new.after {
        breaks.push(ChainBreak::AfterChanged(id.clone()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{PatchRoot, SeedEntry};

    fn patch(id: &str, after: Option<&str>, sha: char) -> PatchEntry {
        PatchEntry {
            id: id.into(),
            blob: format!("{id}.zip"),
            size: 10,
            sha256: sha.to_string().repeat(64),
            after: after.map(str::to_string),
            root: PatchRoot::InstallDir,
            title: None,
            description: None,
        }
    }

    fn manifest(patches: Vec<PatchEntry>) -> Manifest {
        Manifest {
            schema: 1,
            min_launcher: None,
            seed: SeedEntry {
                blob: "seed.zip".into(),
                size: 1,
                sha256: "1".repeat(64),
            },
            patches,
            file_index: None,
        }
    }

    fn base() -> Manifest {
        manifest(vec![
            patch("001", None, '2'),
            patch("002", Some("001"), '3'),
        ])
    }

    #[test]
    fn identical_manifests_pass() {
        assert_eq!(append_only(&base(), &base()), Ok(()));
    }

    #[test]
    fn an_appended_patch_passes() {
        let mut next = base();
        next.patches.push(patch("003", Some("002"), '4'));
        assert_eq!(append_only(&base(), &next), Ok(()));
    }

    #[test]
    fn a_removed_patch_is_a_break() {
        let mut next = base();
        next.patches.pop();
        assert_eq!(
            append_only(&base(), &next),
            Err(vec![ChainBreak::PatchRemoved("002".into())])
        );
    }

    #[test]
    fn a_reordered_patch_is_a_break() {
        let mut next = base();
        next.patches.swap(0, 1);
        let breaks = append_only(&base(), &next).unwrap_err();
        assert!(breaks.contains(&ChainBreak::PatchMoved("001".into())));
        assert!(breaks.contains(&ChainBreak::PatchMoved("002".into())));
    }

    #[test]
    fn a_changed_sha_is_a_break() {
        let mut next = base();
        next.patches[1].sha256 = "9".repeat(64);
        assert_eq!(
            append_only(&base(), &next),
            Err(vec![ChainBreak::PatchChanged {
                id: "002".into(),
                field: "sha256",
            }])
        );
    }

    #[test]
    fn a_changed_after_is_a_break() {
        let mut next = base();
        next.patches[1].after = None;
        assert_eq!(
            append_only(&base(), &next),
            Err(vec![ChainBreak::AfterChanged("002".into())])
        );
    }

    #[test]
    fn a_changed_seed_is_a_break() {
        let mut next = base();
        next.seed.sha256 = "8".repeat(64);
        assert_eq!(
            append_only(&base(), &next),
            Err(vec![ChainBreak::SeedChanged])
        );
    }
}
