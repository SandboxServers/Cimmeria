---
title: "Launcher Distribution Setup"
type: how-to
audience: operators
last_updated: 2026-10-11
companion_docs:
  - launcher-guide.md
  - launcher-self-update.md
  - ../../crates/launcher/desktop/docs/release.md
---

# Launcher Distribution Setup

Operational runbook for everything the launcher reads from or writes to
remote services, and the three keys that protect them:

- **Content distribution** — seed, patch zips, manifest, signature.
  Hosted on **GitHub Releases**. Anonymous read, generous bandwidth,
  zero infra to operate.
- **Manifest signing** — an Ed25519 key pair. The private key is kept
  offline; the public key is compiled into release builds from the
  `LAUNCHER_MANIFEST_PUBKEY_HEX` secret.
- **Launcher updates** — a Minisign key pair that signs the launcher zip and
  its updater feed. The private key is in Key Vault and a GitHub environment;
  the public key is compiled in from the `LAUNCHER_UPDATER_PUBKEY` variable.
- **Debug-log uploads** — a single PUT per click into an Azure Blob
  container, through a create-only SAS URL compiled in from the
  `LAUNCHER_LOG_SAS_URL` secret. The only operator-managed infrastructure.
  *Legacy:* the desktop launcher's **Upload debug logs** no longer uses this
  path. It posts log lines to the telemetry server
  ([telemetry.md § Launcher log ingest](../operations/telemetry.md#launcher-log-ingest)),
  and LF-18 removes the SAS URL from the build.

Re-read this when rotating a key or the log-upload SAS, or when moving content
to a new release tag. How a launcher release is built and published, job by job,
is in the [desktop launcher release guide](../../crates/launcher/desktop/docs/release.md);
this page covers the services and keys it depends on.

> **Why this layout?** GitHub Releases already hosts the launcher itself, so
> reusing it for content keeps the infra footprint minimal and gives us free
> build provenance. Log uploads can't sensibly live on GitHub Releases
> (per-player writes from an unauthenticated client), so they stay on Azure
> Blob with a create-only SAS — see Cady's review on PR #343 for the design
> discussion.

---

## Part 1 — Content distribution via GitHub Releases

### Layout

Two release tag families carry content:

| Tag | Mutability | Contents |
|---|---|---|
| `content-current` | **mutable** (overwrite on each publish) | `manifest.json`, `manifest.json.sig` |
| `content-YYYY-MM-DD-NNN` | **immutable** (never overwrite) | `seed.zip`, `patches/*.zip` |

The launcher reads the manifest from the rolling `content-current` tag:

```text
https://github.com/SandboxServers/Cimmeria/releases/download/content-current/manifest.json
```

The manifest's `blob` fields use **absolute URLs** pointing into the
immutable per-publication release tags:

```json
{
  "schema": 1,
  "seed": {
    "blob": "https://github.com/SandboxServers/Cimmeria/releases/download/content-2026-05-20-001/seed.zip",
    "size": 5234567890,
    "sha256": "abc123..."
  },
  "patches": [
    {
      "id": "001-base",
      "blob": "https://github.com/SandboxServers/Cimmeria/releases/download/content-2026-05-20-001/001-base.zip",
      ...
    }
  ]
}
```

This split gives both properties at once:

- The manifest is **mutable** so you can publish a new content drop by
  overwriting `manifest.json` in `content-current`.
- The seed and patches are **immutable** so existing installs can
  resume partial downloads safely — the bytes at a given URL never
  change once published.

### Launcher releases

Launcher releases are two more tag families, built and published by
[`launcher-release.yml`](../../.github/workflows/launcher-release.yml) when a
maintainer comments `/release-launcher` on a merged PR:

| Tag | Mutability | Contents |
|---|---|---|
| `launcher-YYYYMMDD-<sha7>` | **immutable**, one per release | `StargateWorlds-Launcher-windows-x64.zip` with its `.zip.sha256` and `.zip.sig`, `StargateWorlds-Launcher-macos-arm64.zip` with its `.zip.sha256`, and the client-patches UI overlay when one is committed |
| `launcher-current` | **mutable**, refreshed by each signed release (D-LX7) | The two zips with their `.sha256`, the Windows `.zip.sig`, and the updater feed `latest.json` |

Players and docs link to the stable `launcher-current` URLs. The launcher's
updater reads `latest.json` there, which names the zip on the dated release
([self-update](launcher-self-update.md)). `launcher-current` is refreshed only
by a signed release that is the day's first, and is never marked as the
repository's latest release. The [release guide](../../crates/launcher/desktop/docs/release.md#what-a-release-publishes)
has the exact rules, and how to recover a failed or partial run.

To make players update, set the content manifest's optional `min_launcher` to
the release's tag and re-sign the manifest; see
[launcher-guide.md](launcher-guide.md#minimum-launcher-version-min_launcher).
Publish the launcher release before the manifest that requires it.

### Publishing a content drop

```powershell
# 1. Build and sign the manifest (manifest.json -> manifest.json.sig).
$tag  = "content-$(Get-Date -Format yyyy-MM-dd)-001"
cargo run -p cimmeria-patchset -- sign manifest.json --key <path to manifest-signing.key>
cargo run -p cimmeria-patchset -- verify manifest.json --pubkey <public key hex>

# 2. Create the immutable per-publication release containing seed +
#    patches. Mark as a pre-release so it doesn't show on the front
#    page of the repo's releases list.
gh release create "$tag" --notes "Content drop $tag" --prerelease `
  seed.zip patches/*.zip

# 3. Update the rolling content-current release with the new manifest +
#    signature. --clobber overwrites the existing assets in place.
gh release upload content-current --clobber `
  manifest.json manifest.json.sig
```

Create the immutable release **first** and update `content-current` second, so
no launcher ever reads a manifest whose blobs don't exist yet.

### File index

`file-index.json` lists the path, size and SHA-256 of every file the release lays down before setup. It holds no game content: no file bytes, only those three fields (D-AI5). Build it from the local seed and patch archives once the patch list is final:

```powershell
cargo run -p cimmeria-launcher-core --bin build-file-index -- `
  --manifest manifest.json --seed seed.zip --patch <id>=<zip> ... --out file-index.json
```

- Attach `file-index.json` to its own release, `content-file-index-<yyyymmdd>` (see the publication bullet below).
- Paste the printed `file_index` entry into `manifest.json` before signing, and set its `blob` to the attached asset's full https URL.
- `pack-client-overlay` drops a stale index: when it appends a patch, it sets `file_index` to `None` and prints a reminder. Rebuild the index after adding patches, before signing.
- A missing or mismatched index makes the launcher build one from the seed. That is a fallback, never a wrong verdict.
- Publishing is an owner step: no agent signs, uploads or reads the release key. The runbook, in order:
  1. Build the tools with a lane build: `$env:LANE_VERBOSE='1'; pwsh tools/build-lane/lane.ps1 cargo build --locked -p cimmeria-launcher-core --bins -p cimmeria-patchset --bin cimmeria-patchset`. The lane log prints `target=<dir>`; the exes are in `<dir>\debug`.
  2. Prepare: `pwsh tools/content-release/prepare-file-index.ps1 -Seed seed.zip -Patch <id>=<zip> ... -Current manifest.json -Out <dir> -BlobPath <full https URL of the file-index.json asset> -PubKey <release public key, hex> -BuildExe <dir>\debug\build-file-index.exe -VerifyExe <dir>\debug\verify-file-index.exe`. It verifies the current manifest, builds `file-index.json`, writes `manifest.candidate.json` (the current manifest plus its `file_index` entry, nothing else changed) and runs `verify-file-index`. It stops on any FAIL and never signs or uploads.
  3. Chain rule: the candidate only appends. The seed and every patch keep their order, `id`, `after`, `size`, `sha256` and `blob`. `after` links are never remapped; a reordered patch or a changed `after` fails `append_only`, so a changed patch list is a new publication.
  4. Parity: the same run rebuilds the index from the local seed and patches and compares the parsed values with the candidate's. This is the launcher-verify step before signing; it matches the launcher's own fallback build (above).
  5. Owner only, with the offline release key on the owner's machine: `cimmeria-patchset sign manifest.json --key <offline key file>`, then `cimmeria-patchset verify manifest.json --pubkey <hex>`. Upload `file-index.json` to the `content-file-index-<yyyymmdd>` release first, then upload `manifest.json` and `manifest.json.sig` to `content-current` with `--clobber`. Then run LT-27 U7 on a laptop that would not adopt.
- Each publication uploads `file-index.json` to a **new** GitHub release `content-file-index-<yyyymmdd>`. The tracked script `tools/content-release/prepare-file-index.ps1` checks it. `-BlobPath` is that asset's full URL, `https://github.com/<owner>/<repo>/releases/download/content-file-index-<yyyymmdd>/file-index.json`. The launcher accepts an https URL on any host, or a relative name.
- Publish and roll back (owner only; this list expands step 5 above and replaces its upload and swap order):
  1. Back up the live `manifest.json` and `manifest.json.sig` to a backup folder.
  2. Sign: `cimmeria-patchset sign manifest.json --key <offline key file>`.
  3. Run `cimmeria-patchset verify manifest.json --pubkey <hex>`.
  4. Upload `file-index.json` to the new `content-file-index-<yyyymmdd>` release.
  5. Swap: `gh release upload content-current --clobber manifest.json manifest.json.sig`.
  6. Download the live `manifest.json` and the new `file-index.json` again and run `verify-file-index --manifest <downloaded manifest.json> --file-index <downloaded file-index.json> --previous <backup>\manifest.json`. On any FAIL, roll back at once (step 7).
  7. Rollback, if needed: `gh release upload content-current --clobber <backup>\manifest.json <backup>\manifest.json.sig`.
- The index carries only paths, sizes and hashes. The seed is never rehosted.

### Initial setup

One-time:

```powershell
# Create the rolling tag. Empty release; we'll upload manifest.json on
# the first content drop.
gh release create content-current --notes "Rolling manifest pointer"
```

`launcher-current` needs no setup: the first signed launcher release creates
it.

### Publishing the client-patches UI overlay

The client-patches DLL ships inside the launcher zip
(`windows\cimmeria_client_patches.dll`, pinned by checksum), so a launcher
release ships it. Its UI overlay (the Lua and layout files under
`crates/client-patches/overlay/`, listed in its `MANIFEST.txt`) ships as a
content patch instead. Every launcher release packs it with
`pack-client-overlay` and attaches two assets to the dated launcher release:

- `bm-ui-overlay-<hash>.zip`: the overlay files, laid out as they go
  under the client's `SGWGame/` directory. `<hash>` is the first 12 hex
  digits of the zip's SHA-256, so an unchanged overlay packs to the same
  id and a changed one to a new id.
- `bm-ui-overlay-<hash>.entry.json`: its manifest entry, with the blob
  URL pointing at that release (an immutable tag) and
  `"root": "sgw_game"`.

To publish it, add the entry to the content manifest and sign the
result. The tool does the merge, putting the entry `after` the current
last patch and leaving the manifest unchanged when the id is already
there:

```bash
cargo run -p cimmeria-launcher-core --bin pack-client-overlay -- \
  --overlay crates/client-patches/overlay \
  --out-dir dist \
  --blob-base-url https://github.com/SandboxServers/Cimmeria/releases/download/<launcher tag> \
  --manifest manifest.json --manifest-out manifest.new.json
# Check manifest.new.json, sign it (Part 2), then upload it as
# manifest.json with its .sig to content-current.
```

Run it at the commit the launcher release was built from, or the hash
will not match the attached zip. The release notes say when an overlay
was attached.

---

## Part 2 — Manifest signing keypair

The launcher refuses any manifest whose `.sig` doesn't verify against
the embedded public key. This closes the manifest-tampering threat:
even if someone compromises the GitHub repo or MITMs the download, a
fake manifest won't have a valid signature.

### Generating the signing key

One-time, on a trusted offline machine:

```bash
# Generate 32 bytes of cryptographic randomness — that's the private key.
openssl rand -hex 32 > manifest-signing.key
chmod 600 manifest-signing.key

# Derive the public key with any Ed25519 library, e.g.:
python -c "import sys;from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey;from cryptography.hazmat.primitives import serialization as s;k=Ed25519PrivateKey.from_private_bytes(bytes.fromhex(open('manifest-signing.key').read().strip()));print(k.public_key().public_bytes(s.Encoding.Raw,s.PublicFormat.Raw).hex())"
# Prints 64 hex chars — that's the public key.
```

Or, with the repo's own tool:
`cargo run -p cimmeria-patchset -- pubkey --key manifest-signing.key`.

Store the **private key** (`manifest-signing.key`) in a password
manager, hardware token, or offline encrypted backup. **Never** commit
it to the repo and never paste it into a shell history.

Store the **public key** (the printed 64-hex-char string) as a GitHub
Actions secret:

- Repo Settings → Secrets and variables → Actions → New repository secret
- Name: `LAUNCHER_MANIFEST_PUBKEY_HEX`
- Value: the 64-hex-char public key string

The release workflow passes it to the launcher build steps only (the test
suites use the development key and fail with it set). The build compiles it in
through `option_env!` as `MANIFEST_SIGNING_PUBKEY` in
[`crates/launcher-core/src/manifest.rs`](../../crates/launcher-core/src/manifest.rs).
A release build without a usable key verifies nothing and fails closed.

### Signing a manifest

Sign with `cargo run -p cimmeria-patchset -- sign manifest.json --key
<key file>`, which writes `manifest.json.sig`, and check it with
`cimmeria-patchset verify manifest.json --pubkey <hex>` before
uploading. The launcher enforces the signature (`verify_manifest_signature`
in `manifest.rs`), so an unsigned or wrongly-signed manifest is rejected
outright with no fallback. Sign on the machine that holds the key; never put
the key in CI.

The signing operation is:

```text
sig_bytes = ed25519_sign(private_key, manifest.json contents)
manifest.json.sig = hex_encode(sig_bytes)   # 128 chars
```

Any Ed25519 library produces the same bytes given the same key and
input. The hex encoding is a convenience: it makes `cat manifest.json.sig`
in CI logs readable, and trips on a stray newline (the launcher's
verifier trims whitespace before decoding).

### Rotating the signing key

To rotate (recommended every 12 months, or immediately if the private
key is suspected compromised):

1. Generate a new keypair (above).
2. Update `LAUNCHER_MANIFEST_PUBKEY_HEX` repo secret with the new
   public key.
3. Cut a new launcher release via `/release-launcher`. The new
   binary bakes in the new pubkey and accepts only manifests signed
   with the new private key.
4. **Critical:** any existing launcher in the wild was built against
   the old pubkey. It will keep accepting old-signed manifests; you
   cannot retroactively invalidate them. To force a hard cutover,
   pause publishing new content for the old launcher and announce that
   players must update.

There is no built-in multi-key support today. Future work could embed
a small set of pubkeys and accept signatures from any of them — useful
for zero-downtime rotation.

---

## Part 3 — Launcher updater signing key

The launcher updates itself only from a zip whose Minisign signature verifies
against the public key compiled into it (D-LX9). This key is separate from the
manifest key: the manifest key says what game content to install, the updater
key says which launcher binaries to run.

Where it lives (generated on 2026-10-11):

| What | Where |
|---|---|
| The encrypted secret key | Key Vault `cimmeria-kv`, secret `launcher-updater-minisign-key`; and the `LAUNCHER_UPDATER_MINISIGN_KEY` secret of the `launcher-release` GitHub environment |
| Its password | Key Vault `cimmeria-kv`, secret `launcher-updater-minisign-password`; and the `LAUNCHER_UPDATER_MINISIGN_PASSWORD` environment secret |
| The public key | Key Vault `cimmeria-kv`, secret `launcher-updater-pubkey`; and the repository variable `LAUNCHER_UPDATER_PUBKEY` |

The two secrets are **environment** secrets, not repository secrets, so only the
release workflow's `sign` job sees them, and the `launcher-release` environment
admits the `main` branch only. The public key is a variable rather than a
secret: it is public, and a visible value makes a mismatch easy to spot.

Without the secrets a release still builds and publishes its dated release, but
unsigned: no feed, no `launcher-current` refresh, so the stable links don't move.
Without the variable, launchers build with their updater **Disabled**. The
private key without the variable fails the `sign` job.

To generate the pair for a new repository or a rotation, follow the
[key holder steps](../../crates/launcher/desktop/docs/release.md#key-holder-steps-d-lx9)
in the release guide. **Rotating strands every launcher built with the old public
key**: it can no longer verify a release and never updates again, so its players
replace it once by hand from the stable link. Rotate only if the key is
compromised, and announce it as for the egui retirement
([announcement](../analysis/launcher-consolidation/announcement.md)).

---

## Part 4 — Debug-log uploads via Azure Blob

> [!WARNING]
> **Legacy.** The desktop launcher's upload is a log ingest into SigNoz (see
> [telemetry.md § Launcher log ingest](../operations/telemetry.md#launcher-log-ingest)),
> and `LAUNCHER_LOG_SAS_URL` is unused by it. LF-18 removes the secret from the
> build and this Part with it. Nothing below is needed for a launcher that ingests
> logs. A launcher built before LF-15 still uploads here.

Log uploads need a write-capable destination that the launcher can reach
without per-user credentials; GitHub Releases doesn't support
unauthenticated writes. The launcher's **Upload debug logs** button zips the
game's logs and PUTs the zip once as `logs/<host>-<utc>-<digest>.zip` (D-LX11).

### One-time setup

```bash
az group create --name <resource-group> --location <region>

az storage account create \
  --name <storage-account> \
  --resource-group <resource-group> \
  --sku Standard_LRS \
  --kind StorageV2

# Container is NOT public — only the log-upload SAS writes to it. No
# anonymous reads needed; logs are only accessed by operators via the
# Azure portal or `az storage blob` commands.
az storage container create \
  --name <container> \
  --account-name <storage-account>
```

### Mint the log-upload SAS

A SAS with **create-only** permission (`c`) on that one container, with an
expiry about a year out. Do not grant `r`, `l`, `d`, or `w`: create cannot
overwrite an existing blob, and every upload's name is unique, so a leaked SAS
can add blobs but never replace other players' logs. Write (`w`) would let it
overwrite them.

```bash
EXPIRY=$(date -u -d "+365 days" +%Y-%m-%dT%H:%MZ)

az storage container generate-sas \
  --account-name <storage-account> \
  --name <container> \
  --permissions c \
  --expiry "$EXPIRY" \
  --https-only \
  --output tsv
```

Build the SAS URL, `https://<storage-account>.blob.core.windows.net/<container>?<sas>`,
and store it as the GitHub Actions secret `LAUNCHER_LOG_SAS_URL`. The release
workflow passes it to the launcher build steps, and the desktop engine compiles
it in with `option_env!` (`engine/src/storage/log_upload/store.rs`). A build
without it hides the button.

Higher-security alternative: use a user-delegation SAS bound to a
service principal, with a stored access policy. Skip if the threat model
(Part 5 below) doesn't justify the extra ops.

### Optional lifecycle policy

Auto-delete uploaded log zips after 90 days to cap storage cost:

```json
{
  "rules": [
    {
      "name": "expire-debug-logs",
      "enabled": true,
      "type": "Lifecycle",
      "definition": {
        "filters": { "blobTypes": ["blockBlob"], "prefixMatch": ["<container>/logs/"] },
        "actions": { "baseBlob": { "delete": { "daysAfterModificationGreaterThan": 90 } } }
      }
    }
  ]
}
```

---

## Part 5 — Threat model: secrets in the client binary

The log-upload SAS is **compiled into the released launcher** with
`option_env!("LAUNCHER_LOG_SAS_URL")`. Anyone who can run `strings` on the
binary or attach a debugger can extract and reuse the SAS URL. The two public
keys are compiled in too, but they are public by design.

This is a deliberate trade-off — the alternative is a server-issued
short-lived upload token, which requires a server-side issuer endpoint
that doesn't exist yet (see Cady's review on PR #343 for the longer
discussion).

| Concern | Mitigation |
|---|---|
| Read other players' logs | SAS has `c` (create) permission only — no read/list/delete |
| Overwrite other players' logs | Create cannot overwrite an existing blob, and every upload name is unique |
| Tamper with seed / patches / manifest | **Closed by manifest signing** (Part 2). Even an attacker with full container write can't ship a valid signed manifest. |
| Ship a malicious launcher update | **Closed by updater signing** (Part 3). The feed and zip must verify against the compiled-in Minisign key, and the signed version must match the offer. |
| Storage cost amplification (junk uploads) | Lifecycle policy auto-deletes after 90 days (Part 4, optional). Account-level spending cap on the storage account caps the worst case. |
| Identity exposure | None — SAS is a signed query string, doesn't carry account keys or AAD tokens. |

### Operator action items before publishing a launcher release

1. **Storage cost cap.** Subscription → Cost Management → Budgets →
   alert at the monthly budget you're comfortable with (e.g. $10/month).
   Alerts at 50%, 75%, 90%, 100%.
2. **Upload-rate anomaly alert.** Storage Account → Monitoring →
   Alerts → Signal `Transactions` filtered to `ApiName = PutBlob` and
   `BlobType = BlockBlob`, dynamic threshold over a 1h window.
3. **Rotation cadence.** Regenerate the log SAS every 6 months and
   cut a new launcher release. The previous binary keeps working
   until its baked-in SAS expires; rotation overlap is the natural
   deprecation path for older launcher versions.

If at any point the SAS is observed being abused, the rotation
procedure below gives you a same-day path to cut off the extracted
credential.

### Rotation

1. Generate a new SAS (Part 4).
2. Update the `LAUNCHER_LOG_SAS_URL` GitHub secret with the new value.
3. Cut a new launcher release via `/release-launcher`. The new binary
   bakes in the new SAS, and launchers update to it from Settings.
4. Old launcher binaries continue working until the old SAS expires.

There is no revocation pathway short of expiry unless you use a
user-delegation SAS with a stored access policy whose validity you
can shorten. Account-key-signed SAS revocation requires rotating the storage
account key, which would break any other use of the account.

---

## Cross-references

- Player and operator guide: [docs/client/launcher-guide.md](launcher-guide.md)
- Launcher self-update: [docs/client/launcher-self-update.md](launcher-self-update.md)
- Release workflow and its jobs: [crates/launcher/desktop/docs/release.md](../../crates/launcher/desktop/docs/release.md), [.github/workflows/launcher-release.yml](../../.github/workflows/launcher-release.yml)
- Manifest signing code: [`crates/launcher-core/src/manifest.rs`](../../crates/launcher-core/src/manifest.rs) (`MANIFEST_SIGNING_PUBKEY`, `verify_manifest_signature`)
- Log-upload code: [`crates/launcher/desktop/engine/src/storage/log_upload/`](../../crates/launcher/desktop/engine/src/storage/log_upload/)
- Updater code: [`crates/launcher/desktop/engine/src/storage/updater/`](../../crates/launcher/desktop/engine/src/storage/updater/)
