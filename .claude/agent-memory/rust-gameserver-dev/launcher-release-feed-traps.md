---
name: launcher-release-feed-traps
description: Desktop launcher release feed (LX-15) and macOS job (LX-28) - zip must have the exe at its root, key/signature encodings, same-day prerelease policy, job/token layout, bash 3.2 trap, how to prove a feed against the engine
metadata:
  type: project
---

Facts found while doing LX-15 (2026-10-11):

- The Windows zip-swap self-update (`engine/.../apply/zip_swap.rs::extract`) refuses a zip without exactly one root `.exe`. The pre-LX-15 `build-desktop.sh package` used `shutil.make_archive(..., base_dir=name)`, which nests everything in one folder, so every release zip before LX-15 would have been refused as `Package`. Check zip layout whenever packaging changes.
- Encodings the engine reads: public key = base64 of the whole Minisign `.pub` file text; feed `signature` = base64 of the whole `.sig` document; trusted comment tab-separated with exactly one `version:` field equal to the feed version. `npx @tauri-apps/cli@2 signer generate` keys work unchanged (secret = base64 of encrypted key box; `minisign` crate `SecretKeyBox::from_string` + `into_secret_key(Some(pw))`).
- Stable channel refuses SemVer prereleases, so a same-day `YYYY.M.D-N` build is published dated-only; `launcher-current` keeps the day's first release. SemVer orders `YYYY.M.D` above `YYYY.M.D-2`, so `policy::Config::newer` strips the installed version's prerelease before comparing; without that a hand-installed `-2` is "updated" back to the day's first build.
- Workflow layout after the LX-15 review: build job (no Minisign secrets) -> `sign` job in environment `launcher-release` (key as environment secrets, no node/cache action, `release_feed` built in a secret-free step, sign runs the exe) -> `publish` (contents: write, idempotent with `--clobber`). Environment secrets are invisible to jobs without `environment:`, so key-presence checks live in the `sign` job. `gh release create` without `--target` tags the default branch's HEAD, not the dispatched sha. `cmd | grep -q` under `pipefail` can fail on SIGPIPE: capture first, then grep a herestring.
- Prove any feed change with `engine/examples/release_feed.rs` (`keygen`/`sign`/`verify`), which runs `updater::verify_windows_release` (the engine's real parse + Minisign + zip checks). Build it `--release`: scrypt key decryption is very slow in debug.
- The egui self-update skips tags with no date (`launcher-current`) and looks only for `sgw-launcher-<tag>.exe`; LX-15 renamed the egui asset `sgw-launcher-macos-wine-<tag>.exe` (D-LX2/D-LX14).
- LX-28 (2026-10-11) added the macOS app: `desktop-windows` hands its staged Play pieces (`play-pieces`) to a separate `mac-pieces` job, because dependents wait for a whole job, so extra steps at the end of `desktop-windows` still delay `sign`/`publish`. `desktop-macos` calls the reusable `launcher-desktop-macos.yml` (also the PR dry run), `egui-macos-wine` is a read-only build, and `publish-macos` attaches both macOS assets. Rule: no job that runs cargo/npm/Tauri CLI/cache action gets a write token; write jobs run only `tools/launcher-release/publish.sh` from a checkout. A `continue-on-error` failure in a green job is not retried by "Re-run failed jobs". Workflow files have the 700-line cap too. macOS updates are manual (shell updater is Windows-only). macOS runners may run `/bin/bash` 3.2: `"${arr[@]}"` on an empty array is "unbound" under `set -u`; use `${arr[@]+"${arr[@]}"}`. actionlint/shellcheck are not installed locally: download the release exes into the scratchpad.

**Why:** a release zip or feed the launcher refuses fails silently for players (they just never update).
**How to apply:** touching `launcher-release.yml`, `build-desktop.sh` or the updater policy, rerun the local `release_feed` sign+verify proof.

**2026-10-11:** LX-18 deleted the egui launcher and removed the `egui-macos-wine` job, so no release publishes any `sgw-launcher-*.exe`. The egui self-update bullet above now only explains why installed egui copies never update (D-LX2, announce only).
