# Dev-Session Telemetry — Operations

Operator-facing runbook for the launcher telemetry pipeline:
provisioning the shared secret, rotating it, enabling/disabling
ingest, and reading the data downstream.

## Architecture at a glance

```text
launcher                  cimmeria-server                Functions ingest
────────────────          ──────────────                ──────────────
sgw-launcher.exe          /auth/dev-session             /api/upload-chunk
   ├─ identity.json       POST → HMAC token             POST → Cosmos
   ├─ tail Binaries/sessions/*.log
   ├─ POST upload-chunk  ─────────────────────────────▶
   ├─ game exits
   └─ POST upload-bundle ─────────────────────────────▶ /api/upload-bundle
                                                       POST → Blob Storage
```

Two binaries share one secret: `cimmeria-server` mints HMAC-SHA256
tokens, the Functions app verifies them. The launcher itself holds no
secret — it gets a short-lived token from cimmeria-server at game-
launch and presents it to Functions.

## Secret provisioning (GitHub Actions Secrets)

Single shared secret, **64 random bytes**, named
`CIMMERIA_TELEMETRY_HMAC_SECRET`. Configured as a GitHub Actions
repo (or org-level) secret in both repos:

- **`SandboxServers/Cimmeria`** — deploy workflow injects it as env
  on the running cimmeria-server process. The server reads
  `std::env::var("CIMMERIA_TELEMETRY_HMAC_SECRET")` at mint/refresh
  time (`crates/admin-api/src/routes/dev_session.rs::load_secret`).
- **`SandboxServers/Cimmeria-MCP`** — Functions deploy reads the
  same value as a Functions app setting; middleware verifies the
  token signature on every `/api/upload-*` request.

### Generating the value

```bash
openssl rand -hex 64
```

Produces 128 hex chars = 64 raw bytes. `load_secret` accepts both
the hex form and a raw-bytes UTF-8 form (any length ≥ 32 bytes).
Anything shorter is rejected with `AuthError::SecretTooShort` —
operator misconfiguration, not silent token issuance against a
weak key.

### Setting the secret

```bash
gh secret set CIMMERIA_TELEMETRY_HMAC_SECRET \
    --repo SandboxServers/Cimmeria \
    --body "$(openssl rand -hex 64)"

gh secret set CIMMERIA_TELEMETRY_HMAC_SECRET \
    --repo SandboxServers/Cimmeria-MCP \
    --body "<same value as above>"
```

For org-level secrets (single source of truth across both repos):

```bash
gh secret set CIMMERIA_TELEMETRY_HMAC_SECRET \
    --org SandboxServers \
    --visibility selected \
    --repos Cimmeria,Cimmeria-MCP
```

### Rotation

1. Generate new value: `openssl rand -hex 64`.
2. Update both GitHub Secrets simultaneously (or the one org-level
   secret).
3. Redeploy cimmeria-server.
4. Redeploy the Functions app.
5. Existing in-flight tokens become invalid mid-flight; affected
   launchers retry through `auth::fetch_dev_session` and get a fresh
   token within seconds. No user-visible disruption beyond a single
   `Telemetry session error` log line.

**Cadence:** rotate every ~90 days or immediately on any suspicion of
exposure. No KeyVault binding — the secret lives only in GitHub
Actions secret store + the running process envs.

## Kill switch

`CIMMERIA_TELEMETRY_KILL_SWITCH=1` on the cimmeria-server process →
every `/auth/dev-session` call returns 503 with `Retry-After: 60`.
The launcher logs a warn and continues launching the game without
telemetry.

```bash
# Pause ingest without redeploy
ssh cimmeria-server "systemctl set-environment CIMMERIA_TELEMETRY_KILL_SWITCH=1 \
    && systemctl restart cimmeria-server"

# Resume
ssh cimmeria-server "systemctl unset-environment CIMMERIA_TELEMETRY_KILL_SWITCH \
    && systemctl restart cimmeria-server"
```

Only the literal value `1` enables the kill switch — `true`/`yes`/etc
are treated as off (intentional crispness of contract).

## Where the data lives

| Artifact | Storage | Retention |
|---|---|---|
| Per-event NDJSON chunks | Cosmos DB `sessions` container | 30 days (TTL) |
| End-of-session bundles (zip) | Blob Storage `session-bundles` container | 90 days |
| `install_id` partition key | Cosmos DB document `id` | Inherits container TTL |

Per-event events carry `session_id` + monotonic `seq`; bundles carry
`(install_id, machine_id, launcher_version, branch, git_sha,
event_count, dropped_lines, zip_sha256)`.

## User opt-out

The launcher's TelemetrySettings config (`telemetry.enabled`,
default `true`) controls whether the "Launch + Telemetry" button is
enabled. When `false`:

- No `/auth/dev-session` POST fires.
- No log tailing.
- No bundle upload.
- The legacy "Launch Atera Debug" button still works (no
  instrumentation).

## Privacy

- `install_id` and `machine_id` are stable per-install identifiers.
  Logged at **debug** level only (info gets an 8-char correlator) to
  reduce leakage through any future log-upload pipeline.
- `machine_id` is `sha256(HKLM\SOFTWARE\Microsoft\Cryptography\MachineGuid)`
  truncated to 16 hex chars — the raw GUID never leaves the dev's
  machine.
- Per-event PII redaction is explicitly out of scope (dev-only data;
  the developer is the device owner).
