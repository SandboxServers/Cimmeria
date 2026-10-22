//! Apply a patch zip to an install.

use std::collections::BTreeSet;
use std::io::Read;
use std::path::Path;

use crate::case_path::resolve_existing_case;
use crate::recipe::{safe_relative, Op, Recipe, Source, DELTA_DIR, RECIPE_NAME};
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
    /// Ops skipped under [`ApplyOptions::tolerate_ops`]. Always empty otherwise.
    pub skipped: Vec<SkippedOp>,
}

/// How [`apply_with`] treats the ops it is given. The default is what
/// [`apply`] does.
#[derive(Debug, Clone, Default)]
pub struct ApplyOptions {
    /// Skip an op that cannot apply and record it in
    /// [`ApplyReport::skipped`], instead of failing the whole apply.
    pub tolerate_ops: bool,
    /// Apply only the ops on these targets. The set holds lower-case `/`
    /// paths relative to the apply dir. `Some` also writes no overlay files.
    pub only_targets: Option<BTreeSet<String>>,
}

/// Why an op was skipped under [`ApplyOptions::tolerate_ops`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    /// A source file is absent from the install.
    SourceMissing,
    /// A source file is not the pinned bytes (or the patch output it builds on).
    SourceMismatch,
    /// The bsdiff delta did not apply to its sources.
    DeltaFailed,
    /// The rebuilt bytes do not have the recipe's result hash.
    ResultMismatch,
}

impl SkipReason {
    /// Stable machine code for this reason.
    pub fn code(self) -> &'static str {
        match self {
            Self::SourceMissing => "source_missing",
            Self::SourceMismatch => "source_mismatch",
            Self::DeltaFailed => "delta_failed",
            Self::ResultMismatch => "result_mismatch",
        }
    }
}

/// An op that [`apply_with`] skipped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedOp {
    /// The target as the recipe writes it.
    pub target: String,
    /// Why the op was skipped.
    pub reason: SkipReason,
}

/// The skip reason for a source error, when a tolerant apply may skip it.
fn skip_reason_for(error: &PatchsetError) -> Option<SkipReason> {
    match error {
        PatchsetError::SourceMissing { .. } => Some(SkipReason::SourceMissing),
        PatchsetError::SourceMismatch { .. } | PatchsetError::PatchOutputMismatch { .. } => {
            Some(SkipReason::SourceMismatch)
        }
        _ => None,
    }
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
///
/// Every path is resolved against the install's own spelling first
/// ([`resolve_existing_case`]): a file that exists keeps its on-disk name
/// whatever case the recipe spells it in, and only a new file takes the
/// recipe's spelling. The published `005-login-delay` recipe names
/// `eula.lua`; writing through the rename under that name turned the stock
/// `EULA.lua` into `eula.lua`, which the game's UI loader cannot find, so
/// the login screen never appeared.
pub fn apply(
    zip_path: &Path,
    install_dir: &Path,
    progress: &mut dyn FnMut(&str),
) -> Result<ApplyReport> {
    apply_with(zip_path, install_dir, &ApplyOptions::default(), progress)
}

/// [`apply`] with `opts`. The default options give exactly [`apply`]'s
/// behaviour. With `tolerate_ops`, an op whose source or delta fails is
/// recorded in [`ApplyReport::skipped`] and the rest still apply. With
/// `only_targets`, ops on other targets are ignored and no overlays are
/// written.
pub fn apply_with(
    zip_path: &Path,
    install_dir: &Path,
    opts: &ApplyOptions,
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
        if let Some(only) = &opts.only_targets {
            if !only.contains(&op.target.to_lowercase()) {
                continue;
            }
        }
        let target = resolve_existing_case(install_dir, &safe_relative(&op.target)?);
        if let Ok(current) = std::fs::read(&target) {
            if sha256_hex(&current) == op.result_sha256 {
                report.already_current.push(op.target.clone());
                continue;
            }
        }
        // The primary starting point first; when its sources are not what
        // the install holds, the first alternative whose sources are. With
        // none matching, the primary's error is the one worth reporting.
        let mut chosen = None;
        let mut first_error = None;
        let starts = std::iter::once((&op.sources, &op.delta))
            .chain(op.alternatives.iter().map(|a| (&a.sources, &a.delta)));
        for (sources, delta) in starts {
            match load_sources(install_dir, sources) {
                Ok(image) => {
                    chosen = Some((image, delta));
                    break;
                }
                Err(e) => {
                    first_error.get_or_insert(e);
                }
            }
        }
        let Some((source_image, delta_name)) = chosen else {
            let error = first_error.expect("an op always has a primary start");
            match skip_reason_for(&error) {
                Some(reason) if opts.tolerate_ops => {
                    report.skipped.push(SkippedOp {
                        target: op.target.clone(),
                        reason,
                    });
                    continue;
                }
                _ => return Err(error),
            }
        };
        let delta_index = archive.index_for_name(delta_name).ok_or_else(|| {
            PatchsetError::Invalid(format!("recipe names missing delta {delta_name}"))
        })?;
        let delta = read_entry(&mut archive, delta_index)?;
        let result = match bspatch(&source_image, &delta) {
            Ok(result) => result,
            Err(_) if opts.tolerate_ops => {
                report.skipped.push(SkippedOp {
                    target: op.target.clone(),
                    reason: SkipReason::DeltaFailed,
                });
                continue;
            }
            Err(detail) => {
                return Err(PatchsetError::Delta {
                    target: op.target.clone(),
                    detail,
                })
            }
        };
        let actual = sha256_hex(&result);
        if actual != op.result_sha256 {
            if opts.tolerate_ops {
                report.skipped.push(SkippedOp {
                    target: op.target.clone(),
                    reason: SkipReason::ResultMismatch,
                });
                continue;
            }
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
            .any(|op| op_sources(op).any(|s| s.path == name))
    };
    pending.sort_by_key(|(_, _, name)| is_source(name));
    for (target, bytes, name) in pending {
        write_atomic(&target, &bytes)?;
        progress(&name);
        report.rebuilt.push(name);
    }

    // A targeted apply writes its ops only; overlays belong to the full set.
    if opts.only_targets.is_some() {
        return Ok(report);
    }
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name()?.into_owned();
        if name == RECIPE_NAME || name.starts_with(DELTA_DIR) || entry.is_dir() {
            continue;
        }
        let rel = entry
            .enclosed_name()
            .ok_or_else(|| PatchsetError::UnsafePath(name.clone()))?;
        let out = resolve_existing_case(install_dir, &rel);
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).map_err(io_err(&out))?;
        write_atomic(&out, &bytes)?;
        progress(&name);
        report.overlay_files += 1;
    }
    Ok(report)
}

/// Every source of an op, across its primary start and its alternatives.
fn op_sources(op: &Op) -> impl Iterator<Item = &Source> {
    op.sources
        .iter()
        .chain(op.alternatives.iter().flat_map(|a| &a.sources))
}

/// Read, normalize and hash-check `sources`, and return their concatenation:
/// the image a delta applies to.
fn load_sources(install_dir: &Path, sources: &[Source]) -> Result<Vec<u8>> {
    let mut image = Vec::new();
    for s in sources {
        let path = resolve_existing_case(install_dir, &safe_relative(&s.path)?);
        if !path.is_file() {
            return Err(PatchsetError::SourceMissing {
                path: s.path.clone(),
            });
        }
        // One read: the hash covers the exact bytes used (for
        // `UpkNormalize` see `transform::load`), and is checked before the
        // transform runs so a file that is not the pinned one is never decoded.
        let bytes = match transform::load_if(&path, &s.transform, |sha| sha == s.sha256)? {
            Ok(bytes) => bytes,
            Err(raw_sha) => {
                if let Some(patch) = &s.output_of {
                    return Err(PatchsetError::PatchOutputMismatch {
                        path: s.path.clone(),
                        patch: patch.clone(),
                        expected: s.sha256.clone(),
                        actual: raw_sha,
                    });
                }
                return Err(PatchsetError::SourceMismatch {
                    path: s.path.clone(),
                    expected: s.sha256.clone(),
                    actual: raw_sha,
                });
            }
        };
        image.extend_from_slice(&bytes);
    }
    Ok(image)
}

fn read_entry(archive: &mut zip::ZipArchive<std::fs::File>, index: usize) -> Result<Vec<u8>> {
    let mut entry = archive.by_index(index)?;
    let mut bytes = Vec::new();
    entry
        .read_to_end(&mut bytes)
        .map_err(|source| PatchsetError::Io {
            // zip 9 made name() fallible; this is already an error path.
            path: String::from_utf8_lossy(entry.name_raw()).into_owned(),
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
