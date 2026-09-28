//! Where a manifest patch's zip entries are extracted: the install
//! directory, or the client's `SGWGame/` directory for a
//! `"root": "sgw_game"` patch (the client-patches UI overlay).

use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::install_layout;
use crate::manifest::{PatchEntry, PatchRoot};

#[derive(Debug, Error)]
#[error(
    "patch {id} installs into the client's SGWGame directory, but there is none beside SGW.exe's \
     directory or in {}",
    .install_dir.display()
)]
pub struct NoSgwGameDir {
    pub id: String,
    pub install_dir: PathBuf,
}

/// The client's `SGWGame/` directory for the install rooted at
/// `install_dir`: the sibling of the directory holding `SGW.exe`
/// ([`install_layout::binaries_dir`]), which is `Working\SGWGame` in the
/// stock tree, else `<install_dir>/SGWGame` for a flattened client.
pub fn sgw_game_dir(install_dir: &Path) -> Option<PathBuf> {
    let binaries = install_layout::binaries_dir(install_dir);
    if let Some(sibling) = binaries.parent().map(|p| p.join("SGWGame")) {
        if sibling.is_dir() {
            return Some(sibling);
        }
    }
    let inside = install_dir.join("SGWGame");
    inside.is_dir().then_some(inside)
}

/// Where `patch`'s zip entries go. A `sgw_game` patch with no `SGWGame/`
/// to go into is refused rather than extracted somewhere the client
/// never reads.
pub fn patch_dest(install_dir: &Path, patch: &PatchEntry) -> Result<PathBuf, NoSgwGameDir> {
    match patch.root {
        PatchRoot::InstallDir => Ok(install_dir.to_path_buf()),
        PatchRoot::SgwGame => sgw_game_dir(install_dir).ok_or_else(|| NoSgwGameDir {
            id: patch.id.clone(),
            install_dir: install_dir.to_path_buf(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn patch(root: PatchRoot) -> PatchEntry {
        PatchEntry {
            id: "p".into(),
            blob: "b".into(),
            size: 1,
            sha256: "h".into(),
            after: None,
            root,
        }
    }

    /// `<root>\Working\Binaries\SGW.exe` + `<root>\Working\SGWGame`.
    fn stock_tree(root: &Path) {
        let binaries = root.join("Working").join("Binaries");
        std::fs::create_dir_all(&binaries).unwrap();
        std::fs::write(binaries.join("SGW.exe"), b"").unwrap();
        std::fs::create_dir_all(root.join("Working").join("SGWGame")).unwrap();
    }

    #[test]
    fn install_dir_patches_extract_into_the_install_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            patch_dest(dir.path(), &patch(PatchRoot::InstallDir)).unwrap(),
            dir.path()
        );
    }

    /// The install path is the client root (#986): the overlay goes to
    /// `Working\SGWGame`, beside `Working\Binaries`.
    #[test]
    fn sgw_game_patches_go_to_working_sgwgame_from_the_client_root() {
        let dir = tempfile::tempdir().unwrap();
        stock_tree(dir.path());
        assert_eq!(
            patch_dest(dir.path(), &patch(PatchRoot::SgwGame)).unwrap(),
            dir.path().join("Working").join("SGWGame")
        );
    }

    /// An older config that points straight at `Working\Binaries` lands in
    /// the same place.
    #[test]
    fn sgw_game_patches_find_sgwgame_from_a_binaries_path() {
        let dir = tempfile::tempdir().unwrap();
        stock_tree(dir.path());
        let binaries = dir.path().join("Working").join("Binaries");
        assert_eq!(
            patch_dest(&binaries, &patch(PatchRoot::SgwGame)).unwrap(),
            dir.path().join("Working").join("SGWGame")
        );
    }

    #[test]
    fn a_flattened_client_uses_sgwgame_inside_the_install_dir() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("SGW.exe"), b"").unwrap();
        std::fs::create_dir_all(dir.path().join("SGWGame")).unwrap();
        assert_eq!(
            patch_dest(dir.path(), &patch(PatchRoot::SgwGame)).unwrap(),
            dir.path().join("SGWGame")
        );
    }

    #[test]
    fn sgw_game_patch_without_sgwgame_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let binaries = dir.path().join("Working").join("Binaries");
        std::fs::create_dir_all(&binaries).unwrap();
        std::fs::write(binaries.join("SGW.exe"), b"").unwrap();
        let err = patch_dest(dir.path(), &patch(PatchRoot::SgwGame)).unwrap_err();
        assert_eq!(err.id, "p");
    }
}
