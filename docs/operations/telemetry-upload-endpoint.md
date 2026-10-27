# Launcher Telemetry: Upload Endpoint, Refusals and the Colo Route

| | |
|---|---|
| **Type** | Reference, with a diagnostic checklist at the end |
| **Audience** | Operators and engineers running or debugging a cimmeria-server that launchers mint from |
| **Last updated** | 2026-10-21 (LT-04, launcher-telemetry wave 1) |
| **Companions** | [telemetry.md](telemetry.md) (the runbook: secret, kill switch, quotas), [dev-session-telemetry.md](../architecture/dev-session-telemetry.md) (design), [launcher-summary-telemetry.md](../architecture/launcher-summary-telemetry.md) (the anonymous summary route), [observability-target-catalog.md](../architecture/observability-target-catalog.md) (row fields), [discord-notifications.md](../architecture/discord-notifications.md) (which rows post) |

This page is where the server's answers to three questions live: where does
a mint tell a launcher to upload, which requests does the server refuse and
how does it say so, and how does the colo's https address reach the server.
The runbook for the secret, the kill switch and the quota variables stays in
[telemetry.md](telemetry.md).

## What has run and what has not

| Claim | State |
|---|---|
| The resolve order, the boot and mint log rows, the 503 `no_upload_endpoint`, the refusal rows, the forwarded-for rule and the no-address rule | Pinned by unit and handler tests in `crates/admin-api` and `crates/discord` (the tests are named in the code beside each rule). Documented here from the code at the wave-1 integration tip |
| A deployed server answering any of this | **Never run.** The server changes (PR S) have not been deployed. Nothing on this page has been observed in a real SigNoz |
| The five routes through `https://play.cimmeria.app` | The Caddy route was put in place by hand on 2026-10-21 and probed for status codes by the owner's hands-on session. A launcher has never posted through it, and the server that answers behind it does not have these changes yet |
| The server seeing the proxy as a loopback peer (OPS-LT2) | **Not checked.** The code assumes it; [the check](#colo-deploy-checks) is written below and has not been run |
| The Caddyfile surviving a `board-proxy` redeploy (OPS-LT1) | **Open.** The file is host config and is not yet mirrored into the private agent-board repo |

## The privacy rule

No launcher or telemetry log row carries an IP address, and nothing is
hashed. The statement of the rule, once, is
[telemetry.md § No address in any log](telemetry.md#no-address-in-any-log).
Everything below assumes it: a refusal row names the reason and a count,
never the caller.

## Where a mint sends the upload

The mint (`POST /api/auth/dev-session`) puts an upload base URL in its
response, and the refresh repeats it. The launcher appends `/upload-chunk`
and `/upload-bundle`. The server resolves the URL in this order and stops at
the first that applies
(`crates/admin-api/src/routes/dev_session/upload_endpoint.rs`, `resolve`):

| # | Source | When it applies | URL | `upload_endpoint_source` |
|---|---|---|---|---|
| 1 | The env override | `CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT` is set and not blank. Blank counts as unset: `docker/compose.yml` passes `${VAR:-}`, which sets `""` | The value, trimmed, used as given | `env` |
| 2 | An https-proxied `Host` | The request has a valid `Host` and `X-Forwarded-Proto` is exactly `https` | `https://<Host>/api/telemetry` | `request_host` |
| 3 | A one-host dev box | The `Host` is a loopback name or address and the peer is loopback | `http://<Host>/api/telemetry` | `loopback_dev` |
| 4 | The built-in https default | A constant in the code. It is `None` until LT-41 sets it to `https://play.cimmeria.app/api/telemetry` | The constant | `default` |
| 5 | Nothing | None of the above | The mint answers **503** `no_upload_endpoint` and signs nothing | |

A `Host` is valid when it is a DNS name or IPv4 address of letters, digits,
`.` and `-` (1 to 253 characters) or a bracketed IPv6 address, with an
optional port of one to five digits. Anything else is skipped, so a hostile
`Host` cannot put a path or credentials into the URL the server hands back.

There is no localhost or plain-http fallback. Row 3 is the one place a
plain-http URL is derived, and only for a loopback `Host` from a loopback
peer. A server with no env value, no https-proxied `Host` and no default
refuses the mint rather than send a remote launcher to its own machine.
Trusting `X-Forwarded-Proto` in row 2 is by design: a remote client can only
make the server hand back an https URL to the host it already named.

The proxy in front of the server must therefore pass `Host`,
`X-Forwarded-Proto: https` and `X-Forwarded-For` (see [the quota
key](#the-forwarded-for-quota-key)). The colo's Caddy does.

**Self-hosters on Docker Desktop.** Docker Desktop publishes ports through a
bridge, so the server sees a non-loopback peer even for a launcher on the
same machine, and row 3 does not apply. With no env value and no https proxy
the mint answers 503 until you set `CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT` to an
https URL you serve. A plain-http value still works as an explicit override,
with a WARN (below); launchers from LI-09 on refuse it (`game_session_mint_failed`, `reason = endpoint_refused`), so only older launchers upload to it.

### Log rows the resolution writes

| When | Level | Fields | Meaning |
|---|---|---|---|
| Server start | WARN | `reason = upload_endpoint_unset`, `upload_endpoint_source` | No env value and no built-in default: remote mints are refused unless a request carries an https-proxied `Host` |
| Server start | WARN | `reason = upload_endpoint_loopback` | The env value is a loopback URL; remote launchers will be told to upload to themselves |
| Server start | WARN | `reason = upload_endpoint_plain_http` | The env value is `http://`; used as given, but uploads cross the network unencrypted |
| Server start | WARN | `reason = upload_endpoint_not_https` | The env value is not an `https://` URL (a mistyped scheme); used as given |
| Server start | INFO | `upload_endpoint_source`, `upload_endpoint` | An https value or the built-in default is in effect |
| A mint | WARN, at most one per 10 minutes | `reason = upload_endpoint_unreachable_for_peer`, `upload_endpoint_source`, `suppressed` | A loopback endpoint was advertised to a non-loopback peer. No URL, no peer |
| A mint that succeeds | INFO | `upload_endpoint_source`, `upload_endpoint_https`, `caller_forwarded`, `peer_loopback` | Which row of the table answered, and whether the URL is https. Never the URL. `caller_forwarded` is true when the quota-key address differs from the socket peer (the proxy hop); `peer_loopback` is true when the socket peer is loopback. Booleans only, never an address. The refresh INFO row carries the same two |
| A mint or refresh with no endpoint | WARN, throttled | `reason = dev_session_no_upload_endpoint`, `route`, `suppressed` | The 503 above |

The boot lines print the env URL only when its host is a DNS name, and then
as `scheme://host[:port]` with no credentials, path or query. A URL whose
host is an IP address is never printed, because that would be an address in
a log.

## The colo route

`https://play.cimmeria.app` is served by the board's Caddy reverse proxy
(the `board-proxy` container). It forwards exactly five routes to the
server's login port on loopback `:8081` and answers 404 for every other
path:

- `POST /api/auth/dev-session`
- `POST /api/auth/dev-session/refresh`
- `POST /api/telemetry/upload-chunk`
- `POST /api/telemetry/upload-bundle`
- `POST /api/telemetry/launcher-summary`

Once [auth-https.md](auth-https.md) is applied (it is not yet), the same
Caddy also forwards `/auth/v1/*` and `/SGWLogin/*` to the auth proxy
listener on loopback `:13480`.

Plain `http://play.cimmeria.app:8081` still serves the same five routes, so
a launcher that predates this change keeps working. Nothing else from the
admin API is reachable on either address, except that `:8081` also serves
the two anonymous cooked-data presync routes, which the proxy does not
forward ([admin-api.md](../tools/admin-api.md#cooked-data-presync-routes-login-port)).
Launchers from LI-09 on never use it for telemetry. They mint, refresh and
upload through the https route, so
that route, and the Caddyfile it depends on (OPS-LT1), carries all game
telemetry from those launchers.

The Caddyfile is host config. It is not generated and, until OPS-LT1 is
done, it is not in any repository. **If `board-proxy` is redeployed from
the agent-board repo before the file is mirrored there, the redeploy
removes the route** (and the sign-in routes, once auth-https.md is
applied), and the symptom is a 404 from every launcher call on the https
address. OPS-LT1 (an owner or ops task, tracked in the
launcher-telemetry README under "Owner and ops tasks") is to copy the
live Caddyfile into the private agent-board repo before the next
redeploy.

Rollback: restore the newest `Caddyfile.bak-*` beside the live file, then
run `docker restart board-proxy`. The https route disappears and the plain
http port is unaffected.

The Cloudflare Tunnel that earlier versions of the runbook described was
never deployed on the colo; there is no tunnel hostname to configure.

## The forwarded-for quota key

Behind Caddy the server's socket peer is loopback for every caller. Without
a fix, every https launcher would share one quota bucket (summaries 12 a
minute, chunks 600 a minute, bundles 30 an hour, mints 120 an hour). So the
server computes a **quota key address** (`forwarded_for::rate_limit_ip`,
`crates/admin-api/src/routes/dev_session/forwarded_for.rs`):

- If the peer is loopback and the request has an `X-Forwarded-For` header,
  the key is the **rightmost entry of the last such header**, if it parses as
  a bare IP address. A client can add entries to the left but the proxy
  appends the real caller on the right, so the rightmost entry is the one a
  caller cannot choose.
- In every other case the key is the socket peer. A non-loopback peer's
  header is ignored entirely, so a caller who reaches the port directly
  cannot pick its own key.
- The key is used for three things: the per-address quota counters (mint,
  refresh, summary, chunk and bundle); the per-address upload slot (one chunk
  and one bundle in flight); and, in process memory only, the address the
  client-telemetry registry records a stream under (D-LTL4). It is never
  logged, persisted, returned, or put in a field, span or the session record.

Which callers still share a bucket: a NAT or office egress (many launchers,
one address), any proxy that is not on loopback, and, if the container sees
Caddy as its bridge gateway rather than as loopback, every https caller (the
forwarded header is then ignored). Whether the colo's container sees Caddy
as loopback is OPS-LT2 and is unverified; see [the check](#colo-deploy-checks).
For each of those cases the advice is the same: raise the per-address quota
(see [telemetry.md § Changing a quota](telemetry.md#changing-a-quota)).

## Refusal rows

Three families of row say that the server refused a launcher or uploader.
All of them carry a reason and a count and none carries an address. A
refusal that repeats is counted instead of written, and the next row carries
the count in `suppressed`.

### Dev-session mint and refresh

One row per `(route, reason)` per 60 seconds, where `route` is `mint` or
`refresh` and a quota refusal is keyed on its `scope` as well
(`crates/admin-api/src/routes/dev_session/refusal_log.rs`). Filter:

```text
service.name = 'cimmeria-server' AND reason = '<reason>'
```

| `reason` | Level | Cause | Also carries |
|---|---|---|---|
| `dev_session_quota_exceeded` | WARN | A mint or refresh quota is full (429) | `scope`, `retry_after_secs` |
| `dev_session_kill_switch` | WARN | `CIMMERIA_TELEMETRY_KILL_SWITCH=1` (503) | |
| `dev_session_no_upload_endpoint` | WARN | [Row 5 above](#where-a-mint-sends-the-upload) (503) | |
| `dev_session_secret_unusable` | ERROR | The HMAC secret is missing or short (500) | |
| `dev_session_bad_token` | INFO | A refresh token that fails verification (401) | |
| `dev_session_expired` | INFO | A refresh token past its expiry (401) | `exp`, `now` |
| `dev_session_lifetime_exceeded` | INFO | A refresh past `CIMMERIA_TELEMETRY_MAX_SESSION_SECS` (401) | `elapsed`, `cap` |
| `dev_session_bad_request` | DEBUG | A malformed `install_id` or metadata field (400). Not throttled, never forwarded | |

### Uploads (`upload-chunk`, `upload-bundle`)

One throttled row per `(uploader, reason)` on `launcher.ingest`, with a
window that doubles from 10 s to 15 minutes. The reasons, the throttle and
the fields are in [telemetry.md § Upload size and rate
limits](telemetry.md#upload-size-and-rate-limits). The row carries `event`:

- `upload_auth_refused` for `missing_token`, `bad_token` and `token_expired`;
- `upload_refused` for every other reason.

A launcher **log session** (`purpose = logs`) adds five reasons. `purpose_target`
is a credential-class refusal, so it writes `upload_auth_refused` (SigNoz-only).
`log_session_budget`, `log_install_budget`, `log_ip_budget` and
`log_global_budget` write `upload_refused`, which still posts to Discord at
WARN under the same throttle. Their statuses, `Retry-After` values and the
env names that tune them are in [telemetry.md § Launcher log
ingest](telemetry.md#launcher-log-ingest).

```text
service.name = 'cimmeria-server' AND scope_name = 'launcher.ingest'
  AND event = 'upload_refused'
```

Swap in `upload_auth_refused` for the credential refusals. The ingest rows
carry no `peer` field; they used to, and a saved view that groups by `peer`
now groups by nothing and needs rewriting.

### Launcher summaries

A request to `/api/telemetry/launcher-summary` that the server refuses as a
whole writes one `launcher_summary_refused` row on `launcher.ingest` (WARN,
`crates/admin-api/src/routes/telemetry/launcher_summary/refusal.rs`): one
per reason per minute, with `route = launcher_summary`, `reason`, `status`
and `suppressed`. The reason names are the ingest route's where the same
refusal exists, so one filter counts both routes:

| `reason` | `status` | HTTP body code |
|---|---|---|
| `kill_switch` | 503 | `paused` |
| `rate_limited` | 429 | `over_quota` |
| `unsupported_media_type` | 415 | `unsupported_media_type` |
| `body_too_large` | 413 | `too_large` |
| `bad_request` | 400 | `bad_request` |

```text
service.name = 'cimmeria-server' AND scope_name = 'launcher.ingest'
  AND event = 'launcher_summary_refused'
```

An invalid summary inside a valid body is not a refusal: it is answered
`rejected` in its position and counted in the `launcher_summary_batch` row.
A request refused as a whole writes no batch row, so this row is the only
trace of it.

### Which rows reach Discord

`upload_refused` rows still post at WARN (a `busy` refusal is INFO and does
not). `upload_auth_refused` and `launcher_summary_refused` are SigNoz-only:
the first is the uploader's own state, the second is driven by an anonymous
caller who could otherwise fill the errors channel at will. The list is
`SIGNOZ_ONLY_EVENTS` in `crates/discord/src/layer/`; the table is in
[discord-notifications.md](../architecture/discord-notifications.md).

## Accepted launcher event targets

A `launcher` session may post chunk rows whose event name is one of these
twelve, and only these; any other is dropped
(`LAUNCHER_TARGETS` in `crates/admin-api/src/routes/telemetry/launcher_rows.rs`):

```text
client.launcher.adoption   client.launcher.startup    client.launcher.crash
client.launcher.update     client.launcher.game_update client.launcher.install
client.launcher.repair     client.launcher.launch     client.launcher.settings
client.launcher.telemetry  client.launcher.ui         client.launcher.login
```

The same list, in the same order, is
`crates/launcher-core/src/telemetry/launcher_targets.txt`. Golden tests in
the launcher engine and in `cimmeria-admin-api` pin both; change the three
together. The server accepts the list; no shipped launcher sends most of
them yet.

A log session is the exception to this list. Its token (`purpose = logs`)
may post only `client.launcher.log` and `client.launcher.game_log`
(`LAUNCHER_LOG_TARGETS` in `launcher_log_rows.rs`, mirrored by
`launcher_log_targets.txt`), and an event session may post neither. See
[telemetry.md § Launcher log ingest](telemetry.md#launcher-log-ingest).

## Colo deploy checks

Run these after the server with the wave-1 changes is deployed. None has
been run.

1. **Boot line.** `docker logs cimmeria 2>&1 | Select-String 'upload_endpoint'`
   (or `grep` on a Linux host) shows either the INFO line or a WARN with
   `reason = upload_endpoint_unset`. On the colo, with the env value
   `https://play.cimmeria.app/api/telemetry` set, expect the INFO line with
   `upload_endpoint_source = env`.
2. **Status-code probe through https.** From a machine that is not the
   colo, with `curl.exe` on Windows:

   ```powershell
   curl.exe -s -o NUL -w "%{http_code}`n" -X POST -H "Content-Type: application/json" -d "{}" https://play.cimmeria.app/api/auth/dev-session
   ```

   A reachable route answers `422` (the body is incomplete); `404` means the
   Caddy route is missing (OPS-LT1) and `503` means the kill switch, a
   missing endpoint or a missing secret (read the server's refusal row). The
   same probe at `/api/telemetry/launcher-summary` should answer `400`, and
   at `/api/config` should answer `404`. The `400` and the `404` are
   expected from the code and the route list; they were not probed through
   Caddy.
3. **OPS-LT2: does the server see the proxy as loopback?** The forwarded
   address is honored only from a loopback peer, so the per-address quotas
   work per caller only if it does. The summary quota (12 a minute) is the
   quickest way to see it:
   1. From machine A, send 13 `POST`s of `{}` to the summary URL inside one
      minute. Every request counts toward the quota even when it is
      malformed, so the first 12 answer `400` and the 13th answers `429`.
   2. At once, from machine B on a different network, send one. A `400`
      means the server keyed on B's own address: the proxy is loopback and
      the header is trusted. A `429` means B shares A's bucket: the server
      sees the proxy as a non-loopback peer (the container's bridge gateway,
      most likely), and the header is being ignored.
   3. If it is a `429`, record it for the owner. The packet decision D-LT20
      (trust the header past loopback) applies only in that case; nothing in
      the code or the config answers it today.

   Wait a minute before repeating, so the first test's counts expire.
4. **Mint through https.** After a real launcher mints through the https
   address, the mint INFO row should show `upload_endpoint_source =
   request_host` (or `env` if the override is set) and
   `upload_endpoint_https = true`.

## No launcher rows in SigNoz

A launcher is opted in and nothing appears in `cimmeria-client`. Work down
this list and stop at the first row that explains it.

1. **The boot WARN.** `reason = upload_endpoint_unset`, `_loopback`,
   `_plain_http` or `_not_https` at server start says the endpoint is
   misconfigured. Fix `CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT` and recreate the
   container.
2. **A mint that was refused.** Filter `reason` on the
   [dev-session table](#dev-session-mint-and-refresh):
   `dev_session_no_upload_endpoint` (no endpoint to hand out),
   `dev_session_quota_exceeded` (a shared address), `dev_session_kill_switch`
   or `dev_session_secret_unusable`. No mint row at all means the request
   never reached the server: check the [probe](#colo-deploy-checks).
3. **The mint INFO row.** `Minted dev-session telemetry token` with
   `upload_endpoint_source` and `upload_endpoint_https = true` proves the
   launcher was told where to upload. If `_https = false`, the env override
   is plain http, and launchers from LI-09 on refuse it.
4. **An upload refusal.** `event = 'upload_refused'` or `'upload_auth_refused'`
   on `launcher.ingest`, with its `reason` (`rate_limited`, `token_expired`,
   `body_too_large`, `kill_switch`, ...).
5. **Client rows.** Only now look at `service.name = 'cimmeria-client'`.
   A launcher row from a `launcher` session has `cimmeria.session_kind =
   'launcher'` and a `client_target` from the [accepted list](#accepted-launcher-event-targets).
   A target outside the list is dropped silently by design.

The launcher side has three checks that need no server log:

6. **The setup-diagnostics line.** After a press in Settings, the box's line says whether summaries are sent. "Saved but not sent yet" means there is no https endpoint (the first login server is an IP, localhost or a single-label name, or has credentials); a launcher on the default list now has an endpoint through the https upgrade (LI-06).
7. **`pipeline_recovered` on `launcher.telemetry`.** One row when the upload pipeline comes back after failures. Its fields are `mint_failed`, `held` and `dropped` (counts of the failed sends), `events_lost` (events dropped since the last successful send), `outage_ms` and `last_reason`. It appears once per recovery, after sends resume, so its absence proves nothing; a launcher that never recovers shows no row at all.
8. **`game_session_mint_failed` on `launcher.telemetry`.** A game session could not be minted, so Play went on with no game diagnostics. The row carries a `reason` (`timeout`, `transport`, `http_status`, `decode`, `endpoint_refused` or `no_https_endpoint`), `http_status` only when the login server replied, and `duration_ms`.

For the anonymous summary route there is no mint: check the
`launcher_summary_refused` rows, then the `launcher_summary_batch` row, then
`launcher.summary` in `cimmeria-client`.
