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
4. Existing in-flight tokens become invalid mid-flight; affected
   launchers retry through `auth::fetch_dev_session` and get a fresh
   token within seconds. No user-visible disruption beyond a single
   `Telemetry session error` log line.

**Cadence:** rotate every ~90 days or immediately on any suspicion of
exposure. The secret lives only in GitHub Actions secret store + the
running cimmeria-server process env.

## Pointing the launcher at a non-localhost server

The server serves the four dev-session telemetry routes (mint, refresh
and the two uploads; see [Endpoints](#endpoints)) on two listeners: the
admin API port (`8443`, private, no authentication) and the public SOAP
login port (`LOGON_PORT`, `8081`), which every player's game client
already reaches. Decision (@Cadacious, 2026-09-29): plain HTTP on the
login port is acceptable, because the game's own login already sends the
player's password over plain HTTP on that port.

The code also merges a fifth route, `/api/telemetry/launcher-summary`, on
both listeners (see [Launcher summaries](#launcher-summaries)). Unlike the
other four it is anonymous: it takes no token. The owner approved serving
it on the public port on 2026-10-04, which extends that decision to it as a
fifth route. No launcher build has a summary endpoint, so nothing calls it
yet. Nothing else from the admin API is served on `8081`.

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
   telemetry.<your-domain>`, and start the `cloudflared` container on
   `signoz-net` as [signoz-remote-access.md](signoz-remote-access.md)
   shows (the colo runs no tunnel as of 2026-10-04). The `path` rule
   keeps the rest of the admin API, which has no authentication (#439),
   off the public hostname.
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
   `docker compose up -d` in `/opt/cimmeria`. Leave out `-f compose.yml`:
   it would override `COMPOSE_FILE` and recreate the container without
   its overlays (Discord file, lab endpoint).
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
telemetry. Uploads stop too, including those from sessions that
started before the switch was thrown: a token already issued does not
get past it. Queued events are not lost while it is on. The launcher
keeps them in its on-disk queue: a current launcher sends no further
chunk until the `Retry-After` has passed, and launchers released before
the upload limits retry on every flush, about every 2 s, until the
switch is released. A session that ends while the switch is on loses
its end-of-session bundle, which is not retried. The DLL keeps its
batch, as for any failed upload.

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
connected players are dropped, and this container reseeds its database on
every start ([container.md → Volume / persistence](container.md#volume--persistence)).
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
| `CIMMERIA_TELEMETRY_REFRESH_QUOTA_PER_IP` | `480` | Refreshes with a valid token per peer address per window. Charged only after the token verifies. |
| `CIMMERIA_TELEMETRY_REFRESH_BAD_QUOTA_PER_IP` | `30` | Refresh calls whose token fails verification, per peer address per window. A separate counter, so junk tokens cannot spend the valid-token allowance. |
| `CIMMERIA_TELEMETRY_SUMMARY_QUOTA_PER_IP` | `12` | Requests to `/api/telemetry/launcher-summary` per peer address per minute (owner decision, 2026-10-04). The window is a fixed 60 s of the route's own. The route is anonymous, so this is its rate limit. Charged before the body is read or anything is parsed, so malformed, oversized and refused requests count too. Shared by everyone behind one address: see [Launcher summaries](#launcher-summaries). |
| `CIMMERIA_TELEMETRY_MAX_SESSION_SECS` | `86400` | How long one minted session may be extended by chained refreshes. Not a quota: `0` (or a negative value) does **not** disable the cap, it refuses every refresh. |
| `CIMMERIA_TELEMETRY_UPLOAD_QUOTA_PER_SESSION` | `120` | Requests to `/api/telemetry/upload-chunk` per session (the token's `session_id`) per minute. The launcher and its DLL share one token and each post about every 2 s. See [Upload size and rate limits](#upload-size-and-rate-limits). |
| `CIMMERIA_TELEMETRY_UPLOAD_QUOTA_PER_IP` | `600` | Requests to `/api/telemetry/upload-chunk` per peer address per minute, across every session behind it. |
| `CIMMERIA_TELEMETRY_BUNDLE_QUOTA_PER_SESSION` | `6` | Requests to `/api/telemetry/upload-bundle` per session per hour. |
| `CIMMERIA_TELEMETRY_BUNDLE_QUOTA_PER_IP` | `30` | Requests to `/api/telemetry/upload-bundle` per peer address per hour. |

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

Recreating the container restarts the game server and reseeds its
database, as under [Kill switch](#kill-switch), and this compose form has
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

The per-address upload quotas share the same caveat: behind one proxy
or NAT, every session counts against one address. 600 chunks a minute
covers about ten sessions; raise `CIMMERIA_TELEMETRY_UPLOAD_QUOTA_PER_IP`
(and `CIMMERIA_TELEMETRY_BUNDLE_QUOTA_PER_IP`) for more.

### Symptoms and what to change

| Symptom | Cause | Fix |
|---|---|---|
| Developers see "Launch + Telemetry" fall back to a plain launch, server logs a WARN with `reason=dev_session_quota_exceeded`, `scope=mint/ip` | Shared egress address | Raise `CIMMERIA_TELEMETRY_MINT_QUOTA_PER_IP`, or `0` to disable |
| Mint returns `400 Invalid branch: must not contain control characters` (or another metadata field) | The launcher sent a field with a control character or over 256 bytes; refused so it cannot forge a log line | Fix the launcher-side value; the server does not strip it |
| One machine repeatedly refused while others are fine | A launcher relaunching in a loop | Investigate that install before raising `..._PER_INSTALL` |
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
the bag to 64 keys. A cut value ends in `...[truncated, N bytes]`, with
the original length, and a cut bag gains `_truncated_keys`.

Every refusal and every truncation writes a `warn` on `launcher.ingest`:

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
`dropped_estimate` for a truncation, `peer`, and `session_id` /
`install_id` once the token verified. The row never carries the payload.
`dropped_estimate` is in the unit of the budget that was hit: rows for a
chunk (past the expansion cap, estimated from the compression ratio),
files for `budget = zip entries`, **bytes** for `budget = expanded
bytes` (the sizes of the bundle files not replayed), and lines for
`budget = lines`.
Repeats are throttled: one row per uploader and reason per 10 s, the next
row's `suppressed` counting the ones held back, and at most 50 rows per
10 s in all.

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
| `cimmeria-client` | `launcher.summary` | The desktop launcher's attempt summaries: `event = launcher_summary` per accepted summary and `event = launcher_phase` per timed phase. See [Launcher summaries](#launcher-summaries) |
| `cimmeria-server` | `launcher.ingest` | Server-side accept counters per chunk, with the session totals; a `warn` with `reason = session_over_budget` when the runaway guard suppressed events; a throttled `warn` per refused upload, with its `route` and `reason` ([Upload size and rate limits](#upload-size-and-rate-limits)). Also `event = launcher_summary_batch`, one row per launcher-summary request that reached validation |
| `cimmeria-server` | `launcher.bundle` | Bundle metadata and refusals (caps, bad entries) |
| none | `launcher.key_dump` | Encryption key material; `off` in every OTLP filter, never leaves the host |

Every uploaded `cimmeria-client` row carries `session_id` and `install_id`
(the token's claims), `cimmeria.session_kind` (`lab` or `player`) and `lab`
(`true`/`false`), plus `ts_ms` and `seq`. `launcher.summary` rows are the
exception: they carry none of those identifiers, and their
`cimmeria.session_kind` is `launcher_summary` (a row label only; no such
session can be minted). `client.native` rows also carry
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

**No launcher sends one today.** Every distributed build has no summary
endpoint, so there is nothing to operate yet, and nothing here has run
against a deployed server or a real SigNoz. What follows is what the server
code does when a summary arrives.

- **Consent.** The desktop launcher's own `launcher_summary_consent`
  preference, off by default. It is separate from the telemetry opt-in under
  [Player opt-in](#player-opt-in) and never turns on game or DLL telemetry.
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
  per peer address per minute (owner decision, 2026-10-04). The default is
  `12`, and `0` disables it. The window is a fixed 60 seconds of this
  route's own, and `CIMMERIA_TELEMETRY_QUOTA_WINDOW_SECS` does not change
  it. Over the limit the route answers 429 with a `Retry-After` of 61
  seconds at most, and the launcher stops that export cycle
  without retrying and keeps its rows for the next one. Every request is
  counted, the malformed and the oversized (413) ones too; only a 503 under
  the kill switch is not. An IPv4 peer seen as `::ffff:a.b.c.d` on a
  dual-stack listener is counted as `a.b.c.d`.
- **Kill switch.** The route answers 503 under
  [the kill switch](#kill-switch), before the quota is charged.
- **Where the rows land.** `launcher.summary` rows go to
  `service.name = cimmeria-client`; the per-request `launcher_summary_batch`
  row goes to `cimmeria-server` under `launcher.ingest`. A request refused
  as a whole (kill switch, rate limit, content type, query string,
  oversized or malformed body) writes neither, and the handler logs nothing
  about the refusal.
- **Duplicates.** The server remembers the last 16,384 accepted ids in
  memory. A restart forgets them, so a summary resent after one is accepted
  again.
- **Reading the rows.** The dashboard and saved-view fixtures, and the notes
  on what each row means, are in
  [signoz/launcher-summary-views.md](signoz/launcher-summary-views.md). The
  fixtures have not been imported into any SigNoz.

**A shared address shares the limit.** The allowance belongs to the
peer address, and no `X-Forwarded-For` header is read. Behind a NAT, a
reverse proxy or a tunnel every launcher arrives from one address, so the
default allows 12 summary requests per minute for all of them together.
A launcher sends a few requests an hour, so that is room for many of them,
but anyone behind that address can also spend the allowance with 12
requests of any content and turn the others' posts into 429s until the
minute ends, and do it again the next minute. Check the limit against the
number of launchers behind the address before pointing any launcher at the
route, and raise it if they would need more. The commands, for systemd and
for compose, are under [Changing a quota](#changing-a-quota).

**Do not act on these rows.** The route is anonymous, so anyone can post
correctly shaped rows within the rate limit. Rows are self-reported; they
are useful for spotting failure patterns and must never drive server state,
alerts, success-rate claims or SLOs. The strict schema means nothing but
closed enum values, bounded integers, UUIDs and a version triple can ever
be stored.

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

The desktop launcher (`crates/launcher/desktop/`, macOS and Windows) has the
same opt-in as its own choice: the **Send game diagnostics while playing**
checkbox in Settings, off by default and separate from its setup-diagnostics
checkbox. While it is on, Play mints a session on the login server, writes
`current-session.json` beside the game and injects the DLL after the client
patches. Its sessions carry the tags `desktop-launcher`, the host OS and `wine`
or `native`. It does not tail logs or upload bundles, and it mints one token
per Play without refreshing it. The capture switches stay off unless a
developer sets `CIMMERIA_CLIENT_CAPTURE` in the launcher's own environment.
Contract: [launch.md § Opt-in game telemetry](../../crates/launcher/desktop/docs/launch.md#opt-in-game-telemetry).

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
| `/api/telemetry/upload-chunk` | POST | bearer, rate- and size-limited | Streaming events (gzip(NDJSON)). |
| `/api/telemetry/upload-bundle` | POST | bearer, rate- and size-limited | End-of-session zip (multipart). |
| `/api/telemetry/launcher-summary` | POST | none (anonymous), strict payload, rate-limited per address | Desktop-launcher attempt summaries (JSON). Takes no token. See [Launcher summaries](#launcher-summaries). |

A 503 with `Retry-After: 60` on any of these routes means the kill switch
is on; a 503 with `Retry-After: 5` on an upload route means every upload
slot was busy. A 429 with `Retry-After` means a quota was hit — see
[Mint and refresh quotas](#mint-and-refresh-quotas). A 413 on an upload
route means it passed a size budget — see
[Upload size and rate limits](#upload-size-and-rate-limits). A 401 on upload
endpoints means the token expired, was never valid, or does not carry
the `telemetry.write` scope; a 401 on refresh additionally means the
session passed its lifetime cap, and the launcher's answer to all of
them is to mint a fresh session. The summary route never answers 401: it
reads no token.
