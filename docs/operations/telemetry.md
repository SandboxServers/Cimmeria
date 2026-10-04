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

The server serves the four telemetry routes (see [Endpoints](#endpoints))
on two listeners: the admin API port (`8443`, private, no
authentication) and the public SOAP login port (`LOGON_PORT`, `8081`),
which every player's game client already reaches. Nothing else from the
admin API is served on `8081`. Decision (@Cadacious, 2026-09-29): plain
HTTP on the login port is acceptable, because the game's own login
already sends the player's password over plain HTTP on that port.

The launcher sends telemetry to:

- any `https://` address;
- plain `http://` on the player's own machine (`localhost`, `127.x`,
  `::1`);
- plain `http://` to the exact host and port of one of the launcher's
  `http://` login servers (the list written into `LoginInternal.lua`; the
  default is `http://play.cimmeria.app:8081`).

That rule applies to `telemetry.auth_url` in `launcher-config.json`
(default `http://play.cimmeria.app:8081/api`) and to every
`upload_endpoint` the server hands back, including after a token
refresh. Anything else fails the session before a byte is sent, with
"telemetry needs an https:// server address" in the status log
([`telemetry/endpoint.rs`](../../crates/launcher/src/telemetry/endpoint.rs)).
Another port on a login server's host, such as `8443`, is still refused.

**Existing launcher configs.** Config schema 2 moved the default
`auth_url`. A schema-1 `launcher-config.json` whose `auth_url` is exactly
the old default, `http://localhost:8443/api`, is rewritten to the new
default once, on the first start of the new launcher. Any other value is
kept. A developer with a local server sets `http://localhost:8443/api`
(or `http://localhost:8081/api`) again afterwards, and it sticks.

The dev-session mint hands the launcher an `upload_endpoint` URL from
`CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT`. The default,
`http://localhost:8443/api/telemetry`, only works when the launcher and
the server share a host. A public server sets it to its own login port:

```bash
CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT="http://<public host>:8081/api/telemetry"
```

An HTTPS route (for example a Cloudflare Tunnel hostname, see step 2
below) also works and is optional.

**Sending your own telemetry to the colo over SSH** still works: forward
the colo's admin port with
`ssh -f -N -o ExitOnForwardFailure=yes -o ServerAliveInterval=30 -L 127.0.0.1:8443:127.0.0.1:8443 <colo host>`
and set `telemetry.auth_url` to `http://localhost:8443/api`. The colo's
`upload_endpoint` then points at the login port, which the default login
server list allows.

## Enable client telemetry on the colo

Until the secret below is set, the colo mints nothing: every launcher
and lab session gets a 500 from `/api/auth/dev-session` and
`cimmeria-client` stays empty. The server says so itself: at startup it
logs `dev-session telemetry: every mint and upload will be refused`
(WARN, `reason = dev_session_secret_unusable`), and each refused request
logs the same reason at ERROR. When it is working, the startup line is
`dev-session telemetry: mint and ingest enabled` with the
`upload_endpoint` it hands out.

1. **Generate the secret** on any machine: `openssl rand -hex 64`.
2. **Optional: an HTTPS route.** Not needed for players: launchers reach
   the telemetry routes on the public login port. If you also want an
   HTTPS address, add a Cloudflare Tunnel hostname (for example
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
   CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT=http://play.cimmeria.app:8081/api/telemetry
   ```

   With the step-2 route, the upload endpoint may be
   `https://telemetry.<your-domain>/api/telemetry` instead.
   `docker/compose.yml` passes both through as `${VAR:-}`. A blank upload
   endpoint falls back to `http://localhost:8443/api/telemetry`, which
   only a launcher on the colo itself can reach.
4. **Recreate the container** so it picks up the environment:
   `docker compose -f compose.yml up -d cimmeria`.
5. **Check it.** `docker logs cimmeria 2>&1 | grep "dev-session telemetry"`
   shows the enabled line. From any machine,
   `curl -s -o /dev/null -w '%{http_code}\n' -X POST -H 'Content-Type: application/json' -d '{}' http://play.cimmeria.app:8081/api/auth/dev-session`
   returns `422` (the body is incomplete, so the route is reachable and
   parsing), not `404` or `500`.
6. **Launchers need nothing.** A current launcher's default `auth_url`
   and login server already point at the colo; the player only ticks the
   telemetry opt-in (see [Player opt-in](#player-opt-in)).

The lab supervisor mints the same way, from the login server in the
client's `LoginInternal.lua` (`CIMMERIA_LAB_SERVER_URL` overrides it), and
refuses to launch without a token unless `CIMMERIA_LAB_TELEMETRY=optional`;
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

## Client engine-log capture switches

The injected client DLL ships the engine layer's logs (BigWorld messages, UE3
and log4cxx lines, debug strings, first-chance faults) as `client.bw.*`,
`client.ue3.*`, `client.log4cxx.*` and `client.os.*` events, rate-limited per
distinct message. Two switches change how much it captures. Both are off by
default and are read once at boot:

| Switch | Effect | When to use it |
|---|---|---|
| `unfilter` | lifts the client's own log thresholds (BigWorld filter, log4cxx `is*Enabled`, the UE3 suppress flag), so the client and the sinks see messages it normally drops. Also changes what the client writes to `SGWDebugLog.log` and `OutputDebugString`. | a debug or lab session chasing a silent client failure; not a player build |
| `firehose` | raises the per-message rate limit from burst 8 / 4 per second to burst 64 / 64 per second, and lowers the hitch threshold to 100 ms | a short repro where dropped repeats matter |

Turn them on with a top-level `capture` block in `current-session.json`
(`"capture": { "unfilter": true }`), or for a hand-run client with the
environment variable `CIMMERIA_CLIENT_CAPTURE=unfilter,firehose`. One event
per session that installed hooks, `client.hooks.capabilities`, lists which
hooks went in and which switches are on; the same line is in
`cimmeria-client-telemetry.log` next to `SGW.exe`. Event catalog:
[client-telemetry.md](../architecture/client-telemetry.md#engine-layer-log-sinks-and-subsystem-seams).

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
  one row per DLL start, with `dll_version` (and `dll_flavor`, `player`
  or `lab-bridge`, inside `fields`). A `player` session must never show
  `lab-bridge`.
- `service.name = 'cimmeria-client' AND client_target = 'client.telemetry_dll.launch'`:
  the launcher's record of whether an opted-in player's telemetry DLL
  went in (`outcome` = `injected`, `unavailable` or `inject_failed`, in
  `fields`). An `injected` row with no `client.dll.attached` row from the
  same `session_id` means the DLL loaded but never read its session.
- `service.name = 'cimmeria-client' AND client_target = 'client.hooks.fingerprint'`:
  `fingerprint_usable = false` means the DLL met an SGW.exe build it does
  not know and installed no hooks.
- `service.name = 'cimmeria-client' AND client_target = 'client.launcher.install_result'`:
  what the player's last Install / Update did to each patch
  (`patch.<id>` = `applied`, `already`, `failed` or
  `skipped_dependency`, with `patch.<id>.reason`, and for a failed hash
  check the file and both sha256s). Written at install time, uploaded
  with the next session.
- `service.name = 'cimmeria-client' AND client_target = 'client.patches.counts'`:
  the client-patches DLL's claimed / delivered / dropped counts and the
  last reason per counter (`dropped_no_overlay.last_reason` names a
  missing UI overlay). Counts are lower bounds; see
  [dev-session-telemetry.md](../architecture/dev-session-telemetry.md#client-patches-counts-event).

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
reports, and the ability rows `client.ability.*`, which the DLL already
throttles per name) are replayed. Every chunk that suppressed something logs:

```text
service.name = 'cimmeria-server' AND scope_name = 'launcher.ingest'
  AND reason = 'session_over_budget'
```

with `suppressed`, `session_accepted_total`, `session_suppressed_total`,
`budget_per_window` and `window_secs`. The per-chunk `debug` line carries
the same session totals. The session table holds 1,024 sessions. A new
session evicts the least recently seen one whose one-minute window has
ended; if every tracked session is inside its window, the newcomer is
counted in a shared `<overflow>` entry instead, so no live session ever
gets its budget back early. A chunk refused for a malformed line spends
no budget: the whole chunk is parsed before any event is counted.

## Player opt-in

Telemetry is **opt-in**. The launcher's `TelemetrySettings`
(`telemetry.opted_in` in `launcher-config.json`, default `false`) is
off until the player turns it on, from the one-time "Help us fix
bugs?" prompt or the **Send telemetry (opt-in)** checkbox beside the
launch buttons. Both save at once, and apply from the next launch.
Launchers before this change wrote `"enabled": true` on the player's
behalf; that key is ignored, so every existing install starts opted
out.

While it is on, **Launch SGW.exe** mints a player session before the
game starts and injects the telemetry DLL after the client patches (owner
decision 2026-09-29). Release launchers carry the DLL, the build without
the `lab-bridge` feature, so no player's machine gets the lab's command
port. If the DLL is missing, or is a `lab-bridge` build, the launcher
says so in its status log and starts the game without it; play is never
blocked. See [client-telemetry.md](../architecture/client-telemetry.md#who-gets-the-dll).

While it is off:

- No `/api/auth/dev-session` POST fires.
- No log tailing.
- No bundle upload.
- The telemetry DLL is not injected; the launch injects the client
  patches only.
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
- Per-event PII redaction is out of scope. The data is no longer
  dev-only: opted-in players' telemetry DLL events carry the session's
  install and machine ids, the interface (CEGUI) log text and the names
  of the game messages the client handles. The opt-in prompt says what is sent; whether that
  wording is enough for public players is an open owner question.

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
