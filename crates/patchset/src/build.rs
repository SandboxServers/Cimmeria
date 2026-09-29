//! Build a patch zip from a spec, a stock client and a patched client.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use zip::write::SimpleFileOptions;

use crate::case_path::resolve_existing_case;
use crate::recipe::{safe_relative, Op, Recipe, Source, Transform, RECIPE_NAME, RECIPE_SCHEMA};
use crate::{io_err, sha256_hex, transform, PatchsetError, Result};

/// A patch spec, committed next to the files it ships
/// (`data/client-patches/<id>/patch.json`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    pub id: String,
    /// Short name for the launcher's "Changes to your client" list.
    /// Printed into the manifest entry; not part of the zip.
    #[serde(default)]
    pub title: Option<String>,
    /// What the patch changes, in a sentence or two, for the same list.
    #[serde(default)]
    pub description: Option<String>,
    /// Files rebuilt from the stock client by delta.
    #[serde(default)]
    pub ops: Vec<SpecOp>,
    /// Files shipped whole. Only for content that is entirely ours.
    #[serde(default)]
    pub files: Vec<SpecFile>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpecOp {
    /// Install-relative path of the patched file.
    pub target: String,
    pub sources: Vec<SpecSource>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpecSource {
    pub path: String,
    #[serde(default)]
    pub transform: Transform,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpecFile {
    /// Install-relative destination.
    pub path: String,
    /// Relative to the spec's directory.
    pub from: String,
}

/// What [`build`] produced, for the tool's summary line.
#[derive(Debug, Clone)]
pub struct BuildReport {
    pub zip: Vec<u8>,
    /// Per op: target, delta size, target size.
    pub deltas: Vec<(String, usize, usize)>,
}

impl Spec {
    pub fn load(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path).map_err(io_err(path))?;
        Ok(serde_json::from_slice(&bytes)?)
    }
}

/// Build the patch zip for `spec`. Sources are read from `stock_root`,
/// targets from `patched_root`, and overlay files from `spec_dir`.
///
/// Every spec path must be spelled as the stock client spells whatever
/// part of it exists there ([`PatchsetError::CaseMismatch`]). The game
/// looks some files up case-sensitively, and a recipe spelled `eula.lua`
/// for the stock `EULA.lua` is how 005-login-delay shipped.
///
/// The zip is deterministic: entries in a fixed order, stored, with a fixed
/// timestamp, so rebuilding an unchanged patch gives the same bytes and the
/// same manifest hash.
pub fn build(
    spec: &Spec,
    spec_dir: &Path,
    stock_root: &Path,
    patched_root: &Path,
) -> Result<BuildReport> {
    let mut recipe = Recipe {
        schema: RECIPE_SCHEMA,
        ops: Vec::new(),
    };
    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
    let mut deltas = Vec::new();

    for op in &spec.ops {
        check_stock_case(stock_root, &op.target)?;
        for s in &op.sources {
            check_stock_case(stock_root, &s.path)?;
        }
    }
    for f in &spec.files {
        check_stock_case(stock_root, &f.path)?;
    }

    for (n, op) in spec.ops.iter().enumerate() {
        let mut image = Vec::new();
        let mut sources = Vec::new();
        for s in &op.sources {
            let path = stock_root.join(safe_relative(&s.path)?);
            let (bytes, sha256) = transform::load(&path, s.transform)?;
            image.extend_from_slice(&bytes);
            sources.push(Source {
                path: s.path.clone(),
                sha256,
                transform: s.transform,
            });
        }
        let target_path = patched_root.join(safe_relative(&op.target)?);
        let target = std::fs::read(&target_path).map_err(io_err(&target_path))?;
        let delta = bsdiff(&image, &target).map_err(|detail| PatchsetError::Delta {
            target: op.target.clone(),
            detail,
        })?;
        // Prove the delta before shipping it.
        let rebuilt =
            crate::apply::bspatch(&image, &delta).map_err(|detail| PatchsetError::Delta {
                target: op.target.clone(),
                detail,
            })?;
        if rebuilt != target {
            return Err(PatchsetError::Delta {
                target: op.target.clone(),
                detail: "delta does not reproduce the target".into(),
            });
        }
        let delta_name = format!("deltas/{n:03}.bsdiff");
        deltas.push((op.target.clone(), delta.len(), target.len()));
        recipe.ops.push(Op {
            target: op.target.clone(),
            sources,
            delta: delta_name.clone(),
            result_sha256: sha256_hex(&target),
        });
        entries.push((delta_name, delta));
    }

    for f in &spec.files {
        let rel = safe_relative(&f.path)?;
        let from: PathBuf = spec_dir.join(&f.from);
        let bytes = std::fs::read(&from).map_err(io_err(&from))?;
        entries.push((rel.to_string_lossy().replace('\\', "/"), bytes));
    }

    let mut zip_bytes = Vec::new();
    {
        let mut zw = zip::ZipWriter::new(std::io::Cursor::new(&mut zip_bytes));
        let opts = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .last_modified_time(zip::DateTime::default());
        if !recipe.ops.is_empty() {
            zw.start_file(RECIPE_NAME, opts)?;
            let json = serde_json::to_vec_pretty(&recipe)?;
            zw.write_all(&json)
                .map_err(io_err(Path::new(RECIPE_NAME)))?;
        }
        for (name, bytes) in &entries {
            zw.start_file(name.as_str(), opts)?;
            zw.write_all(bytes).map_err(io_err(Path::new(name)))?;
        }
        zw.finish()?;
    }
    Ok(BuildReport {
        zip: zip_bytes,
        deltas,
    })
}

/// Refuse a spec path whose existing part the stock tree spells in
/// another case.
fn check_stock_case(stock_root: &Path, spec_path: &str) -> Result<()> {
    let rel = safe_relative(spec_path)?;
    let spelled = stock_root.join(&rel);
    let on_disk = resolve_existing_case(stock_root, &rel);
    // Path equality compares components byte for byte, so case counts.
    if on_disk == spelled {
        return Ok(());
    }
    let on_disk = on_disk
        .strip_prefix(stock_root)
        .unwrap_or(&on_disk)
        .to_string_lossy()
        .replace('\\', "/");
    Err(PatchsetError::CaseMismatch {
        path: spec_path.to_string(),
        on_disk,
    })
}

fn bsdiff(source: &[u8], target: &[u8]) -> std::result::Result<Vec<u8>, String> {
    let mut delta = Vec::new();
    qbsdiff::Bsdiff::new(source, target)
        .compare(std::io::Cursor::new(&mut delta))
        .map_err(|e| e.to_string())?;
    Ok(delta)
}
