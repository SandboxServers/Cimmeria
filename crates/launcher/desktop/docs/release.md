# Releasing the desktop launcher

> How-to and reference · desktop launcher maintainers · 2026-10-05

A maintainer comments `/release-launcher` on a merged PR. That dispatches
[`launcher-release.yml`](../../../../.github/workflows/launcher-release.yml),
which builds from `main` and publishes one GitHub release, tagged
`launcher-<date>-<sha7>`, with two launchers in it:

| Launcher | Asset | Job |
|---|---|---|
| The original launcher (`sgw-launcher`) | `sgw-launcher-<tag>.exe` and its `.sha256` | `release` |
| The desktop launcher, Windows | `stargate-worlds-launcher-desktop-windows-<tag>.zip` and its `.sha256` | `desktop-windows` |

The original launcher is published first. The desktop job runs after it and
only adds an asset, so if it fails the release stands as it was. The original
launcher updates itself from the exact name `sgw-launcher-<tag>.exe` and does
not see the zip.

The desktop launcher for Windows is a **preview**. The macOS build is not
released at all yet; [below](#macos-is-deferred) says why.

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
| `verify` | The launcher is 64-bit and carries each staged file's digest; the three files beside it are 32-bit and are the staged ones; the telemetry DLL is the player build; a tagged build carries its tag |
| `package <tag>` | Zips the launcher with the `windows/` and `graphics/` directories beside it, and writes the `.sha256` |

The launcher is not one file. At Play it reads the launch worker and the DLLs
from `windows/` beside the executable and refuses any whose digest is not the
one compiled in ([launch](launch.md)). That is why the pins are taken from the
staged files in the same job that compiles them in, and why the package is a
zip of that layout.

A release build also sets, on the build step only:

| Variable | Value | Why |
|---|---|---|
| `LAUNCHER_MANIFEST_PUBKEY_HEX` | the repository secret | The key the release manifest is signed with. Without it a release build can verify nothing. The test suites use the development key and fail with this set, so no other step has it |
| `CIMMERIA_LAUNCHER_TAG`, `CIMMERIA_LAUNCHER_BUILD_EPOCH` | the release's tag and stamp time | The same identity as the original launcher of that release, for a manifest's `min_launcher` |
| `CIMMERIA_BUILD_BRANCH`, `CIMMERIA_BUILD_GIT_SHA` | `main`, the commit | Recorded in a game-telemetry session |

## What the preview does not have

- **An installer.** The launcher's own update path hands off to an NSIS
  installer ([updater](updater.md)). Nothing builds one, and that path is
  untested, so this build cannot update itself. No update feed or signing key
  is configured either; Settings shows the updater as disabled.
- **A signature.** It is unsigned, like the original launcher. The launch
  worker injects DLLs into the game, which is what Defender and SmartScreen
  react to.
- **The WebView2 runtime.** Windows 11 has it and Windows 10 usually does.
  Nothing here installs it.
- **A release name.** The window title and the UI still say "development".
- **Broad testing.** One native Windows machine ran a debug build on
  2026-10-04: Install, Play, login and character select. Repair, uninstall,
  game Update and adopting an existing copy were not run on Windows. The
  release profile is built by CI and has not been run by a person.

## macOS is deferred

The workflow has no macOS job. The comment at the end of
`launcher-release.yml` lists what one needs; in short:

- A **Developer ID Application** certificate. The identity used so far is an
  Apple Development one on a maintainer's machine, which is good for that
  developer's own Macs only.
- **Notarization** with `notarytool` and stapling. Gatekeeper blocks a
  downloaded app without it.
- The **hardened runtime** and whatever entitlements starting Wine under it
  needs, which has not been worked out.
- The **Windows-built pieces** the Mac bundle carries (three workers and the
  two DLLs), passed from a Windows job and staged before compiling.
- A decision on **redistributing** the Wine runtime, the D3D9 layer and the
  x87 accelerator.

Until then Mac builds are made and signed locally for testing
([Wine validation](wine-validation.md#packaged-helper-staging-and-mac-build)).
