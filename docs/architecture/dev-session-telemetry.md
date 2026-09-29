# Dev-Session Telemetry — Architecture

> **Last updated**: 2026-09-29

How the launcher streams a developer's session (Atera client log,
BigWorld `sgwdebuglog*`, end-of-session bundle) to the cimmeria-server
ingest endpoint, which replays it through `tracing` so it lands in
SigNoz alongside the server's own logs and Mercury packet stream.
The operator-facing runbook lives in
[`docs/operations/telemetry.md`](../operations/telemetry.md); this
document is the design rationale and component map.

The ingest target was previously the Cosmos-backed `Cimmeria-MCP`
Azure Function. With the SigNoz migration that path is retired —
uploads now land on cimmeria-server's admin port and flow into SigNoz
through the OTLP exporter. See [observability.md](observability.md)
for the broader pipeline.

## Goal

Every developer running the dev build captures their session
automatically — zero interaction beyond launching the game. The Atera
client log is the primary diagnostic value because it surfaces
undelivered packets and packet-numbered reliability events the server
cannot observe.

## Trust model

Launcher-mediated credentials, HMAC-token auth, single-party verifier.

- The launcher holds **no** static secret. It fetches a per-session
  HMAC-SHA256 token from cimmeria-server at game-launch.
- cimmeria-server holds the **only** static secret
  (`CIMMERIA_TELEMETRY_HMAC_SECRET`) and is the only party that
  verifies tokens — the upload-chunk and upload-bundle endpoints live
  on the same server that mints them. No cross-service secret
  synchronization (the prior Cimmeria-MCP write path required mirror
  copies of the secret in two repos; that's gone).
- Minting needs no credential: the `sub` claim is the caller's own
  `install_id`, so anyone who can reach the admin port or the public
  login port (see [Where the routes are served](#where-the-routes-are-served))
  can mint.
  Account-bound auth is a v2 concern and needs a launcher-side
  handshake. What bounds the damage meanwhile is not authentication
  but three limits — see [Mint and refresh limits](#mint-and-refresh-limits).

## Component map

| Crate / module | Role |
|---|---|
| `crates/launcher/src/identity.rs` | Mint-once per-install identity (`install_id`, `machine_id`). |
| `crates/launcher/src/telemetry/auth.rs` | Launcher → server `/auth/dev-session` client + proactive-refresh policy. |
| `crates/launcher/src/telemetry/tail.rs` | Polling tailer (2 s) over `Binaries/sessions/*.log` + `sgwdebuglog*`. Handles per-minute Atera rotation. |
| `crates/launcher/src/telemetry/events.rs` | NDJSON event schema + Atera log-line parser. |
| `crates/launcher/src/telemetry/queue.rs` | Crash-safe on-disk JSONL queue (100 MiB cap, drop-oldest). |
| `crates/launcher/src/telemetry/chunk.rs` | Gzipped NDJSON POST to `/api/upload-chunk`. |
| `crates/launcher/src/telemetry/bundle.rs` | End-of-session multipart POST to `/api/upload-bundle`. |
| `crates/launcher/src/telemetry/session.rs` | `current-session.json` writer (reserved for future Lua-side hook). |
| `crates/launcher/src/telemetry/process_watch.rs` | `spawn_blocking` wait on the game (`Child::wait` for a plain launch, `RunningProcess::wait` for an injected one) — game-exit signal without burning an async worker. |
| `crates/launcher/src/telemetry/patch_log.rs` | Reads the client-patches DLL's `cimmeria-client-patches.log` and yields one `client.patches.boot` event per session. |
| `crates/launcher/src/telemetry/runner.rs` | Per-session loop: tail → enqueue → flush → on-exit bundle. |
| `crates/launcher/src/telemetry/mod.rs` | `Telemetry` orchestrator (`start_session` / `enqueue` / `flush` / `refresh_if_due` / `upload_bundle`). |
| `crates/admin-api/src/routes/dev_session/` | Server-side `/api/auth/dev-session` + `/refresh` endpoints (mint + verify), quota tables. |
| `crates/admin-api/src/routes/telemetry/` | Server-side `/api/telemetry/upload-{chunk,bundle}` ingest. Validates the HMAC token, decompresses gzip(NDJSON) or unzips bundle, replays each event through `tracing::*` (`replay.rs`) so the OTLP layer ships it to SigNoz. The client-side rows go to the `cimmeria-client` service. |
| `crates/client-telemetry/src/uploader.rs` | The injected DLL's own uploader: reads `current-session.json`, POSTs gzip(NDJSON) `client_native` batches to `<upload_endpoint>/upload-chunk`. |
| `crates/lab/src/supervisor/telemetry_session.rs` | The Live Research Lab supervisor's mint: one lab session (`session_kind = "lab"`) per client launch, written into the session file the DLL reads. |

## Where the rows land

Everything a session uploads is replayed through `tracing`; the
server's log routing (`crates/server/src/logging/filters.rs`) sends the
client-side rows (`client.native`, `launcher.client_log`,
`launcher.debug_log`, `launcher.session_meta`) to the SigNoz service
`cimmeria-client` and nowhere else. Each row carries the token's
`session_id` and `install_id`, `cimmeria.session_kind` (`lab` or
`player`) and `lab`; `client.native` rows add the DLL's event name as
`client_target` and a few keys lifted out of its `fields` bag. The full
table is in [observability.md](observability.md#log-indexes-and-parity-with-the-log-files).

## The upload endpoint is a base

`upload_endpoint` in the mint response, and so in
`current-session.json`, is the upload **base**
(`https://host/api/telemetry`, default `http://localhost:8443/api/telemetry`):
the launcher appends `/upload-chunk` and `/upload-bundle`. The DLL used
to POST to it verbatim, so every batch from a launcher-written session
went to the base path and was refused; it now appends `/upload-chunk`
unless the value already ends with it (`uploader::chunk_url`). A blank
`CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT` (compose passes `${VAR:-}`) falls
back to the default instead of handing callers `""`.

## Where the routes are served

The four routes (`/api/auth/dev-session`, `/api/auth/dev-session/refresh`,
`/api/telemetry/upload-chunk`, `/api/telemetry/upload-bundle`) are served
on two listeners, at the same paths:

- the admin API listener (`ADMIN_BIND:8443`), private and
  unauthenticated (#439);
- the public SOAP login listener (`LOGON_PORT`, 8081, and the auth TLS
  listener when it is configured), always on.

Decision (@Cadacious, 2026-09-29): a remote player's launcher must work
with no config and no secret, and the login port is the one address every
player already reaches. Plain HTTP there adds no exposure the game does
not already have: the client's own SOAP login sends the password over
plain HTTP on that port. No TLS or tunnel is required; a Cloudflare route
in front of the admin port remains an option.

Wiring: `cimmeria_admin_api::login_port_telemetry_router()` builds a
stateless router holding only those four routes (with their body limits
and the same per-request trace span as the admin router). The composition
root (`crates/server/src/main.rs`) hands it to the auth service through
`AuthService::set_public_routes` before `start_all`, and the auth service
merges it next to `/SGWLogin/*`. The auth crate sits below the admin API
in the crate graph, so it takes the router rather than building it. Both
auth listeners serve with connect info, which the mint and refresh quotas
need. Nothing else under `/api`, `/ws` or Swagger is served on the login
port; `login_port_router_does_not_expose_admin_routes` pins that.

A public server sets `CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT` to its login
port (`http://<host>:8081/api/telemetry`) so uploads go there too.

## Launcher endpoint policy

The launcher checks `telemetry.auth_url` before minting, and every
`upload_endpoint` the server returns (at mint and at each refresh),
against one rule (`crates/launcher/src/telemetry/endpoint.rs`,
`EndpointPolicy`):

- `https://` anywhere;
- plain `http://` to loopback;
- plain `http://` to the exact host and port of one of the launcher's
  `http://` login servers.

Anything else is refused before a byte is sent. The third case exists for
the login-port mount above: the player already talks to that host and port
in plaintext. Another port on the same host (the admin API's 8443) and
look-alike hosts are refused, and an `https://` login server does not
vouch for plain http to its host. The telemetry HTTP client never follows
redirects, so a checked request cannot be bounced elsewhere.

The default `auth_url` is `http://play.cimmeria.app:8081/api`, on the
default login server. Launcher config schema 2 migrates a schema-1 file
whose `auth_url` is exactly the old default (`http://localhost:8443/api`)
to the new default once, and writes the file back; any other value is
kept.

## Lab sessions

The lab supervisor mints from `CIMMERIA_LAB_SERVER_URL` (default
`http://127.0.0.1:8443`) with `session_kind = "lab"`, and writes the
token, the server's `session_id` and the upload endpoint into the
session file (`CIMMERIA_LAB_UPLOAD_ENDPOINT` overrides the endpoint).
The lab bridge's own per-launch token stays in the `lab` block and never
leaves the machine: the telemetry token is a different value. A failed
mint does not stop the launch; the file then carries an empty token (the
bridge still starts, because `telemetry.enabled` stays true) and
`lab_client_start` reports why under `telemetry`.

## Session lifecycle

Two buttons run a session. **Launch + Telemetry** (the Atera debug
path) is traced below. **Launch SGW.exe** runs one too when
the player opted in (`telemetry.opted_in`, off by default) and the
identity loaded (`worker/launch_sgw.rs`, `worker/launch_telemetry.rs`).
Since 2026-09-29 that session starts *before* the game: the injected
`cimmeria-client-telemetry` DLL reads its token and upload endpoint
from `current-session.json` as it boots, so the file must exist first.
The handshake gets at most 10 s; a failed or slow one is reported and
the game starts without the telemetry DLL, so it delays play by no more
than that. The game then starts with the client-patches DLL and the
telemetry DLL, in that order, and the session follows it. The launcher
sends no `session_kind`, so both the launcher's uploads and the DLL's
carry `cimmeria.session_kind = player`. That session also records the
`client.patches.boot` and `client.telemetry_dll.launch` events described
after this sequence. Without the opt-in, **Launch SGW.exe** starts no
session and injects the client patches only.

```text
1. User clicks "Launch + Telemetry" in the launcher UI
2. App composes LaunchTelemetryConfig from LauncherIdentity + LauncherConfig
3. Worker.spawn_launch_with_telemetry:
   a. Telemetry::start_session
      └─ POST /api/auth/dev-session → token, session_id, upload_endpoint
      └─ write current-session.json
   b. launch_atera_debug_with_child → Child handle
   c. spawn runner::run_session
4. runner::run_session loop (every flush_interval_ms):
   a. tailer.refresh + tick → TailedLines
   b. parse_client_log_line → TelemetryEvent
   c. telemetry.enqueue (writes to DiskQueue)
   d. telemetry.refresh_if_due (rotates token at 75% TTL elapsed)
   e. telemetry.flush → POST /api/upload-chunk (gzip NDJSON)
5. process_watch::wait_for_exit resolves
6. Final tick + final flush
7. telemetry.upload_bundle:
   a. build zip via logs::build_log_zip
   b. multipart POST /api/upload-bundle (metadata JSON + zip)
8. Worker emits Event::TelemetrySessionComplete(outcome)
```

### Client-patches boot event

A **Launch SGW.exe** session passes the runner a `PatchLogWatcher`. On
each tick it reads `cimmeria-client-patches.log` next to `SGW.exe`
(ignoring a file older than the launch), and once the DLL has logged
how its bootstrap ended it enqueues one `client_native` event with
target `client.patches.boot`. When the DLL was not injected, the event
goes out on the first tick; when the game exits first, it goes out
with whatever the log showed. Fields:

| Field | Values |
|---|---|
| `injection` | `injected`, `opted_out`, `unavailable`, `inject_failed` (the launcher's side) |
| `log_found` | whether this launch's log existed |
| `dll_version` | from the DLL's `attached, version …` line |
| `fingerprint.<site>` | `stock`, `chained`, `unknown_hook`, `mismatch`, `unreadable` per hooked site |
| `fingerprint_ok` | every site `stock` or `chained` |
| `verdict`, `verdict_detail` | `installed`, `nothing_installed`, `hook_failed` or `none`, with the DLL's message |

The level is `info` when the DLL was injected and installed its hooks,
`warn` otherwise. In SigNoz, `target = 'client.patches.boot'` answers
"why did the Black Market window not open for this player" without a
repro. The log lines this parses are listed in the client-patches
[README](../../crates/client-patches/README.md#log).

### Telemetry DLL launch event

The same session records one `client_native` event with target
`client.telemetry_dll.launch`, before the runner starts, saying whether
the telemetry DLL went in: `outcome` is `injected` (with `dll_path`),
`unavailable` (with `reason`: not bundled, or a `lab-bridge` build
refused) or `inject_failed` (the game was started without it). The
level is `info` for `injected`, `warn` otherwise. An opted-in session
with no `client.dll.attached` row from the DLL can be told apart from
one whose DLL never went in.

## Failure modes

| Scenario | Behavior |
|---|---|
| `/auth/dev-session` returns 503 (kill switch) | Launcher logs warn, falls back to `launch_atera_debug` (no telemetry). Game launches. |
| `/auth/dev-session` network failure | Same as kill switch — game launches without telemetry. |
| **Launch SGW.exe**: handshake fails or takes over 10 s | Session error in the status log; the game launches with the client patches only, no telemetry DLL. |
| **Launch SGW.exe**: telemetry DLL missing or a `lab-bridge` build | Status line `In-game telemetry: unavailable: …`; the game launches without it; the session still runs (`client.telemetry_dll.launch`, `outcome = unavailable`). |
| **Launch SGW.exe**: injecting both DLLs fails | Retried with the client patches alone, then plainly; `outcome = inject_failed`. |
| DLL session outlives its 8 h token | The DLL never refreshes: its uploads stop being accepted. The launcher's own uploads refresh and continue. |
| Chunk POST 401 | `ChunkError::TokenRejected` → next tick fires `refresh_if_due`. |
| Chunk POST 503 | `ChunkError::KillSwitch { retry_after_secs }` honored. Events stay on disk. |
| Launcher killed mid-session | Game keeps running (no Job Object). Events stay in `telemetry-queue.jsonl`. Next launch drains them via `recover_pending_on_startup`. |
| Game crashes | Same as clean exit from the runner's perspective — `child.wait()` returns. Final flush + bundle upload still fire. |
| Disk full mid-enqueue | Enqueue surfaces the IO error; the event is lost on the stack. Telemetry is supplementary — never load-bearing. |

## Token format

```text
payload = base64url(JSON {iss, sub, sid, iat, exp, scope, kind?})
sig     = base64url(HMAC-SHA256(secret, payload))
token   = payload || "." || sig
```

`iss` = `"cimmeria-server"`. `sub` = install_id. `sid` = session_id.
`scope` = `["telemetry.write"]`. `kind` = `"lab"` for a session minted
with `"session_kind": "lab"`, and **absent** for a player's launcher, so
a player token is byte-identical to one minted before the claim existed
and older tokens still decode. It is signed, so an upload cannot relabel
itself, but minting is open, so it is a filter label and never a
privilege. A refresh keeps it. Any other requested kind is a 400.

`iat` is the **original mint** time and survives every refresh, so
`exp` − `iat` is 8 hours only on a freshly minted token and shrinks
across a refresh chain. That is deliberate: it is what makes the
chain bounded. The launcher's proactive-refresh policy
(`should_refresh`) uses its own locally recorded issue time and never
reads this claim, so nothing client-side depends on the difference.

8-byte URL-safe base64-no-pad on both segments. Constant-time
signature comparison via `Hmac::verify_slice` defends against timing
oracles.

## Mint and refresh limits

Three things stand in for the authentication the mint endpoint does
not have.

**Scope.** A minted token carries only `telemetry.write`, and
`verify_bearer` in `crates/admin-api/src/routes/telemetry/handlers.rs`
refuses a token without it. The scope is checked at ingest rather
than assumed from the mint path, so narrowing what a token may do
stays a one-line change on the server.

**Quotas.** Mint and refresh are counted per peer address, and mint
additionally per `install_id`, over a fixed window. Refresh verifies
the token before charging its counter, and counts tokens that fail
verification on a separate, tighter counter, so junk sent from an
address shared with real launchers cannot lock them out. Over quota
returns 429 with a `Retry-After` the launcher's back-off path already
honours. Defaults and env-var names are in
[telemetry.md](../operations/telemetry.md#mint-and-refresh-quotas).

Two properties are worth stating because they shaped the
implementation:

- **The per-IP quota is the load-bearing control; the per-`install_id`
  one is a speed bump.** `install_id` is chosen by the caller, so an
  attacker rotates it. The IP counter is charged before `install_id`
  is validated, which is what makes rotation pointless from one
  address. (A body that fails JSON extraction is rejected by the
  framework before the handler runs and is not charged; no token is
  issued on that path.)
- **The counter store cannot grow.** Both keys are caller-supplied, so
  a map keyed on either would be its own memory-exhaustion vector.
  The store is instead a fixed 4096-slot array indexed by
  `hash(key) % N`, and two keys that land on the same slot share its
  counter. Resetting the slot on a key change instead would let a
  caller holding two colliding addresses (two IPv6 /64s, say) alternate
  them and reset its own counter on every request. The hash is SipHash
  with a per-process random seed, so a collision cannot be picked or
  precomputed; what remains is an unaimable ~1-in-4096 chance that two
  live callers share a bucket. IPv6 keys fold to the /64 prefix,
  because a single host is routinely handed a whole /64.
- **Caller metadata is checked before it is logged.** `machine_id`,
  `branch`, `git_sha` and `launcher_version` reach the INFO mint line,
  and the plain `fmt` log sinks do not escape them, so a value with a
  control character (or longer than 256 bytes) is refused with 400.
  Both routes also cap the request body at 8 KiB, because the JSON
  body is parsed before any quota is charged.

Behind a reverse proxy (a Cloudflare Tunnel, say) every request
arrives from the proxy, so the per-IP quota degenerates to a global
cap. No forwarded-for header is read: an unconditional read would let
any caller set its own quota key. Operators fronting the admin port
should raise `CIMMERIA_TELEMETRY_MINT_QUOTA_PER_IP` accordingly. On the
login port each launcher arrives from its own address, so the quota keys
on the player.

**Bounded refresh chain.** `refresh` preserves the original `iat` and
refuses once the session passes `CIMMERIA_TELEMETRY_MAX_SESSION_SECS`
(24 h by default), clamping the last token's `exp` to the deadline
rather than overshooting it. Without this, each refresh granted a
fresh full TTL and one leaked token could be walked forward forever.
At the cap the launcher sees a 401 on refresh and mints a fresh
session, the same path it already takes for an expired token.

## Backpressure + queue overflow

- **In-memory channel:** none — events go straight to disk via
  `DiskQueue::enqueue`. The flush cadence (2 s) is what bounds latency.
- **Disk queue cap:** 100 MiB. Crossing triggers
  `compact_to_retain_tail` which retains the most recent ~80 MiB
  (drop-oldest, line-aligned) and bumps a cumulative dropped-line
  counter.
- **Bundle metadata** carries `dropped_lines` so server-side can
  reconcile streamed totals vs. on-disk losses.

## Idempotency

- Per-event uniqueness: `(session_id, seq)` — server dedupes via
  Cosmos upsert. A retried chunk after a network blip is a no-op.
- Per-bundle uniqueness: `(session_id, zip_sha256)` — server-side
  Blob upload uses the sha as the blob name, so a retry overwrites
  with identical bytes.

## Why polling for the tail

`notify` would give sub-second latency but adds a native-deps stack
(libinotify on Linux, ReadDirectoryChangesW on Windows). Against the
2 s flush cadence the latency win is marginal; polling is dep-free
and tests cleanly without a real filesystem watcher harness.

## Why `std::process::Child` and not `tokio::process::Child`

The tokio variant kills the child on drop unless explicitly told not
to. The spec requires "Launcher death must NOT kill the game" —
`std::process::Child` has no kill-on-drop, so even an unexpected
launcher exit leaves SGW.exe running. The cost is one
`spawn_blocking` task to host `child.wait()`, which is cheap.

## Not in scope

- Server-side Mercury frame capture — the client log already carries
  the packet-numbered reliability events.
- Per-event PII redaction — dev-only data, the dev IS the device
  owner.
- Multi-tenant Cosmos isolation — single-tenant for now.
- LLM auto-summary at bundle ingest — companion Functions repo issue.
