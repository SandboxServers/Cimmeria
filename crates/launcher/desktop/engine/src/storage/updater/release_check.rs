//! The release feed every Windows release build compiles in, and the checks a
//! player's launcher runs on what it serves, callable on local files. Release
//! tooling (`examples/release_feed.rs`, LX-15) runs them on the zip and
//! `latest.json` a release is about to publish, so a feed, signature or package
//! the launcher would refuse fails CI instead of reaching players.
use super::{apply, policy, Config, Error};
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
/// The feed's platform key for the Windows zip.
pub const WINDOWS_PLATFORM: &str = "windows-x86_64";

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

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::STANDARD, Engine};
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
            "platforms": { WINDOWS_PLATFORM: {
                "signature": STANDARD.encode(signature.to_string()),
                "url": ZIP_URL,
            }}
        });
        let key = STANDARD.encode(keys.pk.to_box().unwrap().to_string());
        let config = release_config(&key, "2026.10.10", WINDOWS_PLATFORM).unwrap();
        (config, serde_json::to_vec(&feed).unwrap())
    }

    fn check(config: &Config, feed: &[u8], package: &[u8]) -> Result<String, Error> {
        let dir = tempfile::tempdir().unwrap();
        verify_windows_release(config, feed, package, &dir.path().join("stage"))
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
}
