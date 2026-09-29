# Dev-Session Telemetry — Operations

Operator-facing runbook for the launcher and client telemetry pipeline:
provisioning the shared secret, rotating it, enabling it on the colo,
enabling/disabling ingest, and reading the data downstream.

Everything the players' machines upload (the injected
`cimmeria-client-telemetry` DLL's events and the game-log lines the
launcher tails) lands in SigNoz as its own service,
`service.name = cimmeria-client`. Lab sessions are tagged
`cimmeria.session_kind = lab`.

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

The launcher sends telemetry only to `https://` addresses, or to plain
`http://` on the player's own machine (`localhost`, `127.x`, `::1`). That
applies to the auth URL in `launcher-config.json` (`telemetry.auth_url`,
default `http://localhost:8443/api`) and to every `upload_endpoint` the
server hands back. Anything else fails the session before a byte is sent,
with "telemetry needs an https:// server address" in the status log
([`telemetry/endpoint.rs`](../../crates/launcher/src/telemetry/endpoint.rs)).
A remote server therefore needs TLS in front of its admin port, for
example the Cloudflare Tunnel below; the colo's plain `8443` is not
enough.

**Sending your own telemetry to the colo, with SSH access.** Forward the
colo's admin port to your machine, then launch with the default
`telemetry.auth_url`:

```bash
ssh -f -N -o ExitOnForwardFailure=yes -o ServerAliveInterval=30   -L 127.0.0.1:8443:127.0.0.1:8443 <colo host>
```

The launcher then talks to `http://localhost:8443/api`, which the rule
above allows, and the colo's default `upload_endpoint`
(`http://localhost:8443/api/telemetry`) resolves through the same tunnel.
The colo needs `CIMMERIA_TELEMETRY_HMAC_SECRET` set (see
[`docker/compose.yml`](../../docker/compose.yml)). This does not work
with a local server also on port 8443. Players without SSH access need
the HTTPS route.

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

## Enable client telemetry on the colo

Until both values below are set, the colo mints nothing: every launcher
and lab session gets a 500 from `/api/auth/dev-session` and
`cimmeria-client` stays empty. The server says so itself: at startup it
logs `dev-session telemetry: every mint and upload will be refused`
(WARN, `reason = dev_session_secret_unusable`), and each refused request
logs the same reason at ERROR. When it is working, the startup line is
`dev-session telemetry: mint and ingest enabled` with the
`upload_endpoint` it hands out.

1. **Generate the secret** on any machine: `openssl rand -hex 64`.
2. **Give the admin port an HTTPS address.** Players' launchers refuse
   plain `http://` to another machine, and `8443` on the colo is plain
   HTTP. Add a Cloudflare Tunnel hostname (for example
   `telemetry.<your-domain>`) to `/etc/cloudflared/config.yml` that sends
   only the telemetry routes to `http://cimmeria:8443`, and give that
   hostname **no** Cloudflare Access policy (the launcher cannot answer an
   Access login; the routes carry their own HMAC token):

   ```yaml
   ingress:
     - hostname: telemetry.<your-domain>
       path: ^/api/(auth/dev-session|telemetry/)
       service: http://cimmeria:8443
     # ... the existing signoz.<your-domain> rule ...
     - service: http_status:404
   ```

   Route DNS with `cloudflared tunnel route dns cimmeria-signoz
   telemetry.<your-domain>`, and run the tunnel profile
   (`--profile tunnel`). The `path` rule keeps the rest of the admin API,
   which has no authentication (#439), off the public hostname.
3. **Set both variables in `/opt/cimmeria/.env`** (the file
   `docker compose` reads beside `compose.yml`; it is never committed):

   ```bash
   CIMMERIA_TELEMETRY_HMAC_SECRET=<the 128 hex chars from step 1>
   CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT=https://telemetry.<your-domain>/api/telemetry
   ```

   `docker/compose.yml` passes both through as
   `${VAR:-}`. A blank upload endpoint falls back to
   `http://localhost:8443/api/telemetry`, which only a launcher on the
   colo itself can reach.
4. **Recreate the container** so it picks up the environment:
   `docker compose -f compose.yml up -d cimmeria`.
5. **Check it.** `docker logs cimmeria 2>&1 | grep "dev-session telemetry"`
   shows the enabled line. From any machine,
   `curl -s -o /dev/null -w '%{http_code}\n' -X POST -H 'Content-Type: application/json' -d '{}' https://telemetry.<your-domain>/api/auth/dev-session`
   returns `422` (the body is incomplete, so the route is reachable and
   parsing), not `500` or a Cloudflare error page.
6. **Point launchers at it.** Each player's `launcher-config.json` needs
   `telemetry.auth_url = "https://telemetry.<your-domain>/api"` and the
   telemetry opt-in; see [Player opt-in](#player-opt-in).

The lab supervisor mints the same way against `CIMMERIA_LAB_SERVER_URL`;
see [the lab guide](../guides/live-research-lab.md).

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

## Mint and refresh quotas

Anyone who can route TCP to the admin port can mint a telemetry
token, so mint and refresh are rate-limited. A caller over quota gets
`429` with a `Retry-After` header; the launcher already backs off on
that header and falls back to launching without telemetry.

| Variable | Default | Counts |
|---|---|---|
| `CIMMERIA_TELEMETRY_QUOTA_WINDOW_SECS` | `3600` | The window everything below is counted over. |
| `CIMMERIA_TELEMETRY_MINT_QUOTA_PER_IP` | `120` | Mints per peer address per window. |
| `CIMMERIA_TELEMETRY_MINT_QUOTA_PER_INSTALL` | `30` | Mints per `install_id` per window. |
| `CIMMERIA_TELEMETRY_REFRESH_QUOTA_PER_IP` | `480` | Refreshes with a valid token per peer address per window. Charged only after the token verifies. |
| `CIMMERIA_TELEMETRY_REFRESH_BAD_QUOTA_PER_IP` | `30` | Refresh calls whose token fails verification, per peer address per window. A separate counter, so junk tokens cannot spend the valid-token allowance. |
| `CIMMERIA_TELEMETRY_MAX_SESSION_SECS` | `86400` | How long one minted session may be extended by chained refreshes. Not a quota: `0` (or a negative value) does **not** disable the cap, it refuses every refresh. |

Setting a quota to `0` disables that counter. A value that does not
parse falls back to the default rather than refusing to serve — an
operator typo must not take telemetry offline.

**Raise the per-IP mint quota if the admin port sits behind a proxy
or a shared egress address.** The counter keys on the peer address,
and no `X-Forwarded-For` header is read (reading one unconditionally
would let any caller pick its own quota key), so behind a Cloudflare
Tunnel or an office NAT every launcher shares one bucket. The default
of 120/hour covers a small team; a larger one, or a CI fleet, needs
more.

Behind a shared address the mint quota is also a denial-of-service
lever: mint takes no credential, so anyone who can reach the port can
spend the shared bucket and refuse every launcher behind it for the
rest of the window. Raise the limit there, or set it to `0`. Refresh
is not exposed the same way, because it charges its main counter only
after the token verifies.

### Symptoms and what to change

| Symptom | Cause | Fix |
|---|---|---|
| Developers see "Launch + Telemetry" fall back to a plain launch, server logs a WARN with `reason=dev_session_quota_exceeded`, `scope=mint/ip` | Shared egress address | Raise `CIMMERIA_TELEMETRY_MINT_QUOTA_PER_IP`, or `0` to disable |
| Mint returns `400 Invalid branch: must not contain control characters` (or another metadata field) | The launcher sent a field with a control character or over 256 bytes; refused so it cannot forge a log line | Fix the launcher-side value; the server does not strip it |
| One machine repeatedly refused while others are fine | A launcher relaunching in a loop | Investigate that install before raising `..._PER_INSTALL` |
| Telemetry stops partway through a very long session | Session passed `CIMMERIA_TELEMETRY_MAX_SESSION_SECS` | Expected; the next launch mints a fresh session. Raise the cap only with a reason |

Session-lifetime cap: `iat` records the original mint and is not
reset by a refresh, so chained refreshes cannot extend one token
indefinitely. Near the cap the last refresh hands back a token
expiring exactly at the deadline rather than a full 8 hours past it.

## Where the data lives

Every uploaded event is replayed through the server's `tracing`
subscriber and lands in SigNoz / ClickHouse. What the client sent goes to
its own service; what the server says about the upload stays with the
server:

| `service.name` | Target (`scope_name`) | What it is |
|---|---|---|
| `cimmeria-client` | `client.native` | The injected DLL's events; the DLL's own event name is `client_target` and the log body |
| `cimmeria-client` | `launcher.client_log` | Game log lines the launcher tailed, and the per-line replay of end-of-session bundles |
| `cimmeria-client` | `launcher.debug_log` | `sgwdebuglog` lines |
| `cimmeria-client` | `launcher.session_meta` | Session boundaries and rotation events |
| `cimmeria-server` | `launcher.ingest` | Server-side accept counters per chunk, with the session totals; a `warn` with `reason = session_over_budget` when the runaway guard suppressed events |
| `cimmeria-server` | `launcher.bundle` | Bundle metadata and refusals (caps, bad entries) |
| none | `launcher.key_dump` | Encryption key material; `off` in every OTLP filter, never leaves the host |

Every `cimmeria-client` row carries `session_id` and `install_id` (the
token's claims), `cimmeria.session_kind` (`lab` or `player`) and `lab`
(`true`/`false`), plus `ts_ms` and `seq`. `client.native` rows also carry
`client_level`, the DLL's `fields` bag as JSON in `fields`, and, when the
event has them, `account_id`, `player_id`, `method_index`, `level_name`,
`dll_version`, `fingerprint_usable`, and on a governor rollup
`rollup_target` and `rollup_count`, as attributes of their own. The
resource has `cimmeria.source = client`, `cimmeria.deploy_env` and the
ingesting server as `cimmeria.ingest_host`. The full attribute table is in
[observability.md](../architecture/observability.md#log-indexes-and-parity-with-the-log-files).

Useful first queries:

- `service.name = 'cimmeria-client' AND cimmeria.session_kind = 'lab'`: a
  lab run, newest first.
- `service.name = 'cimmeria-client' AND client_target = 'client.dll.attached'`:
  one row per DLL start, with `dll_version`.
- `service.name = 'cimmeria-client' AND client_target = 'client.hooks.fingerprint'`:
  `fingerprint_usable = false` means the DLL met an SGW.exe build it does
  not know and installed no hooks.

Retention is whatever the ClickHouse TTL says (see
[signoz-deployment.md](signoz-deployment.md#retention)).

## Volume control and the runaway guard

The telemetry DLL summarizes high-rate streams before upload. The design is in
[client-telemetry.md](../architecture/client-telemetry.md#volume-control-the-governor).
In SigNoz this looks like:

- **Rollups.** `client_target = 'client.telemetry.rollup'` rows replace the
  hot streams (`client.engine.sequence_tick`, `client.lua.pcall`, ...) and
  the overflow of busy ones. Each row has `rollup_target` and `rollup_count`
  as attributes of their own. The exact per-target total over any range is
  `sum(rollup_count)` grouped by `rollup_target`, plus the plain rows of
  that target, plus their `repeat_count`.
- **Repeats.** A row with `repeat_count` in its `fields` stands for that
  many further identical events after the one before it.
- **Health.** `client_target = 'client.telemetry.health'` arrives once a
  minute per session. A `warn` health row means the DLL lost events: the
  upload ring was full (`ring_dropped_total`) or batches were discarded after
  failed POSTs (`upload_dropped_total`).

To upload full volume from a lab session on purpose, add `"capture":
{"raw": true}` to its `current-session.json`, or set
`CIMMERIA_CLIENT_CAPTURE=raw` for SGW.exe. `firehose` widens the budgets
but keeps summarizing the hot streams. The DLL's local log records which
mode ran.

On the server, `/api/telemetry/upload-chunk` counts events per session and
allows 30,000 replayed events per session per minute. That is about 300
times what a governed client sends, and enough for a `raw` lab session.
Over the budget the chunk is still accepted, so the uploader does not
retry it, but only warn/error rows, session metadata and the must-keep DLL
families (boot, hooks, entity lifecycle, Mercury anomalies, governor
reports) are replayed. Every chunk that suppressed something logs:

```text
service.name = 'cimmeria-server' AND scope_name = 'launcher.ingest'
  AND reason = 'session_over_budget'
```

with `suppressed`, `session_accepted_total`, `session_suppressed_total`,
`budget_per_window` and `window_secs`. The per-chunk `debug` line carries
the same session totals. The session table holds the 1,024 most recently
seen sessions.

## Player opt-in

Telemetry is **opt-in**. The launcher's `TelemetrySettings`
(`telemetry.opted_in` in `launcher-config.json`, default `false`) is
off until the player turns it on, from the one-time "Help us fix
bugs?" prompt or the **Send telemetry (opt-in)** checkbox beside the
launch buttons. Both save at once. Launchers before this change
wrote `"enabled": true` on the player's behalf; that key is ignored,
so every existing install starts opted out. While it is off:

- No `/api/auth/dev-session` POST fires.
- No log tailing.
- No bundle upload.
- The legacy "Launch Atera Debug" button still works (no
  instrumentation).

## Privacy

- `install_id` and `machine_id` are stable per-install identifiers.
  The mint endpoint logs them at **debug** level only (info gets an
  8-char correlator) to reduce leakage through any future log-upload
  pipeline. `install_id` is rejected at mint unless it is 1-128 bytes
  of ASCII alphanumerics, `-` or `_`, so a caller cannot forge log
  lines or unbounded SigNoz field values through it.
- `machine_id` is `sha256(HKLM\SOFTWARE\Microsoft\Cryptography\MachineGuid)`
  truncated to 16 hex chars — the raw GUID never leaves the dev's
  machine.
- KeyDump events carry encryption key material from the client. They
  are replayed at **debug** level only, so the default file sinks and
  admin WebSocket drop them, and `launcher.key_dump` is `off` in every
  OTLP filter, so SigNoz never receives them either.
- Per-event PII redaction is explicitly out of scope (dev-only data;
  the developer is the device owner).

## Endpoints

| Path | Method | Auth | Purpose |
|---|---|---|---|
| `/api/auth/dev-session` | POST | none (anyone can mint), quota-limited | Mint a token for a launcher session, or a lab session with `"session_kind": "lab"` (any other value is a 400). |
| `/api/auth/dev-session/refresh` | POST | bearer (own token), quota-limited | Extend an almost-expired token, up to the session cap. |
| `/api/telemetry/upload-chunk` | POST | bearer | Streaming events (gzip(NDJSON)). |
| `/api/telemetry/upload-bundle` | POST | bearer | End-of-session zip (multipart). |

A 503 with `Retry-After` on any of these means the kill switch is on.
A 429 with `Retry-After` means a quota was hit — see
[Mint and refresh quotas](#mint-and-refresh-quotas). A 401 on upload
endpoints means the token expired, was never valid, or does not carry
the `telemetry.write` scope; a 401 on refresh additionally means the
session passed its lifetime cap, and the launcher's answer to all of
them is to mint a fresh session.
