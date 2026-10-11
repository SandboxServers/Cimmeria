# Releasing the desktop launcher

> How-to and reference · desktop launcher maintainers · 2026-10-11

A maintainer comments `/release-launcher` on a merged PR. That dispatches
[`launcher-release.yml`](../../../../.github/workflows/launcher-release.yml),
which builds from `main` and publishes the desktop launcher for Windows as a
zip. Players download it from the stable link:

<https://github.com/SandboxServers/Cimmeria/releases/download/launcher-current/StargateWorlds-Launcher-windows-x64.zip>

## What a release publishes

| Release | Assets | Written when |
|---|---|---|
| Dated, `launcher-YYYYMMDD-<sha7>` | `StargateWorlds-Launcher-windows-x64.zip`, `.zip.sha256`, `.zip.sig` (with the updater key), the client-patches overlay patch when one is committed, and `sgw-launcher-macos-wine-<tag>.exe` with its `.sha256` | Every release |
| Rolling, `launcher-current` (D-LX7) | `StargateWorlds-Launcher-windows-x64.zip`, `.zip.sha256`, `.zip.sig`, `latest.json` | Only with the updater key, and only for the day's first release |

The `.sha256` is in `sha256sum` format (`<hex>  <file name>`). The `.zip.sig` is
the Minisign signature document. `latest.json` is the updater feed every
release launcher reads ([updater](updater.md)).

| Job | What it does |
|---|---|
| `desktop-windows` | Stamps the tag and version, builds and checks the launcher, zips it, signs it and writes the feed (with the key), checks the feed as a launcher would, creates the dated release, then refreshes `launcher-current` |
| `egui-macos-wine` | After the dated release exists, builds the original egui launcher and attaches it as `sgw-launcher-macos-wine-<tag>.exe` (D-LX14, below). A failure here leaves the release as it is |

### The egui launcher (D-LX2, D-LX14)

The release no longer publishes `sgw-launcher-<tag>.exe`, the exact name the egui
launcher updates itself from, so Windows players of the egui launcher are not
offered later releases and move to the zip (D-LX2: announced, no bridge build).
Mac players run the egui launcher under Wine
([macOS guide](../../../../docs/guides/macos.md)) until the macOS desktop build
ships (LX-28), so the release still carries it, under a macOS-labelled name the
egui self-update does not look for. Those players download it again for each
release. LX-18 deletes the job with the egui launcher, after LX-28.

## Version and tag

The tag keeps its form, `launcher-YYYYMMDD-<sha7>`, from the run's UTC date and
the dispatched commit. The compiled version (D-LX8) is SemVer `YYYY.M.D` from
the same date, without leading zeros (`2026.10.11`), stamped as
`CIMMERIA_LAUNCHER_VERSION`. The updater compares it with the feed's version.

When a dated `launcher-YYYYMMDD-*` release already exists for that day, the
version is `YYYY.M.D-N`, where N counts this release among the day's releases
(the second is `-2`). A re-run of the same commit leaves its own tag out of the
count, so it keeps its version. The stable channel refuses a prerelease, so a
same-day rebuild is never offered as an update: it is published as its dated
release only, unsigned, and `launcher-current` keeps the day's first release.
The job summary says so.

## What the Windows job builds

[`tools/launcher-release/build-desktop.sh`](../../../../tools/launcher-release/build-desktop.sh)
runs it in stages. `launcher-desktop.yml` runs the same stages on every
desktop-launcher PR (`desktop release build dry run (Windows)`), without a tag
or a manifest key, so the release build is tested before a release needs it.

| Stage | What it does |
|---|---|
| `helpers` | Builds, for `i686-pc-windows-msvc`: `cimmeria-launch-worker.exe` (this workspace's `cimmeria-runtime-probe` package), `cimmeria_client_patches.dll` and the player build of `cimmeria_client_telemetry.dll` (root workspace, no `lab-bridge`) |
| `stage` | Copies the three into `shell/resources/windows/` through [`tools/stage-helper.py`](../tools/stage-helper.py), which checks each one's architecture and digest and refuses a lab-bridge telemetry DLL |
| `launcher` | Builds the UI, then the 64-bit launcher with `CIMMERIA_LAUNCH_HELPER_SHA256`, `CIMMERIA_CLIENT_PATCHES_SHA256` and `CIMMERIA_CLIENT_TELEMETRY_SHA256` set to the SHA-256 of the staged files |
| `verify` | The launcher is 64-bit and carries each staged file's digest; the three files beside it are 32-bit and are the staged ones; the telemetry DLL is the player build; a release build carries its tag, its version and, when set, the updater public key |
| `package` | Zips the launcher with the `windows/` and `graphics/` directories beside it into `artifacts/StargateWorlds-Launcher-windows-x64.zip`, and writes the `.sha256`. The executable is at the zip's root: the self-update swap refuses a zip without exactly one root `.exe` |
| `sign <version> <url> <notes>` | Signs the zip with the updater key and writes `artifacts/StargateWorlds-Launcher-windows-x64.zip.sig` and `artifacts/latest.json` |
| `verify-feed <version>` | Runs the launcher engine's own feed, signature and zip checks on those files against `LAUNCHER_UPDATER_PUBKEY` |
| `feed-dry-run <version>` | `sign` and `verify-feed` with a throwaway key made in the job; the PR dry run's step |

The launcher is not one file. At Play it reads the launch worker and the DLLs
from `windows/` beside the executable and refuses any whose digest is not the
one compiled in ([launch](launch.md)). That is why the pins are taken from the
staged files in the same job that compiles them in, and why the package is a
zip of that layout.

## The signed feed

`sign` and `verify-feed` run
[`engine/examples/release_feed.rs`](../engine/examples/release_feed.rs), so the
release is signed and checked by the same code a player's launcher runs:
`updater::verify_windows_release` parses the feed, verifies the signature and
signed version with `minisign-verify`, requires a `.zip` URL, and extracts the
zip with every entry check of the swap. A feed, signature or package the
launcher would refuse fails the job before anything is published.

The trusted comment is Tauri's plus the version field the launcher requires,
separated by tabs (`\t` below), with exactly one `version:`:

```text
timestamp:<unix seconds>\tfile:StargateWorlds-Launcher-windows-x64.zip\tversion:<YYYY.M.D>
```

`latest.json` is Tauri v2's static format. The platform `signature` is base64 of
the whole `.zip.sig` document:

```json
{
  "version": "2026.10.11",
  "notes": "Stargate Worlds launcher 2026.10.11 (launcher-20261011-abcdef1).",
  "pub_date": "2026-10-11T10:52:22Z",
  "platforms": {
    "windows-x86_64": {
      "signature": "<base64 of the .zip.sig file>",
      "url": "https://github.com/SandboxServers/Cimmeria/releases/download/launcher-20261011-abcdef1/StargateWorlds-Launcher-windows-x64.zip"
    }
  }
}
```

The feed names the zip on the dated release, not on `launcher-current`. The
signature pins those exact bytes, and a dated release's assets never change, so
a launcher that reads the feed while `launcher-current` is being refreshed is
still pointed at a zip that matches it. `launcher-current` deletes and uploads
each asset again (a release asset cannot be replaced in place), `latest.json`
last. It is never marked latest, and its tag has no date, which the egui
launcher's self-update skips.

## Secrets and variables

| Name | Kind | Used by | Value |
|---|---|---|---|
| `LAUNCHER_UPDATER_MINISIGN_KEY` | secret | the `sign` step only | The Minisign secret key, base64 of the encrypted key file (what `tauri signer generate` writes). Unset: no signature, no feed, no `launcher-current` (D-LX9) |
| `LAUNCHER_UPDATER_MINISIGN_PASSWORD` | secret | the `sign` step only | The key's password |
| `LAUNCHER_UPDATER_PUBKEY` | repository variable | the build, `verify` and `verify-feed` | The matching public key, base64 of the `.pub` file. A variable, not a secret: it is public, and a variable is visible, so a mismatch is easy to see. Unset, the build's updater is Disabled. The private key without it fails the job |
| `LAUNCHER_MANIFEST_PUBKEY_HEX` | secret | the build steps | The key the release manifest is signed with, 64 hex characters. Without it a release build can verify nothing. The test suites use the development key and fail with this set, so no other step has it |
| `LAUNCHER_LOG_SAS_URL` | secret | the build steps | A write-only SAS URL for the log container, `https://<account>.blob.core.windows.net/<container>?<sas>` (D-LX11). Where Settings' "Upload debug logs" PUTs the zip. Unset, the button is hidden. It is compiled in with `option_env!`, so anyone holding the exe can extract it. Give it create permission only (`sp=c`) on that one container, with an expiry: create cannot overwrite an existing blob, and every upload's name is unique (host, time and digest), so a leaked SAS can add blobs but never replace other players' logs. Write (`w`) would let it overwrite them |

The build step also sets `CIMMERIA_LAUNCHER_VERSION`, `CIMMERIA_LAUNCHER_TAG`
and `CIMMERIA_LAUNCHER_BUILD_EPOCH` (the tag and stamp time, for a manifest's
`min_launcher`), and `CIMMERIA_BUILD_BRANCH` and `CIMMERIA_BUILD_GIT_SHA`
(recorded in a game-telemetry session).

### Key holder steps (D-LX9)

Until these are done every release is unsigned, `launcher-current` is not
written, the stable link above does not resolve, and the launcher's updater is
Disabled. Someone with Key Vault and repository-settings access does this once:

1. Generate the pair with the Tauri CLI, entering a strong password when asked:

   ```powershell
   npx @tauri-apps/cli@2 signer generate -w launcher-updater.key
   ```

   It writes `launcher-updater.key` (the encrypted secret key, base64) and
   `launcher-updater.key.pub` (the public key, base64). Both are already in the
   form the workflow and the launcher read; do not decode them.
2. Store the secret key file's contents and the password in Key Vault
   `cimmeria-kv`.
3. In the repository's Actions settings, set the secrets
   `LAUNCHER_UPDATER_MINISIGN_KEY` (the contents of `launcher-updater.key`) and
   `LAUNCHER_UPDATER_MINISIGN_PASSWORD` (the password), and the variable
   `LAUNCHER_UPDATER_PUBKEY` (the contents of `launcher-updater.key.pub`).
4. Delete the local key files.
5. Run a release. Its summary says it was signed and refreshed
   `launcher-current`, and the stable link downloads the zip.

Launchers built before the variable was set have no public key and never
update themselves; players replace them once from the stable link. Rotating the
key strands every launcher built with the old one in the same way, so keep it.

## What a release does not have yet

- **A signature.** It is unsigned (D-LX10). The launch worker injects DLLs
  into the game, which is what Defender and SmartScreen react to.
- **The WebView2 runtime.** Windows 11 has it and Windows 10 usually does.
  Nothing here installs it. When it is missing, the launcher says so at start
  and offers Microsoft's download page instead of exiting silently.
- **A release name in the UI.** The window title and the UI still say
  "development" until LX-17.
- **macOS.** The desktop launcher's macOS build is LX-28: an ad-hoc signed
  `StargateWorlds-Launcher-macos-arm64.zip` from a macOS job of this workflow
  (D-LX14). Until then Mac builds are made and signed locally for testing
  ([Wine validation](wine-validation.md#packaged-helper-staging-and-mac-build)),
  and Mac players use the egui launcher above.
