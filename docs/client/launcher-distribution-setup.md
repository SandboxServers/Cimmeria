# Launcher Distribution Setup

Operational runbook for everything the launcher reads from or writes to
remote services:

- **Content distribution** — seed zip, patch zips, manifest, signature.
  Hosted on **GitHub Releases**. Anonymous read, generous bandwidth,
  zero infra to operate.
- **Manifest signing** — Ed25519 keypair, private key kept offline,
  pubkey baked into release builds via a GitHub Actions secret.
- **Debug-log uploads** — single PUT per click into an Azure Blob
  container scoped to a `logs/` prefix via a SAS token. The only
  remaining piece of operator-managed infrastructure.

Once-only setup. Re-read when rotating the manifest signing key, the
log-upload SAS, or migrating to a new content release tag.

> **Why this layout?** The MMO emulator already uses GitHub Releases
> for the launcher binary itself, so reusing it for content keeps the
> infra footprint minimal and gives us free signed-build provenance.
> Log uploads can't sensibly live on GitHub Releases (per-player writes
> from an unauthenticated client) so they stay on Azure Blob with a
> create-only SAS — see Cady's review on PR #343 for the design
> discussion.

---

## Part 1 — Content distribution via GitHub Releases

### Layout

Two release tag families:

| Tag | Mutability | Contents |
|---|---|---|
| `content-current` | **mutable** (overwrite on each publish) | `manifest.json`, `manifest.json.sig` |
| `content-YYYY-MM-DD-NNN` | **immutable** (never overwrite) | `seed.zip`, `patches/*.zip` |

The launcher's default manifest URL points at the rolling
`content-current` tag:

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

### Publishing a content drop

```powershell
# 1. Build and sign the manifest.
$priv = Get-Content -Raw .\secrets\manifest-signing.key  # 64 hex chars
$tag  = "content-$(Get-Date -Format yyyy-MM-dd)-001"

# Sign manifest.json — see "Generating the signing key" below for the
# helper script that produces manifest.json.sig from manifest.json.
.\tools\sign-manifest.ps1 -KeyHex $priv -Manifest manifest.json

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

The manifest references the immutable tag's assets by absolute URL,
so it doesn't matter whether `content-current` or `content-YYYY-MM-DD-NNN`
is created first — both must be live before any launcher resolves them.
A safe ordering: create the immutable release **first**, then update
`content-current` second.

### Initial setup

One-time:

```powershell
# Create the rolling tag. Empty release; we'll upload manifest.json on
# the first content drop.
gh release create content-current --notes "Rolling manifest pointer"

# Optional: pin it as the repo's "latest" release for casual visibility.
# Skip this if you'd rather have the launcher-XXXX release be the
# user-facing one.
# gh release edit content-current --latest
```

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

# Derive the public key (run via the launcher's test helper or any
# Ed25519 library; the launcher repo ships an example):
cargo run -p sgw-launcher --example pubkey-from-priv \
  --release -- $(cat manifest-signing.key)
# Prints 64 hex chars — that's the public key.
```

Store the **private key** (`manifest-signing.key`) in a password
manager, hardware token, or offline encrypted backup. **Never** commit
it to the repo and never paste it into a shell history.

Store the **public key** (the printed 64-hex-char string) as a GitHub
Actions secret:

- Repo Settings → Secrets and variables → Actions → New repository secret
- Name: `LAUNCHER_MANIFEST_PUBKEY_HEX`
- Value: the 64-hex-char public key string

The release workflow ([`.github/workflows/launcher-release.yml`](../../.github/workflows/launcher-release.yml))
injects it into the build via `env: LAUNCHER_MANIFEST_PUBKEY_HEX`,
which the build picks up via `option_env!` in
[`crates/launcher/src/manifest.rs`](../../crates/launcher/src/manifest.rs).

### Signing a manifest

The publish flow above invokes `tools\sign-manifest.ps1` (or your
preferred equivalent). The signing operation is:

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
3. Cut a new launcher release via `/release-launcher` ChatOps. The new
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

## Part 3 — Debug-log uploads via Azure Blob

This piece is unchanged from the original Azure-only design. Log
uploads need a write-capable destination that the launcher can reach
without per-user credentials; GitHub Releases doesn't support
unauthenticated writes.

### One-time setup

```bash
az group create --name cimmeria --location <region>

az storage account create \
  --name cimmeriastorage \
  --resource-group cimmeria \
  --sku Standard_LRS \
  --kind StorageV2

# Container is NOT public — only the log-upload SAS writes to it. No
# anonymous reads needed; logs are only accessed by operators via the
# Azure portal or `az storage blob` commands.
az storage container create \
  --name sgw \
  --account-name cimmeriastorage
```

### Mint the log-upload SAS

A SAS with **create-only** permission on the `logs/` prefix, valid for
~1 year. Do not grant `r`, `l`, `d`, or `w` — only `c`.

```bash
EXPIRY=$(date -u -d "+365 days" +%Y-%m-%dT%H:%MZ)

az storage account generate-sas \
  --account-name cimmeriastorage \
  --services b \
  --resource-types co \
  --permissions c \
  --expiry "$EXPIRY" \
  --https-only \
  --output tsv
```

Build the SAS URL: `https://cimmeriastorage.blob.core.windows.net/sgw?<sas>`.
Add as a GitHub Actions secret named `LAUNCHER_LOG_SAS_URL`.

Higher-security alternative: use a user-delegation SAS bound to a
service principal, with a stored access policy scoping it to the
`logs/` prefix specifically. Skip if the threat model (Part 4 below)
doesn't justify the extra ops.

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
        "filters": { "blobTypes": ["blockBlob"], "prefixMatch": ["sgw/logs/"] },
        "actions": { "baseBlob": { "delete": { "daysAfterModificationGreaterThan": 90 } } }
      }
    }
  ]
}
```

---

## Part 4 — Threat model: SAS in the client binary

The log-upload SAS is **baked into the released launcher .exe at
compile time** via `option_env!("LAUNCHER_LOG_SAS_URL")` in
[`crates/launcher/src/config.rs`](../../crates/launcher/src/config.rs).
Anyone who can `strings` the binary or attach a debugger can extract
and reuse the SAS URL.

This is a deliberate trade-off — the alternative is a server-issued
short-lived upload token, which requires a server-side issuer endpoint
that doesn't exist yet (see Cady's review on PR #343 for the longer
discussion).

| Concern | Mitigation |
|---|---|
| Read other players' logs | SAS has `c` (create) permission only — no read/list/delete |
| Tamper with seed / patches / manifest | **Closed by manifest signing** (Part 2). Even an attacker with full container write can't ship a valid signed manifest. |
| Storage cost amplification (junk uploads) | Lifecycle policy auto-deletes after 90 days (Part 3, optional). Account-level spending cap on the storage account caps the worst case. |
| Identity exposure | None — SAS is a signed query string, doesn't carry account keys or AAD tokens. |

### Operator action items before publishing the first launcher release

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

1. Generate a new SAS (Part 3).
2. Update the `LAUNCHER_LOG_SAS_URL` GitHub secret with the new value.
3. Cut a new launcher release via `/release-launcher`. The new binary
   bakes in the new SAS.
4. Old launcher binaries continue working until the old SAS expires.

There is no revocation pathway short of expiry unless you use a
user-delegation SAS with a stored access policy whose validity you
can shorten. Account SAS revocation requires rotating the storage
account key, which would break any other use of the account.

---

## Cross-references

- Launcher design: [docs/client/sgw-launcher.md](sgw-launcher.md)
- User + operator guide: [docs/client/launcher-guide.md](launcher-guide.md)
- Release workflow: [.github/workflows/launcher-release.yml](../../.github/workflows/launcher-release.yml)
- Manifest signing code: [`crates/launcher/src/manifest.rs`](../../crates/launcher/src/manifest.rs) (`MANIFEST_SIGNING_PUBKEY`, `verify_manifest_signature`)
- Log-upload code: [`crates/launcher/src/logs.rs`](../../crates/launcher/src/logs.rs) (`upload_blob`)
