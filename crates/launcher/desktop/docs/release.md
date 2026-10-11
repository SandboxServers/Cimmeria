# Releasing the desktop launcher

> How-to and reference · desktop launcher maintainers · 2026-10-11 (macOS release, LX-28)

A maintainer comments `/release-launcher` on a merged PR. That dispatches
[`launcher-release.yml`](../../../../.github/workflows/launcher-release.yml),
which builds from `main` and publishes the desktop launcher as a zip for
Windows and, ad-hoc signed, for macOS on Apple silicon (LX-28, D-LX14). A
dispatch on any other branch fails at the build job's first step, and the
signing job's environment admits `main` only. Players download it from the
stable links:

- Windows: <https://github.com/SandboxServers/Cimmeria/releases/download/launcher-current/StargateWorlds-Launcher-windows-x64.zip>
- macOS: <https://github.com/SandboxServers/Cimmeria/releases/download/launcher-current/StargateWorlds-Launcher-macos-arm64.zip>

## What a release publishes

| Release | Assets | Written when |
|---|---|---|
| Dated, `launcher-YYYYMMDD-<sha7>` | `StargateWorlds-Launcher-windows-x64.zip`, `.zip.sha256`, `.zip.sig` (with the updater key), the client-patches overlay patch when one is committed, `StargateWorlds-Launcher-macos-arm64.zip` with its `.zip.sha256`, and `sgw-launcher-macos-wine-<tag>.exe` with its `.sha256` | Every release; the macOS zip when the macOS build succeeded |
| Rolling, `launcher-current` (D-LX7) | `StargateWorlds-Launcher-windows-x64.zip`, `.zip.sha256`, `.zip.sig`, `latest.json`, `StargateWorlds-Launcher-macos-arm64.zip`, `.zip.sha256` | Only with the updater key, and only for the day's first release |

The `.sha256` is in `sha256sum` format (`<hex>  <file name>`). The `.zip.sig` is
the Minisign signature document. `latest.json` is the updater feed every
release launcher reads ([updater](updater.md)). The macOS zip has no `.zip.sig`
and is not in `latest.json`: macOS updates are manual (below).

If the macOS build fails, the release still publishes everything else, and
`launcher-current` keeps whatever macOS zip it had beside the new Windows one.
Its notes name the macOS zip only while one is there, so their link never
fails. How to recover depends on where the macOS build failed:

- **`desktop-macos` failed** (the run shows it red): "Re-run failed jobs"
  within a day reruns it and `publish-macos`, which attaches the zip. After a
  day the Windows pieces it needs (kept one day) are gone.
- **The macOS pieces failed in `desktop-windows`** (its last three steps;
  `desktop-windows` still shows green, and `desktop-macos` fails at once with
  "did not build the macOS app's Windows pieces"): "Re-run failed jobs" would
  only rerun `desktop-macos`, which fails again. Use **"Re-run all jobs"**. The
  stamp step recognises the run and keeps its tag and version; every job then
  builds again, `sign` signs the rebuilt zip, and the publish jobs replace this
  release's own assets and notes on the dated release and, when signed, on
  `launcher-current`.

### Jobs

The updater signing key never shares a job with the build's third-party code
(node, npm packages, the cache action, the Tauri CLI), and the build jobs never
see it. No job that runs that code has a token that can write: the two jobs
that write, `publish` and `publish-macos`, run no build and no third-party
action, only [`tools/launcher-release/publish.sh`](../../../../tools/launcher-release/publish.sh)
from a checkout of the released commit.

| Job | What it does | What it can see |
|---|---|---|
| `desktop-windows` | Refuses any ref but `main`, stamps the tag and version ([`stamp.sh`](../../../../tools/launcher-release/stamp.sh)), builds and checks the launcher, zips it, and uploads the zip and `.sha256` as one artifact and the overlay patch as another. Last, it builds the five Windows pieces the macOS app carries and uploads them as a third artifact, kept one day; those steps have their own timeouts and `continue-on-error`, so a failure or hang there fails only the macOS build. Its outputs carry the zip's SHA-256 and the SHA-256 of the pieces' list | `contents: read`; `LAUNCHER_MANIFEST_PUBKEY_HEX` and `LAUNCHER_LOG_SAS_URL` on the launcher build step only; the `LAUNCHER_UPDATER_PUBKEY` variable. Never the Minisign secrets |
| `desktop-macos` | The reusable [`launcher-desktop-macos.yml`](../../../../.github/workflows/launcher-desktop-macos.yml), which the PR dry run also calls. On `macos-15` (arm64), from the same commit: checks the pieces against `desktop-windows`' output, stages them, fetches the pinned D9VK, builds the app, signs it ad hoc, checks it, zips it and uploads the zip and `.sha256` as an artifact. Its output carries the zip's SHA-256 | `contents: read`; `LAUNCHER_MANIFEST_PUBKEY_HEX` and `LAUNCHER_LOG_SAS_URL` on the app build step only. No updater key of either kind |
| `sign` | Not for a same-day build. In the `launcher-release` environment: downloads the zip artifact only and checks it against the build job's digest output, builds the `release_feed` tool with cargo (no node, no cache action) in a step with no secret, then signs, writes `latest.json`, checks the feed as a launcher would, and uploads the `.zip.sig` and `latest.json` as an artifact | `contents: read`; the Minisign key and password, on the sign step only; the public-key variable. Its actions are GitHub's own and this repository's toolchain action (SHA-pinned inside) |
| `publish` | When `sign` succeeded or was skipped: checks the zip's digest again, creates or updates the dated release, then, when signed, refreshes `launcher-current` | `contents: write` (the `GITHUB_TOKEN`); the artifacts. No secret |
| `egui-macos-wine` | Builds the original egui launcher and uploads it as an artifact named for the release, `sgw-launcher-macos-wine-<tag>.exe` with its `.sha256` (D-LX14, below). A failure here leaves the release as it is | `contents: read`; the manifest key and the log SAS for its build |
| `publish-macos` | After `publish`, `desktop-macos` and `egui-macos-wine`, when `publish` succeeded and either build did: checks each artifact against its build job's digest, attaches the egui exe and then the macOS zip to the dated release, each with its notes paragraph (one job, because both rewrite the notes), and, when `publish` refreshed `launcher-current`, replaces the macOS zip there | `contents: write`; the artifacts. No secret, no third-party action, no build |

Every third-party action in the workflow is pinned to a full commit SHA.

A run is safe to re-run. `publish` replaces the dated release's notes and
assets when the tag exists (`gh release upload --clobber`) instead of failing,
and the `launcher-current` refresh uploads each asset with `--clobber`, with
any error failing the job, so a re-run repairs a refresh that stopped part-way.

### The egui launcher (D-LX2, D-LX14)

The release no longer publishes `sgw-launcher-<tag>.exe`, the exact name the egui
launcher updates itself from, so Windows players of the egui launcher are not
offered later releases and move to the zip (D-LX2: announced, no bridge build).
Every dated release's notes say so, telling those players their old launcher
will no longer update and to download the zip from the stable link; the
repository README points there too.
Some Mac players still run the egui launcher under Wine
([macOS guide](../../../../docs/guides/macos.md)), so the release still carries
it, under a macOS-labelled name the egui self-update does not look for, until
LX-18 deletes the job with the egui launcher. Those players download it again
for each release, or move to the macOS desktop launcher below.

## Version and tag

The tag keeps its form, `launcher-YYYYMMDD-<sha7>`, from the run's UTC date and
the dispatched commit. The compiled version (D-LX8) is SemVer `YYYY.M.D` from
the same date, without leading zeros (`2026.10.11`), stamped as
`CIMMERIA_LAUNCHER_VERSION`. The updater compares it with the feed's version.
`shell/build.rs` refuses a value that is not SemVer and merges it over
`tauri.conf.json`'s `version` through `TAURI_CONFIG` (LX-17), so the exe's file
version, Settings' About line and the updater all read the same value. Unset
(a local build), the version is `0.1.0`.

When dated `launcher-YYYYMMDD-*` releases already exist for that day, the
version is `YYYY.M.D-N`, where N is this release's place among the day's
releases by creation time (the second is `-2`):

- **A re-run** of a run that already published its tag keeps that tag's
  place, so it keeps its version. The dated release's notes link the run that
  published it, which is how the stamp step tells a re-run from a new dispatch.
- **The same commit dispatched again on the same day** would need the same
  tag, so the stamp step refuses it, before anything is built, with a message
  saying so. Merge another commit first, or release it the next day (UTC).

The stable channel refuses a prerelease, so a same-day rebuild is never offered
as an update: it is published as its dated release only, unsigned, and
`launcher-current` keeps the day's first release. The job summary says so. A
player who installed a same-day rebuild by hand is not "updated" back to the
day's first build: the updater counts an installed prerelease as its base
version, so it offers that player the next day's release and nothing older
([updater](updater.md)).

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
| `feed-tool` | Builds the `release_feed` example (release profile) that `sign` and `verify-feed` run. The `sign` job runs it in a step with no secret: compiling it runs its dependencies' build scripts and proc macros, which must never see the key |
| `sign <version> <url> <notes>` | Signs the zip with the updater key and writes `artifacts/StargateWorlds-Launcher-windows-x64.zip.sig` and `artifacts/latest.json`. Runs the built tool directly, no cargo |
| `verify-feed <version>` | Runs the launcher engine's own feed, signature and zip checks on those files against `LAUNCHER_UPDATER_PUBKEY` |
| `feed-dry-run <version>` | `feed-tool`, then `sign` and `verify-feed` with a throwaway key made in the job; the PR dry run's step |

The launcher is not one file. At Play it reads the launch worker and the DLLs
from `windows/` beside the executable and refuses any whose digest is not the
one compiled in ([launch](launch.md)). That is why the pins are taken from the
staged files in the same job that compiles them in, and why the package is a
zip of that layout.

## What the macOS job builds

[`tools/launcher-release/build-desktop-macos.sh`](../../../../tools/launcher-release/build-desktop-macos.sh)
runs it in stages on an arm64 `macos-15` runner, inside the reusable
`launcher-desktop-macos.yml`. `launcher-desktop.yml` calls the same workflow on
every desktop-launcher PR (`desktop release build dry run (macOS)`), stamped as
a same-day release (a tag for the PR's commit and a `YYYY.M.D-2` version) with
the development manifest key, so the tag check and the same-day Info.plist
rule run on every PR. It has no release secret and uploads to no release. Why each
piece is shipped, fetched or left out, and the job's design, are in the
[LX-28 worknote](../../../../docs/analysis/launcher-consolidation/worknotes/lx-28-redistribution.md).

The app carries five Windows pieces, two more than the Windows zip: the 64-bit
archive worker and the 32-bit prerequisite worker run under Wine for Install.
`build-desktop.sh mac-helpers` builds those two on the Windows runner, and
`mac-pieces` copies all five into one artifact with a `mac-pieces.sha256` list.
The list's own SHA-256 reaches the macOS job as a job output, separately from
the artifact. The three Play pieces are the files the Windows launcher was
pinned to, so both platforms ship the same bytes.

| Stage | What it does |
|---|---|
| `pieces <dir> <sha256>` | Checks `mac-pieces.sha256` against the Windows job's output, then stages each of the five pieces through [`tools/stage-helper.py`](../tools/stage-helper.py), which checks its digest against the list, its architecture and role, and refuses a lab-bridge telemetry DLL |
| `d3d9` | Downloads the pinned D9VK `d3d9.dll` (WoWSilicon v3.2.2, [graphics resource](../shell/resources/graphics/README.md)) and checks its size and SHA-256. `D9VK-LICENSE.txt` ships beside it |
| `app` | Builds the UI, then `cargo tauri build --bundles app` (release profile, Tauri CLI 2.12.1 to match the `tauri` crate) with the five pins and `CIMMERIA_D3D9_SHA256`. The two rosettax87 pins stay unset: the optional accelerator is left out. With a version, `--config` gives it to the Info.plist as well as the binary |
| `sign` | `codesign --force --deep --sign -`: an ad-hoc signature, no hardened runtime, no entitlements. The only Mach-O in the app is the launcher; the Windows pieces and `d3d9.dll` are resources the seal covers |
| `verify` | `codesign --verify --deep --strict`; `Signature=adhoc` and no `runtime` flag; the executable is arm64 only; each bundled piece and `d3d9.dll` is the staged file and its digest is compiled in; the telemetry DLL is the player build; no rosettax87; a release build carries its tag and version, and Info.plist's `CFBundleShortVersionString` matches |
| `package` | `ditto -c -k --sequesterRsrc --keepParent` into `artifacts/StargateWorlds-Launcher-macos-arm64.zip`, which keeps the bundle exactly as signed; unzips a copy and checks its seal again; writes the `.sha256` |

The Wine runtime is not in the zip. The launcher downloads it at the first
install from WoWSilicon's `wine-runtime-r17` release with a pinned size,
SHA-256 and tree digest (`engine/src/mac_runtime/`).

### What a Mac player sees

The app is signed ad hoc, not with a Developer ID, and it is not notarized;
both stay out of scope by D-LX14. On a Mac that has never run it, macOS blocks
the first open. The player unzips it, drags "Stargate Worlds Launcher" to
Applications in Finder (so it does not run from a randomized read-only copy),
opens it once, then clicks **Open Anyway** in System Settings > Privacy &
Security and enters their password. After that it opens normally. The app needs
Rosetta 2. An ad-hoc signature has no team identity, so after each new version
macOS asks again for folder access.

### macOS updates are manual

The feed offers `windows-x86_64` only, and the shell builds its updater for
Windows only, so every macOS build shows the updater as **Disabled**. A Mac
player downloads each new release from the stable link. The engine's macOS
bundle swap exists but has never run on a packaged app; turning it on needs a
`darwin-aarch64` feed entry, a signed `.app.tar.gz`, one feed job for both
platforms and a Mac update UAT (the worknote's "Self-update on macOS", a
follow-up packet).

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
still pointed at a zip that matches it. `launcher-current` replaces each asset
with `gh release upload --clobber` (which deletes and uploads it; a release
asset cannot be replaced in place), `latest.json` last. It is never marked latest, and its tag has no date, which the egui
launcher's self-update skips.

## Secrets and variables

| Name | Kind | Used by | Value |
|---|---|---|---|
| `LAUNCHER_UPDATER_MINISIGN_KEY` | secret of the `launcher-release` environment | the `sign` job's sign step only | The Minisign secret key, base64 of the encrypted key file (what `tauri signer generate` writes). Unset: no signature, no feed, no `launcher-current` (D-LX9) |
| `LAUNCHER_UPDATER_MINISIGN_PASSWORD` | secret of the `launcher-release` environment | the `sign` job's sign step only | The key's password |
| `LAUNCHER_UPDATER_PUBKEY` | repository variable | the build, `verify` and `verify-feed` | The matching public key, base64 of the `.pub` file. A variable, not a secret: it is public, and a variable is visible, so a mismatch is easy to see. Unset, the build's updater is Disabled. The private key without it fails the `sign` job |
| `LAUNCHER_MANIFEST_PUBKEY_HEX` | secret | the build steps | The key the release manifest is signed with, 64 hex characters. Without it a release build can verify nothing. The test suites use the development key and fail with this set, so no other step has it |
| `LAUNCHER_LOG_SAS_URL` | secret | the build steps | A write-only SAS URL for the log container, `https://<account>.blob.core.windows.net/<container>?<sas>` (D-LX11). Where Settings' "Upload debug logs" PUTs the zip. Unset, the button is hidden. It is compiled in with `option_env!`, so anyone holding the exe can extract it. Give it create permission only (`sp=c`) on that one container, with an expiry: create cannot overwrite an existing blob, and every upload's name is unique (host, time and digest), so a leaked SAS can add blobs but never replace other players' logs. Write (`w`) would let it overwrite them |

The build step also sets `CIMMERIA_LAUNCHER_VERSION`, `CIMMERIA_LAUNCHER_TAG`
and `CIMMERIA_LAUNCHER_BUILD_EPOCH` (the tag and stamp time, for a manifest's
`min_launcher`), and `CIMMERIA_BUILD_BRANCH` and `CIMMERIA_BUILD_GIT_SHA`
(recorded in a game-telemetry session).

### Key holder steps (D-LX9)

Done on 2026-10-11: the key pair is in Key Vault `cimmeria-kv` as
`launcher-updater-minisign-key`, `launcher-updater-minisign-password` and
`launcher-updater-pubkey`, and the secrets and variable below are set. The
steps stay here for a rotation or a new repository. Without them every release
is unsigned, `launcher-current` is not written, the stable links above do not
resolve, and the launcher's updater is Disabled. Someone with Key Vault and
repository-settings access does this once:

1. Generate the pair with the Tauri CLI, entering a strong password when asked:

   ```powershell
   npx @tauri-apps/cli@2 signer generate -w launcher-updater.key
   ```

   It writes `launcher-updater.key` (the encrypted secret key, base64) and
   `launcher-updater.key.pub` (the public key, base64). Both are already in the
   form the workflow and the launcher read; do not decode them.
2. Store the secret key file's contents and the password in Key Vault
   `cimmeria-kv`.
3. In the repository's settings, under **Environments**, open the
   `launcher-release` environment (restricted to the `main` branch) and set
   the environment secrets `LAUNCHER_UPDATER_MINISIGN_KEY` (the contents of
   `launcher-updater.key`) and `LAUNCHER_UPDATER_MINISIGN_PASSWORD` (the
   password). Not repository secrets: those would reach every job. Then, under
   **Secrets and variables → Actions → Variables**, set the repository
   variable `LAUNCHER_UPDATER_PUBKEY` (the contents of
   `launcher-updater.key.pub`).
4. Delete the local key files.
5. Run a release. Its summary says it was signed and refreshed
   `launcher-current`, and the stable link downloads the zip.

Launchers built before the variable was set have no public key and never
update themselves; players replace them once from the stable link. Rotating the
key strands every launcher built with the old one in the same way, so keep it.

## What a release does not have yet

- **A signature.** The Windows zip is unsigned (D-LX10). The launch worker
  injects DLLs into the game, which is what Defender and SmartScreen react to.
  The macOS app is signed ad hoc only; Developer ID signing and notarization
  are out of scope (D-LX14).
- **The WebView2 runtime.** Windows 11 has it and Windows 10 usually does.
  Nothing here installs it. When it is missing, the launcher says so at start
  and offers Microsoft's download page instead of exiting silently.
- **macOS self-update.** Manual, above.
- **An Intel Mac build.** The macOS zip is arm64 only.
