//! What a patch zip reads and writes, for light adoption (LA-01).
//!
//! A light adoption has no stock seed to replay against, so the launcher
//! needs each patch's contract up front: the files its recipe reads (with
//! the hashes it expects), and the files it leaves behind (with their
//! hashes). [`summarize`] reads that from the zip alone, and [`pending`]
//! says which patches a player's folder has not yet taken.
//!
//! Paths are client-root relative with `/` separators. A `sgw_game` patch's
//! zip entries land in `Working/SGWGame`, so they are prefixed with that.

use std::collections::BTreeSet;
use std::collections::HashSet;
use std::io::Read;
use std::path::Path;

use cimmeria_patchset::recipe::{safe_relative, DELTA_DIR};
use cimmeria_patchset::{Recipe, RECIPE_NAME};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::manifest::{PatchEntry, PatchRoot};

/// The client's `SGWGame` directory in the stock tree, relative to the
/// client root (`Working/SGWGame`).
const SGW_GAME_DIR: &str = "Working/SGWGame";

/// One file a patch depends on or produces: a client-root relative path
/// and the lower-hex SHA-256 of its bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pinned {
    /// Client-root relative, `/`-separated, with the patch's destination
    /// prefix.
    pub path: String,
    /// Lower-hex SHA-256 the file must have.
    pub sha256: String,
    /// Other lower-hex SHA-256 values this source may carry instead, from
    /// the op's `alternatives` for the same path, sorted and unique. Empty
    /// for an output.
    pub alternatives: Vec<String>,
}

/// A patch's contract: its state key, the files it reads and the files it
/// writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchSummary {
    /// The id the launcher records once applied, see [`PatchEntry::state_key`].
    pub state_key: String,
    /// Each recipe op's sources, in recipe order, each (path, hash) once.
    pub sources: Vec<Pinned>,
    /// Each recipe op's target, then every other zip entry except the
    /// recipe, the `deltas/` entries and directories, in zip order.
    pub outputs: Vec<Pinned>,
}

/// Why a patch zip could not be summarized.
#[derive(Debug, Error)]
pub enum PatchSummaryError {
    #[error("cannot read the patch zip: {0}")]
    Io(#[from] std::io::Error),
    #[error("cannot read the patch zip: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("the patch recipe is not valid: {0}")]
    Recipe(String),
    #[error("the patch zip names an unsafe path: {0}")]
    Path(String),
}

/// Reads the contract of the patch at `zip`, which `entry` describes.
///
/// A zip without a recipe contributes every entry as an output and no
/// sources.
pub fn summarize(zip: &Path, entry: &PatchEntry) -> Result<PatchSummary, PatchSummaryError> {
    let prefix = match entry.root {
        PatchRoot::InstallDir => "",
        PatchRoot::SgwGame => SGW_GAME_DIR,
    };
    let mut archive = zip::ZipArchive::new(std::fs::File::open(zip)?)?;
    let recipe = read_recipe(&mut archive)?;
    let mut summary = PatchSummary {
        state_key: entry.state_key(),
        sources: Vec::new(),
        outputs: Vec::new(),
    };

    if let Some(recipe) = &recipe {
        let mut seen = HashSet::new();
        for op in &recipe.ops {
            for source in &op.sources {
                let sha256 = source.sha256.to_ascii_lowercase();
                let path = client_path(prefix, &source.path)?;
                if !seen.insert((path.to_ascii_lowercase(), sha256.clone())) {
                    continue;
                }
                let alternatives: BTreeSet<String> = op
                    .alternatives
                    .iter()
                    .flat_map(|alt| &alt.sources)
                    .filter(|alt| alt.path.eq_ignore_ascii_case(&source.path))
                    .map(|alt| alt.sha256.to_ascii_lowercase())
                    .filter(|alt| *alt != sha256)
                    .collect();
                summary.sources.push(Pinned {
                    path,
                    sha256,
                    alternatives: alternatives.into_iter().collect(),
                });
            }
            summary.outputs.push(Pinned {
                path: client_path(prefix, &op.target)?,
                sha256: op.result_sha256.to_ascii_lowercase(),
                alternatives: Vec::new(),
            });
        }
    }

    for index in 0..archive.len() {
        let mut file = archive.by_index(index)?;
        let name = file.name()?.to_string();
        if file.is_dir() || name == RECIPE_NAME || name.starts_with(DELTA_DIR) {
            continue;
        }
        let sha256 = hash_reader(&mut file)?;
        summary.outputs.push(Pinned {
            path: client_path(prefix, &name)?,
            sha256,
            alternatives: Vec::new(),
        });
    }
    Ok(summary)
}

/// Which of `summaries` are not current, as indices in signed order.
///
/// A patch is current when, for every output, `have(path)` equals that
/// output's hash or the hash the same path has in the output of a later
/// patch. `have` gets the output path as written in the summary and
/// returns the lower-hex hash the player's file has, or `None` when the
/// file is missing; the caller does any case-blind lookup. Paths are
/// compared case-blind between patches.
pub fn pending(summaries: &[PatchSummary], have: impl Fn(&str) -> Option<String>) -> Vec<usize> {
    (0..summaries.len())
        .filter(|&index| !is_current(summaries, index, &have))
        .collect()
}

fn is_current(
    summaries: &[PatchSummary],
    index: usize,
    have: &impl Fn(&str) -> Option<String>,
) -> bool {
    summaries[index].outputs.iter().all(|output| {
        let Some(found) = have(output.path.as_str()) else {
            return false;
        };
        found.eq_ignore_ascii_case(&output.sha256)
            || summaries[index + 1..].iter().any(|later| {
                later.outputs.iter().any(|o| {
                    o.path.eq_ignore_ascii_case(&output.path)
                        && o.sha256.eq_ignore_ascii_case(&found)
                })
            })
    })
}

/// The recipe in `archive`, if it has one.
fn read_recipe(
    archive: &mut zip::ZipArchive<std::fs::File>,
) -> Result<Option<Recipe>, PatchSummaryError> {
    let Some(index) = archive.index_for_name(RECIPE_NAME) else {
        return Ok(None);
    };
    let mut bytes = Vec::new();
    archive.by_index(index)?.read_to_end(&mut bytes)?;
    Recipe::parse(&bytes)
        .map(Some)
        .map_err(|e| PatchSummaryError::Recipe(e.to_string()))
}

/// `rel` as a client-root relative `/` path under `prefix` (empty for the
/// install directory). Rejects what the patch zip reader would reject.
fn client_path(prefix: &str, rel: &str) -> Result<String, PatchSummaryError> {
    let path = safe_relative(rel).map_err(|_| PatchSummaryError::Path(rel.to_string()))?;
    let parts: Vec<&str> = path.iter().filter_map(|p| p.to_str()).collect();
    let joined = parts.join("/");
    Ok(if prefix.is_empty() {
        joined
    } else {
        format!("{prefix}/{joined}")
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Lower-hex SHA-256 of everything `reader` yields.
fn hash_reader(reader: &mut impl Read) -> std::io::Result<String> {
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex(&hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::io::Write;
    use std::path::PathBuf;

    fn entry(id: &str, root: PatchRoot) -> PatchEntry {
        PatchEntry {
            id: id.into(),
            blob: "b".into(),
            size: 1,
            sha256: "h".into(),
            after: None,
            root,
            title: None,
            description: None,
        }
    }

    /// Writes a zip of `entries` into `dir` and returns its path.
    fn zip_file(dir: &Path, name: &str, entries: &[(&str, &[u8])]) -> PathBuf {
        let path = dir.join(name);
        let mut zw = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        for (entry, body) in entries {
            zw.start_file(*entry, zip::write::SimpleFileOptions::default())
                .unwrap();
            zw.write_all(body).unwrap();
        }
        zw.finish().unwrap();
        path
    }

    fn sha(bytes: &[u8]) -> String {
        hex(&Sha256::digest(bytes))
    }

    fn pinned(path: &str, sha256: &str) -> Pinned {
        Pinned {
            path: path.into(),
            sha256: sha256.into(),
            alternatives: Vec::new(),
        }
    }

    /// Three patches that each write `Data/x.bin`, with bodies `one`,
    /// `two` and `three`.
    fn three_patches(dir: &Path) -> Vec<PatchSummary> {
        let bodies: [&[u8]; 3] = [b"one", b"two", b"three"];
        bodies
            .iter()
            .enumerate()
            .map(|(i, body)| {
                let id = (i + 1).to_string();
                let zip = zip_file(dir, &format!("{id}.zip"), &[("Data/x.bin", *body)]);
                summarize(&zip, &entry(&id, PatchRoot::InstallDir)).unwrap()
            })
            .collect()
    }

    #[test]
    fn outputs_of_a_plain_zip_are_hashed_entries() {
        let dir = tempfile::tempdir().unwrap();
        let zip = zip_file(
            dir.path(),
            "p.zip",
            &[("Data/a.bin", b"aaa"), ("b.txt", b"bb")],
        );
        let s = summarize(&zip, &entry("p", PatchRoot::InstallDir)).unwrap();
        assert_eq!(s.state_key, "p");
        assert!(s.sources.is_empty());
        assert_eq!(
            s.outputs,
            vec![
                pinned("Data/a.bin", &sha(b"aaa")),
                pinned("b.txt", &sha(b"bb"))
            ]
        );
    }

    /// The op's source and target get the `sgw_game` prefix, its
    /// alternative's hash lands in `alternatives`, and the delta entries and
    /// recipe are not outputs.
    #[test]
    fn recipe_ops_give_sources_and_results() {
        const RECIPE: &str = r#"{"schema":1,"ops":[{"target":"Data/c.bin","sources":[{"path":"Data/a.bin","sha256":"AA"}],"delta":"deltas/c.bin","result_sha256":"CC","alternatives":[{"sources":[{"path":"data/A.bin","sha256":"AB"}],"delta":"deltas/c.alt"}]}]}"#;
        let dir = tempfile::tempdir().unwrap();
        let zip = zip_file(
            dir.path(),
            "r.zip",
            &[
                ("cimmeria-patch.json", RECIPE.as_bytes()),
                ("deltas/c.bin", b"d1"),
                ("deltas/c.alt", b"d2"),
                ("Data/extra.bin", b"x"),
            ],
        );
        let s = summarize(&zip, &entry("r", PatchRoot::SgwGame)).unwrap();
        assert_eq!(s.state_key, "r@sgw_game");
        assert_eq!(
            s.sources,
            vec![Pinned {
                path: "Working/SGWGame/Data/a.bin".into(),
                sha256: "aa".into(),
                alternatives: vec!["ab".into()],
            }]
        );
        assert_eq!(
            s.outputs,
            vec![
                pinned("Working/SGWGame/Data/c.bin", "cc"),
                pinned("Working/SGWGame/Data/extra.bin", &sha(b"x")),
            ]
        );
    }

    #[test]
    fn a_stock_folder_has_every_patch_pending() {
        let dir = tempfile::tempdir().unwrap();
        let summaries = three_patches(dir.path());
        let stock: HashMap<String, String> =
            HashMap::from([("Data/x.bin".to_string(), sha(b"stock"))]);
        let have = |p: &str| stock.get(p).cloned();
        assert_eq!(pending(&summaries, have), vec![0, 1, 2]);
    }

    #[test]
    fn a_folder_at_patch_2_of_3_has_only_the_last_pending() {
        let dir = tempfile::tempdir().unwrap();
        let summaries = three_patches(dir.path());
        let at_two: HashMap<String, String> =
            HashMap::from([("Data/x.bin".to_string(), sha(b"two"))]);
        let have = |p: &str| at_two.get(p).cloned();
        assert_eq!(pending(&summaries, have), vec![2]);
    }

    /// A folder holding patch 1's output for a path that patch 2 rewrote
    /// is still current for patch 1.
    #[test]
    fn a_later_output_counts_as_current() {
        let dir = tempfile::tempdir().unwrap();
        let summaries = three_patches(dir.path());
        let at_two: HashMap<String, String> =
            HashMap::from([("Data/x.bin".to_string(), sha(b"two"))]);
        let have = |p: &str| at_two.get(p).cloned();
        assert!(!pending(&summaries[..2], have).contains(&0));
    }
}
