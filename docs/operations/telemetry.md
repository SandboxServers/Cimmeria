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
  - `launcher.bundle.crash_dump` — one row per minidump found in a
    bundle (see [Crashes and exits](#crashes-and-exits))
  - `client.native` — events from the telemetry DLL inside `SGW.exe`;
    the DLL's own event name is the `client_target` field
  - `launcher.ingest` — server-side accept/reject counters
- `session_id` — the launcher session UUID minted by dev-session
- `install_id` — stable per-install identifier

Retention is whatever the ClickHouse TTL says (see
[signoz-deployment.md](signoz-deployment.md#retention)).

### Crashes and exits

The telemetry DLL reports how each session ended. With the server's
OTLP export on, query `service.name = cimmeria-client` and filter on
`client_target` (every field below is its own attribute); the same
events are also replayed under `target = client.native`, with the
fields as one JSON string:

| `client_target` | Level | Meaning |
|---|---|---|
| `client.crash` | ERROR | The client crashed. `fault` is `module+offset` (for example `SGW.exe+0x00016ec5`), plus `exception_code`, `exception_name`, `access` / `access_address` for access violations, `thread_id`, `source` (`game_minidump`: the game's own crash handler ran; `unhandled_filter`: nothing handled the fault), and `dump_file`. `last_seq` is the producer's sequence number at the crash: the session's events below it are what led up to it. |
| `client.crash.dump` | INFO / WARN | Whether `dump_file` was written, its size (`dump_bytes`), or why not (`reason`, `os_error`). |
| `client.exit` | INFO, WARN after a crash | The process is leaving: `path` is `crt_exit` (a normal quit) or `exit_process` (UE3's forced exit), with `exit_code` and `after_crash`. |
| `client.crash.installed` | INFO | Crash capture is live (`iat_hooks` of 4 swapped, and the filter it chains to). |

A session with `client.dll.attached` but neither `client.crash` nor
`client.exit` vanished: the process was killed, or it died in a way
no filter saw (a stack overflow can be one).

The minidump itself stays on the player's machine at
`Binaries/sessions/crash-<session_id>-<ts_ms>.dmp`, with a
`.jsonl` sidecar holding the same fields as `client.crash` (not
`.json`, which the launcher never uploads). The
launcher's end-of-session bundle ships the whole `sessions/` folder,
so both reach the server when the player quits through the launcher.
The server does not store the dump: it logs one
`launcher.bundle.crash_dump` row (WARN) with `path`, `dump_bytes`,
`minidump_valid`, `exception_code`, `exception_address` and
`thread_id` read from the dump's exception stream, and replays the
sidecar as a `launcher.client_log` line. To analyse the dump, ask the
player for the file, then read it as
[docs/client/crash-dumps.md](../client/crash-dumps.md) describes.
The game's own, possibly full-memory, dump under
`Binaries/CrashDumps/` is not shipped.

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
  ship at **debug** level only — the default file sinks and admin
  WebSocket drop them; only SigNoz (when configured for debug ingest)
  retains them. Use this lever to keep raw keys off long-lived
  on-disk storage.
- Per-event PII redaction is explicitly out of scope (dev-only data;
  the developer is the device owner).

## Endpoints

| Path | Method | Auth | Purpose |
|---|---|---|---|
| `/api/auth/dev-session` | POST | none (anyone can mint), quota-limited | Mint a token for a launcher session. |
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
