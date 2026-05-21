# Launcher Guide

How the Stargate Worlds launcher works from the user's seat, and how
operators prepare and publish patches for it to consume.

This is the practical, day-to-day doc. For architecture and rationale
see [sgw-launcher.md](sgw-launcher.md). For the one-time Azure backend
setup see [launcher-storage-setup.md](launcher-storage-setup.md).

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
  - [The launch buttons](#the-launch-buttons)
  - [Uploading debug logs](#uploading-debug-logs)
- [Part 2 — For operators (publishing patches)](#part-2--for-operators-publishing-patches)
  - [Step 1 — Build the seed](#step-1--build-the-seed)
  - [Step 2 — Build a patch zip](#step-2--build-a-patch-zip)
  - [Step 3 — Write the manifest](#step-3--write-the-manifest)
    - [Schema versioning policy](#schema-versioning-policy)
  - [Step 4 — Upload to Azure Blob](#step-4--upload-to-azure-blob)
  - [Append-only invariants](#append-only-invariants)
  - [Future automation](#future-automation)
- [Troubleshooting](#troubleshooting)

---

## Part 1 — For players

### What it does (end-to-end)

```text
1. Run sgw-launcher.exe (single ~5 MB file).
2. Window appears with three editable fields:
     - Install dir    (default: C:\Program Files\Stargate Worlds)
     - Server host    (default: play.cimmeria.gg) — gets patched into SGW.exe
     - Manifest URL   (default: <azure-blob>/sgw/manifest.json)
3. Launcher auto-fetches manifest.json on startup.
4. It compares the manifest against <install_dir>/launcher-installed.json:
     a. If seed hash differs → seed not installed → "Install / Update" enabled
     b. If any declared patch is missing → button enabled
     c. If everything matches → "✔ Install is up to date"
5. Click "Install / Update":
     - Download seed.zip (resumable via HTTP Range) → verify sha256 → extract
     - For each missing patch in declared order: download → verify → extract (overlay)
     - Byte-patch SGW.exe .rdata: replace "www.stargateworlds.com" with server host
6. Click "Launch SGW" (or "Launch Atera Debug" / "Fix ASLR" if those files
   are present in the install directory).
7. After playing, click "Upload Debug Logs" to zip+upload logs in one shot.
```

### State files

Three JSON files persist across runs:

| File | Lives | Contents |
|------|-------|----------|
| `<exe>/launcher-config.json` | next to `launcher.exe` | `install_path`, `server_host`, `manifest_url` |
| `<install>/launcher-installed.json` | in the game dir | `seed_sha256`, `applied_patches: ["001-base", "002-mercury", …]` |
| `<exe>/uploaded.json` | next to `launcher.exe` | `[{sha256, blob_name, uploaded_at}, …]` — log-upload dedupe ledger |

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
     - Stream the body into <install>/.tmp-seed-<sha-prefix>.zip.
     - SHA-256 verify against manifest.seed.sha256 — bail on mismatch.
     - Extract the zip into <install>/ (overwrites colliding files).
     - state.seed_sha256 = manifest.seed.sha256
     - state.applied_patches = []   ← reseeding invalidates patch history
     - Persist state.

3. For each patch in manifest.patches (declared order):
     - Skip if patch.id is already in state.applied_patches.
     - GET <manifest_base>/<patch.blob> → tmp → SHA-256 verify → extract overlay.
     - Append patch.id to state.applied_patches.
     - Persist state after every patch (survives mid-update crash).

4. If SGW.exe still contains the bytes "www.stargateworlds.com":
     - Overwrite with server_host, zero-padded to the original 22-byte slot.
     - Idempotent — no-op when the binary has already been patched.
```

Hitting **Cancel** flips a `CancellationToken` that the download stream
checks on every chunk. A cancelled install leaves the `.tmp-*.zip` file
on disk, so re-clicking Install / Update picks up where it stopped via
`Range: bytes=N-`.

### The launch buttons

Three buttons, each shown only when the relevant files exist in the
install directory:

| Button | Shown when | What it runs |
|--------|------------|--------------|
| **Launch SGW.exe** | `SGW.exe` exists | `CreateProcess(<install>/SGW.exe)` with `cwd = <install>` |
| **Launch Atera Debug** | `AteraLoader.exe` **and** `AtreaGameDebug.bat` both present | `cmd /C AtreaGameDebug.bat` (cwd = install dir) |
| **Fix ASLR** | `AtreaFixASLR.bat` present | `cmd /C AtreaFixASLR.bat` |

The Atera files are **not** shipped by the launcher or any of its
patches. Developers and modders drop the Atera tarball into the install
directory themselves; the launcher detects the files and surfaces the
buttons.

The Atera debug build requires ASLR disabled on `SGW.exe`. Click
**Fix ASLR** once after a fresh install, then the debug bat works on
every subsequent launch.

For what Atera actually does at runtime see
[../technical/ateraloader-exe.md](../technical/ateraloader-exe.md) and
[../technical/atrealoader-config.md](../technical/atrealoader-config.md).

### Uploading debug logs

The **Upload Debug Logs** button collects:

- `<install>/Binaries/sgwdebuglog*` — BigWorld Mercury unicode log
- `<install>/Binaries/sessions/**` — per-session logs

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

The seed is a single zip containing **the entire game installation as
you want a fresh install to look**:

```text
sgw-0.8348.1.4046.zip
├── SGW.exe
├── Binaries/
│   └── (engine DLLs, paks, etc.)
├── Content/
│   └── (all .upk files, audio, etc.)
└── (everything else from the install)
```

The launcher doesn't care about the internal structure — it just unpacks
the zip into the install directory. By convention the seed contains the
**unpatched** `SGW.exe` (still pointing at `www.stargateworlds.com`).
The launcher's `.rdata` byte-patch runs at the end of `install_all`
after all manifest patches have been applied, so the SGW.exe inside the
seed should be the original CME one.

How you actually source the seed contents is out of scope for the
launcher repo. Two reasonable options:

- Extract the official CME beta client from the archive.org RAR-of-CABs,
  then zip the extracted directory.
- Roll a curated fresh-install snapshot that bundles common content
  fixes you don't want shipping as separate patches.

Compute the seed zip's SHA-256 — it becomes `manifest.seed.sha256`.

### Step 2 — Build a patch zip

A patch zip contains **only the files that changed**, laid out exactly
as they go into the install directory:

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

The launcher checks `schema == SUPPORTED_SCHEMA` and bails on mismatch.
There is no multi-schema support today. Bumping the schema is therefore
a hard cutover that requires every player to be on a launcher build
that understands the new schema.

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

### Step 4 — Upload to Azure Blob

Layout in the storage container (`sgw`):

```text
https://<account>.blob.core.windows.net/sgw/
├── manifest.json
├── seed/
│   └── sgw-0.8348.1.4046.zip
├── patches/
│   ├── 001-base.zip
│   ├── 002-mercury-config.zip
│   └── ...
└── logs/                              ← launcher writes here only
    └── <host>-<utc>-<digest12>.zip
```

A rough operator workflow using PowerShell + `az` CLI:

```powershell
# 1. Build a patch zip from the changed files.
Compress-Archive `
  -Path 'Binaries\res\server\mercury.xml' `
  -DestinationPath '002-mercury-config.zip' `
  -CompressionLevel Optimal

# 2. Compute sha + size to paste into manifest.json.
$sha  = (Get-FileHash 002-mercury-config.zip -Algorithm SHA256).Hash.ToLower()
$size = (Get-Item 002-mercury-config.zip).Length
"sha:  $sha"
"size: $size"

# 3. Edit manifest.json (append the new patch entry with sha + size).

# 4. Upload patch + updated manifest — patch FIRST, manifest SECOND.
az storage blob upload `
  --account-name <acct> --container-name sgw `
  --file 002-mercury-config.zip `
  --name patches/002-mercury-config.zip

az storage blob upload `
  --account-name <acct> --container-name sgw `
  --file manifest.json --name manifest.json --overwrite
```

**Ordering matters.** Upload the patch blob first, then overwrite the
manifest. If a launcher fetches `manifest.json` between the two
uploads, it would see a patch entry referencing a blob that doesn't
exist yet and fail with a 404 on the patch GET. Manifest-last is the
safe ordering.

The container, SAS minting, and lifecycle policy live in
[launcher-storage-setup.md](launcher-storage-setup.md).

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

There's no patch-prep tooling in the repo yet. Reasonable next steps
when you actually need to ship patches regularly:

- A `tools/patch-builder/` script that takes a directory of changed
  files + a patch id, produces the zip, computes hash + size, updates a
  local manifest working copy, and (optionally) uploads.
- A GitHub Actions workflow that takes the same inputs and publishes to
  Azure on `/release-patch` ChatOps — mirroring the existing
  `/release-launcher` pattern from
  [.github/workflows/launcher-release-on-comment.yml](../../.github/workflows/launcher-release-on-comment.yml).

Neither exists yet. Both are straightforward to add once you've shipped
the first patch by hand and know what feels right.

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
- **HTTP 404**: the manifest URL is wrong, or the storage container
  isn't publicly readable. Operators: check that the `sgw` container
  was created with `--public-access blob` per the storage runbook.
- **HTTP 403**: the container exists but isn't public. Operators: same
  fix as 404.
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

### "Install failed: Unexpected HTTP 4xx/5xx for …"

The seed or patch blob URL returned an unexpected status.

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

### SGW.exe launches but can't reach the server

- Verify the **Server host** field is set to your operator's emulator
  hostname.
- Click **Save** then **Install / Update** — the hostname patch only
  fires during install / update, not on every launch.
- Check the `.rdata` patch took effect: open `SGW.exe` in a hex editor
  and search for `www.stargateworlds.com`. If still present, the
  install didn't complete the post-install patch step.

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
- [launcher-storage-setup.md](launcher-storage-setup.md) — Azure container + SAS setup runbook
- [`crates/launcher/`](../../crates/launcher/) — source code
- [.github/workflows/launcher-release.yml](../../.github/workflows/launcher-release.yml) — release pipeline
