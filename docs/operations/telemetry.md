# Dev-Session Telemetry — Operations

> **Type:** How-to · **Audience:** Telemetry operators · **Updated:** 2026-10-23
> **Companions:** [Architecture](../architecture/dev-session-telemetry.md), [native phase contract](../../crates/launcher/desktop/docs/launcher-telemetry.md)

Operator-facing runbook for the launcher and client telemetry pipeline:
provisioning the shared secret, rotating it, enabling it on the colo,
enabling/disabling ingest, and reading the data downstream.

Everything the players' machines upload (the injected
`cimmeria-client-telemetry` DLL's events and the game-log lines the
launcher tails) lands in SigNoz as its own service,
`service.name = cimmeria-client`. Sessions carry `cimmeria.session_kind`
with one of three values: `player`, `lab` and `launcher`. A launcher
session is minted by the desktop launcher for its own game-folder steps,
without a game, for opted-in players; the one exception is a press of Upload debug logs, which mints a short `purpose = logs` session whatever the opt-in says (see [Launcher log ingest](#launcher-log-ingest)). It is short, write-only and
has its own scope.

## Architecture at a glance

```text
launcher                            cimmeria-server
────────────────                    ────────────────
desktop launcher (Play)             /api/auth/dev-session
   ├─ game-telemetry.json identity  POST → HMAC token
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
loading the secret itself — see `verify_bearer` in
`crates/admin-api/src/routes/telemetry/upload_gate.rs`.

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
4. Existing in-flight tokens become invalid mid-flight. A launcher's own
   uploads get a 401, mint a fresh token through
   `auth::fetch_dev_session` (`crates/launcher-core/src/telemetry/auth.rs`)
   and carry on. A game already running keeps its DLL's old token, so the
   DLL's uploads stop until the next Play. Players see nothing.

**Cadence:** rotate every ~90 days or immediately on any suspicion of
exposure. The secret lives only in GitHub Actions secret store + the
running cimmeria-server process env.

## Pointing the launcher at a server

The server serves the four dev-session telemetry routes (mint, refresh
and the two uploads; see [Endpoints](#endpoints)) on two listeners: the
admin API port (`8443`, private, no authentication) and the public SOAP
login port (`LOGON_PORT`, `8081`), which every player's game client
already reaches. On the colo a Caddy reverse proxy also serves exactly
these routes, and the summary route, at `https://play.cimmeria.app`; see
[the colo route](telemetry-upload-endpoint.md#the-colo-route). A launcher
reaches the login port's routes only as `https://<name>` on 443. Plain http
goes only to this machine.

The code also merges a fifth route, `/api/telemetry/launcher-summary`, on
both listeners (see [Launcher summaries](#launcher-summaries)). Unlike the
other four it is anonymous: it takes no token. The owner approved serving
it on the public port on 2026-10-04. No launcher build has a summary
endpoint, so nothing calls it yet. Nothing else from the admin API is
served on `8081` or through the proxy.

**Upload endpoints are https.** The mint hands the launcher an
`upload_endpoint` URL. The server never falls back to localhost or to plain
http for a remote caller: it uses `CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT`, else
`https://<Host>/api/telemetry` when a proxy says the request was https, and
otherwise refuses the mint with 503 `no_upload_endpoint`. The full order,
the boot warnings and the Docker Desktop caveat are in
[telemetry-upload-endpoint.md](telemetry-upload-endpoint.md#where-a-mint-sends-the-upload).
LT-41 will make `https://play.cimmeria.app/api/telemetry` the built-in
default; until it lands there is none, so a server without the env value
and without an https proxy mints nothing.

**What the launcher accepts today.** The launcher sends telemetry to any
`https://` address, and to plain `http://` only on the player's own machine
(`localhost`, `127.x`, `::1`)
([`telemetry/endpoint.rs`](../../crates/launcher-core/src/telemetry/endpoint.rs)).
An `http://` login server on a dotted DNS name, including a LAN name such as
`nas.lan`, is reached as `https://<name>` on 443 (`https_upgrade_origin`,
LI-09). A failed connection simply sends nothing, so a self-hosted server needs
https on 443 for the five telemetry routes. An IP or single-word login server
gets no game telemetry. A launcher from before LI-09 also accepted plain
`http://` to the exact host and port of a login server. The desktop launcher
mints from the installation's first login server
(`https://<name>/api/auth/dev-session` for an `http://` login server on a
dotted name).
A refused address leaves the telemetry DLL out before a byte is sent, and
Settings shows the last Play's status as `endpoint_refused`. To point a
launcher at a local server, put it first in **Settings → Login servers**
(for example `Local = http://localhost:8081`).

**The retired egui launcher** kept a separate `telemetry.auth_url` in its
`launcher-config.json`. The desktop launcher's legacy import keeps that
value in its record but mints from the login server list.

**Sending your own telemetry to the colo over SSH** still works: forward
the colo's admin port with
`ssh -f -N -o ExitOnForwardFailure=yes -o ServerAliveInterval=30 -L 127.0.0.1:8443:127.0.0.1:8443 <colo host>`
and set `telemetry.auth_url` to `http://localhost:8443/api`. The colo's
`upload_endpoint` is its https address, which the launcher accepts.

## Enable client telemetry on the colo

Until the secret below is set, the colo mints nothing: every launcher
and lab session gets a 500 from `/api/auth/dev-session` and
`cimmeria-client` stays empty. The server says so itself: at startup it
logs `dev-session telemetry: every mint and upload will be refused`
(WARN, `reason = dev_session_secret_unusable`), and each refused request
logs the same reason at ERROR. When it is working, the startup line is
`dev-session telemetry: mint and ingest enabled`, followed by an
`upload_endpoint_source` line (an INFO, or a WARN with a `reason` when the
endpoint is missing or not https; see
[the boot rows](telemetry-upload-endpoint.md#log-rows-the-resolution-writes)).

1. **Generate the secret** on any machine: `openssl rand -hex 64`.
2. **Check the https route.** Launchers from LI-09 on need it: they mint,
   refresh and upload through `https://<name>` on 443 and never over plain
   http. It is also the address the server hands out. The colo's Caddy (`board-proxy`) forwards the five
   telemetry routes (and the sign-in routes, see `auth-https.md`) from
   `https://play.cimmeria.app` and answers 404 for everything else. No
   Cloudflare Tunnel is involved: one was planned and was never deployed.
   The Caddyfile is host config and is not yet mirrored into the
   agent-board repo (OPS-LT1), so a `board-proxy` redeploy can drop the
   route. Probe and rollback are in
   [telemetry-upload-endpoint.md](telemetry-upload-endpoint.md#the-colo-route).
3. **Set both variables in `/opt/cimmeria/.env`** (the file
   `docker compose` reads beside `compose.yml`; it is never committed):

   ```bash
   CIMMERIA_TELEMETRY_HMAC_SECRET=<the 128 hex chars from step 1>
   CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT=https://play.cimmeria.app/api/telemetry
   ```

   `docker/compose.yml` passes both through as `${VAR:-}`. A blank upload
   endpoint counts as unset: the server then derives an https URL from the
   proxied `Host`, and refuses the mint (503 `no_upload_endpoint`) if the
   request did not come through the proxy. There is no localhost fallback.
4. **Recreate the container** so it picks up the environment:
   `docker compose up -d` in `/opt/cimmeria`. Leave out `-f compose.yml`:
   it would override `COMPOSE_FILE` and recreate the container without
   its overlays (Discord file, lab endpoint).
5. **Check it.** `docker logs cimmeria 2>&1 | grep -E "dev-session telemetry|upload_endpoint"`
   shows the enabled line and an INFO `upload_endpoint_source` (not a WARN).
   From any machine:

   ```powershell
   curl.exe -s -o NUL -w "%{http_code}`n" -X POST -H "Content-Type: application/json" -d "{}" https://play.cimmeria.app/api/auth/dev-session
   ```

   returns `422` (the body is incomplete, so the route is reachable and
   parsing), not `404` or `500`. The colo-specific checks, including whether
   the server container sees the proxy as loopback (OPS-LT2), are under
   [Colo deploy checks](telemetry-upload-endpoint.md#colo-deploy-checks).
   None of this has run: the server that answers behind the route does not
   have the https-only changes deployed yet.
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
telemetry. Uploads stop too, including those from sessions that
started before the switch was thrown: a token already issued does not
get past it. Queued events are not lost while it is on. The launcher
keeps them in its on-disk queue: a current launcher sends no further
chunk until the `Retry-After` has passed, and launchers released before
the upload limits retry on every flush, about every 2 s, until the
switch is released. A session that ends while the switch is on loses
its end-of-session bundle, which is not retried. The DLL keeps its
batch, as for any failed upload.

`CIMMERIA_TELEMETRY_LAUNCHER_DISABLED=1` turns off the launcher kind alone:
a launcher mint and a launcher chunk answer 503, while player and lab
sessions carry on. The master switch above still stops everything.

Each refused dev-session request logs one row on `cimmeria-server`, so a
stream that stops has a line saying why: the switch is a WARN with
`reason = dev_session_kill_switch`. A refresh refused for an expired token,
for a session past its lifetime cap, or for a token that does not decode is
an INFO with `reason = dev_session_expired`, `dev_session_lifetime_exceeded`
or `dev_session_bad_token`.

Every telemetry route checks the switch:

| Route | Under the kill switch |
|---|---|
| `/api/auth/dev-session` (every session kind) | 503 + `Retry-After: 60` |
| `/api/auth/dev-session/refresh` | 503 + `Retry-After: 60` |
| `/api/telemetry/launcher-summary` | 503 + `Retry-After: 60`, before the quota is charged and before anything the caller sent, the body included, is read |
| `/api/telemetry/upload-chunk` | 503 + `Retry-After: 60`, before the token is checked or the body read |
| `/api/telemetry/upload-bundle` | 503 + `Retry-After: 60`, before the token is checked or the body read |

```bash
# Pause ingest without redeploy
ssh cimmeria-server "systemctl set-environment CIMMERIA_TELEMETRY_KILL_SWITCH=1 \
    && systemctl restart cimmeria-server"

# Resume
ssh cimmeria-server "systemctl unset-environment CIMMERIA_TELEMETRY_KILL_SWITCH \
    && systemctl restart cimmeria-server"
```

On a compose deployment (the colo) the switch is a line in the `.env`
beside `compose.yml` (`/opt/cimmeria/.env` on the colo), which
`docker/compose.yml` passes to the server as
`${CIMMERIA_TELEMETRY_KILL_SWITCH:-}`. Run these in that directory:

```bash
# Pause: add the line, then recreate the container so it gets the new environment.
# The leading newline keeps it off the end of a last line that has none.
printf '\nCIMMERIA_TELEMETRY_KILL_SWITCH=1\n' >> .env
docker compose -f compose.yml up -d cimmeria

# Resume: remove the line, then recreate again
sed -i '/^CIMMERIA_TELEMETRY_KILL_SWITCH=/d' .env
docker compose -f compose.yml up -d cimmeria
```

No line, or a line with nothing after the `=`, is the switch off: compose
then hands the server an empty value, and only `1` turns it on.

**Recreating the container is not free.** It restarts the game server, so
connected players are dropped. The container keeps accounts and characters
across the restart; the resources tables reseed only if the image changed
([container.md → Volume / persistence](container.md#volume--persistence)).
A running container's environment cannot be changed, so there is no
compose form that avoids it. This compose form is written from
`docker/compose.yml` and has not been run on the colo.

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
| `CIMMERIA_TELEMETRY_QUOTA_WINDOW_SECS` | `3600` | The window the mint and refresh quotas below are counted over. The summary quota does not use it. |
| `CIMMERIA_TELEMETRY_MINT_QUOTA_PER_IP` | `120` | Mints per peer address per window. |
| `CIMMERIA_TELEMETRY_MINT_QUOTA_PER_INSTALL` | `30` | Mints per `install_id` per window. |
| `CIMMERIA_TELEMETRY_LAUNCHER_MINT_QUOTA_PER_IP` | `30` | Launcher-kind mints per peer address per window, on top of the shared per-IP table. |
| `CIMMERIA_TELEMETRY_LAUNCHER_MINT_QUOTA_PER_INSTALL` | `12` | Launcher-kind mints per `install_id` per window, on their own table. |
| `CIMMERIA_TELEMETRY_REFRESH_QUOTA_PER_IP` | `480` | Refreshes with a valid token per peer address per window. Charged only after the token verifies. |
| `CIMMERIA_TELEMETRY_REFRESH_BAD_QUOTA_PER_IP` | `30` | Refresh calls whose token fails verification, per peer address per window. A separate counter, so junk tokens cannot spend the valid-token allowance. |
| `CIMMERIA_TELEMETRY_SUMMARY_QUOTA_PER_IP` | `12` | Requests to `/api/telemetry/launcher-summary` per peer address per minute (owner decision, 2026-10-04). The window is a fixed 60 s of the route's own. The route is anonymous, so this is its rate limit. Charged before the body is read or anything is parsed, so malformed, oversized and refused requests count too. Shared by everyone behind one address: see [Launcher summaries](#launcher-summaries). |
| `CIMMERIA_TELEMETRY_MAX_SESSION_SECS` | `86400` | How long one minted session may be extended by chained refreshes. Not a quota: `0` (or a negative value) does **not** disable the cap, it refuses every refresh. |
| `CIMMERIA_TELEMETRY_UPLOAD_QUOTA_PER_SESSION` | `120` | Requests to `/api/telemetry/upload-chunk` per session (the token's `session_id`) per minute. The launcher and its DLL share one token and each post about every 2 s. See [Upload size and rate limits](#upload-size-and-rate-limits). |
| `CIMMERIA_TELEMETRY_UPLOAD_QUOTA_PER_IP` | `600` | Requests to `/api/telemetry/upload-chunk` per peer address per minute, across every session behind it. |
| `CIMMERIA_TELEMETRY_BUNDLE_QUOTA_PER_SESSION` | `6` | Requests to `/api/telemetry/upload-bundle` per session per hour. |
| `CIMMERIA_TELEMETRY_BUNDLE_QUOTA_PER_IP` | `30` | Requests to `/api/telemetry/upload-bundle` per peer address per hour. |

"Peer address" in this table and the launcher table below means the quota
key address: the socket peer, or, behind a loopback proxy, the caller the
proxy's `X-Forwarded-For` names. The rule is in
[telemetry-upload-endpoint.md](telemetry-upload-endpoint.md#the-forwarded-for-quota-key).

A launcher token lives 15 minutes, carries no account or machine, is
never refreshed (a refresh answers 400), and its mints also count against
the shared per-IP quota.

Setting a quota to `0` disables that counter. A value that does not
parse falls back to the default rather than refusing to serve — an
operator typo must not take telemetry offline. A blank value does not
parse, so it is the default too.

### Changing a quota

The server reads these variables from its own environment, so a change
needs a restart. The example raises the launcher-summary limit to 60.

```bash
# systemd
ssh cimmeria-server "systemctl set-environment CIMMERIA_TELEMETRY_SUMMARY_QUOTA_PER_IP=60 \
    && systemctl restart cimmeria-server"

# Back to the default
ssh cimmeria-server "systemctl unset-environment CIMMERIA_TELEMETRY_SUMMARY_QUOTA_PER_IP \
    && systemctl restart cimmeria-server"
```

On a compose deployment (the colo), in the directory that holds
`compose.yml` and `.env`:

```bash
# Set it: one line in .env (edit the line if it is already there), then recreate
printf '\nCIMMERIA_TELEMETRY_SUMMARY_QUOTA_PER_IP=60\n' >> .env
docker compose -f compose.yml up -d cimmeria

# Back to the default: remove the line, then recreate again
sed -i '/^CIMMERIA_TELEMETRY_SUMMARY_QUOTA_PER_IP=/d' .env
docker compose -f compose.yml up -d cimmeria
```

Recreating the container restarts the game server and keeps accounts and
characters (resources reseed only on a new image), as under [Kill switch](#kill-switch), and this compose form has
not been run on the colo either.

`docker/compose.yml` passes six telemetry variables to the server:
`CIMMERIA_TELEMETRY_HMAC_SECRET`, `CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT`,
`CIMMERIA_TELEMETRY_KILL_SWITCH`,
`CIMMERIA_TELEMETRY_SUMMARY_QUOTA_PER_IP`,
`CIMMERIA_TELEMETRY_UPLOAD_QUOTA_PER_IP` and
`CIMMERIA_TELEMETRY_BUNDLE_QUOTA_PER_IP`. The other variables in the
table above are not passed through: a line for one of them in `.env`
reaches nothing until the variable is also added to the `cimmeria`
service's `environment:` block.

**Behind the colo's proxy the key is the caller.** The counters key on the
address the quota rule picks. When the socket peer is loopback, which is
what Caddy looks like to the server, the key is the rightmost entry of the
last `X-Forwarded-For` header; from any other peer the header is ignored,
so a caller who reaches the port directly cannot choose its own key. The
key is used only to count and is never logged. Whether the colo's container
sees Caddy as loopback has not been checked (OPS-LT2).

**Raise the per-IP quotas where the key is still shared:** a NAT or office
egress, any proxy that is not on loopback, and a container that sees the
proxy as its bridge gateway (in that case every https caller shares one
bucket until the check in OPS-LT2 or a trust decision, D-LT20, resolves it).
The default of 120 mints an hour covers a small team; a larger one, or a CI
fleet, needs more. 600 chunks a minute covers about ten sessions; raise
`CIMMERIA_TELEMETRY_UPLOAD_QUOTA_PER_IP` (and
`CIMMERIA_TELEMETRY_BUNDLE_QUOTA_PER_IP`) for more.

Behind a shared address the mint quota is also a denial-of-service
lever: mint takes no credential, so anyone who can reach the port can
spend the shared bucket and refuse every launcher behind it for the
rest of the window. Raise the limit there, or set it to `0`. Refresh
is not exposed the same way, because it charges its main counter only
after the token verifies.

### Symptoms and what to change

| Symptom | Cause | Fix |
|---|---|---|
| Players' Play sessions start without the telemetry DLL (Settings shows `session_unavailable`), server logs a WARN with `reason=dev_session_quota_exceeded`, `scope=mint/ip` | Shared egress address | Raise `CIMMERIA_TELEMETRY_MINT_QUOTA_PER_IP`, or `0` to disable |
| Mint returns `400 Invalid branch: must not contain control characters` (or another metadata field) | The launcher sent a field with a control character or over 256 bytes; refused so it cannot forge a log line | Fix the launcher-side value; the server does not strip it |
| One machine repeatedly refused while others are fine | A launcher relaunching in a loop | Investigate that install before raising `..._PER_INSTALL` |
| Mint answers `503 no_upload_endpoint`, server logs `reason=dev_session_no_upload_endpoint` | No env value, no https-proxied `Host`, no built-in default | Set `CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT` to an https URL; see [the checklist](telemetry-upload-endpoint.md#no-launcher-rows-in-signoz) |
| Telemetry stops partway through a very long session | Session passed `CIMMERIA_TELEMETRY_MAX_SESSION_SECS` | Expected; the next launch mints a fresh session. Raise the cap only with a reason |
| Uploads answer 429, server logs a WARN on `launcher.ingest` with `reason=rate_limited` | Many sessions behind one address, or one uploader posting too often | Raise `CIMMERIA_TELEMETRY_UPLOAD_QUOTA_PER_IP` for a shared address; investigate a single session first |
| Uploads answer 413, `reason=over_budget` with a `budget` | One upload passed a size budget | See [Upload size and rate limits](#upload-size-and-rate-limits); the budgets are fixed in code |

Session-lifetime cap: `iat` records the original mint and is not
reset by a refresh, so chained refreshes cannot extend one token
indefinitely. Near the cap the last refresh hands back a token
expiring exactly at the deadline rather than a full 8 hours past it.

## Upload size and rate limits

`/api/telemetry/upload-chunk` and `/api/telemetry/upload-bundle` check,
in this order and before any of the body is read: the kill switch (503),
the bearer token (401), the session's and then the address's rate quota
(429 + `Retry-After`, see the table above), and a free upload slot (503 +
`Retry-After: 5`): 4 chunks and 2 bundles are worked on at once
server-wide, and one address has at most 1 chunk and 1 bundle in flight.
Then the body is read, and it must arrive within 30 s (past that, 400
with `reason = body_read_failed`). Some budgets refuse the upload with
413; the expansion budgets truncate it instead: what fits is replayed
and the answer is a 200 with `"truncated": true` in its body.

| Route | Budget | Limit | Past it | Why this value |
|---|---|---|---|---|
| chunk | compressed body | 16 MiB | 413 | The cap launchers have always had: older launchers post their whole backlog as one chunk and retry a refused one forever. A current launcher sends 1 MiB of NDJSON per chunk (`chunk_max_bytes`), which gzips far smaller |
| chunk | decompressed NDJSON | 8 MiB | cut at the last whole row | 8× `chunk_max_bytes`; 4× the DLL's largest retained batch |
| chunk | rows | 10,000 | the first 10,000 replayed | 5× the DLL's largest retained batch (`max_batch` 1,000, about 2,000 after a failed POST) |
| bundle | zip part | 32 MiB | 413 | A session's logs are about 50 MiB before zipping |
| bundle | `metadata` part | 16 KiB | 413 | A dozen JSON fields |
| bundle | multipart parts, zip parts | 8, 1 | 413 | The launcher sends one of each |
| bundle | zip entries, hard cap | 16,384 | 413, from the zip's end record before it is opened | Opening a zip reads every entry's header |
| bundle | files replayed | 1,024 | the newest replayed | Session logs rotate every minute; a long session leaves a few hundred |
| bundle | expanded bytes, all files | 64 MiB | a file whose declared size would pass it is skipped and smaller files still replay; a file that expands past what it declared stops the replay | Checked on each file's declared size, then on the bytes actually read |
| bundle | replayed lines | 250,000 | replay stops | |

Only the client's debug log (`SGWDebugLog*`) and `*.log` entries are replayed.
Any other entry, such as a session's key dump (`*-keys.txt`) or
`current-session.json`, is never read: it is counted in the bundle answer's
`skipped_not_log` field and in no row. Launchers that still zip those files
are covered by this, so their secrets never reach SigNoz.

Bundle files are replayed in order of the zip entry's modification time,
newest first, and among equal times in reverse archive order. A current
launcher records each file's own modification time, so the session that
just ended is what survives a budget. Launchers released before the
upload limits give every entry the same time, so their files are taken
in reverse archive order, which is reverse path order. A chunk refused
with 413 replays nothing. A chunk row that is not UTF-8, not an event
the server knows, or longer than 64 KiB (not parsed at all) is skipped
and counted in the response's `bad_rows`;
only a body that is not gzip is refused (400). Uploaded strings are cut before they
reach a log row: log messages to 4 KiB, file names, levels, categories and
event names to 256 bytes, each value in a DLL `fields` bag to 2 KiB, and
the bag to 64 keys. A value in the bag that is itself an array or an
object is replaced by its JSON text, so in the replayed `fields` attribute
it appears as a quoted string (`"ids":"[1,2,3]"`), not as nested JSON. A
cut value ends in `...[truncated, N bytes]`, with the original length,
and a cut bag gains `_truncated_keys`.

Every refusal and every truncation writes a row on `launcher.ingest`, at `warn`
except a `busy` refusal, which is `info`: it is answered 503 with `Retry-After`
and the uploader retries on its next flush, so it is back-pressure, not a fault,
and it stays out of the Discord errors channel.

```text
service.name = 'cimmeria-server' AND scope_name = 'launcher.ingest'
  AND route IN ('upload-chunk', 'upload-bundle')
```

with `reason` (`kill_switch`, `missing_token`, `bad_token`,
`token_expired`, `rate_limited`, `busy`, `body_too_large`, `over_budget`,
`bad_gzip`, `bad_zip`, `bad_multipart`, `body_read_failed`,
`secret_unusable`, and `chunk_truncated` / `bundle_truncated` /
`bad_rows` for an upload that was accepted in part, the last with a
`bad_rows` count), `budget` and `limit` for a size
refusal or truncation, `kept` (rows or lines replayed) and
`dropped_estimate` for a truncation, `pool` (`peer_share` or `route`) and `limit` for a `busy` refusal, `cimmeria.session_kind`, and `session_id` /
`install_id` once the token verified. The row never carries the payload
or the caller's address.
`dropped_estimate` is in the unit of the budget that was hit: rows for a
chunk (past the expansion cap, estimated from the compression ratio),
files for `budget = zip entries`, **bytes** for `budget = expanded
bytes` (the sizes of the bundle files not replayed), and lines for
`budget = lines`.
A refusal row also carries `event`: `upload_auth_refused` when the
uploader's credential was refused (`missing_token`, `bad_token`,
`token_expired`), which stays in SigNoz and never posts to Discord, and
`upload_refused` for every other refusal.
Repeats are throttled per uploader and reason: the first refusal writes a
row at once and the repeats inside the pair's window are held back, the
next row's `suppressed` counting them. The window starts at 10 s and
doubles with every row the pair writes, up to 15 minutes, so a client
retrying every 12 s writes about ten rows in its first hour and four an
hour after that. It drops back to 10 s once the pair has had no refusal
for 15 minutes. At most 50 rows are written per 10 s across all
uploaders, with a separate 50 for `upload_auth_refused` rows, so a crowd
of stale tokens cannot hold back the first rows of refusals that still
post. A row a ceiling held back is written on the pair's first refusal
after the ceiling reopens, with the count in `suppressed`. Two uploaders
whose pairs share a throttle slot do not reset each other's window; if
both of a newcomer's slots are taken by active pairs, its rows are
written untracked, under the ceiling.

Any backlog in the launcher's on-disk queue (a server outage of a few
minutes, a kill switch, or a launcher killed mid-session) is posted when
the server answers again. A current launcher splits it into chunks of
`chunk_max_bytes` and 1,000 rows, and drops a chunk the server answers
with 413 or any other 4xx but 401, 408 and 429 instead of retrying it
(the drop is counted in the bundle
metadata's `dropped_lines`). Launchers released before the upload limits
post the whole queue as one chunk and re-queue it on any error; the
server truncates such a chunk rather than refusing it, so the backlog is
consumed, minus what was cut, and the `chunk_truncated` rows show how much
for that `session_id`. The same launchers bundle every past session's
logs at exit; the server keeps the newest files within the bundle
budgets. A current launcher bundles only the files written to during the
session.

### Launcher sessions

A `launcher` session has its own limits, all refused before the body is
read where the route allows it. Its chunks take 2 reserved upload slots
(so launcher chunks never use up the route's slots the game DLL needs; the per-address limit of one chunk in flight is shared, so a game chunk from the same address waits for a launcher chunk in flight), and a launcher
token is refused on the bundle route (403, `reason = forbidden`); bundles
are not part of the launcher kind.

| Limit | Value | Env name (default) |
|---|---|---|
| chunks per session per minute | 10 | `CIMMERIA_TELEMETRY_LAUNCHER_CHUNK_QUOTA_PER_SESSION` |
| chunks per install per minute | 20 | `CIMMERIA_TELEMETRY_LAUNCHER_CHUNK_QUOTA_PER_INSTALL` |
| chunks per quota-key address per minute | 60 | `CIMMERIA_TELEMETRY_LAUNCHER_CHUNK_QUOTA_PER_IP` |
| rows per session, lifetime | 300 | `CIMMERIA_TELEMETRY_LAUNCHER_ROWS_PER_SESSION` |
| rows per minute, all launcher callers | 3,000 | `CIMMERIA_TELEMETRY_LAUNCHER_ROWS_PER_MINUTE` |
| compressed chunk body | 64 KiB | the chunk rules above (413) |
| decompressed chunk | 256 KiB | the chunk rules above (truncated) |
| one chunk row | 4 KiB | the chunk rules above (`bad_rows`) |
| rows per chunk | 200 | the chunk rules above (truncated) |

The launcher paces itself under the 10 chunks a minute session quota (D-LTL1): it sends at most one chunk about every 7 s, and it flushes when a launch finishes and on exit (at most 3 s). A normal run stays inside the quota, and a retry has room.

A row the launcher kind may keep has one of twelve event names and a
`ClientNative` shape: `client.launcher.` followed by `adoption`, `startup`,
`crash`, `update`, `game_update`, `install`, `repair`, `launch`, `settings`,
`telemetry`, `ui` or `login`
([the list](telemetry-upload-endpoint.md#accepted-launcher-event-targets)).
Any other event is dropped. The row's fields are
scalar only: at most 16 keys, each key at most 32 bytes, each string value at
most 256 bytes. Keys that could name a host, an account, a machine or a
session (`ip`, `host`, `account`, `user`, `email`, `machine_id`,
`install_id`, `session_id` and so on) are dropped, and values that look like
an IP, a drive path or a rooted path are scrubbed to `<removed>`. Launcher
rows are never named or lifted into entity attributes, so they cannot
stand in for a game entity. The rules are in `launcher_rows.rs`.

### Launcher log ingest

The desktop launcher's **Upload debug logs** button posts the launcher's and the game's recent log lines to the same `/telemetry/upload` endpoint as everything else, through a `logs`-purpose launcher session. The rows land in SigNoz. This is reference for the operator: what a log session may send, what the server refuses and which knob moves each limit. The player-side behaviour is in [launcher-guide.md](../client/launcher-guide.md); the wire rules for the endpoint are in [telemetry-upload-endpoint.md](telemetry-upload-endpoint.md).

Status: the server and engine code and their unit tests are merged (launcher-uat-fixes, waves 3 and 4). No live upload through the colo has run yet; see [What is tested](#what-is-tested-and-what-is-not).

**The session.** The launcher mints a `kind = "launcher"` session with the optional `purpose: "logs"` claim. The mint takes the same quota tables as any launcher mint. A `logs` claim on a non-launcher kind is `400 purpose_kind`; an unknown purpose is `400 purpose_unknown`. The token lives 900 s and a launcher token is never refreshed, so a long upload re-mints. A log session posts through the same `/telemetry/upload` path with its own limits: 1,000 rows, 512 KiB decompressed and 64 KiB compressed per chunk, and 20 chunks per session per minute (`rate_limited`, 429). The install window (20 chunks a minute) and the address window (60 chunks a minute) are shared with event sessions.

**The rows.** A log session may post two targets, `client.launcher.log` and `client.launcher.game_log` (`LAUNCHER_LOG_TARGETS`). The server forces `level` to `info`; the line's own level travels in `log_level`. The field set is closed, and a row with an unknown key or a bad value is dropped and counted, never repaired:

| Field | Rule |
|---|---|
| `message` | required; scrubbed, then cut (below) |
| `upload_id` | required; 16 lowercase hex digits, one per upload press |
| `log_level` | `TRACE`, `DEBUG`, `INFO`, `WARN` or `ERROR` |
| `log_target` | at most 64 bytes of `[A-Za-z0-9_:.-]` |
| `file` | at most 64 bytes of `[A-Za-z0-9_.-]` |
| `line_no` | integer, 1 or more |
| `truncated` | bool |
| `launcher_version` | at most 32 bytes of `[A-Za-z0-9_.+-]` |
| `os` | at most 16 bytes of `[a-z0-9_]` |

A row over 4,096 bytes is dropped server-side. The server scrubs `message` word by word: a word that the launcher-row scrub flags (a path, an address, a key-like value) becomes `<removed>`, a flagged path also removes the path-tail words after it, and control and invisible format characters become spaces first. There is no content filter (D-LU3): chat lines and character names in game logs go up, and only the scrubs apply. The message is then cut to 2,048 bytes on a character boundary and `...[truncated]` is appended.

A session of one purpose may not post the other's targets. An event session that posts a log target, or a log session that posts any `LAUNCHER_TARGETS` row, gets `403 purpose_target` for the whole chunk, checked after decode and before any budget is charged. Other unknown targets in a log chunk are dropped and counted.

**Row budgets.** A log chunk is admitted whole: the server checks all four ceilings, charges all four, or refuses the chunk. Each ceiling is an env var read at startup (the fallback is the default if the value is unset or unparsable).

| Reason | Default | Env var | Status and `Retry-After` |
|---|---|---|---|
| `log_session_budget` | 40,000 rows per session | `CIMMERIA_TELEMETRY_LOG_ROWS_PER_SESSION` | 429, no `Retry-After` (60 s when the 4,096-entry session table is full, which fails closed) |
| `log_install_budget` | 80,000 rows per install per UTC day | `CIMMERIA_TELEMETRY_LOG_ROWS_PER_INSTALL_DAY` | 429, to UTC midnight |
| `log_ip_budget` | 400,000 rows per address per UTC day | `CIMMERIA_TELEMETRY_LOG_ROWS_PER_IP_DAY` | 429, to UTC midnight |
| `log_global_budget` | 40,000 rows per minute, all installs | `CIMMERIA_TELEMETRY_LOG_ROWS_PER_MINUTE` | 503, seconds left in the minute plus 1 |

The session and install ceilings count rows kept; the address and minute ceilings count rows received. Each table holds 4,096 entries; a full day table drops past-day entries first, then the entry with the fewest rows today. The address is the quota-key address (the forwarded address when the proxy is trusted), so players behind one NAT share `log_ip_budget`. All four are speed bumps, not defences: they bound a flood, they do not authenticate a row. Anyone who can mint a log session can post forged log rows within the budgets.

**Refusals in the logs.** `purpose_target` is a credential-class refusal and logs as `upload_auth_refused` (SigNoz only). The four `log_*_budget` reasons are not credential refusals and log as `upload_refused`, which posts to Discord at WARN under the existing throttle (10 s doubling to 15 min). A busy day can therefore post a budget warning; that is the throttle working, not a fault. To change a ceiling see [Changing a quota](#changing-a-quota).

**One upload, in numbers.** One press sends at most 20,000 launcher lines and 20,000 game lines, so at most 40,000 rows: exactly the session ceiling. The install ceiling of 80,000 allows about two full uploads a UTC day; the third gets `log_install_budget`, and the launcher shows the wait. That holds for an install with a stored identity (one that has opted in once). An install that never opted in sends under a fresh install id on every press, so only the address ceiling (log_ip_budget) bounds it. The launcher posts launcher lines first, then game lines, in one session with one `upload_id`. Chunks hold at most 384 KiB raw and 64 KiB gzipped, and are paced 3.5 s apart (at most 18 a minute, under the 20-a-minute session window). On a 429 or 503 the upload ends as partial with a wait of the server's `Retry-After`, never less than 60 s; there is no retry loop. The launcher moves its read offsets forward only after each accepted chunk, so the next press resumes where the server stopped. A partial on launcher rows sends no game rows.

**Game logs.** The launcher reads `sgwdebuglog*` from the game's `Binaries` folder and `sessions/**/*.log` (key dumps and `current-session.json` are skipped). It detects UTF-16LE by byte-order mark or NUL pattern, else reads UTF-8 lossily. A raw line is cut at 32 KiB before the local scrub (word scrub and address removal), then cut to 2,048 bytes with `truncated` set. Offsets are keyed by relative path with a head hash, so a rewritten file restarts at 0.

**Finding an upload.** The player sees "Sent N log lines. Reference `<upload_id>`." In SigNoz the line text is the log body; the other fields sit in the JSON-text `fields` attribute, so filter with `fields CONTAINS '<upload_id>'`. Whether the id also surfaces as a flat `upload_id` attribute has not been checked against a real row. Launcher rows carry target `client.launcher.log`, game rows `client.launcher.game_log`.

#### What is tested, and what is not

Tested (unit tests, not run for this docs change): the server log-chunk limits, the four budgets and `purpose_target` (`upload_limits_tests/log_chunk.rs`, `launcher_log_budget_tests.rs`), the refusal event choice (`refusal_log.rs`), the env override for `CIMMERIA_TELEMETRY_LOG_ROWS_PER_SESSION`, and on the engine side the chunking, pacing, offsets, line reading and game-line reading (`ingest_tests`, `ingest_flow_tests`, `lines_tests`, `game_lines_tests`) plus the frontend states (`log-upload-uat.mjs`).

Not tested: any live upload through the colo, the SigNoz filter above, a real `sgwdebuglog` through the server scrub, whether the other three row-budget env vars take effect as documented (only the session override has a test), and the Discord throttle on budget refusals. Treat this section as the code's intent until the live UAT runs.

## Where the data lives

Every uploaded event is replayed through the server's `tracing`
subscriber and lands in SigNoz / ClickHouse. What the client sent goes to
its own service; what the server says about the upload stays with the
server:

| `service.name` | Target (`scope_name`) | What it is |
|---|---|---|
| `cimmeria-client` | `client.native` | The injected DLL's events; the DLL's own event name is `client_target` and the log body |
| `cimmeria-client` | `launcher.client_log` | Game log lines the launcher tailed, and the per-line replay of end-of-session bundles. A bundle line carries `level` (the log4cxx level it names: `DEBUG`, `INFO`, `WARN`, `ERROR` or `FATAL`), and no `level` when it names none; the row's severity is unchanged |
| `cimmeria-client` | `launcher.debug_log` | `sgwdebuglog` lines |
| `cimmeria-client` | `launcher.session_meta` | Session boundaries and rotation events. `kind=game_exited` carries the game's `pid` and `exit_code` (the exit code is absent when the OS reports none); the desktop launcher also sends `host_pid` and `early` |
| `cimmeria-client` | `launcher.summary` | The desktop launcher's attempt summaries: `event = launcher_summary` per accepted summary and `event = launcher_phase` per timed phase. See [Launcher summaries](#launcher-summaries) |
| `cimmeria-server` | `launcher.ingest` | Server-side accept counters per chunk, with the session totals; a `warn` with `reason = session_over_budget` when the runaway guard suppressed events; a throttled row per refused upload (`warn`, except `busy`, which is `info`), with its `route`, `reason` and `event` (`upload_auth_refused` for a missing, bad or expired token, SigNoz-only; `upload_refused` for every other refusal) ([Upload size and rate limits](#upload-size-and-rate-limits)). Also `event = launcher_summary_batch`, one row per launcher-summary request that reached validation, and `event = launcher_summary_refused` (WARN, SigNoz-only) for a summary request refused as a whole ([reasons](telemetry-upload-endpoint.md#launcher-summaries)). None of these rows carries a `peer` |
| `cimmeria-server` | `launcher.bundle` | Bundle metadata and refusals (caps, bad entries) |
| none | `launcher.key_dump` | Encryption key material; `off` in every OTLP filter, never leaves the host |

Every uploaded `cimmeria-client` row carries `session_id` and `install_id`
(the token's claims), `cimmeria.session_kind` (`lab`, `player` or `launcher`) and `lab`
(`true`/`false`), plus `ts_ms` and `seq`. `launcher.summary` rows are the
exception: they carry none of those identifiers, and their
`cimmeria.session_kind` is `launcher_summary` (a row label only; no such
session can be minted, and it is not the `launcher` session kind). `client.native` rows also carry
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
  `lab-bridge`. The row's `pid` field (in `fields`) is the process that
  wrote its chunks: the `X-Cimmeria-Client-Pid` header, from TG-PIPE-02.
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
- `service.name = 'cimmeria-client' AND client_target = 'client.launcher.adoption' AND cimmeria.session_kind = 'launcher'`:
  the desktop launcher's adoption steps. Its `fields` hold `step`,
  `decision_outcome`, `mode`, `error`, `unsafe_total` and `unsafe_entries`.
  Opted-in players only, sent as they happen.
  A query meaning "a player's game session" should filter
  `cimmeria.session_kind = 'player'`, not `!= 'lab'`: launcher rows can be
  forged by anyone who mints a session, so funnel counts built from them
  measure what clients claim.
- `service.name = 'cimmeria-client' AND client_target = 'client.patches.counts'`:
  the client-patches DLL's claimed / delivered / dropped counts and the
  last reason per counter (`dropped_no_overlay.last_reason` names a
  missing UI overlay). Counts are lower bounds; see
  [dev-session-telemetry.md](../architecture/dev-session-telemetry.md#client-patches-counts-event).

Retention is whatever the ClickHouse TTL says (see
[signoz-deployment.md](signoz-deployment.md#retention)).

## Launcher light-adoption and update events

These events come from the desktop launcher's light adoption, its seedless Update and
its launch tags. They are `tracing` events with a closed `event` name; the upload
pipeline gives them the `client.launcher.<subsystem>` target of
[the table above](#where-the-data-lives) (for example `client.launcher.adoption`). None
carries a path, URL or host: the only free text is a sample of at most 10 index paths
(see below). The fields of the existing rows are not repeated here.

| Event | Target (subsystem) | Level | Fields |
| --- | --- | --- | --- |
| `adoption_light_planned` | `adoption` | info | `patches_total`, `patches_current`, `patches_pending`, `download_bytes` |
| `adoption_light_replayed` | `adoption` | info | `patches_replayed`, `replacements`, `ops_skipped`, `patches_incomplete` |
| `adoption_light_replay_failed` | `adoption` | warn | `code` (a closed rejection code) |
| `adoption_index_compared` | `adoption` | info | `state`, `stock`, `patched`, `prepared`, `changed`, `missing`, `extra`, `changed_sample`, `missing_sample` |
| `patch_incomplete` | `launcher.install` | info | `patch` (`patch:<id>`), `ops_skipped` |
| `patch_retry_failed` | `launcher.install` | warn | `patch` (`patch:<id>`), `reason` (a closed install error code) |
| `install_stage_failed` | `launcher.install` | warn | `reason` (a closed install error code), `http_status` (when the server replied), `unpack_kind` (a closed unpack code, only for `reason = unpack`) |
| `repair_stage_failed` | `launcher.repair` | warn | `reason` (a closed install error code), `http_status` (when the server replied), `unpack_kind` (a closed unpack code, only for `reason = unpack`) |
| `update_staged` (gained fields) | `launcher.game_update` | info | `mode` (`incremental` or `full`), `ops_skipped`, `stage_linked`, `stage_copied`; the full path always sends `full`, 0, 0, 0 |
| `update_seedless_declined` | `launcher.game_update` | info | `reason`: `previous_release_unverified` or `ledger_mismatch` |
| `update_incremental_failed` | `launcher.game_update` | warn | `reason`: `fetch_failed`, `hash_mismatch`, `write_set_unreadable`, `seed_stage_failed` or `rename_failed` |
| `launch_started` (gained fields) | `launcher.launch` | info | `adoption_check` (`light`, `full` or `none`), `patches_incomplete` |
| `launch_tags_unavailable` | `launcher.launch` | warn | none beyond `event`; the launch goes on tagged `none`, 0 |
| `install_phase_finished` | `launcher.install` | info | `operation`, `phase` (`download`, `copy`, `check`, `extraction` or `expansion`), `bytes`, `files`, `elapsed_ms` |
| `seed_unpack_started` | `launcher.install` | info | `check` (`sha256` or `hashed_while_copied`), `bytes` |
| `seed_archive_picked` | `launcher.install` | info | `bytes` (the player's archive size) |
| `log_upload_post` | `launcher.telemetry` | info | `scheme`, `host` (`-` when unknown), `status`; never the URL |
| `proton_physx` | `launcher.proton` | info or warn | `result` (closed), `reason` (closed); never a path. **Local log only**: `launcher.proton` is not in `launcher_targets.txt`, so SigNoz never receives it |
| `proton_directx` | `launcher.proton` | info | `dll` (`d3dx9_40.dll` or `xinput1_3.dll`), `state` (`present` or `installed`). Local log only |
| `proton_directx_failed` | `launcher.proton` | warn | no DLL name yet (FU-17). Local log only |
| `proton_refused` | `launcher.proton` | warn | `reason` (`unknown_proton` or a save failure), when the Settings picker refuses a choice. Local log only |
| `display_settings` (Linux) | `launcher.launch` | info or warn | `saved_options="seeded"` on a first Play, with a benign `display_settings_not_applied` WARN until the game writes `SGWEngine.ini` (F-U5) |

- `changed_sample` and `missing_sample` hold at most 10 paths, comma-joined, at most
  600 characters, taken only from index entries the release names (`changed`,
  `missing`), never from `extra` files (D-LA17).
- The `launcher.install.detail` and `launcher.repair.detail` targets (events
  `install_stage_detail` and `repair_stage_detail`) stay in the local log and are
  never forwarded.
- The shell's `mode_chosen` and `folder_chosen` steps on `client.launcher.adoption`
  count a Light adoption as `in_place` (`decision_outcome = in_place` and
  `mode = in_place`). A funnel that splits `in_place` from `copy` therefore cannot
  tell Light from the hidden verified in-place mode; split on `adoption_light_planned`
  instead.
- The `completed.light` flag and `LaunchStatus.patches_incomplete` / `ops_skipped` are
  IPC status fields, not telemetry; see the
  [migration contract](../../crates/launcher/desktop/docs/migration.md#adopting-an-existing-folder).
- Nothing in this table has been seen in SigNoz from a real launcher run. The rows
  are asserted by captured-event unit tests only.

## Launcher adoption and Play phase timing

Use this when a player reports slow adoption or a long wait after Play. These
are consented launcher events under `client.native`, not anonymous
`launcher.summary` rows. Keep identifying query values and raw rows out of
public reports.

1. Bound the query to the reported time window and launcher run. Filter service
   `cimmeria-client`, `scope_name = client.native`, and `client_target` equal to
   `client.launcher.adoption` or `client.launcher.launch`.
2. Read the JSON text in `attributes_string['fields']`. The phase lifecycle,
   correlation values, duration and scan counts are nested there, not guaranteed
   to be top-level SigNoz attributes. Confirm field names against the
   [desktop contract](../../crates/launcher/desktop/docs/launcher-telemetry.md).
3. Pair starts and finishes within the same process/run and operation. Sort by
   timestamp, then compare completed durations and file/byte workloads. A slow
   scan suggests a different next investigation from a slow admission or helper
   handshake. These measurements do not by themselves identify a disk, antivirus,
   network or server cause.
   Repeated phase names can be separate legitimate passes, including idempotent
   adoption application; use `phase_sequence` to keep their pairs separate.
4. Compare `game_started` startup duration with later DLL initialization, login,
   synchronization and world-entry rows separately. Do not add overlapping
   nested phase durations or count background resynchronization as launcher
   startup time.
   Packaged `startup_ms` includes native blocking-worker queue wait measured by
   `play_dispatch_queue`, but excludes browser/UI time before IPC receipt.
   Its endpoint is the helper's `ProcessStarted` reply, not the first rendered
   frame. Inspect requests do not produce a queue timer.

For ClickHouse SQL queries against the logs table, this projection makes the
nested data inspectable without assuming automatic field promotion. Add the
appropriate bounded timestamp predicate in the query tool before executing it:

```sql
SELECT timestamp,
       attributes_string['client_target'] AS client_target,
       JSONExtractString(attributes_string['fields'], 'event') AS event,
       JSONExtractString(attributes_string['fields'], 'phase') AS phase,
       JSONExtractUInt(attributes_string['fields'], 'operation') AS operation,
       JSONExtractUInt(attributes_string['fields'], 'phase_sequence') AS phase_sequence,
       attributes_string['fields'] AS phase_fields
FROM signoz_logs.distributed_logs_v2
WHERE resources_string['service.name'] = 'cimmeria-client'
  AND scope_name = 'client.native'
  AND timestamp > toUnixTimestamp(now() - INTERVAL 1 DAY) * 1000000000
  AND attributes_string['client_target'] IN
      ('client.launcher.adoption', 'client.launcher.launch')
ORDER BY timestamp
LIMIT 200
```

This is a query recipe, not a committed live-tested dashboard. The existing
[launcher-session limits](#launcher-sessions) still apply: 300 rows per session,
200 per chunk and at most 16 scalar keys per row, plus the existing rate limits.
Forwarding is opt-in and not durably queued. Opt-out, process death, quota
exhaustion and failed uploads can each remove a finish. Report incomplete pairs
as missing evidence, never as zero duration or a confirmed hang. An old launcher
cannot retrospectively emit the new timings.

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
throttles per name) are replayed, up to 3,000 more per session per minute.
Past that allowance they are suppressed too, so a client that marks every
row as a warning is held like any other. Every chunk that suppressed
something logs:

```text
service.name = 'cimmeria-server' AND scope_name = 'launcher.ingest'
  AND reason = 'session_over_budget'
```

with `suppressed`, `session_accepted_total`, `session_suppressed_total`,
`budget_per_window`, `priority_allowance_per_window` and `window_secs`. The per-chunk `debug` line carries
the same session totals. The session table holds 1,024 sessions. A new
session evicts the least recently seen one whose one-minute window has
ended; if every tracked session is inside its window, the newcomer is
counted in a shared `<overflow>` entry instead, so no live session ever
gets its budget back early. A chunk refused for a malformed line spends
no budget: the whole chunk is parsed before any event is counted.

## Launcher summaries

The desktop launcher (`crates/launcher/desktop/`) can report how each
install, runtime-setup, repair, uninstall or launch attempt ended: one row
of closed values per attempt, with no session, install or machine
identifier. The design and the wire contract are in
[launcher-summary-telemetry.md](../architecture/launcher-summary-telemetry.md).

**No launcher sends one today.** A launcher takes its endpoint from its first login server when that is https, else from that server's https upgrade when it is an http DNS name (the same name on 443, `login_server_https_upgrade`, LI-06), else from the built-in default, which is not set yet. An IP, localhost or single-label http login server sends nothing. Nothing here has run against a deployed server or a real SigNoz. What follows is what the server code does when a summary arrives.

- **Https only, and not sent yet.** Summaries and the default upload URL are https only; a plain `http://` address is never used for a summary, except loopback for local fixtures; an http login server on a DNS name is upgraded to https. With no https endpoint, Share setup diagnostics is saved and the box says "saved but not sent yet". That state ends when the built-in https default is set (D-LT17).
- The `summary_endpoint_resolved` line on `launcher.pipeline` carries `outcome` = `login_server`, `login_server_https_upgrade`, `default` or `no_https_endpoint`.

- **Consent.** The desktop launcher's own `launcher_summary_consent`
  preference, off by default, labelled Share setup diagnostics. It is separate from the telemetry opt-in under
  [Player opt-in](#player-opt-in) and never turns on game or DLL telemetry. Nothing from the launcher's own events leaves without Send game diagnostics; this box gates only the anonymous summaries. (Upload debug logs sends log lines on its own press.)
- **No token.** The route is anonymous. The launcher sends one `POST` with
  no `Authorization` header, and there is no mint step. The server never
  reads an `Authorization` header on this route, so a dev-session token is
  worth nothing here, and the route works with no
  `CIMMERIA_TELEMETRY_HMAC_SECRET` set. The dev-session mint refuses
  `"session_kind": "launcher_summary"` like any other unknown kind.
- **Route.** `POST /api/telemetry/launcher-summary`, with
  `Content-Type: application/json` (a `charset` parameter is allowed), at
  most 64 KiB and 32 summaries per request.
- **Only the payload gets in.** The server accepts the exact schema-1 JSON
  body and refuses everything else as a whole: 415 for another content
  type, 400 for a URI with a query string, 413 over 64 KiB, 400 for a body
  that is not the envelope (unparseable JSON, a gzip or NDJSON body, a
  game-telemetry event, a wrong `schema_version`, a key written twice in
  the envelope, 0 or more than 32 summaries). One invalid summary inside a
  valid body is answered `rejected` in its position, and the valid ones
  beside it are still accepted; a summary that writes a key twice is
  invalid.
- **Rate limit.** `CIMMERIA_TELEMETRY_SUMMARY_QUOTA_PER_IP` limits requests
  per quota-key address per minute (owner decision, 2026-10-04). The default is
  `12`, and `0` disables it. The window is a fixed 60 seconds of this
  route's own, and `CIMMERIA_TELEMETRY_QUOTA_WINDOW_SECS` does not change
  it. Over the limit the route answers 429 with a `Retry-After` of 61
  seconds at most, and the launcher stops that export cycle
  without retrying and keeps its rows for the next one. Every request is
  counted, the malformed and the oversized (413) ones too; only a 503 under
  the kill switch is not. An IPv4 peer seen as `::ffff:a.b.c.d` on a
  dual-stack listener is counted as `a.b.c.d`. The summary route's 503 is
  the kill switch only; it has no mint, so it never answers
  `no_upload_endpoint`.
- **Kill switch.** The route answers 503 under
  [the kill switch](#kill-switch), before the quota is charged.
- **Where the rows land.** `launcher.summary` rows go to
  `service.name = cimmeria-client`; the per-request `launcher_summary_batch`
  row goes to `cimmeria-server` under `launcher.ingest`. A request refused
  as a whole (kill switch, rate limit, content type, query string,
  oversized or malformed body) writes neither of those, but it does write one
  `launcher_summary_refused` row on `launcher.ingest` (WARN, SigNoz-only,
  one per reason per minute, with `reason`, `status` and `suppressed`; no
  address). The reasons and a filter for each are in
  [telemetry-upload-endpoint.md](telemetry-upload-endpoint.md#launcher-summaries).
- **Duplicates.** The server remembers the last 16,384 accepted ids in
  memory. A restart forgets them, so a summary resent after one is accepted
  again.
- **Reading the rows.** The dashboard and saved-view fixtures, and the notes
  on what each row means, are in
  [signoz/launcher-summary-views.md](signoz/launcher-summary-views.md). The
  fixtures have not been imported into any SigNoz.

**A shared address shares the limit.** The allowance belongs to the quota
key address. Behind the colo's Caddy that is the caller named by
`X-Forwarded-For`, provided the server sees the proxy as a loopback peer
(OPS-LT2, unchecked); behind a NAT, an office egress or any non-loopback
proxy every launcher arrives from one address, so the default allows 12
summary requests per minute for all of them together. A launcher sends a
few requests an hour, so that is room for many of them, but anyone behind
that address can also spend the allowance with 12 requests of any content
and turn the others' posts into 429s until the minute ends, and do it again
the next minute. Check the limit against the number of launchers behind the
address before pointing any launcher at the route, and raise it if they
would need more. The commands, for systemd and for compose, are under
[Changing a quota](#changing-a-quota).

**Do not act on these rows.** The route is anonymous, so anyone can post
correctly shaped rows within the rate limit. Rows are self-reported; they
are useful for spotting failure patterns and must never drive server state,
alerts, success-rate claims or SLOs. The strict schema means nothing but
closed enum values, bounded integers, UUIDs and a version triple can ever
be stored.

## Player opt-in

Telemetry is **opt-in**. The desktop launcher (`crates/launcher/desktop/`,
Windows and macOS) asks with the **Send game diagnostics while playing**
checkbox in Settings, off by default and separate from its
setup-diagnostics checkbox. The choice is saved in `game-telemetry.json` in
the launcher's state directory, with the install identity minted at the
first opt-in, and applies from the next Play. Upload debug logs is not covered by this choice: pressing it is consent for that one upload (see [Launcher log ingest](#launcher-log-ingest)).

While it is on, Play mints a player session on the installation's first
login server before the game starts, writes `current-session.json` beside
the game and injects the telemetry DLL after the client patches (owner
decision 2026-09-29; the same opt-in covers the camera read at an interact or item use, owner 2026-10-18, [details](../architecture/client-camera-telemetry.md)). Its sessions carry the tags `desktop-launcher`, the
host OS and `wine`, `proton` or `native`. The launcher also tails the game's logs,
uploads them in chunks, refreshes its own token, and at game exit sends a
`game_exited` event and the end-of-session bundle. Release launchers carry
the player build of the DLL, never `lab-bridge`, so no player's machine
gets the lab's command port. With no session, a refused address or an
unwritable session file, the DLL is left out and the game starts without
it; play is never blocked, and Settings shows why. An adopted install
plays without the DLL for now. The capture switches stay off unless a
developer sets `CIMMERIA_CLIENT_CAPTURE` in the launcher's own environment.
Contract: [launch-telemetry.md § Opt-in game telemetry](../../crates/launcher/desktop/docs/launch-telemetry.md#opt-in-game-telemetry);
background: [client-telemetry.md](../architecture/client-telemetry.md#who-gets-the-dll).

The retired egui launcher kept the choice as `telemetry.opted_in` in its
`launcher-config.json`. The desktop launcher's legacy import keeps that
answer in its record; it does not turn game diagnostics on.

While it is off:

- No `/api/auth/dev-session` POST fires.
- No log tailing.
- No bundle upload.
- The telemetry DLL is not injected; the launch injects the client
  patches only.
- The legacy "Launch Atera Debug" button still works (no
  instrumentation).

## Privacy

### No address in any log

Two owner directives (D-LT18, D-LT19) govern what the server writes about a
caller's network address on the launcher and telemetry routes:

- **No IP in any row, and no hash of one.** No log row, span or field on
  `/api/telemetry*` or `/api/auth/dev-session*` carries a caller's address.
  The request span drops `peer` on those paths. The rule does not reach the
  admin-router `admin.request` `unmatched` rows, which keep `peer`, or the
  game-login rows in `crates/auth`.
- **The forwarded address is a quota key and the in-memory stream address, nothing else.** From a loopback
  peer the server counts quotas against the rightmost `X-Forwarded-For`
  entry; it never logs, persists, returns or hashes it. The in-memory
  client-telemetry registry also keys its stream on that caller (D-LTL4); it
  holds the address in process memory only and never logs or writes it. See
  [telemetry-upload-endpoint.md](telemetry-upload-endpoint.md#the-forwarded-for-quota-key).
- **The launcher scrubs its own rows.** Every string of a launcher row, and every line of a launcher log in a debug-log upload, has IPv4 and IPv6 literals replaced with `[ip]` before it is queued or sent (`ip_scrub.rs`); nothing is hashed. Absolute paths in a launcher row's strings become `<path removed>` and the home folder becomes `~` (`launcher_event.rs`).
- **Endpoints are https.** The server hands out only an https upload URL
  (or an explicit env override with a WARN), and the public routes are
  reached over `https://play.cimmeria.app`.

The world-entry correlation matches a telemetry stream to a game session by the
quota-key address (`StreamRecord.peer_ip`, TG-NET-18, D-LTL4). Behind Caddy that
is the forwarded caller, so it finds the stream only when the launcher's mint
and the game login reach the server from the same public address. A launcher
and a game client on different addresses (a dual-stack split, a VPN on one of
them) give `absent`; two machines behind one NAT give `ambiguous`.

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
| `/api/auth/dev-session` | POST | none (anyone can mint), quota-limited | Mint a token for a launcher session (`"session_kind": "launcher"`), or a lab session with `"session_kind": "lab"` (any other value is a 400). |
| `/api/auth/dev-session/refresh` | POST | bearer (own token), quota-limited | Extend an almost-expired token, up to the session cap. A launcher token is refused (400). |
| `/api/telemetry/upload-chunk` | POST | bearer, rate- and size-limited | Streaming events (gzip(NDJSON)). |
| `/api/telemetry/upload-bundle` | POST | bearer, rate- and size-limited | End-of-session zip (multipart). A launcher token is refused (403). |
| `/api/telemetry/launcher-summary` | POST | none (anonymous), strict payload, rate-limited per address | Desktop-launcher attempt summaries (JSON). Takes no token. See [Launcher summaries](#launcher-summaries). |

A 503 with `Retry-After: 60` on any of these routes means the kill switch
is on; a 503 `no_upload_endpoint` on the mint or refresh means the server
has no https upload URL to hand out (see
[telemetry-upload-endpoint.md](telemetry-upload-endpoint.md#where-a-mint-sends-the-upload));
a 503 with `Retry-After: 5` on an upload route means every upload
slot was busy. A 429 with `Retry-After` means a quota was hit — see
[Mint and refresh quotas](#mint-and-refresh-quotas). A 413 on an upload
route means it passed a size budget — see
[Upload size and rate limits](#upload-size-and-rate-limits). A 401 on upload
endpoints means the token expired, was never valid, or does not carry
the `telemetry.write` scope; a 401 on refresh additionally means the
session passed its lifetime cap, and the launcher's answer to all of
them is to mint a fresh session. The summary route never answers 401: it
reads no token.

### Which content a launcher applied

Filter launcher native rows to `client.launcher.install` and
`event=applied_content_identity`. `manifest_sha256` names the retained signed
installed manifest; `overlay_ids` lists fully applied overlays and
`incomplete_overlay_ids` lists partial overlays. `phase=applied` follows
successful publication; `phase=play_admitted` follows Play admission. Neither
is a claim about code actually loaded by a game process. These observations
use the existing consented launcher pipeline and its quotas; absence of a row
is not proof no content was applied. An offered update is not installed
content. See the [desktop launch contract](../../crates/launcher/desktop/docs/launch.md#applied-content-identity).
