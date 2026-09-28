//! Apply a patch zip to an install.

use std::io::Read;
use std::path::Path;

use crate::recipe::{safe_relative, Recipe, DELTA_DIR, RECIPE_NAME};
use crate::{io_err, sha256_hex, transform, PatchsetError, Result};

/// What [`apply`] did.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ApplyReport {
    /// Targets rebuilt from their sources.
    pub rebuilt: Vec<String>,
    /// Targets that already had the result hash.
    pub already_current: Vec<String>,
    /// Overlay files written.
    pub overlay_files: usize,
}

/// Whether `zip_path` carries a recipe.
pub fn has_recipe(zip_path: &Path) -> Result<bool> {
    let file = std::fs::File::open(zip_path).map_err(io_err(zip_path))?;
    let archive = zip::ZipArchive::new(file)?;
    Ok(archive.index_for_name(RECIPE_NAME).is_some())
}

/// Apply the patch zip at `zip_path` to the install rooted at
/// `install_dir`. `progress` gets each target or overlay path as it is
/// written.
///
/// Every op is computed before anything is written, so an op may use a file
/// another op replaces as its source, and a failure (a source that isn't
/// stock, a bad delta) leaves the install untouched. Overlay files go last.
pub fn apply(
    zip_path: &Path,
    install_dir: &Path,
    progress: &mut dyn FnMut(&str),
) -> Result<ApplyReport> {
    let file = std::fs::File::open(zip_path).map_err(io_err(zip_path))?;
    let mut archive = zip::ZipArchive::new(file)?;
    let recipe = match archive.index_for_name(RECIPE_NAME) {
        Some(i) => Recipe::parse(&read_entry(&mut archive, i)?)?,
        None => Recipe {
            schema: crate::recipe::RECIPE_SCHEMA,
            ops: Vec::new(),
        },
    };

    let mut report = ApplyReport::default();
    let mut pending: Vec<(std::path::PathBuf, Vec<u8>, String)> = Vec::new();
    for op in &recipe.ops {
        let target = install_dir.join(safe_relative(&op.target)?);
        if let Ok(current) = std::fs::read(&target) {
            if sha256_hex(&current) == op.result_sha256 {
                report.already_current.push(op.target.clone());
                continue;
            }
        }
        let mut source_image = Vec::new();
        for s in &op.sources {
            let path = install_dir.join(safe_relative(&s.path)?);
            if !path.is_file() {
                return Err(PatchsetError::SourceMissing {
                    path: s.path.clone(),
                });
            }
            // One read: the hash covers the exact bytes used (for
            // `UpkNormalize` see `transform::load`).
            let (bytes, raw_sha) = transform::load(&path, s.transform)?;
            if raw_sha != s.sha256 {
                return Err(PatchsetError::SourceMismatch {
                    path: s.path.clone(),
                    expected: s.sha256.clone(),
                    actual: raw_sha,
                });
            }
            source_image.extend_from_slice(&bytes);
        }
        let delta_index = archive.index_for_name(&op.delta).ok_or_else(|| {
            PatchsetError::Invalid(format!("recipe names missing delta {}", op.delta))
        })?;
        let delta = read_entry(&mut archive, delta_index)?;
        let result = bspatch(&source_image, &delta).map_err(|detail| PatchsetError::Delta {
            target: op.target.clone(),
            detail,
        })?;
        let actual = sha256_hex(&result);
        if actual != op.result_sha256 {
            return Err(PatchsetError::ResultMismatch {
                path: op.target.clone(),
                expected: op.result_sha256.clone(),
                actual,
            });
        }
        pending.push((target, result, op.target.clone()));
    }

    // Write targets that no op reads as a source first, so a crash between
    // two writes never leaves a source already replaced while an op that
    // reads it is still pending (a re-run would then refuse the source).
    // An op always reads its own target, so only other ops count.
    let is_source = |name: &str| {
        recipe
            .ops
            .iter()
            .filter(|op| op.target != name)
            .any(|op| op.sources.iter().any(|s| s.path == name))
    };
    pending.sort_by_key(|(_, _, name)| is_source(name));
    for (target, bytes, name) in pending {
        write_atomic(&target, &bytes)?;
        progress(&name);
        report.rebuilt.push(name);
    }

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        if name == RECIPE_NAME || name.starts_with(DELTA_DIR) || entry.is_dir() {
            continue;
        }
        let rel = entry
            .enclosed_name()
            .ok_or_else(|| PatchsetError::UnsafePath(name.clone()))?;
        let out = install_dir.join(rel);
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).map_err(io_err(&out))?;
        write_atomic(&out, &bytes)?;
        progress(&name);
        report.overlay_files += 1;
    }
    Ok(report)
}

fn read_entry(archive: &mut zip::ZipArchive<std::fs::File>, index: usize) -> Result<Vec<u8>> {
    let mut entry = archive.by_index(index)?;
    let mut bytes = Vec::new();
    entry
        .read_to_end(&mut bytes)
        .map_err(|source| PatchsetError::Io {
            path: entry.name().to_string(),
            source,
        })?;
    Ok(bytes)
}

/// Largest up-front allocation a delta header may ask for. The header's
/// target size is only a hint read from the download; a corrupt one must
/// not abort the launcher. The output still grows past this as needed.
const MAX_PREALLOC: u64 = 64 * 1024 * 1024;

pub(crate) fn bspatch(source: &[u8], delta: &[u8]) -> std::result::Result<Vec<u8>, String> {
    let patcher = qbsdiff::Bspatch::new(delta).map_err(|e| e.to_string())?;
    let hint = patcher.hint_target_size().min(MAX_PREALLOC);
    let mut out = Vec::with_capacity(usize::try_from(hint).unwrap_or(0));
    patcher
        .apply(source, std::io::Cursor::new(&mut out))
        .map_err(|e| e.to_string())?;
    Ok(out)
}

/// Write via a sibling temp file and rename, so a crash never leaves a
/// half-written game file.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(io_err(parent))?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".patching");
    let tmp = std::path::PathBuf::from(tmp);
    std::fs::write(&tmp, bytes).map_err(io_err(&tmp))?;
    std::fs::rename(&tmp, path).map_err(io_err(path))
}
