# Releasing the desktop launcher

> How-to · desktop launcher maintainers · updated 2026-10-24 · companions: [build design](../../../../docs/analysis/launcher-uat-fixes/design-ci-speed.md), [updater](updater.md)

A maintainer comments `/release-launcher` on a merged PR. That dispatches
[`launcher-release.yml`](../../../../.github/workflows/launcher-release.yml),
which builds from `main` and publishes the desktop launcher as a zip for
Windows on x64, a second zip for Windows on Arm and, ad-hoc signed, for macOS on Apple silicon (LX-28, D-LX14). A
dispatch on any other branch fails at the build job's first step, and the
signing job's environment admits `main` only. Players download it from the
stable links:

- Windows: <https://github.com/SandboxServers/Cimmeria/releases/download/launcher-current/StargateWorlds-Launcher-windows-x64.zip>
- Windows on Arm: <https://github.com/SandboxServers/Cimmeria/releases/download/launcher-current/StargateWorlds-Launcher-windows-arm64.zip>
- macOS: <https://github.com/SandboxServers/Cimmeria/releases/download/launcher-current/StargateWorlds-Launcher-macos-arm64.zip>

## Build time and signing isolation

The release-only `release_feed` and overlay CLIs build with the standalone
workspace's `release-tools` profile: release semantics, `opt-level = 0`,
`codegen-units = 256`, and `lto = false`. The overlay CLI uses the desktop
manifest and target directory, avoiding a second root-workspace dependency
build. The app and shipped helpers retain their normal release profile.

The signing job compiles the feed tool before it loads signing secrets. It
still uses a clean runner with no restored cache, node or third-party action;
the private key and password stay in the `launcher-release` environment and
exist only in the signing step. Do not move build caches into this job to save
time. Publication order and release authorization are unchanged.

[Workflow policy tests](../../../../tools/launcher-release/test_workflow_policy.py)
protect those boundaries. The [CI build design](../../../../docs/analysis/launcher-uat-fixes/design-ci-speed.md)
records measured slow steps and deferred options. The first changed run is a
measurement opportunity, not evidence of a speedup before it finishes.

## What a release publishes

| Release | Assets | Written when |
|---|---|---|
| Dated, `launcher-YYYYMMDD-<sha7>` | `StargateWorlds-Launcher-windows-x64.zip` and `StargateWorlds-Launcher-windows-arm64.zip`, each with `.zip.sha256` and `.zip.sig` (required updater signature), the client-patches overlay patch when one is committed, `StargateWorlds-Launcher-macos-arm64.zip` with its `.zip.sha256`, and `StargateWorlds-Launcher-linux-x86_64.AppImage` with its `.AppImage.sha256` | Every release; the macOS zip and the AppImage when their builds succeeded |
| Rolling, `launcher-current` (D-LX7) | Both Windows zips, each with `.zip.sha256` and `.zip.sig`, `latest.json`, `StargateWorlds-Launcher-macos-arm64.zip`, `.zip.sha256`, `StargateWorlds-Launcher-linux-x86_64.AppImage`, `.AppImage.sha256` | Every signed release whose version does not regress the rolling feed |

The `.sha256` is in `sha256sum` format (`<hex>  <file name>`). The `.zip.sig` is
the Minisign signature document. `latest.json` is the updater feed every
release launcher reads ([updater](updater.md)); it carries a `windows-x86_64` and a
`windows-aarch64` entry. The macOS zip and the Linux AppImage have no
signature and are not in `latest.json`: macOS and Linux updates are manual
(below).

If a macOS build fails, the release still publishes everything else, and
`launcher-current` keeps whatever macOS zip it had beside the new Windows one.
Its notes name the macOS zip only while one is there, so their link never
fails. `publish-macos` runs whenever `publish` succeeded, attaches what was
built, and then fails if `mac-pieces` or `desktop-macos` did not succeed. So the run shows both the failed build and `publish-macos`
red, and **"Re-run failed jobs"** reruns the two (and anything they skipped),
which attaches the missing asset. Rerun the macOS build within a day: the macOS
pieces it reads are kept one day. The arm64 build and a rerun of `mac-pieces`
read the Play pieces, which are kept seven days.

A failed Linux build works the same way: `publish-linux` runs whenever `publish`
succeeded, attaches what was built, and fails if `desktop-linux` did not
succeed. The Linux build reads no Windows pieces, so a rerun only has to come
within GitHub's 30-day re-run window, which is also how long the AppImage
artifact is kept.

**"Re-run all jobs"** is the other way, and the one to use when a job that
succeeded must run again. The stamp step recognises the run and keeps its tag
and version; every job builds again, `sign` signs the rebuilt zip, and the
publish jobs replace this release's own assets on the dated release and, when
signed, on `launcher-current`.

The dated release's notes converge whatever the order of runs and re-runs:
`publish.sh` always writes them as the release's base text plus the macOS
paragraph while the release holds the macOS zip and the Linux paragraph while
it holds the AppImage, plus a **Windows on Arm:** paragraph naming the arm64
zip. `publish-linux` waits for `publish-macos`, so the two never rewrite the
notes at once.

### Jobs

The updater signing key never shares a job with the build's third-party code
(node, npm packages, the cache action, the Tauri CLI), and the build jobs never
see it. No job that runs that code has a token that can write: the three jobs
that write, `publish`, `publish-macos` and `publish-linux`, run no build and no
third-party action, only [`tools/launcher-release/publish.sh`](../../../../tools/launcher-release/publish.sh)
from a checkout of the released commit. The macOS pieces have their own job, so
`sign` and `publish`, which wait for all of `desktop-windows`, never wait for
them. `sign` and `publish` do wait for the arm64 build (below).

| Job | What it does | What it can see |
|---|---|---|
| `desktop-windows` | Refuses any ref but `main`, stamps the tag and version ([`stamp.sh`](../../../../tools/launcher-release/stamp.sh)), builds and checks the launcher, zips it, and uploads the zip and `.sha256` as one artifact and the overlay patch as another. Last, it uploads the three staged Play pieces for `mac-pieces` and `desktop-windows-arm64`, kept seven days (no `continue-on-error`: D-WA5). Its outputs carry the zip's SHA-256 and the SHA-256 of the Play pieces' list | `contents: read`, `actions: read` for the run identity reservation; `LAUNCHER_MANIFEST_PUBKEY_HEX` on the launcher build step only; the `LAUNCHER_UPDATER_PUBKEY` variable. Never the Minisign secrets |
| `desktop-windows-arm64` | The reusable [`launcher-desktop-windows-arm64.yml`](../../../../.github/workflows/launcher-desktop-windows-arm64.yml), which the PR dry run also calls. On the hosted `windows-11-arm` runner, from the same commit: checks the tools, takes the i686 Play pieces from `desktop-windows` (checked against its digest output) with `build-desktop.sh play-pieces-in`, builds the native arm64 launcher, runs `verify` and `package`, and uploads the zip and `.sha256` as `launcher-zip-arm64-<tag>`. Its output carries the zip's SHA-256 | `contents: read`; `LAUNCHER_MANIFEST_PUBKEY_HEX` on the launcher build step only. No updater key of either kind |
| `mac-pieces` | On Windows: checks the Play pieces against `desktop-windows`' digest output before using them, builds the archive and prerequisite workers, and uploads all five with a `mac-pieces.sha256` list, kept one day. Its output carries the list's SHA-256 | `contents: read`. No secret |
| `desktop-macos` | The reusable [`launcher-desktop-macos.yml`](../../../../.github/workflows/launcher-desktop-macos.yml), which the PR dry run also calls. On `macos-15` (arm64), from the same commit: checks the pieces against `mac-pieces`' output, stages them, fetches the pinned D9VK, builds the app, signs it ad hoc, checks it, zips it and uploads the zip and `.sha256` as an artifact. Its output carries the zip's SHA-256 | `contents: read`; `LAUNCHER_MANIFEST_PUBKEY_HEX` on the app build step only. No updater key of either kind |
| `sign` | Every release. In the `launcher-release` environment: downloads the two zip artifacts only and checks each against its build job's digest output, builds the `release_feed` tool with cargo (no node, no cache action) in a step with no secret, then signs both zips in one `release_feed sign` call, writes the one `latest.json`, checks the feed as an x64 and an arm64 launcher would, and uploads the two `.zip.sig` files and `latest.json` as the `launcher-feed-<tag>` artifact | `contents: read`; the Minisign key and password, on the sign step only; the public-key variable. Its actions are GitHub's own and this repository's toolchain action (SHA-pinned inside) |
| `publish` | Unless the run was cancelled, when `sign` succeeded: checks both zips' digests again, creates or updates the dated release with both Windows sets, then refreshes `launcher-current` with both sets and `latest.json` last | `contents: write` (the `GITHUB_TOKEN`); the artifacts. No secret |
| `publish-macos` | Unless the run was cancelled, when `publish` succeeded: checks the macOS zip against `desktop-macos`' digest output, attaches it to the dated release and rewrites its notes, replaces the macOS zip on `launcher-current` when `publish` refreshed it, and fails if the macOS build did not succeed | `contents: write`; the artifacts. No secret, no third-party action, no build |
| `desktop-linux` | The reusable [`launcher-desktop-linux.yml`](../../../../.github/workflows/launcher-desktop-linux.yml), which the PR dry run also calls. On `ubuntu-22.04`, from the same commit, with `desktop-windows`' tag, version and epoch: installs the WebKitGTK build packages, builds the AppImage, checks it, and uploads it and its `.sha256` as an artifact. Its output carries the AppImage's SHA-256 | `contents: read`; `LAUNCHER_MANIFEST_PUBKEY_HEX` on the build step only. No updater key of either kind |
| `publish-linux` | After `publish-macos` finished, unless the run was cancelled, when `publish` succeeded: checks the AppImage against `desktop-linux`' digest output, attaches it to the dated release and rewrites its notes, replaces the AppImage on `launcher-current` unless a newer release is current, and fails if the Linux build did not succeed | `contents: write`; the artifacts. No secret, no third-party action, no build |

The arm64 build is on the release's critical path (D-WA5): `sign` and `publish` need
`desktop-windows-arm64`. A feed without `windows-aarch64` gives every arm64
launcher `Error::Platform`, so once a release has carried the arm64 zip, every
feed must carry both keys. If the arm64 job fails, nothing is signed or
published; **"Re-run failed jobs"** reruns it and what it skipped. The Play
pieces it needs stay for seven days, so a rerun on a later day still finds them.

Every third-party action in the workflow is pinned to a full commit SHA.

A run is safe to re-run. `publish` replaces the dated release's notes and
assets when the tag exists (`gh release upload --clobber`) instead of failing,
and the `launcher-current` refresh uploads each asset with `--clobber`, with
any error failing the job, so a re-run repairs a refresh that stopped part-way.
`launcher-desktop.yml`'s `release scripts` job runs `stamp.sh` and `publish.sh`
against a stub `gh` on every PR that touches them
([`test-release-scripts.sh`](../../../../tools/launcher-release/test-release-scripts.sh)).

### The retired egui launcher (D-LX1, D-LX2)

The egui launcher (`sgw-launcher.exe`) was deleted from the repository in LX-18,
and no release builds it. Releases never publish `sgw-launcher-<tag>.exe`, the
exact name it updates itself from, so copies still installed are not offered
later releases and their players move to the zip (D-LX2: announced, no bridge
build). Every dated release's notes say so, telling those players their old
launcher will no longer update and to download the zip from the stable link;
the repository README points there too. Mac players who ran it under Wine move
to the macOS desktop launcher below.

## Version and tag

The tag remains `launcher-YYYYMMDD-<sha7>`. D-LX15 (owner approved
2026-10-23, superseding D-LX8) uses stable SemVer `YYYY.M.(D*100+N)` from
UTC, with slots N=1..99. October 23's first three releases are
`2026.10.2301`, `2026.10.2302`, and `2026.10.2303`. Slot 100 is refused.
The ordinal counts legacy dated releases and exceeds existing encoded slots.
Paginated release history prevents older pages being silently omitted.
The rolling feed also reserves its last encoded slot if a dated release was
deleted. Published notes with compiled identities are checked for precedence;
historical notes without version evidence produce an explicit warning. They
are not evidence of version zero, and this check is not an exhaustive binary
audit of releases predating compiled-version notes.

Old `2026.10.23` and `2026.10.23-2` installations accept `2026.10.2301`.
SemVer compares year then month before patch, so month and year rollovers
remain increasing. Build metadata cannot distinguish releases, and a fourth
numeric component is not SemVer. Prerelease offers remain refused.

The workflow reserves its identity as a run artifact before building and
uses the run's creation time. Reruns recover that identity across midnight;
a rerun without its reservation refuses before building (retention: 90 days).
A different run targeting the same dated commit tag is refused. An older
rerun cannot lower the version in `launcher-current`. A delayed macOS retry
still repairs its dated release, but compares its reserved stable version to
the rolling feed before touching rolling macOS assets or notes. Older retries
leave both intact and report the skip; equal-version retries remain allowed.
An unreadable or invalid rolling feed fails closed.

`CIMMERIA_LAUNCHER_VERSION` supplies the exe file version, About line and
updater comparison through `TAURI_CONFIG`. The build refuses numeric
components above 65535, the Windows VERSIONINFO limit. Maximum patch is
3199. MSI ProductVersion's year limit is incompatible; this release ships a
zip, with no MSI compatibility promise. Local builds remain `0.1.0`.

Every release requires a Minisign signature and verified updater feed.
Missing keys, failed or skipped signing block publication. The key stays in
the `launcher-release` GitHub environment. This does not provide Windows
Authenticode signing or Apple notarization. macOS keeps ad-hoc application
signing and manual updating.

## What the Windows job builds

[`tools/launcher-release/build-desktop.sh`](../../../../tools/launcher-release/build-desktop.sh)
runs it in stages. `launcher-desktop.yml` runs the same stages on every
desktop-launcher PR (`desktop release build dry run (Windows)`), without a tag
or a manifest key, so the release build is tested before a release needs it.
The same workflow calls the arm64 job as `release-dry-run-arm64` (`desktop
release build dry run (Windows arm64)`), and `release-dry-run-feed` (`desktop
release build dry run (updater feed)`) signs and checks the two-zip feed with a
throwaway key. These dry runs don't gate merges (D-WA7).

| Stage | What it does |
|---|---|
| `helpers` | Builds, for `i686-pc-windows-msvc`: `cimmeria-launch-worker.exe` (this workspace's `cimmeria-runtime-probe` package), `cimmeria_client_patches.dll` and the player build of `cimmeria_client_telemetry.dll` (root workspace, no `lab-bridge`) |
| `stage` | Copies the three into `shell/resources/windows/` through [`tools/stage-helper.py`](../tools/stage-helper.py), which checks each one's architecture and digest and refuses a lab-bridge telemetry DLL |
| `launcher` | Builds the UI, then the 64-bit launcher embedding the three staged Play pieces. `CIMMERIA_LAUNCH_HELPER_SHA256`, `CIMMERIA_CLIENT_PATCHES_SHA256` and `CIMMERIA_CLIENT_TELEMETRY_SHA256` independently pin their bytes; packaging builds require all three and fail for missing or mismatched input |
| `verify` | Checks the 64-bit launcher, compiled pins and player-only telemetry identity, then copies only the EXE to a clean directory and runs `--verify-embedded-payload` to prove headless extraction and verification. A release build carries its tag, version and, when set, updater public key |
| `package` | Writes `artifacts/StargateWorlds-Launcher-windows-x64.zip` (`WINDOWS_ARCH=arm64`: `-windows-arm64.zip`) containing exactly one root `cimmeria-launcher-desktop.exe`, then its `.sha256`. The x64 name and signed updater contract stay the same. `verify` requires PE machine `0x8664` for x64 and `0xaa64` for arm64 |
| `feed-tool` | Builds the `release_feed` example (`release-tools` profile) that `sign` and `verify-feed` run. The `sign` job runs it in a step with no secret: compiling it runs its dependencies' build scripts and proc macros, which must never see the key |
| `sign <version> <base-url> <notes>` | Signs every zip in `FEED_ARCHES` (x64 and arm64) with the updater key, offers each zip at `<base-url>/<zip name>`, and writes a `.zip.sig` beside each and one `artifacts/latest.json`. Runs the built tool directly, no cargo |
| `verify-feed <version>` | Runs the launcher engine's own feed, signature and zip checks on those files, per platform, against `LAUNCHER_UPDATER_PUBKEY` |
| `feed-dry-run <version>` | `feed-tool`, then `sign` and `verify-feed` over both zips with a throwaway key made in the job; the step of the PR dry run's feed job |

The Windows download stays a ZIP so browsers do not need to download a bare,
unsigned executable. Inside it, the launcher is one file. On startup it verifies
and, when necessary, repairs its embedded worker and DLLs under
`<app-local-data>/payloads/<content-digest>/windows/`; Play still checks the
independent compiled digests ([launch](launch.md#embedded-windows-play-resources)).
The digest identifies payload bytes, not the release version. Shared leases
keep another active launcher from cleaning an in-use cache version. Cleanup
preserves unknown entries and older loose sidecars beside the launcher.

The Windows ZIP is not the macOS staging source: `mac-pieces` continues to take
the separately staged Play pieces. macOS already packages its helpers and DLLs
inside `Cimmeria Launcher.app` and keeps that layout. WebView2, the game and the
Wine runtime remain separate prerequisites or downloads. Signing secrets are
unchanged. The owner authorized the embedded-payload PR to merge, then a launcher
release from main retaining the Windows ZIP. This is not packaged/game acceptance
or permission for a live game or lab session.

### Windows on Arm

Every release also ships a native arm64 launcher, `StargateWorlds-Launcher-windows-arm64.zip` (D-WA2: feed key `windows-aarch64`, PE machine `0xaa64`). What differs from the x64 build:

- **Native launcher, emulated game.** The launcher EXE is `aarch64-pc-windows-msvc`. The three Play pieces it embeds (the launch worker and two game DLLs) stay `i686`, shared with the x64 zip: the game is 32-bit x86 and runs under Windows' x86 emulation. The arm64 job compiles no `i686` code; it takes the x64 job's staged pieces through `play-pieces-in`.
- **Hosted runner.** The job runs on `windows-11-arm`, free for this public repository; the self-hosted pool has no Arm machine. A native runner lets `verify` run the EXE's embedded-payload probe, which a cross-compile couldn't (D-WA1).
- **Tools step (D-WA8).** The image's tools aren't assumed. The job checks `python`, then `clang` (installs LLVM through Chocolatey when missing, because `ring` and `aws-lc-sys` need it for `aarch64-pc-windows-msvc`), then `rustup` (official `rustup-init` when missing). It pins the default host to `aarch64-pc-windows-msvc` and asserts it after the toolchain action, so a silent x64 toolchain fails there. No new third-party action is added.
- **Compile-time key (D-WA3).** The build target picks the updater key: an arm64 launcher updates to `windows-aarch64`. An x64 launcher on an Arm PC stays x64 and updates to the x64 zip; there is no automatic cross-grade.
- **One feed (D-WA4).** `latest.json` carries both Windows keys. Old x64 launchers read only their own key and don't change.

What is tested: the PR dry run builds and verifies the arm64 zip natively and checks the two-zip feed with a throwaway key. What isn't yet: no release has carried the arm64 zip, and `launcher-release.yml`'s new wiring runs for the first time at the first release. The owner UATs WA-U1..WA-U3 in the [campaign ledger](../../../../docs/analysis/launcher-win-arm64/README.md#owner-uat-no-lab-use) are pending. The bootstrap installer and `setup.ps1` still download the x64 zip by name, which runs emulated on Arm (WA-F1 switches them after the first arm64 release).

## What the macOS job builds

[`tools/launcher-release/build-desktop-macos.sh`](../../../../tools/launcher-release/build-desktop-macos.sh)
runs it in stages on an arm64 `macos-15` runner, inside the reusable
`launcher-desktop-macos.yml`. `launcher-desktop.yml` calls the same workflow on
every desktop-launcher PR (`desktop release build dry run (macOS)`), stamped as
a same-day release (a tag for the PR's commit and a `YYYY.M.(D*100+2)` version) with
the development manifest key, so the tag check and the same-day Info.plist
rule run on every PR. It has no release secret and uploads to no release. Why each
piece is shipped, fetched or left out, and the job's design, are in the
[LX-28 worknote](../../../../docs/analysis/launcher-consolidation/worknotes/lx-28-redistribution.md).

The app carries five Windows pieces, two more than the Windows zip: the 64-bit
archive worker and the 32-bit prerequisite worker run under Wine for Install.
The `mac-pieces` job takes in the three Play pieces `desktop-windows` staged
(`build-desktop.sh play-pieces-in`, checked against that job's digest output),
builds those two with `mac-helpers`, and `mac-pieces` copies all five into one
artifact with a `mac-pieces.sha256` list. Each list's own SHA-256 reaches the
next job as a job output, separately from the artifact. The three Play pieces
are the files the Windows launcher was pinned to, so both platforms ship the
same bytes.

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
the first open. The player unzips it, drags "Cimmeria Launcher" to
Applications in Finder (so it does not run from a randomized read-only copy),
opens it once, then clicks **Open Anyway** in System Settings > Privacy &
Security and enters their password. After that it opens normally. The app needs
Rosetta 2. An ad-hoc signature has no team identity, so after each new version
macOS asks again for folder access.

### macOS updates are manual

The feed offers the two Windows keys only, and the shell builds its updater for
Windows only, so every macOS build shows the updater as **Disabled**. A Mac
player downloads each new release from the stable link. The engine's macOS
bundle swap exists but has never run on a packaged app; turning it on needs a
`darwin-aarch64` feed entry, a signed `.app.tar.gz`, one feed job for both
platforms and a Mac update UAT (the worknote's "Self-update on macOS", a
follow-up packet).

## What the Linux job builds

[`tools/launcher-release/build-desktop-linux.sh`](../../../../tools/launcher-release/build-desktop-linux.sh)
builds an x86-64 AppImage in stages (`app`, `verify`, `package`), run by the
reusable `launcher-desktop-linux.yml`; `launcher-desktop.yml` calls the same
workflow on every desktop-launcher PR as `desktop release build dry run (Linux)`,
with a fixed dry-run tag and version and the development manifest key.

- **`tools`** (run by `app`) puts the three files the Tauri bundler would
  download into its cache, each checked against a SHA-256 pinned in the script:
  AppRun, linuxdeploy and linuxdeploy's AppImage plugin, which provides the
  runtime every AppImage starts with. The bundler uses a cached file instead of
  downloading it. The plugin comes from upstream's moving `continuous` tag, so
  when upstream moves it, the build fails until the pin is updated on purpose.
- **`app`** builds the UI and runs `cargo tauri build --bundles appimage`
  (release profile), with the release's version, tag and manifest key. The
  AppImage carries WebKitGTK and GTK from the build host. Ubuntu 22.04 keeps its
  glibc old enough for SteamOS and other current distributions.
- **`verify`** checks that it is an x86-64 ELF that extracts, that the extracted
  launcher is x86-64 and embeds the release tag and version, and that it
  carries no Windows executable or DLL.
- **`package`** writes `artifacts/StargateWorlds-Launcher-linux-x86_64.AppImage`
  and its `.sha256`.

The Linux launcher installs the game in-process ([README](../README.md#linux)).
It has no Play, Repair, Update or prerequisite backend yet, so it carries none
of the Windows pieces. A download from GitHub loses the executable bit: the
player sets it once (the release notes say how).

### Linux updates are manual

As on macOS: the feed has no Linux key and the shell builds its updater for
Windows only, so the updater shows **Disabled** and a Linux player downloads
each new release from the stable link.

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
timestamp:<unix seconds>\tfile:StargateWorlds-Launcher-windows-x64.zip\tversion:<YYYY.M.(D*100+N)>
```

(`file:` names the signed zip; the arm64 zip's comment names `StargateWorlds-Launcher-windows-arm64.zip`.)

`latest.json` is Tauri v2's static format. The platform `signature` is base64 of
the whole `.zip.sig` document:

```json
{
  "version": "2026.10.11",
  "notes": "Cimmeria Launcher 2026.10.11 (launcher-20261011-abcdef1).",
  "pub_date": "2026-10-11T10:52:22Z",
  "platforms": {
    "windows-x86_64": {
      "signature": "<base64 of the .zip.sig file>",
      "url": "https://github.com/SandboxServers/Cimmeria/releases/download/launcher-20261011-abcdef1/StargateWorlds-Launcher-windows-x64.zip"
    },
    "windows-aarch64": {
      "signature": "<base64 of the arm64 .zip.sig file>",
      "url": "https://github.com/SandboxServers/Cimmeria/releases/download/launcher-20261011-abcdef1/StargateWorlds-Launcher-windows-arm64.zip"
    }
  }
}
```

The feed names the zip on the dated release, not on `launcher-current`. The
signature pins those exact bytes, and a dated release's assets never change, so
a launcher that reads the feed while `launcher-current` is being refreshed is
still pointed at a zip that matches it. `launcher-current` replaces each asset
with `gh release upload --clobber` (which deletes and uploads it; a release
asset cannot be replaced in place), `latest.json` last. It is never marked latest, and its tag has no date, which a
retired egui launcher's self-update skips.

## Secrets and variables

| Name | Kind | Used by | Value |
|---|---|---|---|
| `LAUNCHER_UPDATER_MINISIGN_KEY` | secret of the `launcher-release` environment | the `sign` job's sign step only | The Minisign secret key, base64 of the encrypted key file (what `tauri signer generate` writes). Unset: no signature, no feed, no `launcher-current` (D-LX9) |
| `LAUNCHER_UPDATER_MINISIGN_PASSWORD` | secret of the `launcher-release` environment | the `sign` job's sign step only | The key's password |
| `LAUNCHER_UPDATER_PUBKEY` | repository variable | the build, `verify` and `verify-feed` | The matching public key, base64 of the `.pub` file. A variable, not a secret: it is public, and a variable is visible, so a mismatch is easy to see. Unset, the build's updater is Disabled. The private key without it fails the `sign` job |
| `LAUNCHER_MANIFEST_PUBKEY_HEX` | secret | the build steps | The key the release manifest is signed with, 64 hex characters. Without it a release build can verify nothing. The test suites use the development key and fail with this set, so no other step has it |

The build step also sets `CIMMERIA_LAUNCHER_VERSION`, `CIMMERIA_LAUNCHER_TAG`
and `CIMMERIA_LAUNCHER_BUILD_EPOCH` (the tag and stamp time, for a manifest's
`min_launcher`), and `CIMMERIA_BUILD_BRANCH` and `CIMMERIA_BUILD_GIT_SHA`
(recorded in a game-telemetry session).

### Key holder steps (D-LX9)

Done on 2026-10-11: the key pair is in Key Vault `cimmeria-kv` as
`launcher-updater-minisign-key`, `launcher-updater-minisign-password` and
`launcher-updater-pubkey`, and the secrets and variable below are set. The
steps stay here for a rotation or a new repository. Without them publication fails closed and `launcher-current` is not refreshed; the stable links may not resolve, and the launcher's updater is Disabled. Someone with Key Vault and
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

- **A signature.** The Windows executable lacks Authenticode signing (D-LX10); the updater zip has a required Minisign signature. The launch worker
  injects DLLs into the game, which is what Defender and SmartScreen react to.
  The macOS app is signed ad hoc only; Developer ID signing and notarization
  are out of scope (D-LX14).
- **The WebView2 runtime.** Windows 11 has it and Windows 10 usually does.
  The release doesn't bundle it. When it is missing, the launcher asks at start
  whether to install it (about 2 MB, for the current user only). Yes downloads
  Microsoft's Evergreen Bootstrapper over HTTPS, refuses anything that isn't a
  Windows executable, runs it with `/silent /install` without elevation, waits
  at most 5 minutes, then checks the registry again; when the runtime is there,
  the launcher opens. While it works, an "Installing..." notice stays up (it
  comes back if dismissed and closes by itself), and a second launch shows
  "already setting up" and exits. No, or an install that didn't finish, shows
  the old dialog that offers Microsoft's download page (led by "The automatic
  install didn't finish." followed by the reason, after an attempt), then the
  launcher exits. It tries the install once per start. This runs before logging starts, so it writes no
  telemetry events.
- **macOS self-update.** Manual, above.
- **An Intel Mac build.** The macOS zip is arm64 only.
- **Linux self-update, Play and an Arm build.** Linux updates are manual (above);
  Play has no Wine or Proton backend; the AppImage is x86-64 only.
