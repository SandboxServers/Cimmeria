# Dev-Session Telemetry — Operations

Operator-facing runbook for the launcher telemetry pipeline:
provisioning the shared secret, rotating it, enabling/disabling
ingest, and reading the data downstream.

## Architecture at a glance

```text
launcher                            cimmeria-server
────────────────                    ────────────────
sgw-launcher.exe                    /api/auth/dev-session
   ├─ identity.json                 POST → HMAC token
   ├─ tail Binaries/sessions/*.log
   ├─ POST /api/telemetry/upload-chunk  ───┐
   ├─ game exits                            ▼
   └─ POST /api/telemetry/upload-bundle ──┐ tracing::* replay
                                          ▼
                                          OTLP layer ────▶ SigNoz / ClickHouse
                                          file sinks ────▶ logs/*.log on disk
                                          WebSocket  ────▶ admin UI live stream
```

cimmeria-server holds the HMAC secret and verifies tokens it minted
itself — no external Functions app in the loop. Launcher uploads land
directly on cimmeria-server, get replayed through the `tracing`
subscriber, and reach SigNoz via the OTLP exporter. The same events
also stream to the admin WebSocket and on-disk per-system log files.

> **Historical note.** Earlier iterations of the pipeline routed
> launcher uploads through an external Cosmos-backed Functions app
> (`Cimmeria-MCP`). That path is retired — launcher uploads now go
> straight to cimmeria-server's admin port and flow into SigNoz.
> Cimmeria-MCP retains the *read* side (LLM-mediated queries against
> SigNoz) but no longer writes launcher data. See
> [docs/architecture/observability.md](../architecture/observability.md).

## Secret provisioning (GitHub Actions Secrets)

Single secret, **64 random bytes**, named
`CIMMERIA_TELEMETRY_HMAC_SECRET`. Configured as a GitHub Actions
repo (or org-level) secret on `SandboxServers/Cimmeria`. The deploy
workflow injects it as env on the running cimmeria-server process.
The server reads `std::env::var("CIMMERIA_TELEMETRY_HMAC_SECRET")` at
mint and upload-verify time through a single loader,
`crates/admin-api/src/routes/dev_session.rs::load_secret` (line 266).
The ingest side deliberately calls that same function rather than
loading the secret itself — see
`crates/admin-api/src/routes/telemetry/handlers.rs:268-271`.

The `SandboxServers/Cimmeria-MCP` repo previously held a mirror copy
of this secret for token verification on its end of the upload flow.
With Cimmeria-MCP out of the write path, that secret is no longer
required there — remove it during the next secret rotation.

### Generating the value

```bash
openssl rand -hex 64
```

Produces 128 hex chars = 64 raw bytes. The verifier accepts both the
hex form and a raw-bytes UTF-8 form (any length ≥ 32 bytes). Anything
shorter is rejected with `AuthError::SecretTooShort` — operator
misconfiguration, not silent token issuance against a weak key.

### Setting the secret

```bash
gh secret set CIMMERIA_TELEMETRY_HMAC_SECRET \
    --repo SandboxServers/Cimmeria \
    --body "$(openssl rand -hex 64)"
```

### Rotation

1. Generate new value: `openssl rand -hex 64`.
2. Update the GitHub Secret.
3. Redeploy cimmeria-server.
4. Existing in-flight tokens become invalid mid-flight; affected
   launchers retry through `auth::fetch_dev_session` and get a fresh
   token within seconds. No user-visible disruption beyond a single
   `Telemetry session error` log line.

**Cadence:** rotate every ~90 days or immediately on any suspicion of
exposure. The secret lives only in GitHub Actions secret store + the
running cimmeria-server process env.

## Pointing the launcher at a non-localhost server

The dev-session mint hands the launcher a `upload_endpoint` URL.
Default is `http://localhost:8443/api/telemetry` — fine when the
launcher and the server share a host. For any other topology, set:

```bash
CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT="https://signoz.<your-domain>/api/telemetry"
```

on the cimmeria-server process. If you're routing through the
Cloudflare Tunnel that exposes the SigNoz UI (see
[signoz-remote-access.md](signoz-remote-access.md)), add another
`ingress` rule to your `cloudflared` config pointing
`signoz.<your-domain>/api/telemetry` at the admin-port backend.

## Kill switch

`CIMMERIA_TELEMETRY_KILL_SWITCH=1` on the cimmeria-server process →
every `/api/auth/dev-session` call returns 503 with `Retry-After: 60`.
The launcher logs a warn and continues launching the game without
telemetry. In-flight upload requests are NOT rejected by the kill
switch — they complete using the token they already hold — but new
sessions can't start until the switch is released.

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

Every event ends up in one place: SigNoz / ClickHouse, indexed by:

- `service.name = "cimmeria-server"` (the host that ingested it)
- `target` distinguishes producers:
  - `launcher.client_log` — parsed Atera client log lines
  - `launcher.debug_log` — debug-channel launcher events
  - `launcher.key_dump` — encryption key material (debug level — not
    written to public sinks at info)
  - `launcher.session_meta` — session boundaries and rotation events
  - `launcher.bundle` — per-bundle metadata and per-line replay from
    end-of-session zips
  - `launcher.ingest` — server-side accept/reject counters
- `session_id` — the launcher session UUID minted by dev-session
- `install_id` — stable per-install identifier

Retention is whatever the ClickHouse TTL says (see
[signoz-deployment.md](signoz-deployment.md#retention)).

## User opt-out

The launcher's TelemetrySettings config (`telemetry.enabled`,
default `true`) controls whether the "Launch + Telemetry" button is
enabled. When `false`:

- No `/api/auth/dev-session` POST fires.
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
- KeyDump events carry encryption key material from the client. They
  ship at **debug** level only — the default file sinks and admin
  WebSocket drop them; only SigNoz (when configured for debug ingest)
  retains them. Use this lever to keep raw keys off long-lived
  on-disk storage.
- Per-event PII redaction is explicitly out of scope (dev-only data;
  the developer is the device owner).

## Endpoints

| Path | Method | Auth | Purpose |
|---|---|---|---|
| `/api/auth/dev-session` | POST | none (anyone can mint) | Mint a token for a launcher session. |
| `/api/auth/dev-session/refresh` | POST | bearer (own token) | Extend an almost-expired token. |
| `/api/telemetry/upload-chunk` | POST | bearer | Streaming events (gzip(NDJSON)). |
| `/api/telemetry/upload-bundle` | POST | bearer | End-of-session zip (multipart). |

A 503 with `Retry-After` on any of these means the kill switch is on.
A 401 on upload endpoints means the token expired (refresh) or was
never valid (mint a fresh session).
