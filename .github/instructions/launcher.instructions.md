---
applyTo: "crates/launcher/**"
---

# Launcher review rules

`crates/launcher/src/` is `sgw-launcher`, the Windows launcher players run
today. A behaviour change there reaches every current player on their next
update, so call out any change to what an existing install does. The desktop
launcher under `crates/launcher/desktop/` is a separate Cargo workspace with
its own lockfile and its own workflow (`.github/workflows/launcher-desktop.yml`).
The root workspace's fmt, clippy and hakari checks do not cover it.

## Integrity

- Verify a download's size and SHA-256 before anything opens, extracts or
  executes it. Flag code that hashes one handle and then reopens the file by
  path, unless the first handle denies writes until the use is over.
- A resumed (HTTP Range) download needs the existing prefix verified, or the
  whole file hashed again after the resume.
- Extraction must refuse absolute paths, `..` components and link entries that
  point outside the destination.
- A compiled-in hash pin is the trust anchor. A receipt or staging file that
  the build writes next to the artefact is not one.

## Destructive operations

- Deletion must stay inside a root the launcher can prove it owns. Check for
  symlinks, Windows junctions and other reparse points, case-insensitive
  path comparisons, and races between validation and removal.
- Windows reparse points are not all links. OneDrive Files On-Demand
  placeholders and deduplicated files carry the reparse attribute too, so
  test the reparse tag, not just the attribute.
- Files the OS writes into a folder (`.DS_Store`, `._*`, `desktop.ini`,
  `Thumbs.db`) should not veto an uninstall or cleanup.

## Liveness

Every interrupted, failed or uncertain state the launcher records needs a way
out that the user can reach from the UI. Walk every error, crash and timeout
path, including a crash between two writes. If the only remedy is editing
the state files, flag it. A test that asserts the gate holds also has to
show the escape.

## Long operations

- Bound large transfers and extractions by inactivity (no progress for N
  seconds), not by total time. A total timeout on a multi-GB download or a
  Wine extraction fails on slow links and disks.
- Long work must report progress and honour cancellation. Flag a blocking
  call that hashes or deletes gigabytes while it holds a lock the status UI
  also needs.

## Desktop IPC

Webview commands should carry operation identities and revisions. Flag any
command that takes a path, URL, executable or manifest from the page, and
any PR text that claims the IPC accepts none of those when it does.

## Builds

Desktop builds go through `tools/build-lane/lane.sh` like every other cargo
call. Flag a script that runs `cargo` directly, or that passes `--target-dir`
and so bypasses the lane's per-worktree target directory and its pruning.
A path dependency on a root-workspace crate pulls `cimmeria-workspace-hack`
into the desktop lockfile, so check `.config/hakari.toml` `[final-excludes]`.
