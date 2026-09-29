---
title: "Launcher Guide"
type: how-to
audience: players, operators
last_updated: 2026-09-27
---

# Launcher Guide

How the Stargate Worlds launcher works from the user's seat, and how
operators prepare and publish patches for it to consume.

This is the practical, day-to-day doc. For architecture and rationale
see [sgw-launcher.md](sgw-launcher.md). For the one-time operator
backend setup (GitHub Releases for content, manifest signing keypair,
Azure Blob SAS for log uploads) see
[launcher-distribution-setup.md](launcher-distribution-setup.md).

> **Audience split:**
> [Part 1 — Players](#part-1--for-players) is for anyone running the
> launcher to install and play the game.
> [Part 2 — Operators](#part-2--for-operators-publishing-patches) is for
> whoever publishes patches to the Azure container that players' launchers
> pull from. Skip the one that isn't you.

---

## Contents

- [Part 1 — For players](#part-1--for-players)
  - [What it does (end-to-end)](#what-it-does-end-to-end)
  - [State files](#state-files)
  - [What "Install / Update" does internally](#what-install--update-does-internally)
  - [How the client finds its server](#how-the-client-finds-its-server)
  - [Install layout](#install-layout)
  - [The launch buttons](#the-launch-buttons)
  - [Uploading debug logs](#uploading-debug-logs)
- [Part 2 — For operators (publishing patches)](#part-2--for-operators-publishing-patches)
  - [Step 1 — Build the seed](#step-1--build-the-seed)
  - [Step 2 — Build a patch zip](#step-2--build-a-patch-zip)
  - [Step 3 — Write the manifest](#step-3--write-the-manifest)
    - [Schema versioning policy](#schema-versioning-policy)
  - [Step 4 — Sign + publish to GitHub Releases](#step-4--sign--publish-to-github-releases)
  - [Append-only invariants](#append-only-invariants)
  - [Future automation](#future-automation)
- [Troubleshooting](#troubleshooting)

---

## Part 1 — For players

### What it does (end-to-end)

```text
1. Run sgw-launcher.exe (single ~5 MB file).
2. Window appears with three editable fields:
     - Install dir    (default: %LOCALAPPDATA%\Stargate Worlds)
     - Login servers  (default: Cimmeria = http://play.cimmeria.app:8081),
                      one `Name = URL` per line — written into the client's
                      LoginInternal.lua, the list on the login screen
     - Manifest URL   (default: the GitHub Release `content-current` tag,
                      https://github.com/SandboxServers/Cimmeria/releases/
                      download/content-current/manifest.json)
3. Launcher auto-fetches manifest.json on startup.
4. It compares the manifest against <install_dir>/launcher-installed.json:
     a. If seed hash differs → seed not installed → "Install / Update" enabled
     b. If any declared patch is missing → button enabled
     c. If everything matches → "✔ Install is up to date"
5. Click "Install / Update":
     - Download the seed (resumable via HTTP Range) → verify sha256 → unpack.
       The seed is a zip, or the archive.org client RAR, whose installer
       cabinets are expanded straight into the installed layout
     - For each missing patch in declared order: download → verify → unpack
       (overlay files, and/or deltas rebuilt from your own stock files)
     - Client setup: restore stock file names (EULA.lua), write
       LoginInternal.lua, switch ASLR off in SGW.exe (also done before
       every launch)
6. Click "Launch SGW.exe" (or "Launch Atera Debug" / "Launch + Telemetry" /
   "Fix ASLR" if those files are present in the install directory).
   Telemetry is off unless you turn it on.
7. After playing, click "Upload Debug Logs" to zip+upload logs in one shot.
```

### State files

Four JSON files persist across runs:

| File | Lives | Contents |
|------|-------|----------|
| `<exe>/launcher-config.json` | next to `launcher.exe` | `schema_version`, `install_path`, `login_servers` (`[{name, url}]`), `manifest_url`, and a `telemetry` object (`opted_in`, `prompt_answered`, `auth_url`; `auth_url` defaults to the public server's login port, `http://play.cimmeria.app:8081/api`). An old `server_host` field is ignored. Schema 2 moved that default: a schema-1 file whose `auth_url` is exactly the old `http://localhost:8443/api` is rewritten to the new default once. |
| `<install>/launcher-installed.json` | in the game dir | `seed_sha256`, `applied_patches: ["001-dialog-portraits", …]`, `seed_adopted`. An old `patched_host` field is ignored. |
| `<exe>/uploaded.json` | next to `launcher.exe` | `[{sha256, blob_name, uploaded_at}, …]` — log-upload dedupe ledger |
| `<exe>/telemetry-state.json` | next to `launcher.exe` | per-session telemetry runtime state, kept separate from the config so config rewrites don't churn it |

Putting the installed-state file **inside the game directory** is
deliberate: reinstall the launcher and your install is still recognized;
copy the game directory to another machine and it's still recognized.

### What "Install / Update" does internally

Pseudocode of [`crates/launcher/src/install.rs`](../../crates/launcher/src/install.rs)::`install_all()`:

```text
1. Load <install>/launcher-installed.json (or default if absent).

2. If state.seed_sha256 != manifest.seed.sha256:
     - GET <manifest_base>/<manifest.seed.blob>, sending HTTP `Range: bytes=N-`
       when a .tmp file from a previous attempt exists.
     - Stream the body into <install>/.tmp-seed-<sha-prefix>.download.
       A 416 reply for a full-length file means it is already downloaded.
     - SHA-256 verify against manifest.seed.sha256. On mismatch, delete the
       download and bail (a kept bad file would fail every retry).
     - Unpack into <install>/ (overwrites colliding files), picking the
       format from the file's magic bytes:
         zip → extract directly.
         RAR → extract into <install>/.tmp-unpack/. If that holds a MakeCAB
               set (an .INF with a [cabinet list], e.g. Data\DATA.INF +
               DATA1-4.CAB), expand the cabinets into <install>/ with
               Windows' cabinet.dll; otherwise move the files across.
               Then delete .tmp-unpack/.
       Every extracted file keeps the modified time the archive records
       for it (the cabinets date the client 2009-06-30), the way the stock
       installer does. See "Your ini file is outdated" under
       Troubleshooting for why that matters.
       Hashing and unpacking run on a blocking thread.
     - state.seed_sha256 = manifest.seed.sha256
     - state.applied_patches = []   ← reseeding invalidates patch history
     - Persist state.

3. Rename Working\SGWGame\Cache.en-US to SourceCache.en-us, unless a
   SourceCache.en-us already exists (see "Install layout"). Runs every
   time, so adopted installs get it too.

4. For each patch in manifest.patches (declared order):
     - Skip if patch.id is already in state.applied_patches.
     - GET <manifest_base>/<patch.blob> → tmp → SHA-256 verify → unpack.
       A patch zip with a cimmeria-patch.json recipe is a patch set: see
       "Patch sets" below.
     - Append patch.id to state.applied_patches.
     - Persist state after every patch (survives mid-update crash).

5. Client setup (crates/launcher/src/client_setup/), when SGW.exe exists:
     - Rename any file a patch set writes back to its stock spelling if
       only its case differs (eula.lua -> EULA.lua).
     - Write Working\SGWGame\Content\UI\Startup\Login\LoginInternal.lua
       from the login-server list, if its content changed.
     - Clear IMAGE_DLLCHARACTERISTICS_DYNAMIC_BASE in SGW.exe's PE
       header (one byte, 0x186 in the 0.8348 build), if set.
```

### How the client finds its server

The client's login screen lists the servers `LoginMod.loadServerSystems()`
defines in `LoginInternal.lua`. The stock file points at CME's dead QA and
production login servers; the launcher replaces it with the configured
list, CRLF and ASCII like the rest of the client's UI Lua, at the end of
every install, on **Save**, and before every launch. Names and URLs may not
contain quotes or backslashes, so a setting can't break out of the Lua
string. A Cimmeria auth server listens on `http://<host>:8081`.

Earlier launcher builds instead byte-patched `www.stargateworlds.com` in
`SGW.exe`'s `.rdata`. That string is only the SOAP namespace
`http://www.stargateworlds.com/xml/sgwlogin`, so the patch redirected
nothing and would have broken the namespace the auth server expects. It is
gone; no released launcher ever ran it.

ASLR is switched off because the client-patches DLL and every RE address
in `docs/` assume `SGW.exe` loads at `0x00400000`. The stock exe with that
one byte cleared is byte-identical to a known-good QA client's.

Hitting **Cancel** flips a `CancellationToken` that the download stream
checks on every chunk, and the unpack steps check between files. A
cancelled install leaves the `.tmp-*.download` file on disk, so
re-clicking Install / Update picks up where it stopped via
`Range: bytes=N-`.

Installing from the archive.org client RAR needs roughly 14 GB free on
the install drive at its peak: the 4.1 GB download, the 4.1 GB staged
installer, and the expanded client.

### Install layout

A full install has the original installer's layout:

```text
<install>\Common\...
<install>\Resources\...
<install>\Working\Binaries\SGW.exe   (plus its DLLs, SGWDebugLog.log, sessions\)
<install>\Working\SGWGame\...
```

[`crates/launcher/src/install_layout.rs`](../../crates/launcher/src/install_layout.rs)::`binaries_dir`
finds the directory holding `SGW.exe`, and every launch, the hostname
patch, adoption and the log upload go through it. It also accepts an
install path that points straight at `Binaries` (holds `SGW.exe`) or at
`Working`, so configs from before this layout keep working.

The client reads cooked data (`Cooked*.pak`, `TextStrings.pak`, …) from
two tiers, set in `Working\Engine\Config\GameplayEngine.ini`:

| Tier | INI key | Where it is | Who writes it |
|---|---|---|---|
| Writable | `CachePath=..\SGWGame\Cache` | `Documents\My Games\Firesky\SGWGame\Cache.en-US` | The server's version push, on every connect |
| Bundled, read-only | `SourceCachePath=..\SGWGame\SourceCache` | `<install>\Working\SGWGame\SourceCache.en-us` | Nothing at runtime |

The 2009 cabinets put the bundled PAKs in
`Working\SGWGame\Cache.en-US`, which is neither path. A stock install
logs this once per cooked-data category in `SGWDebugLog.log`:

```text
WARN common - Non-existent source archive directory: <install>\Working\SGWGame\SourceCache.en-US
```

So after the seed is unpacked, the launcher renames that folder to
`SourceCache.en-us`, which silences the warning. Testers had already
done the same by hand, and their `SourceCache.en-us` PAKs are
byte-identical to the cabinets' `Cache.en-US`. It is also why a PAK
placed by hand in `SourceCache.en-us` stays put, while one placed in the
Documents `Cache.en-US` is rewritten the next time the client connects.
The launcher's **Reset client cache** button wipes only the Documents
tier.

### The launch buttons

Four buttons, each enabled only when the relevant files exist in the
binaries directory (see [`crates/launcher/src/app/view.rs`](../../crates/launcher/src/app/view.rs)::`show_launch_panel`):

| Button | Enabled when | What it runs |
|--------|------------|--------------|
| **Launch SGW.exe** | `SGW.exe` exists | Starts `<binaries>/SGW.exe` (cwd = binaries dir) with the client patches loaded, unless you turned them off. With telemetry on, a telemetry session follows the game |
| **Launch Atera Debug** | `AteraLoader.exe` **and** `AtreaGameDebug.bat` both present | `cmd /C AtreaGameDebug.bat` (cwd = binaries dir) |
| **Launch + Telemetry** | Atera available **and** you opted into telemetry **and** launcher identity loaded | Same as Atera Debug, plus the dev-session telemetry pipeline — see [telemetry.md](../operations/telemetry.md) |
| **Fix ASLR** | `AtreaFixASLR.bat` present | `cmd /C AtreaFixASLR.bat` |

Under the buttons, **Load client patches (restores the Black Market
window)** controls `cimmeria-client-patches.dll`, which the launcher
loads into the game on **Launch SGW.exe**. It is on by default and has
nothing to do with the telemetry setting. Turn it off only to rule the
patches out when something misbehaves; the Black Market window will not
open without them. The status log says on every launch when the patches
were not loaded, and why. Atera debug launches never load them.

Beside it, **Send telemetry (opt-in)** is off until you turn it on. On,
the launcher uploads the client's log files while you play, and loads a
small telemetry module (`cimmeria-client-telemetry.dll`) into the game
after the client patches, which records in-game events such as the game
messages the client handles and interface errors. Both go to the
Cimmeria server with this install's random id, so crashes and bugs can
be traced. Telemetry goes to the server's login port, the same address
the game logs in to, so it needs no setup. The module changes nothing
in the game. If the launcher
cannot reach the telemetry server, or the module is missing, the status
log says so and the game starts without it. The launcher also asks once,
at the top of the window; either answer is remembered, and a change
applies from your next launch.

### What the launcher changes in your client

Stargate Worlds needs the stock 2009 client, and Cimmeria changes it.
The **Changes to your client** section lists every change: the login
servers, ASLR flag and stock file names it sets before every launch, the
folder it renames
at install, each patch from the manifest and whether it is applied yet,
what it adds to `SGW.exe` at launch, and what the server sends while you
play. The same list applies whether the launcher downloaded the client
or you adopted your own copy. Details are in
[sgw-launcher.md](sgw-launcher.md#changes-to-your-client).

The Atera files are **not** shipped by the launcher or any of its
patches. Developers and modders drop the Atera tarball into the install
directory themselves; the launcher detects the files and surfaces the
buttons.

The Atera debug build requires ASLR disabled on `SGW.exe`. Click
**Fix ASLR** once after a fresh install, then the debug bat works on
every subsequent launch.

For what Atera actually does at runtime see
[../technical/atrealoader-exe.md](../technical/atrealoader-exe.md) and
[../technical/atrealoader-config.md](../technical/atrealoader-config.md).

### Uploading debug logs

The **Upload Debug Logs** button collects:

- `<binaries>/sgwdebuglog*`, matched case-blind (the client writes
  `SGWDebugLog.log`) — BigWorld Mercury unicode log
- `<binaries>/sessions/**` — per-session logs

The zip stores them under `Binaries/` whatever the directory is called
on disk.

Zips them in memory and PUTs the zip in a single HTTP request to the
Azure storage account, named `logs/<hostname>-<utc>-<digest12>.zip`.

Three rules to keep storage costs minimal:

1. **One PUT per click**, never one-per-file. The whole zip goes up in
   one request.
2. **Content-digest dedupe**: the launcher hashes the *input files*
   (sorted filename + bytes), not the zip itself, and records the hash
   in `<exe>/uploaded.json`. Already-uploaded digest → zero HTTP
   requests, button reports "Already uploaded".
3. **Never automatic**: only fires on explicit button click.

If the launcher you're running has no upload SAS baked in (i.e. it was
built from a PR or a local dev build), the button is disabled with a
friendly note. Only the official release pipeline injects the SAS.

---

## Part 2 — For operators (publishing patches)

The launcher doesn't generate patches — it consumes them. Operators own
the publishing pipeline. There's no automation yet; this section
describes the manual procedure.

### Step 1 — Build the seed

The seed is the whole game as a fresh install should look. There are
two ways to provide it.

**Use the archive.org client RAR as-is (recommended).** The 2009 beta
client, build 0.8348.1.4046, is archived at archive.org item
`StargateWorlds_0.8348.1.4046`. It is a single stored RAR holding the
original installer: `SetupQA.exe` plus a MakeCAB set, `Data\DATA.INF`
and `Data\DATA1.CAB`..`DATA4.CAB` (5,983 files). The launcher expands
the cabinets straight into the installed layout, so the installer never
runs. Point the manifest's seed at the archive.org URL (an absolute
`blob` URL is used as-is), and nothing has to be rehosted. archive.org
serves `Range` requests, so the 4.1 GB download resumes.

| Field | Value |
|---|---|
| `blob` | `https://archive.org/download/StargateWorlds_0.8348.1.4046/Stargate%20Worlds%20%280.8348.1.4046%29%20%282009-06-30%29%20%28beta%29.rar` |
| `size` | `4135724034` |
| `sha256` | `7ba97ed2cb94f86edaba17a513824ae08d0f19920583d2a3da242faf1e034f07` |

archive.org publishes only MD5 and SHA-1 for the file
(SHA-1 `1ad25c4dbd4b8717447b0de7145f5eb52d34ab06`); the SHA-256 above was
computed from a copy whose SHA-1 matches.

**Or roll your own zip** containing the installed layout
(`Common\`, `Resources\`, `Working\...`), for example a curated snapshot
that bundles content fixes you don't want to ship as separate patches.

Either way the seed contains the stock `SGW.exe` and stock
`LoginInternal.lua`: the launcher's client setup (login servers, ASLR)
runs at the end of `install_all`, after all manifest patches.

Compute the seed file's SHA-256 — it becomes `manifest.seed.sha256`.

### Step 2 — Build a patch zip

There are two kinds of patch zip, and the launcher tells them apart by
whether the zip carries a `cimmeria-patch.json` recipe.

**Patch sets (for anything derived from CME files).** The project never
hosts CME bytes. A change to a stock file (a UI Lua file, a map, a cooked
PAK) ships as a bsdiff delta that the launcher applies to the player's own
stock copy, so the zip holds only the bytes we authored. Specs and built
zips live in [`data/client-patches/`](../../data/client-patches/README.md),
built with the `cimmeria-patchset` tool:

```bash
cargo run -p cimmeria-patchset -- build data/client-patches/<id>/patch.json \
  --stock <stock client, seed unpacked, SourceCache renamed> \
  --patched <a client with the change> \
  --out data/client-patches/<id>.zip \
  --blob-url https://raw.githubusercontent.com/SandboxServers/Cimmeria/<commit>/data/client-patches/<id>.zip
```

Give the spec a `title` and a one- or two-sentence `description` of
what the patch changes, in words a player understands; the tool copies
both into the manifest entry, and the launcher shows them in its
**Changes to your client** list. Add the same text to the launcher's
`builtin_description` in `crates/launcher/src/client_changes.rs`: a
test checks the two match.

It prints the manifest entry. Each recipe op pins the SHA-256 of every
source and of the result; the launcher refuses a source that isn't stock,
computes every op before writing anything, and skips ops whose target
already has the result. Maps our `upk_patch` tool wrote are diffed against
the stock map *normalized* (decompressed and rewritten by the same
patcher), which is what keeps a 4 MB map's delta under 2 KB. Files that are
entirely ours ship whole in the same zip (`"files"` in the spec).

**Plain overlay zips (for content that is entirely ours).** A zip
without a recipe contains **only the files that changed**, laid out
exactly as they go into the install directory:

```text
002-mercury-config.zip
└── Binaries/
    └── res/
        └── server/
            └── mercury.xml    ← the file you changed
```

The launcher extracts the patch zip over the install directory using
`zip::ZipArchive`. Existing files at the same paths are **overwritten**;
files not present in the patch are left alone.

Three properties fall out of this simple model:

- **Add files**: include them at their destination path.
- **Modify files**: include the new version at the same path.
- **Delete files**: not supported. If you need to remove a file, either
  (a) ship a new seed and bump `manifest.seed.sha256` to force a fresh
  install for everyone, or (b) include a stub/empty replacement at that
  path.

Every patch gets a stable `id` — convention is `NNN-short-slug` so the
manifest array stays sortable when read by humans. Compute the patch
zip's SHA-256.

### Step 3 — Write the manifest

`manifest.json` lives at the root of the container, alongside the
`seed/` and `patches/` sub-prefixes:

```json
{
  "schema": 1,
  "seed": {
    "blob": "seed/sgw-0.8348.1.4046.zip",
    "size": 5234567890,
    "sha256": "abc123..."
  },
  "patches": [
    {
      "id": "001-base",
      "blob": "patches/001-base.zip",
      "size": 123456,
      "sha256": "def456...",
      "after": null
    },
    {
      "id": "002-mercury-config",
      "blob": "patches/002-mercury-config.zip",
      "size": 23456,
      "sha256": "789abc...",
      "after": "001-base"
    }
  ]
}
```

Rules the launcher enforces (see
[`crates/launcher/src/manifest.rs`](../../crates/launcher/src/manifest.rs)::`validate`):

- `schema` must be `1`. Bumping it invalidates every launcher binary
  built before the bump — serve a legacy manifest at the old URL during
  any transition. See [Schema versioning policy](#schema-versioning-policy)
  below before considering a bump.
- Every `after` reference must point to a patch declared **earlier** in
  the array. Forward references and unknown ids fail validation.
- Patch ids must be unique.
- Array order **is** the application order. `after` is documentation /
  sanity — the launcher applies patches in their order in `patches[]`.

#### Schema versioning policy

The `schema` field is an integer version number. Today only `1` is
supported.

**When to bump:**

- Adding a **required** field to `seed`, `patches[*]`, or top-level →
  bump.
- Removing any existing field → bump.
- Changing field semantics (e.g. reinterpreting `size` as bytes vs
  blocks) → bump.
- Adding an **optional** field with a backwards-compatible default →
  **no bump** needed; older launchers will ignore the new field via
  `#[serde(default)]`.

**Compatibility model:**

The launcher bails on any schema that isn't `1` — the check is a literal
`if self.schema != 1` in `Manifest::validate`
([`crates/launcher/src/manifest.rs:176`](../../crates/launcher/src/manifest.rs)),
not a named constant. There is no multi-schema support today. Bumping the
schema is therefore a hard cutover that requires every player to be on a
launcher build that understands the new schema, and the bump means editing
that literal.

The recommended bump procedure:

1. Cut a new launcher release whose `SUPPORTED_SCHEMA` is the new
   number. The new launcher must accept *both* schemas during the
   transition (temporarily relax the validate check).
2. Wait at least one full release-cadence window for players to update.
3. Publish a manifest at the new schema version.
4. After two cadence windows, the next launcher release can drop the
   compatibility branch.

For changes that don't fit the additive-optional shape, prefer the
new-launcher path over a schema bump — e.g. adding a new manifest URL
prefix and pointing new launcher releases at it, while keeping the old
URL serving the old schema for existing installs.

### Step 4 — Sign + publish to GitHub Releases

The hosting layout uses two release-tag families on the repo:

```text
content-current                ← rolling tag, overwritten each publish
├── manifest.json              ← the manifest
└── manifest.json.sig          ← Ed25519 detached signature (hex)

content-2026-05-20-001         ← immutable, one per content drop
├── seed.zip                   (if this drop ships a new seed)
└── 002-mercury-config.zip     (per-patch zips)
content-2026-05-20-002
├── 003-quest-fixes.zip
└── ...
```

The manifest references the immutable tag's assets by **absolute URL**,
e.g. `https://github.com/<org>/<repo>/releases/download/content-2026-05-20-001/seed.zip`.
The launcher's [`blob_url`](../../crates/launcher/src/manifest.rs)
passes absolute URLs through unchanged; relative `blob` fields still
work (resolved against the manifest URL's container) for legacy / mixed
hosting setups.

A rough operator workflow using PowerShell + `gh`:

```powershell
# 1. Build a patch zip from the changed files.
Compress-Archive `
  -Path 'Binaries\res\server\mercury.xml' `
  -DestinationPath '002-mercury-config.zip' `
  -CompressionLevel Optimal

# 2. Compute sha + size for the manifest entry.
$sha  = (Get-FileHash 002-mercury-config.zip -Algorithm SHA256).Hash.ToLower()
$size = (Get-Item 002-mercury-config.zip).Length
"sha:  $sha"
"size: $size"

# 3. Append a patch entry to manifest.json with the absolute URL for
#    where the asset will live in the immutable release. Pick the tag
#    you're about to create.
$tag = "content-$(Get-Date -Format yyyy-MM-dd)-001"
$blob = "https://github.com/SandboxServers/Cimmeria/releases/download/$tag/002-mercury-config.zip"
# (edit manifest.json by hand or via jq)

# 4. Sign the manifest with the offline private key, producing
#    manifest.json.sig (Ed25519 over the exact manifest bytes, hex), and
#    check it against the public key in the LAUNCHER_MANIFEST_PUBKEY_HEX
#    secret.
cargo run -p cimmeria-patchset -- sign manifest.json --key <path to manifest-signing.key>
cargo run -p cimmeria-patchset -- verify manifest.json --pubkey <public key hex>

# 5. Create the immutable release first (asset must exist before the
#    manifest references it).
gh release create "$tag" --notes "Content drop $tag" --prerelease `
  002-mercury-config.zip

# 6. Update the rolling manifest pointer. --clobber overwrites in place.
gh release upload content-current --clobber `
  manifest.json manifest.json.sig
```

**Ordering matters.** Create the immutable per-publication release
(step 5) **before** updating `content-current` (step 6). A launcher
that fetches `manifest.json` between the two would see an entry
referencing a blob that doesn't exist yet and fail with a 404.

The launcher verifies the signature against the compile-time-embedded
`MANIFEST_SIGNING_PUBKEY`
([`crates/launcher/src/manifest.rs`](../../crates/launcher/src/manifest.rs)::`verify_manifest_signature`).
`cimmeria-patchset pubkey --key <file>` prints the public key for a key
file, to check it matches the secret.

Full operator setup (signing keypair generation, GitHub secrets,
log-upload SAS) lives in
[launcher-distribution-setup.md](launcher-distribution-setup.md).

### Append-only invariants

Once you publish a patch, **never mutate it**:

- Don't change its `blob` path (players who already installed under the
  old path would refetch).
- Don't change its `sha256` (a launcher mid-rerun would see a hash
  mismatch and refuse the install).
- Don't change its `id` (the installed-state file uses ids to know what
  it already applied).

**To fix a broken patch, publish a new patch that overwrites the
affected files.** The manifest is append-only at the patch level.

The seed is replaceable, but it's a heavy operation: a new
`seed.sha256` forces every installed player to re-download the full
seed and re-apply every patch. Reserve seed bumps for major-version
flips.

### Future automation

`cimmeria-patchset` builds patch sets and signs manifests; publishing is
still by hand. A reasonable next step once patches ship regularly is a
GitHub Actions workflow that rebuilds the patch zips, checks they match
the committed ones, and publishes on a `/release-patch` ChatOps comment,
mirroring the `/release-launcher` pattern from
[.github/workflows/launcher-release-on-comment.yml](../../.github/workflows/launcher-release-on-comment.yml).
Signing stays offline.

---

## Troubleshooting

Common issues players hit, with first-line diagnostic steps. Every
error message also lands in the launcher's status panel — copy-paste
that into a bug report if first-line fixes don't help.

### "Manifest error: …"

The launcher couldn't fetch or parse `manifest.json`.

- **`error sending request` / DNS failures**: check your internet
  connection. Verify the **Manifest URL** field in the launcher matches
  what your server operator published.
- **HTTP 404**: the manifest URL is wrong, or the release asset it
  points at was never uploaded. Operators: confirm `manifest.json` is
  attached to the `content-current` release tag, and that the tag
  itself exists.
- **HTTP 403**: the release or repository isn't publicly readable.
  Operators: content is served from GitHub Releases, so a private
  repository will 403 anonymous fetches.
- **JSON parse error**: the manifest is malformed. Operators: validate
  the file with `jq . manifest.json` before uploading.
- **"Unsupported manifest schema N"**: this launcher binary is older
  than the manifest. Download a newer launcher release.

### "Install failed: Hash mismatch for seed/patch …"

The download completed but the file's SHA-256 didn't match the manifest.

- Click **Install / Update** again — the launcher resumes from the
  partial `.tmp-*.zip` file and may correct a transient corruption.
- If it persists: the CDN or the manifest is out of sync. Operators
  should re-publish the affected blob and verify the manifest hash
  matches.

### "Install failed: Unexpected HTTP &lt;status&gt; for …"

The seed or patch blob URL returned a non-success status (anything outside
`2xx` plus the explicit resume case of `206 Partial Content`).

- **404**: a manifest entry references a blob that wasn't uploaded.
  Operators should verify the upload order (patch blob first, then
  manifest — see the storage runbook).
- **403**: blob exists but isn't public.
- **5xx**: Azure transient error. Retry; if it sticks, check the Azure
  status page.

### "Launch failed: File not found"

The launcher tried to launch SGW.exe / a batch file that isn't actually
on disk.

- Verify the **Install dir** field matches where the game is installed.
- Click **Install / Update** to ensure the install is complete.
- For Atera-debug launches: confirm `AteraLoader.exe` and
  `AtreaGameDebug.bat` are both in the install dir alongside SGW.exe.
  These files are not shipped by the launcher.

### "Your ini (..\SGWGame\Config\SGWEditor.ini) file is outdated"

The game asks this on launch when a `Working\SGWGame\Config\Default*.ini`
file's modified time differs from the one it recorded the last time it
ran. Unreal Engine 3 builds your own settings files in
`Documents\My Games\Firesky\SGWGame\Config\SGW*.ini` from the
`Default*.ini` files, and keeps the timestamps of those in each file's
`[INIVersion]` section. A different timestamp looks like a newer
`Default*.ini`.

Launchers released before this fix wrote every client file with the
install time instead of the 2009-06-30 date in the installer cabinets.
If you had run SGW on this PC before, from any install, the first launch
after such an install shows this dialog. A launcher with the fix keeps
the cabinets' dates, as the original installer does, so the dialog does
not appear after a fresh install.

- **Yes** (or **Yes to all**) is safe. The game rebuilds the
  `SGW*.ini` files from the `Default*.ini` files and records the new
  timestamps, so it does not ask again. It resets your settings in those
  files (resolution, keys, audio) to the defaults.
- **No** keeps your settings, and the game asks again on the next
  launch.
- To stop the dialog without losing settings, reinstall with a fixed
  launcher: **Wipe → Client**, then **Install / Update**. The files get
  their 2009 dates back and match what your `SGW*.ini` files recorded.
  If you already answered **Yes** after an older launcher's install,
  your `SGW*.ini` files recorded that install's dates instead, so the
  first launch after the reinstall asks once more.

### Windows Defender or SmartScreen blocks the launcher

Launcher releases are not code-signed, so Windows may warn about the launcher or
quarantine `sgw-start32.exe`. That small helper sits beside
`sgw-launcher.exe` and loads the client patches into the game. It starts
`SGW.exe` and writes into it, which is what antivirus heuristics look
for in an unsigned program.

- **SmartScreen ("Windows protected your PC")** on `sgw-launcher.exe`:
  choose **More info → Run anyway**. Only do this for a launcher you
  downloaded from the project's GitHub Releases page.
- **Defender removed or blocked `sgw-start32.exe`**: the status log shows
  `Client patches: not loaded (… sgw-start32.exe …)` and the game starts
  without them. Restore it from **Windows Security → Virus & threat
  protection → Protection history**, then add an exclusion for the
  folder that holds the launcher (**Virus & threat protection settings →
  Exclusions → Add an exclusion → Folder**). The helper keeps one fixed
  name in that folder across launcher updates, so the exclusion keeps
  working. The launcher never writes it to a temporary folder. From an
  elevated PowerShell:

  ```powershell
  Add-MpPreference -ExclusionPath "C:\path\to\the\launcher\folder"
  ```

Code signing would remove the need for this; it is deferred for now.

### The Black Market window does not open

The Black Market window needs the client patches. Check the status log
from the launch:

- **`Client patches: off (launcher setting)`**: tick **Load client
  patches** and launch again.
- **`Client patches: unavailable: …`**: this launcher build has no DLL
  bundled (a dev build) and there is no `cimmeria-client-patches.dll`
  beside it. Use a released launcher.
- **`Client patches: not loaded (… sgw-start32.exe …)`**: the launcher
  could not run its 32-bit helper, `sgw-start32.exe`, which loads the
  patches into the 32-bit game. A dev build has none unless it sits
  beside the launcher; a released launcher carries it. If security
  software removed it, see
  [Windows Defender or SmartScreen blocks the launcher](#windows-defender-or-smartscreen-blocks-the-launcher).
- **`Client patches: not loaded (… remote_load_failed …)`**: the helper
  ran but the DLL did not load into the game. Include the status log in
  a bug report.
- **No such line**: the patches were loaded. They write
  `cimmeria-client-patches.log` next to `SGW.exe`; a line ending
  `nothing installed` there means this `SGW.exe` is not the build the
  patches know. Include that file in a bug report.

### The game shows the gate backdrop but never the login screen

The launcher released on 2026-09-29 (`launcher-20260929-f518b57`)
renamed `Working\SGWGame\Content\UI\Startup\EULA\EULA.lua` to
`eula.lua` while applying the `005-login-delay` patch. The game looks
that file up case-sensitively, so the EULA screen never loads, and the
login screen comes after it. The client log (and telemetry's
`client.ui.cegui_log`) shows `SGWResourceProvider: 'EULA.lua' does not
exist in group lua`.

- **Fix**: update the launcher. Before every launch, and after every
  **Install / Update**, it renames the file back; the status log says
  "Renamed ... eula.lua back to its stock name EULA.lua".
- **By hand**: rename `eula.lua` to `EULA.lua` in that folder. Keep the
  file; it is the patched one.

Patch sets now keep the on-disk name of every file they change, so a new
install is not affected.

### SGW.exe launches but can't reach the server

- Check the **Login servers** list: one `Name = http://host:8081` line
  per server, with your operator's host. Click **Save**; the status log
  says "Wrote the login server list" when the file changed.
- Open `Working\SGWGame\Content\UI\Startup\Login\LoginInternal.lua` and
  check it lists your server. The launcher rewrites it before every
  launch, so edit the list in the launcher, not the file.
- Pick the right server in the login screen's dropdown.

### "Log upload failed: 4xx/5xx"

- **403 AuthorizationFailure**: the SAS in the launcher binary has
  expired. Players need to download a newer launcher release.
- **403 AuthenticationFailed**: the SAS was malformed. Operators should
  rotate (see storage runbook).
- **413 RequestBodyTooLarge**: the log zip exceeded Azure's 256 MB
  single-PUT block-blob limit. Should not happen in practice (sessions
  rarely produce >100 MB of logs); if it does, report it.

### "Log upload skipped: Already uploaded this exact log set"

This is **not an error** — it means the launcher detected via local
ledger that the same log contents were already uploaded. To force a
re-upload, edit `<launcher.exe dir>/uploaded.json` and remove the
relevant entry, or play a new session to generate fresh logs.

### Where to find logs to share

When asking for support:

- Launcher's own status panel — copy the relevant lines.
- `<launcher.exe dir>/launcher-config.json` — your install path, server
  host, and manifest URL (the relevant config the launcher is using).
- `<install_dir>/launcher-installed.json` — what the launcher thinks is
  installed (seed sha + applied patches).
- For game crashes (not launcher issues): use the **Upload Debug Logs**
  button. Logs land at `logs/<host>-<utc>-<digest>.zip` in the
  operator's storage container.

---

## Cross-references

- [sgw-launcher.md](sgw-launcher.md) — full design and architecture rationale
- [launcher-distribution-setup.md](launcher-distribution-setup.md) — operator setup: GH Releases publishing, manifest signing key, Azure Blob SAS
- [`crates/launcher/`](../../crates/launcher/) — source code
- [.github/workflows/launcher-release.yml](../../.github/workflows/launcher-release.yml) — release pipeline
