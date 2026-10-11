# Launcher release builds

Staged build scripts for the two launchers. The release workflow (`launcher-release.yml`) runs them; `launcher-build.yml` and `launcher-desktop.yml` run the same stages on every launcher PR as a dry run.

| Entry point | Launcher | Stages |
|---|---|---|
| `build.sh <stage>` | `sgw-launcher` (`crates/launcher`) | `i686`, `launcher`, `verify`, `overlay <url>`, `paths` |
| `build-desktop.sh <stage>` | desktop launcher (`crates/launcher/desktop`) | `helpers`, `stage`, `launcher`, `verify`, `package`, `feed-tool`, `sign`, `verify-feed`, `feed-dry-run`, `paths` |

Each script's header comment documents its stages and environment variables. Bash only; CI runs them on Windows runners. Release setup, signing keys and the content feed: [launcher-distribution-setup.md](../../docs/client/launcher-distribution-setup.md).
