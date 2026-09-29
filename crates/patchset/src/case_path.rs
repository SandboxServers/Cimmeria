//! Resolve a recipe path against the install's own spelling of it.
//!
//! The game matches some resource names case-sensitively even on Windows:
//! its CEGUI resource provider looks `EULA.lua` up by the name the stock
//! `.toc` gives it, so a `eula.lua` on disk is "does not exist in group
//! lua" and the login screen never loads. Writing a patched file through a
//! temp file and a rename gives the file the name the rename was given, so
//! a recipe spelled in the wrong case would silently rename the stock file.
//! Every path a patch writes or reads is therefore resolved against the
//! directory listing first, and an existing file keeps its on-disk name.

use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

/// `root` joined with `rel`, with every component that already exists on
/// disk spelled as the directory listing spells it. Components past the
/// first one that doesn't exist keep `rel`'s spelling, so a new file gets
/// the name the recipe gives it.
///
/// An exact match wins over a case-insensitive one, so on a
/// case-sensitive file system holding both `EULA.lua` and `eula.lua` the
/// named one is used. Among several case-insensitive matches the first in
/// sorted order is used, so the result does not depend on listing order.
pub fn resolve_existing_case(root: &Path, rel: &Path) -> PathBuf {
    let mut out = root.to_path_buf();
    let mut resolving = true;
    for component in rel.components() {
        let Component::Normal(name) = component else {
            out.push(component.as_os_str());
            continue;
        };
        if resolving {
            match existing_spelling(&out, name) {
                Some(on_disk) => {
                    out.push(on_disk);
                    continue;
                }
                None => resolving = false,
            }
        }
        out.push(name);
    }
    out
}

/// The name of the entry in `dir` that is `name` ignoring case, if any.
fn existing_spelling(dir: &Path, name: &OsStr) -> Option<std::ffi::OsString> {
    let entries = std::fs::read_dir(dir).ok()?;
    let wanted = name.to_str().map(str::to_lowercase);
    let mut matches: Vec<std::ffi::OsString> = Vec::new();
    for entry in entries.flatten() {
        let candidate = entry.file_name();
        if candidate == name {
            return Some(candidate);
        }
        let same = match (&wanted, candidate.to_str()) {
            (Some(w), Some(c)) => c.to_lowercase() == *w,
            _ => false,
        };
        if same {
            matches.push(candidate);
        }
    }
    matches.sort();
    matches.into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(dir: &Path) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        v.sort();
        v
    }

    #[test]
    fn existing_components_keep_their_on_disk_spelling() {
        let dir = tempfile::tempdir().unwrap();
        let eula = dir.path().join("Working/SGWGame/Content/UI/Startup/EULA");
        std::fs::create_dir_all(&eula).unwrap();
        std::fs::write(eula.join("EULA.lua"), b"stock").unwrap();

        let got = resolve_existing_case(
            dir.path(),
            Path::new("working/sgwgame/content/ui/startup/eula/eula.lua"),
        );
        assert_eq!(
            got,
            dir.path()
                .join("Working/SGWGame/Content/UI/Startup/EULA/EULA.lua")
        );
        assert_eq!(names(&eula), ["EULA.lua"]);
    }

    #[test]
    fn new_components_keep_the_recipe_spelling() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("Working/binaries")).unwrap();
        let got = resolve_existing_case(
            dir.path(),
            Path::new("Working/Binaries/New/SGWLogConfig.xml"),
        );
        assert_eq!(
            got,
            dir.path().join("Working/binaries/New/SGWLogConfig.xml")
        );
    }
}
