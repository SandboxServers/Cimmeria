//! Compares a folder with the signed file index, file by file (LA-06).
//!
//! [`super::check`] and `verification` answer "is this folder the release?".
//! This module answers "how does this folder differ from the seed?", so a
//! light-adopted install can show what it has, what it lacks and what a
//! patch or setup rule left behind. The folder is a map from a `/`-separated
//! client-root path (any case) to its lower-case SHA-256.

use std::collections::{BTreeMap, HashSet};

use serde::Serialize;

use super::{FileIndex, Origin, PathHistory, SetupRule, State};

/// How one file relates to the index. Serialized in snake_case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileClass {
    /// The seed's bytes.
    Stock,
    /// The bytes some signed patch writes.
    Patched,
    /// The bytes a setup rule leaves (the ASLR-off file or a login-servers file).
    Prepared,
    /// Indexed, but matching no layer and no setup rule.
    Changed,
    /// A seed file the folder does not have.
    Missing,
    /// Not in the index at all.
    Extra,
}

/// The per-class counts and the non-`Stock` files of one comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexComparison {
    /// Every class is present; a class with no files counts zero.
    pub counts: BTreeMap<FileClass, u64>,
    /// Every non-`Stock` file, sorted by path. A `Missing` entry carries the
    /// index's spelling, any other entry the folder's.
    pub files: Vec<(String, FileClass)>,
}

const ALL_CLASSES: [FileClass; 6] = [
    FileClass::Stock,
    FileClass::Patched,
    FileClass::Prepared,
    FileClass::Changed,
    FileClass::Missing,
    FileClass::Extra,
];

/// Classifies each folder file against `index`, then counts the seed files
/// the folder lacks.
///
/// Paths match case-blind. A patch-only path that is absent from the folder
/// is not counted: only the seed layer defines what a folder must hold.
pub fn compare(index: &FileIndex, folder: &BTreeMap<String, String>) -> IndexComparison {
    let histories = index.histories();
    let mut counts: BTreeMap<FileClass, u64> = ALL_CLASSES.iter().map(|c| (*c, 0)).collect();
    let mut files = Vec::new();
    let mut present = HashSet::new();
    for (path, hash) in folder {
        let lower = path.to_ascii_lowercase();
        let hash = hash.to_ascii_lowercase();
        let class = classify(index, histories.get(&lower), &lower, &hash);
        *counts.entry(class).or_default() += 1;
        if class != FileClass::Stock {
            files.push((path.clone(), class));
        }
        present.insert(lower);
    }
    for file in &index.seed.files {
        if !present.contains(&file.path.to_ascii_lowercase()) {
            *counts.entry(FileClass::Missing).or_default() += 1;
            files.push((file.path.clone(), FileClass::Missing));
        }
    }
    files.sort();
    IndexComparison { counts, files }
}

/// The class of one folder file. `lower` is its lower-case path, `hash` its
/// lower-case SHA-256, `history` the index's states for `lower` if any.
fn classify(
    index: &FileIndex,
    history: Option<&PathHistory>,
    lower: &str,
    hash: &str,
) -> FileClass {
    if let Some(history) = history {
        if history
            .states
            .iter()
            .any(|s| s.origin == Origin::Seed && state_hex(s) == hash)
        {
            return FileClass::Stock;
        }
        if history
            .states
            .iter()
            .any(|s| matches!(s.origin, Origin::Patch(_)) && state_hex(s) == hash)
        {
            return FileClass::Patched;
        }
    }
    if setup_prepares(index, lower, hash) {
        FileClass::Prepared
    } else if history.is_some() {
        FileClass::Changed
    } else {
        FileClass::Extra
    }
}

/// True when a setup rule names `lower` and `hash` is what that rule leaves:
/// any login-servers file, or the ASLR-off file at the rule's SHA-256.
fn setup_prepares(index: &FileIndex, lower: &str, hash: &str) -> bool {
    index.setup.iter().any(|rule| match rule {
        SetupRule::AslrOff { path, sha256, .. } => {
            path.eq_ignore_ascii_case(lower) && sha256.eq_ignore_ascii_case(hash)
        }
        SetupRule::LoginServers { path } => path.eq_ignore_ascii_case(lower),
    })
}

/// A state's SHA-256 as lower-case hex.
fn state_hex(state: &State) -> String {
    state.sha256.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_index::{
        IndexedFile, PatchLayer, SeedLayer, FILE_INDEX_SCHEMA, SETUP_POLICY_VERSION,
    };

    fn sha(c: char) -> String {
        c.to_string().repeat(64)
    }

    fn file(path: &str, c: char) -> IndexedFile {
        IndexedFile {
            path: path.into(),
            size: 1,
            sha256: sha(c),
        }
    }

    /// Seed: `Stock.pak`, `SGW.exe` (`a`), `Changed.dat`, `Gone.dat`. Patch 0
    /// rewrites `SGW.exe` to `b` and adds `New.bin` (`c`). Setup: an ASLR-off
    /// `Aslr.dll` at `f` and a login-servers `LoginInternal.lua`.
    fn index() -> FileIndex {
        FileIndex {
            schema_version: FILE_INDEX_SCHEMA,
            setup_policy_version: SETUP_POLICY_VERSION,
            seed: SeedLayer {
                sha256: sha('1'),
                files: vec![
                    file("Working/Stock.pak", 'a'),
                    file("Working/Binaries/SGW.exe", 'a'),
                    file("Working/Changed.dat", 'a'),
                    file("Working/Gone.dat", 'a'),
                ],
            },
            patches: vec![PatchLayer {
                state_key: "001".into(),
                sha256: sha('2'),
                inputs: vec![],
                outputs: vec![
                    file("Working/Binaries/SGW.exe", 'b'),
                    file("Working/New.bin", 'c'),
                ],
            }],
            setup: vec![
                SetupRule::AslrOff {
                    path: "Working/Binaries/Aslr.dll".into(),
                    size: 1,
                    sha256: sha('f'),
                },
                SetupRule::LoginServers {
                    path: "Working/SGWGame/Content/UI/Startup/Login/LoginInternal.lua".into(),
                },
            ],
        }
    }

    fn folder(entries: &[(&str, char)]) -> BTreeMap<String, String> {
        entries
            .iter()
            .map(|(path, c)| (path.to_string(), sha(*c)))
            .collect()
    }

    /// One folder that yields every class exactly once.
    #[test]
    fn every_class_counts_once() {
        let tree = folder(&[
            ("WORKING/stock.PAK", 'a'),
            ("Working/Binaries/SGW.exe", 'b'),
            ("Working/Changed.dat", 'd'),
            ("Working/Binaries/Aslr.dll", 'f'),
            (
                "Working/SGWGame/Content/UI/Startup/Login/LoginInternal.lua",
                '9',
            ),
            ("Working/Extra.dat", 'e'),
        ]);
        let got = compare(&index(), &tree);
        let want_counts: BTreeMap<FileClass, u64> = [
            (FileClass::Stock, 1),
            (FileClass::Patched, 1),
            (FileClass::Prepared, 2),
            (FileClass::Changed, 1),
            (FileClass::Missing, 1),
            (FileClass::Extra, 1),
        ]
        .into_iter()
        .collect();
        assert_eq!(got.counts, want_counts);
        // `Gone.dat` is the only seed file absent; `New.bin` is patch-only and absent.
        assert_eq!(
            got.files,
            vec![
                ("Working/Binaries/Aslr.dll".to_string(), FileClass::Prepared),
                ("Working/Binaries/SGW.exe".to_string(), FileClass::Patched),
                ("Working/Changed.dat".to_string(), FileClass::Changed),
                ("Working/Extra.dat".to_string(), FileClass::Extra),
                ("Working/Gone.dat".to_string(), FileClass::Missing),
                (
                    "Working/SGWGame/Content/UI/Startup/Login/LoginInternal.lua".to_string(),
                    FileClass::Prepared
                ),
            ]
        );
    }

    /// Paths match case-blind, and a stock file in any case is `Stock`, not listed.
    #[test]
    fn match_is_case_blind() {
        let got = compare(&index(), &folder(&[("WORKING/STOCK.pak", 'a')]));
        assert_eq!(got.counts[&FileClass::Stock], 1);
        assert!(got.files.iter().all(|(p, _)| p != "WORKING/STOCK.pak"));
        // A changed file keeps the folder's spelling in `files`.
        let got = compare(&index(), &folder(&[("WORKING/CHANGED.DAT", 'd')]));
        assert_eq!(
            got.files,
            vec![
                ("WORKING/CHANGED.DAT".to_string(), FileClass::Changed),
                ("Working/Binaries/SGW.exe".to_string(), FileClass::Missing),
                ("Working/Gone.dat".to_string(), FileClass::Missing),
                ("Working/Stock.pak".to_string(), FileClass::Missing),
            ]
        );
    }

    /// An empty folder misses every seed file, but never a patch-only one.
    #[test]
    fn patch_only_path_absent_is_not_missing() {
        let got = compare(&index(), &BTreeMap::new());
        assert_eq!(got.counts[&FileClass::Missing], 4);
        assert!(got.files.iter().all(|(p, _)| p != "Working/New.bin"));
    }

    /// The ASLR-off rule prepares only the file at its own SHA-256.
    #[test]
    fn aslr_file_at_wrong_sha_is_extra() {
        let got = compare(&index(), &folder(&[("Working/Binaries/Aslr.dll", 'e')]));
        assert_eq!(got.counts[&FileClass::Extra], 1);
        assert_eq!(got.counts[&FileClass::Prepared], 0);
    }
}
