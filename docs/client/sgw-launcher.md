---
title: "SGW Launcher"
type: explanation
audience: engineers
last_updated: 2026-07-25
---

# SGW Launcher

A standalone Windows .exe that installs the SGW client from GitHub
Releases, applies declared patches in order, optionally launches the
debug-Atera path (with or without the dev-session telemetry pipeline),
and uploads debug logs to an Azure Blob SAS URL.

Located in [`crates/launcher/`](../../crates/launcher/) as the
`sgw-launcher` crate. Built with **eframe (egui)** for a small, native
window with no webview dependency.

> **Status:** rewritten 2026-05-20. Supersedes the Tauri prototype and the
> archive.org-RAR install flow described in
> [.claude/plans/2026-03-06-sgw-launcher-design.md](../../.claude/plans/2026-03-06-sgw-launcher-design.md)
> and the task-by-task plan in
> [.claude/plans/2026-03-06-sgw-launcher-plan.md](../../.claude/plans/2026-03-06-sgw-launcher-plan.md).
> Both are kept for historical context; do not implement from them.

---

## What the Launcher Does

| Function | Notes |
|----------|-------|
| **Fetch manifest** | Pulls `manifest.json` + `manifest.json.sig` from GitHub Releases (anonymous GET), and verifies the Ed25519 signature. |
| **Seed install** | Downloads the seed zip (the whole client) once, verifies sha256, extracts to the install dir. |
| **Patch install** | Walks declared patches in order; downloads + extracts each missing patch (overlay over existing files). |
| **Hostname patch** | Rewrites the 22-byte host slot in `SGW.exe` `.rdata` to the configured emulator host — the original `www.stargateworlds.com` literal on a fresh install, or the previously-written host on a re-patch. |
| **Launch SGW** | `CreateProcess(SGW.exe)`. |
| **Launch Atera Debug** | `cmd /C AtreaGameDebug.bat` (enabled only if Atera files were dropped into the install dir). |
| **Launch + Telemetry** | Same as Atera Debug, plus the dev-session telemetry pipeline — mints a token, injects `cimmeria-client-telemetry.dll` into SGW.exe, tails the client logs, and uploads chunks/bundles. See `src/telemetry/`, `src/inject.rs`, and [operations/telemetry.md](../operations/telemetry.md). |
| **Fix ASLR** | `cmd /C AtreaFixASLR.bat` (enabled only if the Atera fix-ASLR bat is present). |
| **Upload debug logs** | Zips `Binaries/sgwdebuglog*` + `Binaries/sessions/**` and PUTs once to the Azure log SAS URL. |

---

## Install Pipeline

```text
1. Fetch manifest.json AND manifest.json.sig from manifest_url
   (anonymous GitHub Releases GET — both URLs are HTTPS-enforced).
2. Verify the Ed25519 signature against the embedded public key. On
   mismatch, refuse the manifest entirely — no unsigned fallback.
3. Compare manifest.seed.sha256 vs installed.seed_sha256:
     - Mismatch → download seed blob, verify sha256, extract zip into
       install_path, reset applied_patches to [] and patched_host to
       None.
     - Match    → skip seed.
4. For each manifest.patches[*] not in installed.applied_patches, in
   order:
     - Download patch blob → verify sha256 → extract zip (overlay).
     - Append id to installed.applied_patches and persist.
5. Compare expected vs persisted patched_host (or detect the original
   CME literal still in the binary):
     - Differs → re-patch SGW.exe `.rdata`, atomically (.exe.patching
       + rename), record the new host in installed.patched_host.
```

Resumable downloads use HTTP `Range`: the launcher tracks `existing_len`
on disk under the tmp path (`<install>/.tmp-seed-<sha-prefix>.zip` or
`.tmp-patch-<id>-<sha-prefix>.zip` — sha included so a republished
patch with the same id but a new sha doesn't accidentally resume against
stale bytes) and asks the server for `bytes=<existing>-` so a killed
seed download picks up where it left off on next run.

Concurrency: a process-wide file lock at `<exe dir>/launcher.lock`
ensures only one launcher instance runs at a time, so two installs
can't race on the same `launcher-installed.json` or tmp file.

State files:

- `<install_path>/launcher-installed.json` — applied-patch ledger (in the
  game directory, so it survives launcher reinstalls and travels with the
  game).
- `<launcher.exe dir>/launcher-config.json` — schema version, install path,
  server host, manifest URL, and telemetry preferences.
- `<launcher.exe dir>/uploaded.json` — log-upload dedupe ledger.
- `<launcher.exe dir>/telemetry-state.json` — per-session telemetry runtime
  state, kept out of the config file so config rewrites don't churn it
  ([`config.rs`](../../crates/launcher/src/config.rs)::`telemetry_state_path`).

---

## Manifest Schema

```json
{
  "schema": 1,
  "seed": {
    "blob": "seed/sgw-0.8348.1.4046.zip",
    "size": 5234567890,
    "sha256": "abc..."
  },
  "patches": [
    { "id": "001-base",    "blob": "patches/001.zip", "size": 123,  "sha256": "...", "after": null },
    { "id": "002-mercury", "blob": "patches/002.zip", "size": 2345, "sha256": "...", "after": "001-base" }
  ]
}
```

- `schema` must be `1`. Bumping invalidates older launchers; serve a
  legacy manifest at the old URL during transitions.
- `blob` may be either an absolute `http(s)://` URL (passed through
  unchanged) or a path relative to the manifest URL's container, in which
  case the launcher derives `<base>/<blob>` by stripping everything after
  the final `/` in `manifest_url`. The GitHub Releases hosting model uses
  absolute URLs so the rolling `content-current` manifest can point at
  immutable per-publication release tags. See
  [`manifest.rs`](../../crates/launcher/src/manifest.rs)::`blob_url`.
- `after` is a forward-declaration check: every referenced patch id must
  have appeared earlier in the array. Order in `patches[]` **is** the
  application order.
- `size` is informational (drives the progress bar's `total` when the
  server doesn't return `Content-Length` for some reason).
- `sha256` is hex, lowercase, of the patch zip contents.

---

## SGW.exe Hostname Patch

CME's SOAP login hostname is hardcoded in SGW.exe's `.rdata` section
(Ghidra analysis confirms `www.stargateworlds.com`, 22 bytes). The
launcher byte-searches for that literal and overwrites it with the
configured `server_host`, zero-padded to 22 bytes. No PE checksum
recalculation needed for `.rdata` edits.

The replacement hostname must be ≤ 22 bytes. The patch is **not** a
simple "skip if the CME literal is absent" no-op: the launcher records
the host it last wrote in `launcher-installed.json`'s `patched_host`, and
`host_differs` re-patches when the configured `server_host` no longer
matches. On a re-patch it searches for the *previous* host as a padded
22-byte run rather than the original literal, so changing `server_host`
and re-running Install / Update correctly rewrites an already-patched
executable.

See [`crates/launcher/src/patch_rdata.rs`](../../crates/launcher/src/patch_rdata.rs).

**Why not hosts file?** Requires admin elevation and affects the whole
system. Direct PE patching is scoped to the game directory.

**Why not DLL injection (AtreaRL)?** Runtime injection requires ASLR
disabled and uses hardcoded patch addresses. The static `.rdata` patch
has no such constraints.

---

## Launch Surface

| Button | Enabled when | Action |
|---|---|---|
| **Launch SGW.exe** | `SGW.exe` exists | `CreateProcess(<install>/SGW.exe)` with `cwd = <install>` |
| **Launch Atera Debug** | `AteraLoader.exe` **and** `AtreaGameDebug.bat` both present | `cmd /C AtreaGameDebug.bat` (cwd = install dir) |
| **Launch + Telemetry** | Atera available, `telemetry.enabled`, and identity loaded | Atera debug launch plus the telemetry pipeline |
| **Fix ASLR** | `AtreaFixASLR.bat` present | `cmd /C AtreaFixASLR.bat` |

The Atera batch files are **not** shipped by the launcher. Players who
want the debug build drop the Atera tarball into the install directory
themselves; the launcher detects the files and surfaces the buttons.
The catalogue of what each bat does lives in
[docs/technical/atrealoader-exe.md](../technical/atrealoader-exe.md)
and [docs/technical/atrealoader-config.md](../technical/atrealoader-config.md).

Atera debug requires ASLR disabled on SGW.exe. The launcher does not
auto-run Fix ASLR — the user clicks the button once after a fresh
install, then the debug bat works on subsequent launches.

---

## Debug Log Upload

Single-PUT upload to Azure Blob via a SAS URL baked into the .exe at
build time (`LAUNCHER_LOG_SAS_URL` env, consumed by `option_env!`).

```text
Inputs   <install>/Binaries/sgwdebuglog*   (BigWorld unicode log)
         <install>/Binaries/sessions/**   (per-session logs)
Output   logs/<hostname>-<utc>-<digest12>.zip
Method   single PUT, x-ms-blob-type: BlockBlob, content-type: application/zip
Dedupe   sha256 of inputs (filename + bytes, sorted) → uploaded.json next to .exe
```

Wallet protection rules:

1. **One PUT per upload click**, never one-per-file. The zip is built
   in memory, hashed, and uploaded in a single call.
2. **Content digest, not zip-bytes hash.** The zip writer's per-entry
   timestamps differ between rebuilds; dedup uses a stable digest over
   `(rel_path, bytes)` pairs in sort order. So re-clicking with
   unchanged logs is free.
3. **Local ledger** at `<launcher.exe dir>/uploaded.json`. Already-seen
   digest → zero HTTP requests, button reports `"Already uploaded …"`.
4. **No background uploads.** Only fires on explicit button click.

The local dev / PR-build pipeline produces a launcher with
`LAUNCHER_LOG_SAS_URL = None`. The button is greyed out with a friendly
"Log upload disabled — built without LAUNCHER_LOG_SAS_URL" note. The
release workflow injects the secret.

See [docs/client/launcher-distribution-setup.md](launcher-distribution-setup.md)
for the operator side: GitHub Releases publish flow for content,
Ed25519 manifest signing setup, and the Azure Blob SAS for log uploads.

---

## Build

```bash
# Iteration (Windows host, native):
cargo build -p sgw-launcher

# Release with log upload enabled:
$env:LAUNCHER_LOG_SAS_URL = "<container-SAS-url>"
cargo build -p sgw-launcher --release

# Output: target/release/sgw-launcher.exe
```

The icon at [`crates/launcher/icons/icon.ico`](../../crates/launcher/icons/icon.ico)
is embedded as a Win32 resource via [`build.rs`](../../crates/launcher/build.rs).

---

## CI

Three GitHub Actions workflows mirror the server's pattern:

| Workflow | File | Trigger |
|---|---|---|
| **launcher** | [`.github/workflows/launcher-build.yml`](../../.github/workflows/launcher-build.yml) | Path-filtered fmt/clippy/build/test/coverage (five jobs; the `coverage` job runs `cargo llvm-cov`) on PRs touching `crates/launcher/**` or `.github/workflows/launcher-*.yml`. |
| **launcher-release** | [`.github/workflows/launcher-release.yml`](../../.github/workflows/launcher-release.yml) | `workflow_dispatch`. Builds release exe with `LAUNCHER_LOG_SAS_URL` injected from secrets, creates a GitHub Release tagged `launcher-<date>-<sha7>`. |
| **launcher-release-on-comment** | [`.github/workflows/launcher-release-on-comment.yml`](../../.github/workflows/launcher-release-on-comment.yml) | Mirror of `release-on-comment.yml` but matches `/release-launcher` on a merged PR. Validates commenter has write access, dispatches `launcher-release.yml`. |

Two repo secrets feed the release build: `LAUNCHER_LOG_SAS_URL` (log
upload) and `LAUNCHER_MANIFEST_PUBKEY_HEX` (the embedded Ed25519 manifest
verification key). PR / build jobs deliberately omit both; only
`launcher-release` reads them. Without `LAUNCHER_LOG_SAS_URL` the release
exe still builds and log upload is permanently disabled; without
`LAUNCHER_MANIFEST_PUBKEY_HEX`, manifest verification fails closed with
`ManifestError::SigningKeyUnavailable`.

The launcher is **excluded** from the main `ci` workflow ([`test.yml`](../../.github/workflows/test.yml))
via the `WORKSPACE_EXCLUDES` env (`--exclude sgw-launcher`) so eframe's
Linux system deps don't slow the rest of the workspace pipeline.

---

## File Layout

```text
crates/launcher/
├── Cargo.toml
├── build.rs                    # winres icon embed
├── binaries/                   # bundled 7za executables
├── gen/schemas/                # windows-schema.json
├── icons/
│   └── icon.ico
└── src/
    ├── main.rs                 # eframe entry, tokio runtime
    ├── app/
    │   ├── mod.rs              # eframe::App — state machine
    │   └── view.rs             # panel rendering
    ├── config.rs               # LauncherConfig (next to .exe)
    ├── manifest.rs             # Manifest schema + fetch + Ed25519 verify
    ├── install.rs              # seed + patches + .rdata patch orchestration
    ├── patch_rdata.rs          # SGW.exe hostname byte-patch
    ├── launch.rs               # SGW.exe + Atera bat detection & spawn
    ├── client_paths.rs         # install-dir path resolution
    ├── identity.rs             # stable per-install identity
    ├── inject.rs               # cimmeria-client-telemetry.dll injection
    ├── logs.rs                 # log collection + zip + Azure PUT
    ├── state.rs                # InstalledState + UploadedLedger
    ├── telemetry/
    │   ├── mod.rs
    │   ├── auth.rs             # dev-session token mint / refresh
    │   ├── session.rs          # session lifecycle
    │   ├── runner.rs           # pipeline driver
    │   ├── tail.rs             # client log tailing
    │   ├── events.rs           # parsed event shapes
    │   ├── queue.rs            # buffering
    │   ├── chunk.rs            # upload-chunk
    │   ├── bundle.rs           # end-of-session upload-bundle
    │   └── process_watch.rs    # game-exit detection
    └── worker/
        ├── mod.rs              # tokio worker
        └── messages.rs         # Command/Event channel types
```

11 top-level files plus three module directories. `app/`, `worker/`, and
`telemetry/` were each promoted from a flat file once they crossed the
4-siblings-on-one-theme threshold in
[CLAUDE.md's file organization rules](../../CLAUDE.md).
