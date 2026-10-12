//! Builds the signed release's per-file index (file-index.json) from the manifest, seed and patch artifacts; prints the manifest file_index entry.
//!
//! ```text
//! build-file-index --manifest manifest.json --seed seed.zip \
//!     --patch <id>=<zip> [--patch <id>=<zip> ...] --out file-index.json
//! ```
//!
//! Every artifact is checked against the manifest's size and sha256 first;
//! a mismatch refuses the run and names the file. The index is built in a
//! temporary work directory and written pretty-printed to `--out`. The tool
//! prints the manifest `file_index` entry with a placeholder blob name: the
//! operator uploads the file, fills in the blob URL, then signs the manifest
//! (docs/client/launcher-distribution-setup.md). It never uploads anything.

use cimmeria_launcher_core::file_index;
use cimmeria_launcher_core::install;
use cimmeria_launcher_core::install_progress::ProgressSink;
use cimmeria_launcher_core::manifest;
use cimmeria_launcher_core::unpack::UnpackSink;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use tokio_util::sync::CancellationToken;

#[derive(Debug, Default)]
struct Args {
    manifest: Option<PathBuf>,
    seed: Option<PathBuf>,
    patches: Vec<(String, PathBuf)>,
    out: Option<PathBuf>,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args::default();
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut value = || it.next().ok_or(format!("{flag} needs a value"));
        match flag.as_str() {
            "--manifest" => args.manifest = Some(value()?.into()),
            "--seed" => args.seed = Some(value()?.into()),
            "--patch" => {
                let v = value()?;
                let (id, zip) = v
                    .split_once('=')
                    .ok_or(format!("--patch takes <id>=<zip>, got {v}"))?;
                args.patches.push((id.to_string(), zip.into()));
            }
            "--out" => args.out = Some(value()?.into()),
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(args)
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
    if actual != sha256 {
        return Err(format!(
            "{name}: sha256 {actual} does not match the manifest's {sha256}"
        ));
    }
    Ok(())
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let manifest_path = args.manifest.ok_or("--manifest is required")?;
    let seed = args.seed.ok_or("--seed is required")?;
    let out = args.out.ok_or("--out is required")?;

    let text = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("{}: {e}", manifest_path.display()))?;
    let manifest: manifest::Manifest = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    manifest.validate().map_err(|e| e.to_string())?;
    check_artifact(&seed, manifest.seed.size, &manifest.seed.sha256)?;

    if args.patches.len() != manifest.patches.len() {
        return Err(format!(
            "{} --patch argument(s) given, the manifest lists {} patch(es)",
            args.patches.len(),
            manifest.patches.len()
        ));
    }
    // In the manifest's signed order, each patch with its zip.
    let mut patches = Vec::with_capacity(manifest.patches.len());
    for entry in &manifest.patches {
        let (_, zip) = args
            .patches
            .iter()
            .find(|(id, _)| *id == entry.id)
            .ok_or(format!("no --patch given for {}", entry.id))?;
        check_artifact(zip, entry.size, &entry.sha256)?;
        patches.push((entry.clone(), zip.clone()));
    }

    let work = tempfile::tempdir().map_err(|e| e.to_string())?;
    let sink = UnpackSink {
        progress: ProgressSink::latest().0,
        label: "file index".to_string(),
        cancel: CancellationToken::new(),
    };
    let index = file_index::build(&seed, &patches, &manifest, work.path(), &sink)
        .map_err(|e| e.to_string())?;

    let json = serde_json::to_string_pretty(&index).map_err(|e| e.to_string())?;
    std::fs::write(&out, format!("{json}\n")).map_err(|e| format!("{}: {e}", out.display()))?;
    let size = std::fs::metadata(&out).map_err(|e| e.to_string())?.len();
    let sha256 = install::hash_file(&out).map_err(|e| e.to_string())?;
    println!(
        "\"file_index\": {{ \"blob\": \"<fill in>\", \"size\": {size}, \"sha256\": \"{sha256}\" }}"
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("build-file-index: {e}");
            ExitCode::FAILURE
        }
    }
}
