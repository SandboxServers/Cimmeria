# Launcher Storage Setup — Azure Blob

Operational runbook for the storage backend the launcher reads (seed +
patches + manifest) and writes (debug-log zips) against.

Once-only setup. Re-read when rotating the log-upload SAS or migrating
to a new storage account.

---

## What gets stored

| Path in container | Producer | Access |
|---|---|---|
| `manifest.json` | published from build tooling (out of scope) | anonymous read |
| `seed/sgw-<version>.zip` | published from build tooling | anonymous read |
| `patches/<id>.zip` | published from build tooling | anonymous read |
| `logs/<host>-<utc>-<digest12>.zip` | launcher's "Upload Debug Logs" button | **create-only** via SAS |

The launcher reads everything except `logs/` anonymously. Writes are
scoped to the `logs/` prefix via a SAS token with `c` (create)
permission and no read/list/delete rights.

---

## One-time setup

### 1. Create the storage account + container

```bash
az group create --name cimmeria --location <region>

az storage account create \
  --name cimmeriastorage \
  --resource-group cimmeria \
  --sku Standard_LRS \
  --kind StorageV2 \
  --allow-blob-public-access true

az storage container create \
  --name sgw \
  --account-name cimmeriastorage \
  --public-access blob
```

`--public-access blob` makes individual blobs anonymously readable
(per-blob URLs work; `?restype=container&comp=list` does not). That's
what we want — players can fetch `manifest.json`, the seed, and patches
without auth, but the container can't be enumerated.

### 2. Mint the log-upload SAS

A user-delegation or account SAS with create-only permission on the
`/logs` prefix, valid for ~1 year. **Do not** grant `r`, `l`, `d`, or
`w` — only `c`.

```bash
# 1 year from now, UTC ISO 8601.
EXPIRY=$(date -u -d "+365 days" +%Y-%m-%dT%H:%MZ)

# Account SAS (simplest — no Entra ID required). For higher security
# preferential is a user-delegation SAS bound to a service principal.
az storage account generate-sas \
  --account-name cimmeriastorage \
  --services b \
  --resource-types co \
  --permissions c \
  --expiry "$EXPIRY" \
  --https-only \
  --output tsv
```

Build the SAS URL as `https://cimmeriastorage.blob.core.windows.net/sgw?<sas>`.
The launcher inserts the blob path (`logs/<host>-<utc>-<digest12>.zip`)
between the container and the query string at upload time.

> **Important:** the SAS does **not** include the `logs/` prefix
> restriction at the SAS level — `c` permission is granted to the whole
> container. The launcher writes only under `logs/` by convention. If
> you need a hard guarantee, use a user-delegation SAS with a stored
> access policy that scopes the prefix.

### 3. Add the SAS URL as a GitHub secret

In the repo settings → Secrets and variables → Actions → New repository
secret:

- Name: `LAUNCHER_LOG_SAS_URL`
- Value: `https://cimmeriastorage.blob.core.windows.net/sgw?sv=...&ss=b&srt=co&sp=c&se=...&sig=...`

The `launcher-release.yml` workflow injects this into the release build
via `env: LAUNCHER_LOG_SAS_URL`, which the build picks up via
`option_env!("LAUNCHER_LOG_SAS_URL")` in
[`crates/launcher/src/config.rs`](../../crates/launcher/src/config.rs).

PR and main-branch builds do **not** read this secret. Their builds
produce a launcher with the upload button disabled — that's intentional;
no test/CI run should ever upload to production storage.

### 4. (Optional) Lifecycle policy on `logs/`

To cap storage costs, add a lifecycle rule that auto-deletes uploaded
log zips after 90 days:

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

Apply via the Azure portal (Storage Account → Data management →
Lifecycle management) or `az storage account management-policy create`.

---

## Rotation

To rotate the log-upload SAS:

1. Generate a new SAS with the command in section 2 above (new expiry).
2. Update the `LAUNCHER_LOG_SAS_URL` GitHub secret with the new value.
3. Cut a new launcher release via `/release-launcher` on a merged PR
   (or `workflow_dispatch` on `launcher-release.yml` directly). The new
   binary bakes in the new SAS.
4. Old launcher binaries continue working until the old SAS expires.

There is no revocation pathway short of expiry unless you use a
user-delegation SAS with a stored access policy whose validity you can
shorten. Account SAS revocation requires rotating the storage account
key, which would break anonymous reads as well.

---

## Cost notes

Anonymous reads are billed per GB of egress + per 10,000 transactions.
Single-shot debug-log uploads are billed per PUT (one transaction per
upload click). The dedupe ledger ensures repeat clicks against unchanged
logs are zero-transaction.

A rough back-of-envelope: 100 active players uploading once a week →
~400 PUTs/month. Negligible at hot-tier prices.

The seed zip is the only large download; consider serving it from a CDN
front-door if monthly egress crosses a meaningful threshold.

---

## Cross-references

- Launcher design: [docs/client/sgw-launcher.md](sgw-launcher.md)
- Release workflow: [.github/workflows/launcher-release.yml](../../.github/workflows/launcher-release.yml)
- Code that reads the SAS: [`crates/launcher/src/config.rs`](../../crates/launcher/src/config.rs) (`LOG_UPLOAD_SAS_URL`)
- Code that performs the PUT: [`crates/launcher/src/logs.rs`](../../crates/launcher/src/logs.rs) (`upload_blob`)
