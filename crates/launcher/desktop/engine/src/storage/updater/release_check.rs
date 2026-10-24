//! The release feed every Windows release build compiles in, and the checks a
//! player's launcher runs on what it serves, callable on local files. Release
//! tooling (`examples/release_feed.rs`, LX-15) runs them on the zip and
//! `latest.json` a release is about to publish, so a feed, signature or package
//! the launcher would refuse fails CI instead of reaching players.
use super::{apply, policy, Config, Error};
use base64::{engine::general_purpose::STANDARD, Engine};
use std::path::Path;

/// The rolling `launcher-current` release's static Tauri feed (LX-15).
pub const RELEASE_FEED_URL: &str =
    "https://github.com/SandboxServers/Cimmeria/releases/download/launcher-current/latest.json";
/// GitHub serves release assets by redirect; every hop is checked against this.
pub const RELEASE_FEED_HOSTS: [&str; 3] = [
    "github.com",
    "objects.githubusercontent.com",
    "release-assets.githubusercontent.com",
];
/// The feed's platform key for the x64 Windows zip (D-WA2).
pub const WINDOWS_X86_64_PLATFORM: &str = "windows-x86_64";
/// The feed's platform key for the arm64 Windows zip (D-WA2).
pub const WINDOWS_AARCH64_PLATFORM: &str = "windows-aarch64";
/// Every Windows key a release feed carries, x64 first.
pub const WINDOWS_PLATFORMS: [&str; 2] = [WINDOWS_X86_64_PLATFORM, WINDOWS_AARCH64_PLATFORM];
/// This build's own key: the compile target picks it (D-WA3).
#[cfg(target_arch = "aarch64")]
pub const WINDOWS_PLATFORM: &str = WINDOWS_AARCH64_PLATFORM;
/// This build's own key: the compile target picks it (D-WA3).
#[cfg(not(target_arch = "aarch64"))]
pub const WINDOWS_PLATFORM: &str = WINDOWS_X86_64_PLATFORM;

/// The release configuration for `public_key` (base64 of the Minisign `.pub`
/// file, as `LAUNCHER_UPDATER_PUBKEY` carries it) and the compiled `current`
/// version.
pub fn release_config(public_key: &str, current: &str, platform: &str) -> Result<Config, Error> {
    let hosts = RELEASE_FEED_HOSTS.iter().map(|h| h.to_string()).collect();
    Config::new(RELEASE_FEED_URL, public_key, current, platform, hosts)
}

/// What a Windows launcher compiled with `config` does with a release, minus
/// the network and the swap: parse `feed` and pick the platform's offer (Check),
/// verify `package` against its signature and signed version (Prepare and
/// Apply), require a zip URL and extract the zip into `stage` with every entry
/// check the swap makes. `stage` must not exist; its parent must. Returns the
/// offered version. A feed that is not newer than `config`'s version is
/// `NotNewer`, which is what the launcher treats as up to date.
pub fn verify_windows_release(
    config: &Config,
    feed: &[u8],
    package: &[u8],
    stage: &Path,
) -> Result<String, Error> {
    let offer = policy::parse(config, feed)?.ok_or(Error::NotNewer)?;
    config.verify(&offer, package)?;
    apply::require_zip_url(&offer.url)?;
    apply::zip_swap::extract(package, stage, uuid::Uuid::nil())?;
    Ok(offer.version)
}

/// One zip's entry in the feed. `signature` is the Minisign signature file's text.
pub struct FeedEntry<'a> {
    pub platform: &'a str,
    pub signature: &'a str,
    pub url: &'a str,
}

/// The Tauri v2 static feed: `{version, notes, pub_date, platforms: {<platform>: {signature, url}}}`,
/// each signature stored as STANDARD base64 of the signature file's text.
/// Err when `entries` is empty, a platform is not in WINDOWS_PLATFORMS, or a platform repeats.
pub fn feed_document(
    version: &str,
    notes: &str,
    pub_date: &str,
    entries: &[FeedEntry<'_>],
) -> Result<serde_json::Value, String> {
    if entries.is_empty() {
        return Err("a release feed needs at least one Windows zip".into());
    }
    let mut platforms = serde_json::Map::new();
    for entry in entries {
        if !WINDOWS_PLATFORMS.contains(&entry.platform) {
            return Err(format!("{} is not a Windows platform key", entry.platform));
        }
        let value = serde_json::json!({
            "signature": STANDARD.encode(entry.signature),
            "url": entry.url,
        });
        if platforms.insert(entry.platform.to_owned(), value).is_some() {
            return Err(format!("{} is in the feed twice", entry.platform));
        }
    }
    Ok(serde_json::json!({
        "version": version,
        "notes": notes,
        "pub_date": pub_date,
        "platforms": platforms,
    }))
}

/// One zip a release is about to publish. `name` is its file name; `signature_file` the `.zip.sig` bytes.
pub struct WindowsPackage<'a> {
    pub platform: &'a str,
    pub name: &'a str,
    pub zip: &'a [u8],
    pub signature_file: &'a [u8],
}

/// What every Windows launcher does with the release, per platform. `scratch` is an existing,
/// canonical directory; the stages are made under it. The error text names the platform.
pub fn check_windows_feed(
    public_key: &str,
    feed: &[u8],
    version: &str,
    packages: &[WindowsPackage<'_>],
    scratch: &Path,
) -> Result<(), String> {
    if packages.is_empty() {
        return Err("a release has no Windows zip to check".into());
    }
    for (i, package) in packages.iter().enumerate() {
        if packages[..i]
            .iter()
            .any(|earlier| earlier.platform == package.platform)
        {
            return Err(format!("{} is in the release twice", package.platform));
        }
        if package.zip.is_empty() {
            return Err(format!("{} zip is empty", package.platform));
        }
    }
    let parsed: serde_json::Value =
        serde_json::from_slice(feed).map_err(|e| format!("the feed is not JSON: {e}"))?;
    let platforms = parsed["platforms"]
        .as_object()
        .ok_or("the feed has no platforms")?;
    let mut feed_keys: Vec<&str> = platforms.keys().map(String::as_str).collect();
    let mut release_keys: Vec<&str> = packages.iter().map(|p| p.platform).collect();
    feed_keys.sort_unstable();
    release_keys.sort_unstable();
    if feed_keys != release_keys {
        return Err(format!(
            "the feed offers {feed_keys:?} but the release has {release_keys:?}"
        ));
    }
    for (i, package) in packages.iter().enumerate() {
        let platform = package.platform;
        let config = release_config(public_key.trim(), "0.0.0", platform)
            .map_err(|e| format!("{platform}: {e:?}"))?;
        let offered = verify_windows_release(
            &config,
            feed,
            package.zip,
            &scratch.join(format!("stage-{i}")),
        )
        .map_err(|e| format!("{platform}: {e:?}"))?;
        if offered != version {
            return Err(format!(
                "{platform}: the feed offers {offered}, not {version}"
            ));
        }
        let entry = &parsed["platforms"][platform];
        let url = entry["url"].as_str().unwrap_or_default();
        if url.rsplit('/').next() != Some(package.name) {
            return Err(format!(
                "{platform}: the zip is {}, but the feed links {url}",
                package.name
            ));
        }
        if entry["signature"].as_str() != Some(STANDARD.encode(package.signature_file).as_str()) {
            return Err(format!(
                "{platform}: the feed's signature is not this zip's .sig"
            ));
        }
        let mut tampered = package.zip.to_vec();
        if let Some(last) = tampered.last_mut() {
            *last ^= 0x01;
        }
        let tampered_stage = scratch.join(format!("tampered-{i}"));
        if verify_windows_release(&config, feed, &tampered, &tampered_stage)
            != Err(Error::Signature)
        {
            return Err(format!(
                "{platform}: a tampered zip was not refused as a bad signature"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    const ZIP_URL: &str = "https://github.com/SandboxServers/Cimmeria/releases/download/launcher-20261011-abcdef1/StargateWorlds-Launcher-windows-x64.zip";

    /// A feed and its config as the release workflow writes and compiles them.
    fn release(version: &str, package: &[u8]) -> (Config, Vec<u8>) {
        let keys = minisign::KeyPair::generate_unencrypted_keypair().unwrap();
        let comment =
            format!("timestamp:1\tfile:StargateWorlds-Launcher-windows-x64.zip\tversion:{version}");
        let signature =
            minisign::sign(None, &keys.sk, Cursor::new(package), Some(&comment), None).unwrap();
        let feed = serde_json::json!({
            "version": version,
            "notes": "n",
            "pub_date": "2026-10-11T00:00:00Z",
            "platforms": { WINDOWS_X86_64_PLATFORM: {
                "signature": STANDARD.encode(signature.to_string()),
                "url": ZIP_URL,
            }}
        });
        let key = STANDARD.encode(keys.pk.to_box().unwrap().to_string());
        let config = release_config(&key, "2026.10.10", WINDOWS_X86_64_PLATFORM).unwrap();
        (config, serde_json::to_vec(&feed).unwrap())
    }

    fn check(config: &Config, feed: &[u8], package: &[u8]) -> Result<String, Error> {
        let dir = tempfile::tempdir().unwrap();
        // macOS temp dirs live under /var, a symlink to /private/var, and the
        // extraction refuses a stage whose parent runs through a link.
        let root = dir.path().canonicalize().unwrap();
        verify_windows_release(config, feed, package, &root.join("stage"))
    }

    #[test]
    fn the_release_layout_is_offered_and_accepted() {
        let zip = apply::zip_swap::zip_fixture(&[
            ("cimmeria-launcher-desktop.exe", b"exe"),
            ("windows/cimmeria_client_patches.dll", b"dll"),
        ]);
        let (config, feed) = release("2026.10.11", &zip);
        assert_eq!(check(&config, &feed, &zip), Ok("2026.10.11".into()));
    }

    #[test]
    fn a_zip_with_the_launcher_in_a_folder_is_refused() {
        // The pre-LX-15 package nested everything in one top-level folder.
        let zip = apply::zip_swap::zip_fixture(&[
            ("launcher/cimmeria-launcher-desktop.exe", b"exe"),
            ("launcher/windows/cimmeria_client_patches.dll", b"dll"),
        ]);
        let (config, feed) = release("2026.10.11", &zip);
        assert_eq!(check(&config, &feed, &zip), Err(Error::Package));
    }

    #[test]
    fn a_same_day_prerelease_or_an_equal_version_is_not_offered() {
        let zip = apply::zip_swap::zip_fixture(&[("cimmeria-launcher-desktop.exe", b"exe")]);
        for version in ["2026.10.11-2", "2026.10.10"] {
            let (config, feed) = release(version, &zip);
            assert_eq!(
                check(&config, &feed, &zip),
                Err(Error::NotNewer),
                "{version}"
            );
        }
    }

    fn feed_url(name: &str) -> String {
        format!("https://github.com/SandboxServers/Cimmeria/releases/download/launcher-20261024-abcdef1/{name}")
    }

    /// Two zips for one release, each signed under its own file name, and the
    /// feed that offers both. Each tuple is (platform, name, zip, signature file text).
    fn two_platform_release(
        version: &str,
    ) -> (
        String,
        Vec<u8>,
        [(&'static str, &'static str, Vec<u8>, String); 2],
    ) {
        let keys = minisign::KeyPair::generate_unencrypted_keypair().unwrap();
        let packages = [
            (
                WINDOWS_X86_64_PLATFORM,
                "StargateWorlds-Launcher-windows-x64.zip",
                &b"x64 exe"[..],
            ),
            (
                WINDOWS_AARCH64_PLATFORM,
                "StargateWorlds-Launcher-windows-arm64.zip",
                &b"arm64 exe"[..],
            ),
        ]
        .map(|(platform, name, exe)| {
            let zip = apply::zip_swap::zip_fixture(&[("cimmeria-launcher-desktop.exe", exe)]);
            let comment = format!("timestamp:1\tfile:{name}\tversion:{version}");
            let signature =
                minisign::sign(None, &keys.sk, Cursor::new(&zip[..]), Some(&comment), None)
                    .unwrap();
            (platform, name, zip, signature.to_string())
        });
        let urls: Vec<String> = packages.iter().map(|p| feed_url(p.1)).collect();
        let entries: Vec<FeedEntry<'_>> = packages
            .iter()
            .zip(&urls)
            .map(|(p, url)| FeedEntry {
                platform: p.0,
                signature: p.3.as_str(),
                url: url.as_str(),
            })
            .collect();
        let feed = feed_document(version, "n", "2026-10-24T00:00:00Z", &entries).unwrap();
        let key = STANDARD.encode(keys.pk.to_box().unwrap().to_string());
        (key, serde_json::to_vec(&feed).unwrap(), packages)
    }

    fn windows_packages<'a>(
        packages: &'a [(&'static str, &'static str, Vec<u8>, String)],
    ) -> Vec<WindowsPackage<'a>> {
        packages
            .iter()
            .map(|(platform, name, zip, signature)| WindowsPackage {
                platform,
                name,
                zip: zip.as_slice(),
                signature_file: signature.as_bytes(),
            })
            .collect()
    }

    /// A fresh canonical scratch directory; the check's stages are never reused.
    fn scratch_dir() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().canonicalize().unwrap();
        (dir, path)
    }

    /// The error text the feed check gives for `feed` checked against `release`.
    fn refusal(key: &str, feed: &[u8], release: &[WindowsPackage<'_>]) -> String {
        let (_dir, scratch) = scratch_dir();
        check_windows_feed(key, feed, "2026.10.11", release, &scratch).unwrap_err()
    }

    #[test]
    fn each_windows_platform_is_offered_its_own_zip() {
        let (key, feed, packages) = two_platform_release("2026.10.11");
        for (platform, _, zip, _) in &packages {
            let config = release_config(&key, "2026.10.10", platform).unwrap();
            assert_eq!(
                check(&config, &feed, zip),
                Ok("2026.10.11".into()),
                "{platform}"
            );
        }
        let x64 = release_config(&key, "2026.10.10", WINDOWS_X86_64_PLATFORM).unwrap();
        assert_eq!(check(&x64, &feed, &packages[1].2), Err(Error::Signature));
    }

    #[test]
    fn a_two_zip_release_passes_the_feed_check() {
        let (key, feed, packages) = two_platform_release("2026.10.11");
        let (_dir, scratch) = scratch_dir();
        let release = windows_packages(&packages);
        assert_eq!(
            check_windows_feed(&key, &feed, "2026.10.11", &release, &scratch),
            Ok(())
        );
    }

    #[test]
    fn the_feed_check_refuses_a_mismatched_release() {
        let (key, feed, packages) = two_platform_release("2026.10.11");
        let both = windows_packages(&packages);
        // Only the x64 zip is in the release; the feed has an extra key.
        let err = refusal(&key, &feed, &both[..1]);
        assert!(err.contains(WINDOWS_AARCH64_PLATFORM), "{err}");
        // The feed carries only the x64 entry; the release has both zips.
        let x64_url = feed_url(packages[0].1);
        let x64_signature = packages[0].3.as_str();
        let x64_feed = feed_document(
            "2026.10.11",
            "n",
            "2026-10-24T00:00:00Z",
            &[FeedEntry {
                platform: WINDOWS_X86_64_PLATFORM,
                signature: x64_signature,
                url: &x64_url,
            }],
        )
        .unwrap();
        let err = refusal(&key, &serde_json::to_vec(&x64_feed).unwrap(), &both);
        assert!(err.contains(WINDOWS_AARCH64_PLATFORM), "{err}");
        // The two signature files are swapped between the zips.
        let mut swapped = windows_packages(&packages);
        swapped[0].signature_file = packages[1].3.as_bytes();
        swapped[1].signature_file = packages[0].3.as_bytes();
        let err = refusal(&key, &feed, &swapped);
        assert!(err.contains(WINDOWS_X86_64_PLATFORM), "{err}");
        // The arm64 zip's name is not the last segment of its feed URL.
        let mut renamed = windows_packages(&packages);
        renamed[1].name = "renamed.zip";
        let err = refusal(&key, &feed, &renamed);
        assert!(err.contains(WINDOWS_AARCH64_PLATFORM), "{err}");
    }

    #[test]
    fn the_feed_builder_refuses_unknown_repeated_or_no_platforms() {
        let url = feed_url("StargateWorlds-Launcher-windows-x64.zip");
        let x64 = || FeedEntry {
            platform: WINDOWS_X86_64_PLATFORM,
            signature: "sig",
            url: &url,
        };
        let date = "2026-10-24T00:00:00Z";
        assert!(feed_document("2026.10.11", "n", date, &[]).is_err());
        let unknown = [FeedEntry {
            platform: "windows-arm64",
            signature: "sig",
            url: &url,
        }];
        assert!(feed_document("2026.10.11", "n", date, &unknown).is_err());
        let repeated = [x64(), x64()];
        assert!(feed_document("2026.10.11", "n", date, &repeated).is_err());
    }

    #[test]
    fn an_unknown_windows_key_is_refused_as_policy() {
        let (key, _, _) = two_platform_release("2026.10.11");
        assert_eq!(
            release_config(&key, "1.0.0", "windows-arm64").err(),
            Some(Error::Policy)
        );
    }

    #[test]
    fn the_build_target_picks_the_windows_key() {
        assert_eq!(
            WINDOWS_PLATFORM == WINDOWS_AARCH64_PLATFORM,
            cfg!(target_arch = "aarch64")
        );
    }
}
