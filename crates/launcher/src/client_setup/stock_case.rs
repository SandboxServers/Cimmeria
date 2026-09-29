//! Put back the stock spelling of the files the patch sets write.
//!
//! The game looks some UI resources up case-sensitively, even on Windows:
//! with `EULA.lua` renamed to `eula.lua` its CEGUI resource provider logs
//! "'EULA.lua' does not exist in group lua", the EULA screen never loads,
//! and the player sees the gate backdrop with no login screen.
//!
//! `cimmeria-patchset` before 2026-09-29 wrote every rebuilt file through
//! a temp file and a rename under the recipe's spelling, and the published
//! `005-login-delay` recipe spells it `eula.lua`, so every install made
//! with launcher-20260929-f518b57 lost the stock name. The patch is
//! recorded as applied and never re-runs, so the name has to be repaired
//! in place; this step does that on every install, update and launch.
//! `cimmeria_patchset::apply` now keeps a file's on-disk name, so new
//! installs never need it.

use std::path::{Path, PathBuf};

/// Every file a published patch set writes, spelled as the 2009 cabinets
/// spell it (their `DATA.INF` file list, checked 2026-09-29; the ones no
/// cabinet ships, spelled as the patch creates them). Bundled data lives
/// under `SourceCache.en-us`, the name `install_layout` gives the
/// cabinets' `Cache.en-US`. `every_patch_target_is_listed` keeps this in
/// step with `data/client-patches/*/patch.json`.
pub const PATCH_TARGETS: &[&str] = &[
    // 001-dialog-portraits
    "Working/SGWGame/Content/UI/CEGUIData/schemes/TaharezLook.scheme",
    "Working/SGWGame/Content/UI/Core/Dialog/Blurb.layout",
    "Working/SGWGame/Content/UI/Core/Dialog/Blurb.lua",
    "Working/SGWGame/Content/UI/Core/Dialog/Dialog.layout",
    "Working/SGWGame/Content/UI/Core/Dialog/Dialog.lua",
    // 002-castle-ring-transport; 007-castle-armory-ring writes fffeffff too.
    "Working/SGWGame/CookedPC/Maps/Castle_CellBlock/Castle_CellBlock-fffdfffc.umap",
    "Working/SGWGame/CookedPC/Maps/Castle_CellBlock/Castle_CellBlock-fffeffff.umap",
    // 003-cooked-data
    "Working/SGWGame/SourceCache.en-us/CookedDataKismetSeqEvent.pak",
    "Working/SGWGame/SourceCache.en-us/CookedDataKismetSetEvent.pak",
    "Working/SGWGame/SourceCache.en-us/CookedInteractionSet.pak",
    // 005-login-delay: the one the published recipe misspells.
    "Working/SGWGame/Content/UI/Startup/EULA/EULA.lua",
    // 006-gate-sound-bank: new files, in the stock `audio/ui` directory.
    "Working/SGWGame/Content/audio/ui/prp_gen.fev",
    "Working/SGWGame/Content/audio/ui/prp_gen_gate.fsb",
];

/// One file [`restore`] renamed, as paths relative to the install.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Restored {
    pub from: PathBuf,
    pub to: PathBuf,
}

/// Rename every [`PATCH_TARGETS`] file whose on-disk name differs from
/// the stock name only in case. A file that is missing, already right, or
/// present under both spellings (a case-sensitive directory) is left
/// alone. Directories are not renamed: the game only cares about the file
/// names, and the stock tree mixes cases itself (`binaries`, `audio`).
pub fn restore(install_dir: &Path) -> std::io::Result<Vec<Restored>> {
    let mut out = Vec::new();
    for target in PATCH_TARGETS {
        let rel = Path::new(target);
        let (Some(parent), Some(stock_name)) = (rel.parent(), rel.file_name()) else {
            continue;
        };
        let dir = cimmeria_patchset::resolve_existing_case(install_dir, parent);
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let wanted = stock_name.to_string_lossy().to_lowercase();
        let mut exact = false;
        let mut other = None;
        for entry in entries.flatten() {
            let name = entry.file_name();
            if name == stock_name {
                exact = true;
            } else if name.to_string_lossy().to_lowercase() == wanted {
                other = Some(name);
            }
        }
        let (false, Some(wrong)) = (exact, other) else {
            continue;
        };
        let from = dir.join(&wrong);
        let to = dir.join(stock_name);
        // A case-only rename; NTFS renames the entry in place.
        std::fs::rename(&from, &to).map_err(|e| {
            std::io::Error::new(
                e.kind(),
                format!(
                    "could not rename {} to {}: {e}",
                    from.display(),
                    stock_name.to_string_lossy()
                ),
            )
        })?;
        tracing::info!(
            from = %from.display(),
            to = %to.display(),
            "restored the stock file name"
        );
        let rel_of = |p: &Path| p.strip_prefix(install_dir).unwrap_or(p).to_path_buf();
        out.push(Restored {
            from: rel_of(&from),
            to: rel_of(&to),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EULA_DIR: &str = "Working/SGWGame/Content/UI/Startup/EULA";

    fn listed(dir: &Path) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        v.sort();
        v
    }

    // Bug shape: an install made with launcher-20260929-f518b57 has the
    // patched EULA script as `eula.lua`, and 005 is recorded as applied,
    // so nothing but this step brings the login screen back. Checked by
    // directory listing, since `exists()` is case-blind on Windows.
    #[test]
    fn a_lowercased_eula_script_gets_its_stock_name_back() {
        let dir = tempfile::tempdir().unwrap();
        let eula = dir.path().join(EULA_DIR);
        std::fs::create_dir_all(&eula).unwrap();
        std::fs::write(eula.join("eula.lua"), b"patched").unwrap();
        std::fs::write(eula.join("EULA.layout"), b"stock").unwrap();

        let restored = restore(dir.path()).unwrap();
        assert_eq!(listed(&eula), ["EULA.layout", "EULA.lua"]);
        assert_eq!(std::fs::read(eula.join("EULA.lua")).unwrap(), b"patched");
        assert_eq!(
            restored,
            [Restored {
                from: Path::new(EULA_DIR).join("eula.lua"),
                to: Path::new(EULA_DIR).join("EULA.lua"),
            }]
        );

        // Idempotent: the second run finds nothing to do.
        assert!(restore(dir.path()).unwrap().is_empty());
        assert_eq!(listed(&eula), ["EULA.layout", "EULA.lua"]);
    }

    /// A stock install, or one without the patched directories at all, is
    /// left exactly as it is, and directory case doesn't matter.
    #[test]
    fn a_stock_install_is_left_alone() {
        let dir = tempfile::tempdir().unwrap();
        assert!(restore(dir.path()).unwrap().is_empty());
        let eula = dir.path().join("working/sgwgame/content/ui/startup/eula");
        std::fs::create_dir_all(&eula).unwrap();
        std::fs::write(eula.join("EULA.lua"), b"stock").unwrap();
        assert!(restore(dir.path()).unwrap().is_empty());
        assert_eq!(listed(&eula), ["EULA.lua"]);
    }

    /// Every op target in every patch spec is listed here, spelled exactly
    /// the same, so a spec in the wrong case (the `eula.lua` bug) or a new
    /// patch nobody added here fails the build. The list itself is the
    /// cabinets' spelling; see [`PATCH_TARGETS`].
    #[test]
    fn every_patch_target_is_listed() {
        let dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/client-patches");
        let mut seen = 0;
        for entry in std::fs::read_dir(&dir).unwrap() {
            let spec_path = entry.unwrap().path().join("patch.json");
            if !spec_path.is_file() {
                continue;
            }
            let spec: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&spec_path).unwrap()).unwrap();
            for op in spec["ops"].as_array().into_iter().flatten() {
                let target = op["target"].as_str().unwrap();
                assert!(
                    PATCH_TARGETS.contains(&target),
                    "{} targets {target}, which PATCH_TARGETS doesn't list in that spelling",
                    spec_path.display()
                );
                seen += 1;
            }
        }
        assert!(seen > 0, "no patch specs found under {}", dir.display());
    }
}
