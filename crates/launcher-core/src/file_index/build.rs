//! Builds a [`FileIndex`] from the local artifacts, state by state. It follows
//! the engine's adoption reference (`storage/adoption/reference.rs`): seed,
//! then each signed patch in order, then the setup rules. Nothing is
//! downloaded or uploaded; the caller has already checked the artifacts.
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use super::{
    FileIndex, FileIndexError, IndexedFile, PatchLayer, SeedLayer, SetupRule, FILE_INDEX_SCHEMA,
    SETUP_POLICY_VERSION,
};
use crate::client_setup::{aslr, login_servers, stock_case};
use crate::install_layout;
use crate::manifest::{Manifest, PatchEntry};
use crate::patch_dest::patch_dest;
use crate::unpack::{self, UnpackSink};

/// The launcher's own state file. It is never part of a release.
const LAUNCHER_STATE_FILE: &str = "launcher-installed.json";

/// One file as the scan sees it: its stock-case path and its digest.
pub(super) struct Scanned {
    path: String,
    size: u64,
    sha256: String,
}

/// Build the index for `manifest` from the `seed` archive and the
/// `patches` (each with its zip, in signed order). The tree is unpacked
/// under `work/tree`, which is replaced if it exists. The caller checks the
/// artifacts against the manifest first.
pub fn build(
    seed: &Path,
    patches: &[(PatchEntry, PathBuf)],
    manifest: &Manifest,
    work: &Path,
    sink: &UnpackSink,
) -> Result<FileIndex, FileIndexError> {
    let tree = work.join("tree");
    if tree.exists() {
        std::fs::remove_dir_all(&tree).map_err(io)?;
    }
    std::fs::create_dir_all(&tree).map_err(io)?;
    unpack::unpack(seed, &tree, sink).map_err(|e| FileIndexError::Io(e.to_string()))?;
    install_layout::place_bundled_cooked_data(&tree).map_err(io)?;
    stock_case::restore(&tree).map_err(io)?;
    // Each state's scan is kept and becomes the next patch's `before`, so the
    // tree is hashed once per state, not twice per patch.
    let mut previous = scan(&tree, sink)?;
    let seed = SeedLayer {
        sha256: manifest.seed.sha256.to_ascii_lowercase(),
        files: previous.values().map(indexed).collect(),
    };

    let mut layers = Vec::with_capacity(patches.len());
    for (entry, zip) in patches {
        let dest = patch_dest(&tree, entry).map_err(|e| FileIndexError::Binding(e.to_string()))?;
        let prefix = relative(&dest, &tree)?;
        let inputs = recipe_sources(zip, &tree, &prefix, sink)?;
        unpack::unpack(zip, &dest, sink).map_err(|e| FileIndexError::Io(e.to_string()))?;
        stock_case::restore(&tree).map_err(io)?;
        let after = scan(&tree, sink)?;
        let outputs = after
            .iter()
            .filter(|(key, now)| {
                previous
                    .get(*key)
                    .is_none_or(|was| was.size != now.size || was.sha256 != now.sha256)
            })
            .map(|(_, now)| indexed(now))
            .collect();
        layers.push(PatchLayer {
            state_key: entry.state_key(),
            sha256: entry.sha256.to_ascii_lowercase(),
            inputs,
            outputs,
        });
        previous = after;
    }

    let exe = install_layout::sgw_exe(&tree);
    let copy = work.join("SGW.exe.aslr");
    std::fs::copy(&exe, &copy).map_err(io)?;
    aslr::disable(&copy).map_err(io)?;
    let (size, sha256) = digest(&copy, sink)?;
    let setup = vec![
        SetupRule::AslrOff {
            path: relative(&exe, &tree)?,
            size,
            sha256,
        },
        SetupRule::LoginServers {
            path: relative(&login_servers::path(&tree), &tree)?,
        },
    ];

    let index = FileIndex {
        schema_version: FILE_INDEX_SCHEMA,
        setup_policy_version: SETUP_POLICY_VERSION,
        seed,
        patches: layers,
        setup,
    };
    index.validate()?;
    Ok(index)
}

/// The op sources of the patch set at `zip`, read at their current bytes in
/// `tree`. A patch set's sources are relative to its own destination, so
/// `prefix` maps them to the client root. A zip without a recipe has none.
/// Each path is listed once.
fn recipe_sources(
    zip: &Path,
    tree: &Path,
    prefix: &str,
    sink: &UnpackSink,
) -> Result<Vec<IndexedFile>, FileIndexError> {
    let file = std::fs::File::open(zip).map_err(io)?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| FileIndexError::Io(e.to_string()))?;
    let Some(index) = archive.index_for_name(cimmeria_patchset::RECIPE_NAME) else {
        return Ok(Vec::new());
    };
    let mut bytes = Vec::new();
    archive
        .by_index(index)
        .and_then(|mut entry| {
            entry
                .read_to_end(&mut bytes)
                .map_err(zip::result::ZipError::Io)
        })
        .map_err(|e| FileIndexError::Io(e.to_string()))?;
    let recipe =
        cimmeria_patchset::Recipe::parse(&bytes).map_err(|e| FileIndexError::Io(e.to_string()))?;

    let mut seen = std::collections::HashSet::new();
    let mut inputs = Vec::new();
    for op in &recipe.ops {
        for source in &op.sources {
            let rel = cimmeria_patchset::recipe::safe_relative(&source.path)
                .map_err(|e| FileIndexError::Io(e.to_string()))?;
            let rel = if prefix.is_empty() {
                rel
            } else {
                Path::new(prefix).join(rel)
            };
            let on_disk = cimmeria_patchset::resolve_existing_case(tree, &rel);
            let path = relative(&on_disk, tree)?;
            if !seen.insert(path.to_ascii_lowercase()) {
                continue;
            }
            let (size, sha256) = digest(&on_disk, sink)?;
            inputs.push(IndexedFile { path, size, sha256 });
        }
    }
    Ok(inputs)
}

/// Every regular file under `root`, keyed by lower-cased path, except the
/// launcher's state file at the root. Checks the cancel switch per file.
pub(super) fn scan(
    root: &Path,
    sink: &UnpackSink,
) -> Result<BTreeMap<String, Scanned>, FileIndexError> {
    let mut out = BTreeMap::new();
    for entry in WalkDir::new(root).sort_by_file_name() {
        check_cancel(sink)?;
        let entry = entry.map_err(|e| FileIndexError::Io(e.to_string()))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = relative(entry.path(), root)?;
        if path.eq_ignore_ascii_case(LAUNCHER_STATE_FILE) {
            continue;
        }
        let (size, sha256) = digest(entry.path(), sink)?;
        out.insert(path.to_ascii_lowercase(), Scanned { path, size, sha256 });
    }
    Ok(out)
}

/// The cancel switch, as the unpack step reports it: a cancelled build ends
/// with the same `Cancelled` text an unpack gives.
fn check_cancel(sink: &UnpackSink) -> Result<(), FileIndexError> {
    if sink.cancel.is_cancelled() {
        return Err(FileIndexError::Io(
            unpack::UnpackError::Cancelled.to_string(),
        ));
    }
    Ok(())
}

/// `path` below `root` as a `/`-separated string. Empty when they are equal.
fn relative(path: &Path, root: &Path) -> Result<String, FileIndexError> {
    let rel = path
        .strip_prefix(root)
        .map_err(|_| FileIndexError::Binding(format!("{} is outside the tree", path.display())))?;
    let parts: Option<Vec<&str>> = rel.iter().map(|part| part.to_str()).collect();
    let parts = parts.ok_or_else(|| FileIndexError::InvalidPath(path.display().to_string()))?;
    Ok(parts.join("/"))
}

fn indexed(file: &Scanned) -> IndexedFile {
    IndexedFile {
        path: file.path.clone(),
        size: file.size,
        sha256: file.sha256.clone(),
    }
}

/// Size and lower-case hex SHA-256 of the file at `path`. Checks the cancel
/// switch per read, so a large file stops promptly too.
fn digest(path: &Path, sink: &UnpackSink) -> Result<(u64, String), FileIndexError> {
    let mut file = std::fs::File::open(path).map_err(io)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    let mut size = 0u64;
    loop {
        check_cancel(sink)?;
        let n = file.read(&mut buf).map_err(io)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        size += n as u64;
    }
    let sha256 = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Ok((size, sha256))
}

fn io(e: std::io::Error) -> FileIndexError {
    FileIndexError::Io(e.to_string())
}
