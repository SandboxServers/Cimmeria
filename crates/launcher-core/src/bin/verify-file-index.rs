//! Checks a candidate manifest and its file index before anyone signs and
//! uploads them (LT-35). Prints `PASS <check>` or `FAIL <check>: <detail>` per
//! check and exits 1 on any FAIL. It never signs, uploads or reads a key.
//!
//! ```text
//! verify-file-index --manifest candidate.json --file-index file-index.json \
//!     [--previous current.json] [--seed seed.zip --patch <id>=<zip> ...]
//! ```
//!
//! With `--seed`, the index is built as `build-file-index` builds it (parity).

use cimmeria_launcher_core::file_index::{self, check};
use cimmeria_launcher_core::install;
use cimmeria_launcher_core::install_progress::ProgressSink;
use cimmeria_launcher_core::manifest::{Manifest, PatchEntry};
use cimmeria_launcher_core::unpack::UnpackSink;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use tokio_util::sync::CancellationToken;

#[derive(Debug, Default)]
struct Args {
    manifest: Option<PathBuf>,
    file_index: Option<PathBuf>,
    previous: Option<PathBuf>,
    seed: Option<PathBuf>,
    patches: Vec<(String, PathBuf)>,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args::default();
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut value = || it.next().ok_or(format!("{flag} needs a value"));
        match flag.as_str() {
            "--manifest" => args.manifest = Some(value()?.into()),
            "--file-index" => args.file_index = Some(value()?.into()),
            "--previous" => args.previous = Some(value()?.into()),
            "--seed" => args.seed = Some(value()?.into()),
            "--patch" => {
                let v = value()?;
                let (id, zip) = v
                    .split_once('=')
                    .ok_or(format!("--patch takes <id>=<zip>, got {v}"))?;
                args.patches.push((id.to_string(), zip.into()));
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(args)
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))
}

/// Refuse `path` unless its size and sha256 are the manifest's.
fn check_artifact(path: &Path, size: u64, sha256: &str) -> Result<(), String> {
    let name = path.display();
    let len = std::fs::metadata(path)
        .map_err(|e| format!("{name}: {e}"))?
        .len();
    if len != size {
        return Err(format!("{name}: {len} bytes, the manifest says {size}"));
    }
    let actual = install::hash_file(path).map_err(|e| format!("{name}: {e}"))?;
    if !actual.eq_ignore_ascii_case(sha256) {
        return Err(format!(
            "{name}: sha256 {actual} does not match the manifest's {sha256}"
        ));
    }
    Ok(())
}

/// The index `build-file-index` makes for `manifest` from the same artifacts.
fn build_from_artifacts(
    manifest: &Manifest,
    seed: &Path,
    patches: &[(String, PathBuf)],
) -> Result<file_index::FileIndex, String> {
    check_artifact(seed, manifest.seed.size, &manifest.seed.sha256)?;
    if patches.len() != manifest.patches.len() {
        return Err(format!(
            "{} --patch argument(s) given, the manifest lists {} patch(es)",
            patches.len(),
            manifest.patches.len()
        ));
    }
    // The manifest's signed order, each patch with its zip.
    let mut layers: Vec<(PatchEntry, PathBuf)> = Vec::with_capacity(manifest.patches.len());
    for entry in &manifest.patches {
        let (_, zip) = patches
            .iter()
            .find(|(id, _)| *id == entry.id)
            .ok_or(format!("no --patch given for {}", entry.id))?;
        check_artifact(zip, entry.size, &entry.sha256)?;
        layers.push((entry.clone(), zip.clone()));
    }

    let work = tempfile::tempdir().map_err(|e| e.to_string())?;
    let sink = UnpackSink {
        progress: ProgressSink::latest().0,
        label: "file index".to_string(),
        cancel: CancellationToken::new(),
    };
    file_index::build(seed, &layers, manifest, work.path(), &sink).map_err(|e| e.to_string())
}

/// Runs every check and prints one line each. `Ok(false)` when any failed.
fn run() -> Result<bool, String> {
    let args = parse_args()?;
    let manifest_path = args.manifest.ok_or("--manifest is required")?;
    let index_path = args.file_index.ok_or("--file-index is required")?;
    let candidate = read(&manifest_path)?;
    let index = read(&index_path)?;
    let previous = args.previous.as_deref().map(read).transpose()?;
    let built = match &args.seed {
        Some(seed) => {
            let manifest: Manifest = serde_json::from_slice(&candidate)
                .map_err(|e| format!("candidate manifest: {e}"))?;
            Some(build_from_artifacts(&manifest, seed, &args.patches)?)
        }
        None => None,
    };
    let findings =
        check::check_publication(&candidate, &index, previous.as_deref(), built.as_ref());
    let mut all_ok = true;
    for finding in &findings {
        if finding.ok {
            println!("PASS {}", finding.check);
        } else {
            all_ok = false;
            println!("FAIL {}: {}", finding.check, finding.detail);
        }
    }
    Ok(all_ok)
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("verify-file-index: {e}");
            ExitCode::FAILURE
        }
    }
}
