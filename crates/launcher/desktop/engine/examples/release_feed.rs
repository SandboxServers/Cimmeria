//! Release tooling for the launcher's signed update feed (LX-15). The release
//! workflow signs the Windows zips and writes `latest.json` with `sign`, and both
//! it and the PR dry run check the result with `verify`, which runs the engine's
//! own feed, signature and zip checks, so a format the launcher would refuse
//! fails CI. `keygen` makes the throwaway key pair the dry run signs with.
//!
//! ```text
//! release_feed keygen <dir>
//! release_feed sign <version> <notes> <out-dir> (<platform> <zip> <url>)+
//! release_feed verify <latest.json> <public-key> <version> (<platform> <zip> <zip.sig>)+
//! ```
//!
//! Keys are in the form Tauri's `signer generate` writes and the launcher
//! compiles in: base64 of the Minisign key file. `sign` reads the secret key
//! from `LAUNCHER_UPDATER_MINISIGN_KEY` and its password from
//! `LAUNCHER_UPDATER_MINISIGN_PASSWORD`; `keygen` encrypts with the latter.
//! Nothing secret is printed.
use base64::{engine::general_purpose::STANDARD, Engine};
use cimmeria_launcher_engine::updater;
use std::path::Path;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const KEY_VAR: &str = "LAUNCHER_UPDATER_MINISIGN_KEY";
const PASSWORD_VAR: &str = "LAUNCHER_UPDATER_MINISIGN_PASSWORD";

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["keygen", dir] => keygen(Path::new(dir)),
        ["sign", version, notes, out, rest @ ..] => {
            sign(version, notes, Path::new(out), &triples(rest)?)
        }
        ["verify", feed, public_key, version, rest @ ..] => {
            verify(Path::new(feed), public_key, version, &triples(rest)?)
        }
        _ => Err("usage: release_feed keygen <dir> | sign <version> <notes> <out-dir> (<platform> <zip> <url>)+ | verify <latest.json> <public-key> <version> (<platform> <zip> <zip.sig>)+".into()),
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

/// Groups the arguments after the fixed ones into `<platform> <zip> <file>` triples.
fn triples<'a>(rest: &[&'a str]) -> Result<Vec<[&'a str; 3]>> {
    if rest.is_empty() || !rest.len().is_multiple_of(3) {
        return Err("expected one or more <platform> <zip> <file> triples".into());
    }
    Ok(rest.as_chunks::<3>().0.to_vec())
}

/// Writes `<out>/<zip name>.sig` for each zip and one `<out>/latest.json` offering them all.
fn sign(version: &str, notes: &str, out: &Path, zips: &[[&str; 3]]) -> Result<()> {
    // The launcher refuses a prerelease and anything that is not SemVer, so a
    // feed carrying one would never be offered; refuse to write it.
    stable_release(version)?;
    let secret = std::env::var(KEY_VAR).map_err(|_| format!("{KEY_VAR} is not set"))?;
    let secret = minisign::SecretKeyBox::from_string(&key_text(&secret)?)?
        .into_secret_key(Some(password()?))
        .map_err(|_| "the updater secret key does not decrypt with its password")?;
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    // (platform, file name, url, signature file text, trusted comment)
    let mut signed: Vec<(&str, String, &str, String, String)> = Vec::new();
    for &[platform, zip, url] in zips {
        let name = Path::new(zip)
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("zip path has no file name")?;
        if signed.iter().any(|(_, other, ..)| other == name) {
            return Err(format!("two zips are named {name}").into());
        }
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
        signed.push((platform, name.to_owned(), url, signature, trusted));
    }
    let entries: Vec<updater::FeedEntry> = signed
        .iter()
        .map(|(platform, _, url, signature, _)| updater::FeedEntry {
            platform,
            signature: signature.as_str(),
            url,
        })
        .collect();
    let pub_date = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    // Validates the entries before anything is written.
    let feed = updater::feed_document(version, notes, &pub_date, &entries)?;
    std::fs::create_dir_all(out)?;
    for (_, name, _, signature, _) in &signed {
        std::fs::write(out.join(format!("{name}.sig")), signature)?;
    }
    std::fs::write(out.join("latest.json"), serde_json::to_vec_pretty(&feed)?)?;
    for (_, name, _, _, trusted) in &signed {
        println!("signed {name} as {version}: {trusted:?}");
    }
    Ok(())
}

/// Runs each Windows release launcher's checks on its zip, as an install of any
/// earlier version would, and confirms the feed offers `version` to each of them.
fn verify(feed: &Path, public_key: &str, version: &str, zips: &[[&str; 3]]) -> Result<()> {
    let feed = std::fs::read(feed)?;
    // Owned copies, so the packages below can borrow them.
    let mut files: Vec<(&str, String, Vec<u8>, Vec<u8>)> = Vec::new();
    for &[platform, zip, sig] in zips {
        let name = Path::new(zip)
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("zip path has no file name")?;
        files.push((
            platform,
            name.to_owned(),
            std::fs::read(zip)?,
            std::fs::read(sig)?,
        ));
    }
    let scratch = tempfile::tempdir()?;
    let root = scratch.path().canonicalize()?;
    let packages: Vec<updater::WindowsPackage> = files
        .iter()
        .map(|(platform, name, zip, sig)| updater::WindowsPackage {
            platform,
            name: name.as_str(),
            zip: zip.as_slice(),
            signature_file: sig.as_slice(),
        })
        .collect();
    updater::check_windows_feed(public_key, &feed, version, &packages, &root)?;
    let platforms: Vec<&str> = zips.iter().map(|zip| zip[0]).collect();
    println!(
        "verified: every Windows launcher ({}) is offered {version} and accepts its zip",
        platforms.join(", ")
    );
    Ok(())
}

/// Keep feed generation aligned with old launchers: offer stable versions only.
fn stable_release(version: &str) -> Result<()> {
    let parsed = semver::Version::parse(version)?;
    if !parsed.pre.is_empty() {
        return Err(format!("{version} is a prerelease, which the stable channel refuses").into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn daily_slot_feeds_are_stable_and_legacy_repeat_feeds_are_refused() {
        for version in ["2026.10.2301", "2026.10.2302", "2026.10.2303", "2027.1.101"] {
            assert!(stable_release(version).is_ok());
        }
        assert!(stable_release("2026.10.23-2").is_err());
    }

    #[test]
    fn triples_needs_whole_groups() {
        assert!(triples(&[]).is_err());
        assert!(triples(&["windows-x86_64", "a.zip"]).is_err());
        assert_eq!(
            triples(&["p1", "a.zip", "u1", "p2", "b.zip", "u2"]).unwrap(),
            vec![["p1", "a.zip", "u1"], ["p2", "b.zip", "u2"]]
        );
    }
}
