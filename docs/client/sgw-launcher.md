---
title: "SGW Launcher"
type: explanation
audience: engineers
last_updated: 2026-09-29
---

# SGW Launcher

A standalone Windows .exe that installs the SGW client from GitHub
Releases, applies declared patches in order, optionally launches the
debug-Atera path (with or without the dev-session telemetry pipeline),
and uploads debug logs to an Azure Blob SAS URL.

The approved cross-platform replacement is being implemented separately in
[`crates/launcher/desktop/`](../../crates/launcher/desktop/README.md). Its native
first-install worker prepares content through the shared pipeline, but its Tauri
UI now connects installation, cancellation and explicit recovery through Effect
on Windows and verified-helper Mac builds, with cached evidence for identical
native retries.
It reports content preparation separately from runtime readiness; launch, repair
and removal remain unavailable. Explicitly confirmed cleanup can remove one
failed/cancelled attempt's owned partial files before a separate retry; it is
not uninstall. Mac builds without the compiled-and-bundled
verified helper cannot install; Wine resume/recovery remains unsupported. Runtime provisioning and game validation remain
separate gates. The existing Windows launcher described here
remains the functional user-facing implementation.

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
| **Seed install** | Downloads the seed (the whole client) once, verifies sha256, unpacks it into the install dir. The seed is a zip, or the archive.org client RAR, whose installer cabinets (`Data\DATA1-4.CAB`) are expanded straight into the installed layout. |
| **Patch install** | Walks declared patches in order; downloads + unpacks each missing patch: overlay files, and patch sets whose deltas rebuild files from the player's own stock copies (`cimmeria-patchset`). |
| **Client setup** | Restores the stock spelling of patched files (`EULA.lua`), writes the configured login servers into `LoginInternal.lua` and switches ASLR off in `SGW.exe`, after every install and before every launch (`src/client_setup/`). |
| **Launch SGW** | Starts `SGW.exe` suspended, injects `cimmeria-client-patches.dll` (unless the player turned it off), and resumes it. With telemetry on, a telemetry session follows the game. See [Client patches DLL](#client-patches-dll). |
| **Launch Atera Debug** | `cmd /C AtreaGameDebug.bat` (enabled only if Atera files were dropped into the install dir). |
| **Launch + Telemetry** | Same as Atera Debug, plus the dev-session telemetry pipeline — mints a token, tails the client logs, and uploads chunks/bundles. It injects no DLL: the Atera bat starts `SGW.exe` itself. See `src/telemetry/` and [operations/telemetry.md](../operations/telemetry.md). |
| **Fix ASLR** | `cmd /C AtreaFixASLR.bat` (enabled only if the Atera fix-ASLR bat is present). |
| **Upload debug logs** | Zips `sgwdebuglog*` (case-blind) + `sessions/**` from the binaries directory and PUTs once to the Azure log SAS URL. |
| **Self-update** | Checks GitHub Releases for a newer `launcher-*` release at startup and, on one click, downloads it, verifies it against the release's `.sha256`, swaps it in for the running exe and relaunches. See [Self-update](#self-update-srcself_update). |

The launch buttons, client setup, adoption and the log upload all
find the game through `src/install_layout.rs`, which resolves the
directory holding `SGW.exe`: `<install>\Working\Binaries` in a full
install, or the install path itself when it points straight at it.

---

## Install Pipeline

```text
1. Fetch manifest.json AND manifest.json.sig from manifest_url
   (anonymous GitHub Releases GET — both URLs are HTTPS-enforced).
2. Verify the Ed25519 signature against the embedded public key. On
   mismatch, refuse the manifest entirely — no unsigned fallback.
3. Compare manifest.seed.sha256 vs installed.seed_sha256:
     - Mismatch → download seed blob, verify sha256, unpack into
       install_path, reset applied_patches to [].
     - Match    → skip seed.
4. Rename Working\SGWGame\Cache.en-US to SourceCache.en-us if needed.
5. For each manifest.patches[*] not in installed.applied_patches, in
   order:
     - Download patch blob → verify sha256 → unpack (overlay, or a
       patch set's deltas) into the install dir, or into the client's
       SGWGame/ directory for a "root": "sgw_game" patch.
     - Append the id (<id>@sgw_game for a sgw_game patch) to
       installed.applied_patches and persist.
6. Client setup: stock file names, LoginInternal.lua and ASLR (see below).
```

A complete-size, SHA-256-authenticated cached seed is reused before making an
HTTP request. A complete cache with a confirmed hash mismatch is removed before a fresh
non-Range download; partial downloads retain Range handling and verification.
Resumable downloads use HTTP `Range`: the launcher tracks `existing_len`
on disk under the tmp path (`<install>/.tmp-seed-<sha-prefix>.download` or
`.tmp-patch-<id>-<sha-prefix>.download` — sha included so a republished
patch with the same id but a new sha doesn't accidentally resume against
stale bytes) and asks the server for `bytes=<existing>-` so a killed
seed download picks up where it left off on next run. A `416` for a
full-length file counts as downloaded, and a file that fails its hash is
deleted, so a bad download can't wedge every later attempt.

Cancellation interrupts waits for HTTP response headers and each response-body
chunk; a stalled response does not require another byte before stopping.
Extraction cancellation is propagated as cancellation rather than an ordinary
patch failure. This does not make every extraction/preparation step immediately
interruptible.

### Unpacking (`src/unpack/`)

The archive format comes from the file's magic bytes, not its name:

- **zip** — extracted directly (`enclosed_name` is the zip-slip gate).
- **RAR** — extracted with the `unrar` crate (RARLAB's UnRAR library)
  into `<install>/.tmp-unpack/`. Entry names are checked before writing.
  If the staged tree holds a MakeCAB cabinet set — an `.INF` with a
  `[cabinet list]` and `[file list]`, which is how the 2009 installer
  (`SetupQA.exe` + `Data\DATA.INF` + `DATA1-4.CAB`) ships its payload —
  the cabinets are expanded in order into the install dir. Otherwise the
  staged files are moved across. The staging dir is removed afterwards.

After cabinet expansion, shared unpack preserves optional `Data/Prerequisites`
as `<extraction destination>/.cimmeria-prerequisites`, alongside `Working`, before
removing staging. These are inert vendor files; nothing is probed or executed.
Publication uses a same-volume rename. An existing retained tree is accepted only
when relative paths, types, file sizes and SHA-256 hashes match exactly; extra or
changed files, links, special files and Windows reparse points are refused.
Cancellation is checked during inventory/hash reads and before publication.
The currently staged Windows helper predates this change: rebuilding it and a
real retention smoke remain required. Existing completed-install receipts do not
prove prerequisites were retained or installed.

Cabinets are expanded with Windows' FDI API (`FDICreate`/`FDICopy` in
`cabinet.dll`, `src/unpack/fdi.rs`), because the installer's cabinets
are 1 GiB volumes with files continued across them, which pure-Rust cab
readers don't follow. The expansion fails if fewer files come out than
the INF lists. Hashing and unpacking run under `spawn_blocking`.

Extracted files keep the modified time the archive records, the way the
stock installer and `expand.exe` do. Cabinet and zip entries carry an
MS-DOS date/time in the builder's local time: `src/unpack/dos_time.rs`
converts it with `DosDateTimeToFileTime` then `LocalFileTimeToFileTime`,
the same calls as Windows' own extractors, and sets the modified and
accessed times. An invalid stamp, or the zip crate's 1980-01-01
"no time" placeholder, keeps the extraction time and never fails the
install. UnRAR restores RAR entry times itself, and the move out of
`.tmp-unpack/` is a rename, which keeps them. Files a patch set rebuilds
by delta are the launcher's own output and keep their write time.

This matters because Unreal Engine 3 stores each `Default*.ini`
timestamp in the `[INIVersion]` section of the player's generated
`Documents\My Games\Firesky\SGWGame\Config\SGW*.ini` and, when one
differs, asks on launch whether to regenerate the "outdated" ini. The
2009 cabinets date the client 2009-06-30, so a launcher that wrote
install-time mtimes triggered that dialog for anyone who had run SGW
before.

On every install, `install_layout::place_bundled_cooked_data` renames
`Working\SGWGame\Cache.en-US` (where the cabinets put the bundled PAKs)
to `SourceCache.en-us`, the read-only tier the client reads through
`SourceCachePath`. See the cache-tier table in
[launcher-guide.md](launcher-guide.md#install-layout).

Concurrency: a process-wide file lock at `<exe dir>/launcher.lock`
ensures only one launcher instance runs at a time, so two installs
can't race on the same `launcher-installed.json` or tmp file.

State files:

- `<install_path>/launcher-installed.json` — applied-patch ledger (in the
  game directory, so it survives launcher reinstalls and travels with the
  game).
- `<launcher.exe dir>/launcher-config.json` — schema version, install path,
  login servers, manifest URL, and telemetry preferences.
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
- `root` (optional) is where the zip's entries go. Omitted or
  `"install_dir"`: the install directory, the one holding `SGW.exe`.
  `"sgw_game"`: the client's `SGWGame/` directory, found as
  `<install>/SGWGame` or, in the stock tree where `SGW.exe` is in
  `Working/Binaries/`, as `<install>/../SGWGame`. A `sgw_game` patch
  with neither fails the install. A launcher older than this field
  extracts such a patch into the install directory and records its id,
  so the launcher records a `sgw_game` patch as `<id>@sgw_game` and a
  newer launcher applies it again in the right place. The
  client-patches UI overlay ships this way; see
  [`patch_dest.rs`](../../crates/launcher/src/patch_dest.rs).
- `title` and `description` (optional) name the patch and say what it
  changes, for the launcher's [Changes to your client](#changes-to-your-client)
  list. They are signed with the rest of the manifest, and older
  launchers ignore them. `cimmeria-patchset build` copies them from the
  patch spec, and `pack-client-overlay` writes the overlay's.

---

## Client Setup (`src/client_setup/`)

**Stock file names.** The game looks some UI resources up
case-sensitively, even on Windows: with `EULA.lua` spelled `eula.lua`
its CEGUI resource provider reports `'EULA.lua' does not exist in group
lua`, and the login screen never appears. `launcher-20260929-f518b57`
did exactly that, because the published `005-login-delay` recipe spells
the file `eula.lua` and `cimmeria-patchset` wrote rebuilt files through a
rename under the recipe's spelling. `client_setup::stock_case` renames
every file in its `PATCH_TARGETS` list (each path a patch set writes,
spelled as the 2009 cabinets' `DATA.INF` spells it) back to that
spelling when only the case differs, so installs made with that launcher
repair themselves. A test keeps the list in step with every
`data/client-patches/*/patch.json` op target.

**Login servers.** The login screen's server list comes from
`LoginMod.loadServerSystems()` in
`Working\SGWGame\Content\UI\Startup\Login\LoginInternal.lua`. The stock
file lists CME's dead QA and production login servers; the launcher
rewrites it from the config's `login_servers` (`[{name, url}]`, default
`Cimmeria = http://play.cimmeria.app:8081`), CRLF and ASCII, only when the
content changes. Names and URLs with a quote or backslash are refused, so
a setting can't break out of the Lua string.

**ASLR.** `IMAGE_DLLCHARACTERISTICS_DYNAMIC_BASE` (0x0040) is cleared in
`SGW.exe`'s optional header (one byte, 0x186 in the 0.8348 build), because
the client-patches DLL and every RE address in `docs/` assume the image
base `0x00400000`. The stock exe with that byte cleared is byte-identical
to a known-good QA client's (`client_setup::aslr` has an opt-in test).

**The retired `.rdata` hostname patch.** Earlier launcher builds
overwrote `www.stargateworlds.com` in `SGW.exe`. Its only ASCII
occurrence is inside the SOAP namespace
`http://www.stargateworlds.com/xml/sgwlogin`, so the patch redirected
nothing and would have broken the namespace the auth server's requests
carry. No released launcher ran it; it was removed with its state field
(`patched_host`, now ignored) and the `server_host` setting.

## Changes to Your Client

The launcher's **Changes to your client** section lists every way it
makes the player's client differ from the stock 2009 install, whether
the launcher downloaded that install or the player pointed it at their
own copy. It is open by default until the launcher manages a client,
so a player sees the list before **Install / Update** or **Adopt**.
Nothing in it can be switched off except the two launch rows; the
patches are what Cimmeria needs to play.

The rows come from one pure function,
[`client_changes.rs`](../../crates/launcher/src/client_changes.rs)::`list`,
in four groups:

| Group | Rows |
|---|---|
| Launcher setup | `LoginInternal.lua` login servers, ASLR off and stock file names (all before every launch), `Cache.en-US` renamed to `SourceCache.en-us` (at install) |
| Patched files | One row per manifest patch, in manifest order, **applied** or **applied on the next Install / Update** from `launcher-installed.json` |
| Added when the game starts | The client-patches DLL (on, or off when **Load client patches** is off) and telemetry (off unless the player opted in; when on, it loads `cimmeria-client-telemetry.dll` too) |
| Sent by the server while you play | The server's cooked data in `Documents\My Games\Firesky\SGWGame\Cache.en-US` |

A patch row uses the manifest's `title` and `description` when present,
else the launcher's built-in text for the patches published before those
fields existed (`builtin_description`), else its id with "no description
yet". Every manifest patch gets a row, described or not. The test
`builtin_catalog_matches_every_patch_spec` fails when a spec under
`data/client-patches/` has no built-in text or its text differs from the
spec's.

### Telemetry is opt-in

`telemetry.opted_in` defaults to `false`. Until the player answers it,
a "Help us fix bugs? (optional)" prompt sits at the top of the window
with **Turn on telemetry** and **No thanks**; either answer sets
`telemetry.prompt_answered` and saves. The **Send telemetry (opt-in)**
checkbox beside the launch buttons changes the choice later. Both write
the status log line. The field used to be `enabled`, default `true`,
and every launcher that saved its config wrote `"enabled": true`
without asking; the rename means those configs load opted out.

Opting in also loads the telemetry DLL into the game (owner decision
2026-09-29). On **Launch SGW.exe** the launcher starts the telemetry
session first (a handshake of at most 10 s that writes
`current-session.json`, which the DLL reads as it boots), then starts
`SGW.exe` with the client-patches DLL and, after it,
`cimmeria-client-telemetry.dll`. Release launchers embed the player
build of the DLL (no `lab-bridge` feature) and write it to
`<launcher dir>/client-telemetry/<sha256 prefix>/`; a dev launcher uses
one beside itself ([`client_telemetry_dll.rs`](../../crates/launcher/src/client_telemetry_dll.rs)).
A `lab-bridge` build is refused. When the session cannot start, or the
DLL is missing or refused, the status log says so (`In-game telemetry:
unavailable: …`) and the game starts without it; if injecting both DLLs
fails, the launcher retries with the client patches alone, then plainly.
Opted out, the launch is the client-patches launch and nothing else. A
change to the checkbox applies from the next launch.

## Patch Sets (`cimmeria-patchset`)

A patch zip with a `cimmeria-patch.json` recipe rebuilds files from the
player's own stock client instead of shipping them, so the project never
hosts CME bytes. Each op names stock sources (with SHA-256), an optional
transform, a bsdiff delta and the result's SHA-256. `upk_normalize`
decompresses a stock package and writes it back through `cimmeria-upk`'s
append-only patcher, the starting point of every map our `upk_patch`
tool built, so a 4 MB map's delta is under 2 KB. The launcher computes all
ops before writing any, writes targets other ops read last, and skips ops
whose target already has the result. Every path is resolved against the
install's own directory listing first, so a file that exists keeps its
on-disk name whatever case the recipe uses, and `build` refuses a spec
path spelled in another case than the stock tree (`CaseMismatch`). The
shipped patches live in
[`data/client-patches/`](../../data/client-patches/README.md).

---

## Launch Surface

| Button | Enabled when | Action |
|---|---|---|
| **Launch SGW.exe** | `SGW.exe` exists | `SGW.exe` started suspended with `cwd = <install>`, the client-patches DLL injected, then resumed. When the player opted in (`telemetry.opted_in`) and the identity loaded, a telemetry session starts first, the telemetry DLL goes in after the client patches, and the session follows the game |
| **Launch Atera Debug** | `AteraLoader.exe` **and** `AtreaGameDebug.bat` both present | `cmd /C AtreaGameDebug.bat` (cwd = install dir) |
| **Launch + Telemetry** | Atera available, `telemetry.opted_in`, and identity loaded | Atera debug launch plus the telemetry pipeline |
| **Fix ASLR** | `AtreaFixASLR.bat` present | `cmd /C AtreaFixASLR.bat` |

The Atera batch files are **not** shipped by the launcher. Players who
want the debug build drop the Atera tarball into the install directory
themselves; the launcher detects the files and surfaces the buttons.
The catalogue of what each bat does lives in
[docs/technical/atrealoader-exe.md](../technical/atrealoader-exe.md)
and [docs/technical/atrealoader-config.md](../technical/atrealoader-config.md).

Atera debug requires ASLR disabled on SGW.exe. The launcher's client
setup now clears it itself before every launch, so the **Fix ASLR**
button is only needed for installs the launcher has never launched.

### Client patches DLL

`cimmeria-client-patches.dll` restores client features the 2009 client
shipped unfinished, first the Black Market window. The decision record
is [client-patches.md](../architecture/client-patches.md). On **Launch
SGW.exe** the launcher:

1. Decides whether to load it ([`client_patches/plan.rs`](../../crates/launcher/src/client_patches/plan.rs)).
   The checkbox **Load client patches (restores the Black Market
   window)** under the launch buttons is `client_patches.enabled` in
   `launcher-config.json`. It is on by default, saved as soon as it
   changes, and independent of the telemetry opt-in.
2. Finds the DLL ([`client_patches/dll_source.rs`](../../crates/launcher/src/client_patches/dll_source.rs)),
   in this order: `client_patches.dll_override` (a tester's own build);
   the copy embedded in release launchers, written to
   `<launcher dir>/client-patches/<sha256 prefix>/cimmeria-client-patches.dll`
   so that a DLL still loaded by a running game is never overwritten;
   then `cimmeria-client-patches.dll` beside the launcher (dev builds
   embed nothing).
3. Runs the 32-bit `sgw-start32.exe` helper, which starts `SGW.exe`
   suspended, injects the DLL, resumes it and reports the pid; the
   launcher then opens that pid to follow the game (see **Bitness**
   below). If injection fails, the helper kills the suspended process
   and the launcher starts the game without the DLL.

Every launch that does not load the DLL says why in the status log
(`Client patches: off (launcher setting)…`, `…unavailable: …`, or
`…not loaded (…)`), so a missing Black Market window is never silent.
Atera debug launches never load it, because the bat starts `SGW.exe`
itself.

**Bitness.** Injection hands a remote thread the injector's own
`LoadLibraryW` address, which only exists in a process of the same
bitness. The launcher is 64-bit and `SGW.exe` is 32-bit, and a 64-bit
process cannot reach the target's 32-bit `LoadLibraryW` either: a
process created suspended has no 32-bit kernel32 mapped yet, and a
thread a 64-bit process starts there runs in 64-bit mode. So the
launcher runs `sgw-start32.exe`, a 32-bit helper (crate
[`cimmeria-start32`](../../crates/start32/), linking only
`cimmeria-client-launch`) that does the suspended launch and the
injection at the right bitness. Release builds embed it and keep it at
one stable path, `<launcher dir>/sgw-start32.exe`, rewritten only when a
new launcher carries different bytes. It has a version resource and an
`asInvoker` manifest, and it is never written to `%TEMP%`, so an
antivirus exclusion for the launcher's folder keeps working across
updates (see the
[launcher guide](launcher-guide.md#windows-defender-or-smartscreen-blocks-the-launcher)).
Releases are unsigned (code signing is deferred). Its
command-line contract is in the
[client-launch README](../../crates/client-launch/README.md); the
telemetry DLL and `cimmeria-lab` reuse it. A direct injection across
bitness is refused with `BitnessMismatch` instead of failing as a bare
`RemoteLoadFailed`. A dev build with no helper says `…unavailable…
sgw-start32.exe…` and starts the game without the DLL.

**Code signing: deferred.** The owner has deferred code signing, so
releases ship unsigned (the release notes say so) and rely on the
measures above plus the
[Defender / SmartScreen workaround](launcher-guide.md#windows-defender-or-smartscreen-blocks-the-launcher).
The two routes on the table for later:

- **SignPath Foundation**: free signing for open-source projects, but it
  needs an OSI-approved license on the repository first.
- **Azure Trusted Signing** (now Artifact Signing): a small monthly fee,
  and GitHub Actions signs in over OIDC, so no signing secret is stored.

Either would sign the launcher, `sgw-start32.exe` and the DLLs (the i686
artifacts before the launcher embeds them), between the stages of
`tools/launcher-release/build.sh`. A PFX-in-a-secret setup is not an
option for a new certificate: since June 2023 publicly trusted
code-signing keys must be generated and kept on hardware (an HSM or a
token), so they cannot be exported as a `.pfx`.

**Order with the telemetry DLL.** Both DLLs MinHook `FEngineLoop::Tick`
and the drop callee. When both go in (an opted-in **Launch SGW.exe**,
and every lab launch), the client-patches DLL goes first. It hooks straight away and normally finds the stock prologues;
the telemetry DLL reads its session file first, then chains on top.
`injection_order` pins this order.

**Telemetry.** With telemetry on, the session reads
`cimmeria-client-patches.log` next to `SGW.exe` and records one
`client.patches.boot` event per session. It carries the launcher's
`injection` outcome (`injected`, `opted_out`, `unavailable`,
`inject_failed`), the DLL version, `fingerprint.<site>` for each hooked
site, `fingerprint_ok`, and the `verdict` (`installed`,
`nothing_installed`, `hook_failed`). See
[`telemetry/patch_log.rs`](../../crates/launcher/src/telemetry/patch_log.rs).
The same session records `client.patches.counts`, the DLL's claimed /
delivered / dropped counts with reasons
([`telemetry/patch_counts.rs`](../../crates/launcher/src/telemetry/patch_counts.rs)),
and an Install / Update run by an opted-in player queues one
`client.launcher.install_result` event with every patch's outcome
([`telemetry/install_result.rs`](../../crates/launcher/src/telemetry/install_result.rs)).
Both are described in
[dev-session-telemetry.md](../architecture/dev-session-telemetry.md#client-patches-counts-event).

---

## Debug Log Upload

Single-PUT upload to Azure Blob via a SAS URL baked into the .exe at
build time (`LAUNCHER_LOG_SAS_URL` env, consumed by `option_env!`).

```text
Inputs   <binaries>/sgwdebuglog*   (BigWorld unicode log, case-blind)
         <binaries>/sessions/**   (per-session logs)
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

## Self-update (`src/self_update/`)

The complete updater contract, release identity, trust checks, swap/rollback flow
and implementation details are in [Launcher self-update](launcher-self-update.md).
Player-facing behavior remains in the [launcher guide](launcher-guide.md#launcher-updates).

## Build

```bash
# Iteration (Windows host, native). Embeds nothing: it loads the patches
# only if cimmeria-client-patches.dll and sgw-start32.exe sit beside it.
cargo build -p sgw-launcher

# A launcher that injects, built the way the release workflow builds it:
# the i686 DLL and helper first, then the 64-bit launcher embedding both.
cargo build -p cimmeria-client-patches --release --target i686-pc-windows-msvc
cargo build -p cimmeria-start32 --release --target i686-pc-windows-msvc
CIMMERIA_CLIENT_PATCHES_DLL=$PWD/target/i686-pc-windows-msvc/release/cimmeria_client_patches.dll \
CIMMERIA_START32_EXE=$PWD/target/i686-pc-windows-msvc/release/sgw-start32.exe \
  cargo build -p sgw-launcher --release

# Output: target/release/sgw-launcher.exe (64-bit)
```

`LAUNCHER_LOG_SAS_URL` enables log upload in any of these builds.
Without `CIMMERIA_CLIENT_PATCHES_DLL` and `CIMMERIA_START32_EXE` the
launcher embeds neither and looks for each beside itself.

The release workflow runs these steps through
[`tools/launcher-release/build.sh`](../../tools/launcher-release/build.sh)
(`i686`, `launcher`, `verify`, `overlay`); `PROFILE=dev` runs them with the dev profile.

The same package builds `pack-client-overlay`, the release tool that
packs the client-patches UI overlay into a manifest patch; see
[launcher-distribution-setup.md](launcher-distribution-setup.md#publishing-the-client-patches-ui-overlay).

The icon at [`crates/launcher/icons/icon.ico`](../../crates/launcher/icons/icon.ico)
is embedded as a Win32 resource via [`build.rs`](../../crates/launcher/build.rs),
which also embeds the client-patches DLL and the `sgw-start32` helper
when `CIMMERIA_CLIENT_PATCHES_DLL` and `CIMMERIA_START32_EXE` are set. A
path that is not a PE image fails the build.

---

## CI

Three GitHub Actions workflows mirror the server's pattern:

| Workflow | File | Trigger |
|---|---|---|
| **launcher** | [`.github/workflows/launcher-build.yml`](../../.github/workflows/launcher-build.yml) | Path-filtered fmt/clippy/build/test/coverage (five jobs; the `coverage` job runs `cargo llvm-cov`) on PRs touching `crates/launcher/**`, `crates/client-launch/**`, `crates/client-patches/overlay/**` or `.github/workflows/launcher-*.yml`. Clippy covers `cimmeria-client-launch` and `cimmeria-start32` too. The test job builds the i686 `sgw-start32` helper and runs the x64 tests with `CIMMERIA_TEST_START32` set, so the helper injects a real DLL into a real 32-bit process. A `release-dry-run` job runs the release build stages (`tools/launcher-release/build.sh`) without signing or publishing, because the release workflow itself only runs on a release. |
| **launcher-release** | [`.github/workflows/launcher-release.yml`](../../.github/workflows/launcher-release.yml) | `workflow_dispatch`. Builds the i686 client-patches DLL and `sgw-start32` helper, then the 64-bit launcher embedding both with `LAUNCHER_LOG_SAS_URL` injected from secrets, verifies them, packs the UI overlay, and creates a GitHub Release tagged `launcher-<date>-<sha7>` with the exe, its `.sha256` (what self-update verifies against) and, when there is an overlay, its patch zip and `.entry.json`. The tag and the build time are stamped before the build and embedded in the launcher (`CIMMERIA_LAUNCHER_TAG`, `CIMMERIA_LAUNCHER_BUILD_EPOCH`). |
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
├── build.rs                    # winres icon embed + client-patches DLL embed
├── binaries/                   # bundled 7za executables
├── gen/schemas/                # windows-schema.json
├── icons/
│   └── icon.ico
└── src/
    ├── main.rs                 # eframe entry, tokio runtime
    ├── app/
    │   ├── mod.rs              # eframe::App — state machine
    │   ├── view.rs             # panel rendering
    │   ├── telemetry_panel.rs  # telemetry opt-in prompt + checkbox
    │   ├── update_banner.rs    # self-update banner + min_launcher gate
    │   └── client_changes_panel.rs  # "Changes to your client" list
    ├── client_changes.rs       # every deviation from the stock client
    ├── config.rs               # LauncherConfig (next to .exe)
    ├── manifest.rs             # Manifest schema + fetch + Ed25519 verify
    ├── install.rs              # seed + patches + client setup orchestration
    ├── install_tests.rs        # its tests
    ├── install_report.rs       # per-patch outcomes of one install run (telemetry)
    ├── client_setup/
    │   ├── mod.rs              # prepare(): run the three steps
    │   ├── stock_case.rs       # rename eula.lua etc. back to the stock case
    │   ├── login_servers.rs    # LoginInternal.lua from the config
    │   └── aslr.rs             # clear DYNAMIC_BASE in SGW.exe
    ├── install_layout.rs       # where SGW.exe lives (Working\Binaries)
    ├── unpack/
    │   ├── mod.rs              # format detection, staging, dispatch
    │   ├── zip.rs              # zip extraction
    │   ├── rar.rs              # RAR extraction (unrar)
    │   ├── cab_set.rs          # MakeCAB installer INF + cabinet set
    │   ├── fdi.rs              # cabinet.dll FDI binding (Windows)
    │   └── test_fixtures.rs    # hand-built RAR + makecab test inputs
    ├── patch_dest.rs           # where a patch extracts (install dir or SGWGame/)
    ├── bundled.rs              # writes the embedded i686 artifacts to disk
    ├── start32_helper.rs       # keeps sgw-start32.exe beside the launcher
    ├── overlay_meta.rs         # UI overlay id prefix + list text (shared with the tool)
    ├── overlay_pack.rs         # UI overlay -> patch zip + entry (tests only here)
    ├── bin/
    │   └── pack-client-overlay.rs  # release tool around overlay_pack.rs
    ├── client_patches/
    │   ├── mod.rs
    │   ├── dll_source.rs       # override / embedded / beside-the-launcher DLL
    │   └── plan.rs             # inject decision + injection order
    ├── client_telemetry_dll.rs # opted-in telemetry DLL: source + lab-build refusal
    ├── client_paths.rs         # install-dir path resolution
    ├── identity.rs             # stable per-install identity
    ├── logs.rs                 # log collection + zip + Azure PUT
    ├── state.rs                # InstalledState + UploadedLedger
    ├── instance_lock.rs        # held launcher.lock, releasable by the update handoff
    ├── self_update/
    │   ├── mod.rs              # check() + apply() orchestration, startup cleanup
    │   ├── build_info.rs       # embedded release tag + build time
    │   ├── releases.rs         # GitHub Releases lookup, host allow-list
    │   ├── version.rs          # ordering rule + min_launcher gate
    │   ├── download.rs         # download beside the exe, size + SHA-256
    │   ├── swap.rs             # rename-aside swap, rollback, relaunch, lock wait
    │   ├── handoff.rs          # start new exe, release lock, exit; startup lock policy
    │   ├── old_process.rs      # end a stuck old launcher (pid or parent, our exe only)
    │   ├── *_tests.rs          # loopback-stub and temp-dir tests
    │   └── testdata/releases.json  # releases-list fixture
    ├── telemetry/
    │   ├── mod.rs
    │   ├── auth.rs             # dev-session token mint / refresh
    │   ├── session.rs          # session lifecycle
    │   ├── runner.rs           # pipeline driver
    │   ├── tail.rs             # client log tailing
    │   ├── events.rs           # parsed event shapes
    │   ├── queue.rs            # buffering
    │   ├── chunk.rs            # upload-chunk
    │   ├── endpoint.rs         # telemetry's own client + https/loopback policy
    │   ├── bundle.rs           # end-of-session upload-bundle
    │   ├── patch_log.rs        # client.patches.boot from the DLL's log
    │   ├── patch_counts.rs     # client.patches.counts (claimed / delivered / dropped)
    │   ├── install_result.rs   # client.launcher.install_result, queued after an install
    │   └── process_watch.rs    # game-exit detection
    └── worker/
        ├── mod.rs              # tokio worker
        ├── launch_sgw.rs       # SGW.exe launch, DLL attempts and fallbacks
        ├── launch_sgw_tests.rs # its tests (exact helper command lines)
        ├── launch_telemetry.rs # opted-in session: start before the game, follow it
        ├── self_update.rs      # update check / apply tasks -> UpdateEvent
        └── messages.rs         # Command/Event channel types
```

Launch and injection (`launch`, `inject`) live in the shared
`cimmeria-client-launch` crate, and patch sets in `cimmeria-patchset`.
`unpack/` and `client_setup/` started as directories; `app/`, `worker/`, and
`telemetry/` were each promoted from a flat file once they crossed the
4-siblings-on-one-theme threshold in
[CLAUDE.md's file organization rules](../../CLAUDE.md).

The experimental desktop launcher now displays signature-verified manifest
patch notes through its Patch Notes tab. This does not establish installed
patch state; its game installation and launch integration remain unfinished.

The desktop engine shares the existing installation and unpacking source.
`install_progress` supports a one-value watch channel for desktop workers; the
existing egui worker retains its legacy event stream through an adapter.
