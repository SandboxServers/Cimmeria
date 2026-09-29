//! `pack-client-overlay`: pack the client-patches UI overlay into a
//! launcher patch zip and its manifest entry (Black Market plan BM-06).
//!
//! ```text
//! pack-client-overlay --overlay crates/client-patches/overlay --out-dir dist \
//!     --blob-base-url https://github.com/SandboxServers/Cimmeria/releases/download/<tag>
//!     [--id-prefix bm-ui-overlay] [--after <patch id>]
//!     [--manifest manifest.json --manifest-out manifest.new.json]
//! ```
//!
//! Writes `<out-dir>/<id>.zip` and `<out-dir>/<id>.entry.json`. With
//! `--manifest`, also writes the manifest with the entry appended. The
//! result is unsigned: signing stays an offline operator step
//! (docs/client/launcher-distribution-setup.md). With nothing to pack (no
//! overlay directory, or an empty `MANIFEST.txt`) it says so and exits 0.
//!
//! The pack logic is the launcher's own `overlay_pack` module, and the
//! entry is the launcher's own `PatchEntry`, so the tool and the reader
//! cannot disagree on the schema.

// The manifest module serves the launcher; this tool uses its types only.
#[allow(dead_code)]
#[path = "../manifest.rs"]
mod manifest;
#[path = "../overlay_meta.rs"]
mod overlay_meta;
#[path = "../overlay_pack.rs"]
mod overlay_pack;

use std::path::PathBuf;
use std::process::ExitCode;

use overlay_pack::{entry, merge, pack, Merged, DEFAULT_ID_PREFIX};

#[derive(Debug, Default)]
struct Args {
    overlay: Option<PathBuf>,
    out_dir: Option<PathBuf>,
    blob_base_url: Option<String>,
    id_prefix: Option<String>,
    after: Option<String>,
    manifest: Option<PathBuf>,
    manifest_out: Option<PathBuf>,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args::default();
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut value = || it.next().ok_or(format!("{flag} needs a value"));
        match flag.as_str() {
            "--overlay" => args.overlay = Some(value()?.into()),
            "--out-dir" => args.out_dir = Some(value()?.into()),
            "--blob-base-url" => args.blob_base_url = Some(value()?),
            "--id-prefix" => args.id_prefix = Some(value()?),
            "--after" => args.after = Some(value()?),
            "--manifest" => args.manifest = Some(value()?.into()),
            "--manifest-out" => args.manifest_out = Some(value()?.into()),
            other => return Err(format!("unknown argument {other}")),
        }
    }
    if args.manifest.is_some() != args.manifest_out.is_some() {
        return Err("--manifest and --manifest-out go together".into());
    }
    Ok(args)
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let overlay = args.overlay.ok_or("--overlay is required")?;
    let out_dir = args.out_dir.ok_or("--out-dir is required")?;
    let base = args.blob_base_url.ok_or("--blob-base-url is required")?;
    let prefix = args.id_prefix.as_deref().unwrap_or(DEFAULT_ID_PREFIX);

    let Some(packed) = pack(&overlay, prefix).map_err(|e| e.to_string())? else {
        println!(
            "no overlay files listed in {}; nothing to pack",
            overlay.display()
        );
        return Ok(());
    };

    std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
    let zip_name = format!("{}.zip", packed.id);
    let blob = format!("{}/{zip_name}", base.trim_end_matches('/'));
    let patch = entry(&packed, blob, args.after);
    let entry_json = serde_json::to_string_pretty(&patch).map_err(|e| e.to_string())?;
    std::fs::write(out_dir.join(&zip_name), &packed.zip).map_err(|e| e.to_string())?;
    std::fs::write(
        out_dir.join(format!("{}.entry.json", packed.id)),
        format!("{entry_json}\n"),
    )
    .map_err(|e| e.to_string())?;
    println!(
        "packed {} file(s) into {zip_name} ({} bytes, sha256 {})",
        packed.files.len(),
        packed.zip.len(),
        packed.sha256
    );

    if let (Some(input), Some(output)) = (args.manifest, args.manifest_out) {
        let text = std::fs::read_to_string(&input).map_err(|e| e.to_string())?;
        let mut m: manifest::Manifest = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        m.validate().map_err(|e| e.to_string())?;
        let outcome = merge(&mut m, patch).map_err(|e| e.to_string())?;
        let json = serde_json::to_string_pretty(&m).map_err(|e| e.to_string())?;
        std::fs::write(&output, format!("{json}\n")).map_err(|e| e.to_string())?;
        match outcome {
            Merged::Appended => println!(
                "appended {} to {}; sign it before publishing",
                packed.id,
                output.display()
            ),
            Merged::AlreadyPresent => println!(
                "{} is already in {}; manifest unchanged",
                packed.id,
                input.display()
            ),
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("pack-client-overlay: {e}");
            ExitCode::FAILURE
        }
    }
}
