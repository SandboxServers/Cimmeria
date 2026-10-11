---
name: launcher-release-feed-traps
description: Desktop launcher release feed (LX-15) - zip must have the exe at its root, key/signature encodings, same-day prerelease policy, how to prove a feed against the engine
metadata:
  type: project
---

Facts found while doing LX-15 (2026-10-11):

- The Windows zip-swap self-update (`engine/.../apply/zip_swap.rs::extract`) refuses a zip without exactly one root `.exe`. The pre-LX-15 `build-desktop.sh package` used `shutil.make_archive(..., base_dir=name)`, which nests everything in one folder, so every release zip before LX-15 would have been refused as `Package`. Check zip layout whenever packaging changes.
- Encodings the engine reads: public key = base64 of the whole Minisign `.pub` file text; feed `signature` = base64 of the whole `.sig` document; trusted comment tab-separated with exactly one `version:` field equal to the feed version. `npx @tauri-apps/cli@2 signer generate` keys work unchanged (secret = base64 of encrypted key box; `minisign` crate `SecretKeyBox::from_string` + `into_secret_key(Some(pw))`).
- Stable channel refuses SemVer prereleases, so a same-day `YYYY.M.D-N` build is published dated-only; `launcher-current` keeps the day's first release.
- Prove any feed change with `engine/examples/release_feed.rs` (`keygen`/`sign`/`verify`), which runs `updater::verify_windows_release` (the engine's real parse + Minisign + zip checks). Build it `--release`: scrypt key decryption is very slow in debug.
- The egui self-update skips tags with no date (`launcher-current`) and looks only for `sgw-launcher-<tag>.exe`; LX-15 renamed the egui asset `sgw-launcher-macos-wine-<tag>.exe` (D-LX2/D-LX14).

**Why:** a release zip or feed the launcher refuses fails silently for players (they just never update).
**How to apply:** touching `launcher-release.yml`, `build-desktop.sh` or the updater policy, rerun the local `release_feed` sign+verify proof.
