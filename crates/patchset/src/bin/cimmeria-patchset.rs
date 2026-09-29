//! `cimmeria-patchset`: build client patch zips, apply them, and sign
//! launcher manifests.
//!
//! ```text
//! cimmeria-patchset build <patch.json> --stock <dir> --patched <dir> --out <zip> [--blob-url <url>]
//! cimmeria-patchset apply <zip> --install <dir>
//! cimmeria-patchset sign <manifest.json> --key <private-key-file>
//! cimmeria-patchset verify <manifest.json> --pubkey <hex>
//! cimmeria-patchset pubkey --key <private-key-file>
//! ```
//!
//! `build` prints the manifest patch entry for the zip. The private key
//! file holds 64 hex characters; keep it offline (see
//! `docs/client/launcher-distribution-setup.md`).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use cimmeria_patchset::{apply, build, signing, Spec};

const USAGE: &str = "usage:
  cimmeria-patchset build <patch.json> --stock <dir> --patched <dir> --out <zip> [--blob-url <url>]
  cimmeria-patchset apply <zip> --install <dir>
  cimmeria-patchset sign <manifest.json> --key <private-key-file>
  cimmeria-patchset verify <manifest.json> --pubkey <hex>
  cimmeria-patchset pubkey --key <private-key-file>";

struct Args {
    positional: Vec<String>,
    flags: Vec<(String, String)>,
}

impl Args {
    fn parse(raw: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut positional = Vec::new();
        let mut flags = Vec::new();
        let mut raw = raw.peekable();
        while let Some(a) = raw.next() {
            if let Some(name) = a.strip_prefix("--") {
                let value = raw.next().ok_or(format!("--{name} needs a value"))?;
                flags.push((name.to_string(), value));
            } else {
                positional.push(a);
            }
        }
        Ok(Self { positional, flags })
    }

    fn flag(&self, name: &str) -> Option<&str> {
        self.flags
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    fn required(&self, name: &str) -> Result<&str, String> {
        self.flag(name).ok_or(format!("missing --{name}"))
    }

    fn arg(&self, index: usize, what: &str) -> Result<&str, String> {
        self.positional
            .get(index)
            .map(String::as_str)
            .ok_or(format!("missing {what}"))
    }
}

fn read_key(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path)
        .map(|s| s.trim().to_string())
        .map_err(|e| format!("reading {path}: {e}"))
}

fn run(args: Args) -> Result<(), String> {
    let e = |err: cimmeria_patchset::PatchsetError| err.to_string();
    match args.arg(0, "a command")? {
        "build" => {
            let spec_path = PathBuf::from(args.arg(1, "<patch.json>")?);
            let spec = Spec::load(&spec_path).map_err(e)?;
            let spec_dir = spec_path.parent().unwrap_or(Path::new("."));
            let report = build::build(
                &spec,
                spec_dir,
                Path::new(args.required("stock")?),
                Path::new(args.required("patched")?),
            )
            .map_err(e)?;
            let out = args.required("out")?;
            std::fs::write(out, &report.zip).map_err(|err| format!("writing {out}: {err}"))?;
            for (target, delta, size) in &report.deltas {
                eprintln!("  {target}: delta {delta} bytes for a {size}-byte file");
            }
            let mut entry = serde_json::json!({
                "id": spec.id,
                "blob": args.flag("blob-url").unwrap_or(out),
                "size": report.zip.len(),
                "sha256": sha256_hex(&report.zip),
            });
            // Shown in the launcher's "Changes to your client" list.
            if let Some(title) = &spec.title {
                entry["title"] = title.clone().into();
            }
            if let Some(description) = &spec.description {
                entry["description"] = description.clone().into();
            }
            println!("{}", serde_json::to_string_pretty(&entry).unwrap());
        }
        "apply" => {
            let zip = PathBuf::from(args.arg(1, "<zip>")?);
            let report = apply(&zip, Path::new(args.required("install")?), &mut |p| {
                eprintln!("  wrote {p}")
            })
            .map_err(e)?;
            println!(
                "rebuilt {}, already current {}, overlay files {}",
                report.rebuilt.len(),
                report.already_current.len(),
                report.overlay_files
            );
        }
        "sign" => {
            let manifest = args.arg(1, "<manifest.json>")?;
            let body =
                std::fs::read(manifest).map_err(|err| format!("reading {manifest}: {err}"))?;
            let sig = signing::sign(&read_key(args.required("key")?)?, &body).map_err(e)?;
            let sig_path = format!("{manifest}.sig");
            std::fs::write(&sig_path, &sig).map_err(|err| format!("writing {sig_path}: {err}"))?;
            println!("wrote {sig_path}");
        }
        "verify" => {
            let manifest = args.arg(1, "<manifest.json>")?;
            let body =
                std::fs::read(manifest).map_err(|err| format!("reading {manifest}: {err}"))?;
            let sig_path = format!("{manifest}.sig");
            let sig = std::fs::read_to_string(&sig_path)
                .map_err(|err| format!("reading {sig_path}: {err}"))?;
            signing::verify(args.required("pubkey")?, &body, &sig).map_err(e)?;
            println!("signature OK");
        }
        "pubkey" => {
            println!(
                "{}",
                signing::public_key(&read_key(args.required("key")?)?).map_err(e)?
            );
        }
        other => return Err(format!("unknown command {other:?}")),
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn main() -> ExitCode {
    let result = Args::parse(std::env::args().skip(1)).and_then(run);
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("error: {msg}\n\n{USAGE}");
            ExitCode::from(2)
        }
    }
}
