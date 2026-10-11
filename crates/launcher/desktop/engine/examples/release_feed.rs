//! Release tooling for the launcher's signed update feed (LX-15). The release
//! workflow signs the Windows zip and writes `latest.json` with `sign`, and both
//! it and the PR dry run check the result with `verify`, which runs the engine's
//! own feed, signature and zip checks, so a format the launcher would refuse
//! fails CI. `keygen` makes the throwaway key pair the dry run signs with.
//!
//! ```text
//! release_feed keygen <dir>
//! release_feed sign <zip> <version> <url> <notes> <out-dir>
//! release_feed verify <latest.json> <zip> <zip.sig> <public-key> <version>
//! ```
//!
//! Keys are in the form Tauri's `signer generate` writes and the launcher
//! compiles in: base64 of the Minisign key file. `sign` reads the secret key
//! from `LAUNCHER_UPDATER_MINISIGN_KEY` and its password from
//! `LAUNCHER_UPDATER_MINISIGN_PASSWORD`; `keygen` encrypts with the latter.
//! Nothing secret is printed.
use base64::{engine::general_purpose::STANDARD, Engine};
use cimmeria_launcher_engine::updater::{self, Error};
use std::path::Path;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const KEY_VAR: &str = "LAUNCHER_UPDATER_MINISIGN_KEY";
const PASSWORD_VAR: &str = "LAUNCHER_UPDATER_MINISIGN_PASSWORD";

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["keygen", dir] => keygen(Path::new(dir)),
        ["sign", zip, version, url, notes, out] => {
            sign(Path::new(zip), version, url, notes, Path::new(out))
        }
        ["verify", feed, zip, sig, public_key, version] => verify(
            Path::new(feed),
            Path::new(zip),
            Path::new(sig),
            public_key,
            version,
        ),
        _ => Err("usage: release_feed keygen <dir> | sign <zip> <version> <url> <notes> <out-dir> | verify <latest.json> <zip> <zip.sig> <public-key> <version>".into()),
    }
}

fn password() -> Result<String> {
    std::env::var(PASSWORD_VAR).map_err(|_| format!("{PASSWORD_VAR} is not set").into())
}

/// `secret.key` and `public.key`, each base64 of the Minisign key file.
fn keygen(dir: &Path) -> Result<()> {
    let keys = minisign::KeyPair::generate_encrypted_keypair(Some(password()?))?;
    std::fs::create_dir_all(dir)?;
    std::fs::write(
        dir.join("secret.key"),
        STANDARD.encode(keys.sk.to_box(None)?.to_string()),
    )?;
    std::fs::write(
        dir.join("public.key"),
        STANDARD.encode(keys.pk.to_box()?.to_string()),
    )?;
    println!("wrote a throwaway key pair to {}", dir.display());
    Ok(())
}

/// The key file text, from either base64 of it (Tauri's form) or the file itself.
fn key_text(value: &str) -> Result<String> {
    let value = value.trim();
    if value.starts_with("untrusted comment:") {
        return Ok(value.to_owned());
    }
    Ok(String::from_utf8(STANDARD.decode(value)?)?)
}

/// Writes `<out>/<zip name>.sig` and `<out>/latest.json`.
fn sign(zip: &Path, version: &str, url: &str, notes: &str, out: &Path) -> Result<()> {
    // The launcher refuses a prerelease and anything that is not SemVer, so a
    // feed carrying one would never be offered; refuse to write it.
    let parsed = semver::Version::parse(version)?;
    if !parsed.pre.is_empty() {
        return Err(format!("{version} is a prerelease, which the stable channel refuses").into());
    }
    let secret = std::env::var(KEY_VAR).map_err(|_| format!("{KEY_VAR} is not set"))?;
    let secret = minisign::SecretKeyBox::from_string(&key_text(&secret)?)?
        .into_secret_key(Some(password()?))
        .map_err(|_| "the updater secret key does not decrypt with its password")?;
    let name = zip
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("zip path has no file name")?;
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    // Tauri's trusted comment plus the one `version:` field the launcher
    // requires to equal the announced version (docs/updater.md).
    let trusted = format!("timestamp:{timestamp}\tfile:{name}\tversion:{version}");
    let signature = minisign::sign(
        None,
        &secret,
        std::fs::File::open(zip)?,
        Some(&trusted),
        None,
    )?
    .to_string();
    std::fs::create_dir_all(out)?;
    std::fs::write(out.join(format!("{name}.sig")), &signature)?;
    // Tauri v2 static feed. The platform signature is base64 of the whole
    // signature file, as Tauri's own `latest.json` carries it.
    let feed = serde_json::json!({
        "version": version,
        "notes": notes,
        "pub_date": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "platforms": {
            updater::WINDOWS_PLATFORM: {
                "signature": STANDARD.encode(&signature),
                "url": url,
            }
        }
    });
    std::fs::write(out.join("latest.json"), serde_json::to_vec_pretty(&feed)?)?;
    println!("signed {name} as {version}: {trusted:?}");
    Ok(())
}

/// Runs a Windows release launcher's checks on the files, as an install of any
/// earlier version would, then confirms a tampered copy is refused so the check
/// cannot pass vacuously.
fn verify(feed: &Path, zip: &Path, sig: &Path, public_key: &str, version: &str) -> Result<()> {
    let feed = std::fs::read(feed)?;
    let package = std::fs::read(zip)?;
    let config = updater::release_config(public_key.trim(), "0.0.0", updater::WINDOWS_PLATFORM)
        .map_err(|e| format!("the public key is refused as a release key: {e:?}"))?;
    let scratch = tempfile::tempdir()?;
    let offered =
        updater::verify_windows_release(&config, &feed, &package, &scratch.path().join("stage"))
            .map_err(|e| format!("a release launcher would refuse this release: {e:?}"))?;
    if offered != version {
        return Err(format!("the feed offers {offered}, expected {version}").into());
    }
    // The `.zip.sig` published beside the zip is the signature the feed carries.
    let parsed: serde_json::Value = serde_json::from_slice(&feed)?;
    let in_feed = parsed["platforms"][updater::WINDOWS_PLATFORM]["signature"]
        .as_str()
        .ok_or("the feed has no Windows signature")?;
    if STANDARD.encode(std::fs::read(sig)?) != in_feed {
        return Err(format!("{} is not the signature in the feed", sig.display()).into());
    }
    let mut tampered = package.clone();
    let last = tampered.len() - 1;
    tampered[last] ^= 0x01;
    match updater::verify_windows_release(&config, &feed, &tampered, &scratch.path().join("t")) {
        Err(Error::Signature) => {}
        other => return Err(format!("a tampered zip was not refused: {other:?}").into()),
    }
    println!("verified: a release launcher is offered {offered} and accepts the zip");
    Ok(())
}
