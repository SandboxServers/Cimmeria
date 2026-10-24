# Launcher release builds

Staged build scripts for the desktop launcher (`crates/launcher/desktop`). The release workflow (`launcher-release.yml`) runs them; `launcher-desktop.yml` runs the same stages on every desktop-launcher PR as a dry run. The egui launcher and its `build.sh` were retired in LX-18.

| Entry point | Platform | Stages |
|---|---|---|
| `build-desktop.sh <stage>` | Windows, x64 or arm64 (`WINDOWS_ARCH`, default x64), and the Windows-built pieces the Mac app carries | `helpers`, `stage`, `launcher`, `verify`, `package`, `mac-helpers`, `mac-pieces`, `play-pieces`, `play-pieces-in`, `feed-tool`, `sign`, `verify-feed`, `feed-dry-run`, `overlay <url>`, `paths` |
| `build-desktop-macos.sh <stage>` | macOS, ad-hoc signed | `pieces`, `d3d9`, `app`, `sign`, `verify`, `package`, `paths` |

`build-desktop-macos.sh` builds the desktop launcher for macOS in stages, as an ad-hoc signed app. `stamp.sh` and `publish.sh` are the first and the publishing steps of `launcher-release.yml`; `test-release-scripts.sh` tests them against a stub `gh`.

`sign` signs both Windows zips and writes one feed; `verify-feed` and `feed-dry-run` check both.

Each script's header comment documents its stages and environment variables. Bash only. Release setup, signing keys and the content feed: [launcher-distribution-setup.md](../../docs/client/launcher-distribution-setup.md).
