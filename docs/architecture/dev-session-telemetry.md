# Dev-Session Telemetry — Architecture

> **Player privacy scope:** The [current collector inventory](../../crates/launcher/desktop/docs/privacy-inventory.md) distinguishes fixed summaries, raw game text/ZIPs, consent and running-DLL limits. Use it for disclosure; historical implementation notes below are not blanket anonymity or redaction guarantees.

> **Type:** Explanation · **Audience:** Telemetry and launcher engineers · **Last updated:** 2026-10-23
> **Companions:** [Operations](../operations/telemetry.md), [native launcher timing](../../crates/launcher/desktop/docs/launcher-telemetry.md)

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

The launcher here is the desktop launcher. The pipeline's pieces live in
`cimmeria-launcher-core` (`crates/launcher-core/src/telemetry/`), moved
there from the egui launcher that LX-18 retired, and the desktop engine
drives them from `crates/launcher/desktop/engine/src/storage/game_telemetry/`.
Its player-facing contract is in the desktop
[launch contract](../../crates/launcher/desktop/docs/launch-telemetry.md#opt-in-game-telemetry).

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
| `crates/launcher/desktop/engine/src/storage/game_telemetry/mod.rs` | Per-install identity (`install_id`, `machine_id`), minted at the first opt-in and kept after an opt-out, in `game-telemetry.json`. |
| `crates/launcher-core/src/telemetry/auth.rs` | Launcher → server `/auth/dev-session` client + proactive-refresh policy. |
| `crates/launcher-core/src/telemetry/tail.rs` | Polling tailer (2 s) over `Binaries/sessions/*.log` + `sgwdebuglog*`. Handles per-minute Atera rotation. |
| `crates/launcher-core/src/telemetry/events.rs` | NDJSON event schema + Atera log-line parser. |
| `crates/launcher-core/src/telemetry/queue.rs` | Crash-safe on-disk JSONL queue (100 MiB cap, drop-oldest). |
| `crates/launcher-core/src/telemetry/chunk.rs` | Gzipped NDJSON POST to `/api/upload-chunk`, and the split of a drained queue into chunks. |
| `crates/launcher-core/src/telemetry/flush.rs` | Flushes the queue as chunks of at most `chunk_max_bytes` of NDJSON and 1,000 rows. A 413, or any 4xx but 401, 408 and 429, drops that chunk into the dropped-lines count; a 429 or 503 holds further flushes off for its `Retry-After`; any other failure re-queues the unsent chunks. |
| `crates/launcher-core/src/telemetry/bundle.rs` | End-of-session multipart POST to `/api/upload-bundle`, carrying the log files written to since the session started. |
| `crates/launcher-core/src/telemetry/session.rs` | `current-session.json` writer: the file the injected DLL reads its token and upload endpoint from at boot. |
| `crates/launcher-core/src/telemetry/process_watch.rs` | `spawn_blocking` wait on the game (`Child::wait` for a plain launch, `RunningProcess::wait` for an injected one) — game-exit signal without burning an async worker. |
| `crates/launcher-core/src/telemetry/patch_log.rs` | Reads the client-patches DLL's `cimmeria-client-patches.log` and yields one `client.patches.boot` event per session. |
| `crates/launcher-core/src/telemetry/patch_counts.rs` | Parses the same log's per-call lines into the DLL's claimed / delivered / dropped counters and yields `client.patches.counts` (at most once a minute when a count moved, and once at game exit). |
| `crates/launcher-core/src/telemetry/install_result.rs` | Builds `client.launcher.install_result` from an `install_report::InstallReport` (one outcome per manifest patch), written to the on-disk queue after each Install, Repair or game Update run when the player opted in. |
| `crates/launcher/desktop/engine/src/storage/game_telemetry/launcher_event.rs` | Builds `client.launcher.adoption` events from the desktop shell's `adoption` log target, scrubbed of the home folder, the user name and absolute paths. |
| `crates/launcher/desktop/engine/src/storage/game_telemetry/launcher_session.rs` | Mints a short `launcher` session, without a game, for an opted-in player, and posts those events at once. Nothing is queued on disk. It re-mints only on 401 and holds after a 429 or 503. `LauncherSender::logs()` is a second sender that mints a `purpose = "logs"` session for Upload debug logs, whatever the opt-in says (D-LU1); it never shares a token with the event session. |
| `crates/launcher-core/src/telemetry/runner.rs` | The egui launcher's per-session loop: tail → enqueue → flush → on-exit bundle. The desktop launcher runs the same steps in its own loop (next row). |
| `crates/launcher-core/src/telemetry/mod.rs` | `Telemetry` orchestrator (`start_session` / `enqueue` / `flush` / `refresh_if_due` / `upload_bundle`). |
| `crates/launcher/desktop/engine/src/storage/game_telemetry/` | The desktop launcher's side: `session.rs` mints the session and writes the marker before Play starts the game, `session_loop/` tails, flushes and sends the exit event and bundle, and `transport/` holds the queue, token refresh and back-off. |
| `crates/admin-api/src/routes/dev_session/` | Server-side `/api/auth/dev-session` + `/refresh` endpoints (mint + verify), quota tables. |
| `crates/admin-api/src/routes/telemetry/` | Server-side `/api/telemetry/upload-{chunk,bundle}` ingest. Validates the HMAC token, decompresses gzip(NDJSON) or unzips bundle, replays each event through `tracing::*` (`replay.rs`) so the OTLP layer ships it to SigNoz. The client-side rows go to the `cimmeria-client` service. |
| `crates/client-telemetry/src/uploader.rs` | The injected DLL's own uploader: reads `current-session.json`, POSTs gzip(NDJSON) `client_native` batches to `<upload_endpoint>/upload-chunk`. When to retry a refused or failed POST, and with which token, is `upload_gate.rs`. |
| `crates/lab/src/supervisor/telemetry_session.rs` | The Live Research Lab supervisor's mint: one lab session (`session_kind = "lab"`) per client launch, written into the session file the DLL reads. |

The server side of the `launcher` kind is in `docs/operations/telemetry.md` § Upload size and rate limits.

## Where the rows land

Everything a session uploads is replayed through `tracing`; the
server's log routing (`crates/server/src/logging/filters.rs`) sends the
client-side rows (`client.native`, `launcher.client_log`,
`launcher.debug_log`, `launcher.session_meta`) to the SigNoz service
`cimmeria-client` and nowhere else. Each row carries the token's
`session_id` and `install_id`, `cimmeria.session_kind` (`lab` or
`player`) and `lab`, and `client_pid` when the upload's `X-Cimmeria-Client-Pid` header carried one (absent from older DLLs, never 0); `client.native` rows add the DLL's event name as
`client_target` and a few keys lifted out of its `fields` bag. The full
table is in [observability.md](observability.md#log-indexes-and-parity-with-the-log-files).

## Native launcher phase timing

Adoption and Play add bounded native phase lifecycles to the existing event
pipeline. Their targets remain `adoption` and `launcher.launch`; the forwarding
layer maps them to `client.launcher.adoption` and `client.launcher.launch`.
Accepted rows land under `client.native` with those `client_target` values and
JSON-text `fields`. They do not go through `launcher.summary`.

Monotonic durations measure native work without depending on clock corrections.
Start/finish pairs distinguish an observed completion from incomplete evidence;
process-local numeric operation/phase correlation avoids exporting durable
operation UUIDs. Scan counts explain workload without a per-file event stream.
The [desktop contract](../../crates/launcher/desktop/docs/launcher-telemetry.md)
owns phase/outcome names and interpretation; the [operator recipe](../operations/telemetry.md#launcher-adoption-and-play-phase-timing)
shows how to read the nested payload.

Existing consent, scrubbers, event-session quotas and lossy forwarding remain
unchanged. Missing finishes cannot establish a hang, and process start cannot
establish login or world arrival. This adds no token fields, secret, destination,
Plan field or server deployment requirement.

## The upload endpoint is a base

`upload_endpoint` in the mint response, and so in
`current-session.json`, is the upload **base**
(`https://host/api/telemetry`): the launcher appends `/upload-chunk` and
`/upload-bundle`. The DLL used to POST to it verbatim, so every batch from a
launcher-written session went to the base path and was refused; it now
appends `/upload-chunk` unless the value already ends with it
(`uploader::chunk_url`).

The server resolves the base in a fixed order
(`routes/dev_session/upload_endpoint.rs`): the
`CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT` env value if it is not blank (compose
passes `${VAR:-}`, so blank means unset); else `https://<Host>/api/telemetry`
when the `Host` header is valid and `X-Forwarded-Proto` is exactly `https`;
else `http://<Host>/api/telemetry` for a loopback `Host` from a loopback
peer (a one-machine dev box); else a built-in https default that is `None`
until LT-41; else the mint and refresh answer **503** `no_upload_endpoint`.
There is no localhost or plain-http fallback for a remote caller. The
reasoning: a launcher told to upload to `http://localhost:8443` by a remote
server sends nothing anywhere useful and the failure is silent, so a missing
endpoint is a loud refusal instead. The table, the boot warnings and the
Docker Desktop caveat are in [telemetry-upload-endpoint.md](../operations/telemetry-upload-endpoint.md#where-a-mint-sends-the-upload).

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
plain HTTP on that port. Since 2026-10-21 the colo also serves the same
routes over https at `https://play.cimmeria.app`, through a Caddy reverse
proxy that forwards only the telemetry routes to the login port and answers
404 for everything else ([the colo route](../operations/telemetry-upload-endpoint.md#the-colo-route)). The server
now hands out the https address, not the plain-http one; the plain-http
login port still serves the routes for launchers that predate this. No
Cloudflare Tunnel was ever deployed.

Wiring: `cimmeria_admin_api::login_port_telemetry_router()` builds a
stateless router holding those four routes and, since 2026-10-04, a fifth:
the launcher-summary ingest. Since 2026-10-26 it also holds the two
anonymous cooked-data presync routes. Launcher summaries do not use
dev-session tokens: that route is anonymous, and the mint refuses `launcher_summary`
as a session kind like any other unknown one (see
[launcher-summary-telemetry.md](launcher-summary-telemetry.md)). The owner
approved serving the fifth route on the login port on 2026-10-04, which
extends the decision above to it. The router comes with the routes'
body limits and a per-request trace span like the admin router's, except
that it records the path without the query string. The composition
root (`crates/server/src/main.rs`) hands it to the auth service through
`AuthService::set_public_routes` before `start_all`, and the auth service
merges it next to `/SGWLogin/*`. The auth crate sits below the admin API
in the crate graph, so it takes the router rather than building it. Both
auth listeners serve with connect info, which the mint and refresh quotas
need. Nothing else under `/api`, `/ws` or Swagger is served on the login
port, apart from the two anonymous cooked-data presync routes
(`/api/cooked/v1/index` and `/api/cooked/v1/category/{id}`, see
[cooked-data-pipeline.md](../engine/cooked-data-pipeline.md#launcher-presync));
`login_port_router_does_not_expose_admin_routes` pins that.

A public server sets `CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT` to an https URL
it serves (the colo: `https://play.cimmeria.app/api/telemetry`), or sits
behind a proxy that passes `Host`, `X-Forwarded-Proto: https` and
`X-Forwarded-For`, in which case the server derives the URL itself.

## Launcher endpoint policy

The launcher checks the address it mints from (the installation's first
login server), and every `upload_endpoint` the server returns (at mint
and at each refresh), against one rule
(`crates/launcher-core/src/telemetry/endpoint.rs`,
`EndpointPolicy`):

- `https://` anywhere;
- plain `http://` to loopback.

Anything else is refused before a byte is sent. An http login server on a
DNS name is reached as `https://<name>` on 443, by the rule summaries use
(`https_upgrade_origin`). An IP or single-word http login server gets no game
telemetry. A dotted LAN name such as `nas.lan` is tried over https, and a
failed connection sends nothing. A plain-http upload URL that the server hands
back is refused, never rewritten. There is no http fallback.

The telemetry HTTP client never follows redirects, so a checked request
cannot be bounced elsewhere.

The desktop launcher mints from `<first login server>/api/auth/dev-session`
(`https://<name>/api/auth/dev-session` for an http login server on a DNS name),
so with the default server list that is the public login port. (The retired
egui launcher kept a separate `telemetry.auth_url` setting, defaulting to
`http://play.cimmeria.app:8081/api`; an imported legacy record keeps that
value unchanged.)

## Lab sessions

The lab supervisor mints with `session_kind = "lab"` from the login URL
of the client's server row in `LoginInternal.lua`, the same login port a
launcher mints from (`CIMMERIA_LAB_SERVER_URL` overrides it), and writes the
token, the server's `session_id` and the upload endpoint into the
session file (`CIMMERIA_LAB_UPLOAD_ENDPOINT` overrides the endpoint).
The lab bridge's own per-launch token stays in the `lab` block and never
leaves the machine: the telemetry token is a different value. A failed
mint stops the launch with the reason. With
`CIMMERIA_LAB_TELEMETRY=optional` the launch goes ahead instead; the file
then carries an empty token (the bridge still starts, because
`telemetry.enabled` stays true) and `lab_client_start` reports why under
`telemetry`.

## Session lifecycle

**Play** runs a session when the player opted in ("Send game diagnostics"
in Settings, off by default). The session starts *before*
the game: the injected `cimmeria-client-telemetry` DLL reads its token and
upload endpoint from `current-session.json` as it boots, so the file must
exist first. The handshake is bounded to 8 s; a failed or slow one leaves
the DLL out and the game starts without it, so it delays play by no more
than that. The game then starts with the client-patches DLL and the
telemetry DLL, in that order, and the session follows it. The launcher
sends no `session_kind`, so both the launcher's uploads and the DLL's
carry `cimmeria.session_kind = player`. That session also records the
`client.patches.boot` and `client.patches.counts` events described after
this sequence. Without the opt-in, Play starts no session and injects the
client patches only. The desktop details (tail baselines, one session at a
time, the `game_exited` event) are in the
[launch contract](../../crates/launcher/desktop/docs/launch-telemetry.md#opt-in-game-telemetry).

The retired egui launcher ran the sequence below from its **Launch +
Telemetry** button (the Atera debug path). It is kept because the core
pieces it names are the ones the desktop session loop calls in the same
order: start the session, tail and enqueue, refresh, flush, then on exit a
final flush and the bundle.

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
   e. telemetry.flush → POST /api/upload-chunk (gzip NDJSON), one chunk
      per chunk_max_bytes / 1,000 rows of the drained queue
5. process_watch::wait_for_exit resolves
6. Final tick + final flush
7. telemetry.upload_bundle:
   a. build zip via logs::build_session_log_zip (files modified since
      the session started, less 10 s)
   b. multipart POST /api/upload-bundle (metadata JSON + zip)
8. Worker emits Event::TelemetrySessionComplete(outcome)
```

### Client-patches boot event

A Play session watches the client-patches log with a `PatchLogWatcher` (`patch_log.rs`). On
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

### Client-patches counts event

The same watcher also reports what the DLL did after it booted: target
`client.patches.counts`, added 2026-09-29 after a playtest where the
only evidence that `onBMOpen` was dropped (the overlay's global
`CimmeriaBM` was not defined) was the player's local log file. Every
per-call line the DLL writes ends in `(#n)`, the running value of one
of its session counters, and the DLL writes only the 1st, 10th, 100th
... occurrence (the send path also the first 100). The highest `n` per
counter is therefore a **lower bound**, and the event says so with
`counts_are_lower_bounds = true`.

| Field | Meaning |
|---|---|
| `claimed`, `not_local_player`, `decode_failed`, `dropped_queue_full`, `delivered`, `dropped_no_overlay`, `dropped_no_handler`, `handler_failed`, `sent`, `send_refused`, `tech_competency_nil`, `natives_registered`, `register_failed` | Highest count seen per DLL counter (0 when never logged) |
| `<counter>.methods` | Comma list of the method, handler or native names those lines named (at most 16) |
| `<counter>.last_reason` | The last line's reason, e.g. `the global CimmeriaBM is not defined, so the UI overlay is not installed`, a decode error, or a send refusal (`offline (…)`) |
| `dropped_total` | Sum of the five drop counters (claimed calls the overlay never got) |
| `lines`, `log_capped` | DLL lines read; `true` once the DLL's 2,000-line cap was reached, so later counts may be missing |
| `final` | `false` for a periodic row, `true` for the one at game exit |

It is sent at most once a minute while a count moves, and once at game
exit, only when the DLL was injected and counted something. The level
is `warn` when any drop, send refusal or registration failure was
counted, `info` otherwise.

### Install result event

Every Install, Repair or game Update run fills an `InstallReport`
([`install_report.rs`](../../crates/launcher-core/src/install_report.rs)):
the seed step, one outcome per manifest patch, and how the run ended.
When the player opted in to telemetry, the launcher writes one
`client_native` event with target `client.launcher.install_result` to
its on-disk telemetry queue, stamped with the install time. No session
exists during an install, so the event ships with the first flush of
the next telemetry session.

| Field | Meaning |
|---|---|
| `result` | `ok`, `patches_failed`, `cancelled` or `error` (the run stopped early) |
| `seed_applied` | this run downloaded and unpacked the seed |
| `patch_count`, `applied`, `already`, `failed`, `skipped_dependency` | patches in the manifest, and how many ended each way |
| `patch.<id>` | that patch's outcome: `applied`, `already`, `failed` or `skipped_dependency` |
| `patch.<id>.reason` | why it failed or was skipped (at most 512 characters) |
| `patch.<id>.mismatch`, `.path`, `.expected_sha256`, `.actual_sha256` | for a failed hash check: `source_mismatch` (a stock file a patch set rebuilds from is not stock, with its path), `result_mismatch` (a rebuilt file came out wrong) or `download` (the blob failed its manifest sha256) |
| `error`, `error.mismatch`, `error.path`, … | the same for a run that stopped early (a seed hash mismatch) |

The level is `info` for `ok` and `cancelled`, `warn` otherwise. Patch
ids become key segments with anything outside `[A-Za-z0-9_-]` replaced
by `_`.

### Telemetry DLL launch event

The retired egui launcher recorded a `client_native` event with target
`client.telemetry_dll.launch` saying whether the telemetry DLL went in.
The desktop launcher sends no such event: it records the outcome of the
last Play in `game-telemetry-status.json` in its state directory
(`attached`, `session_unavailable`, `endpoint_refused`,
`session_not_written`) and shows it in Settings. A session that never
went in has no session id, so there is nothing to tell apart in SigNoz;
an opted-in session with launcher rows but no `client.dll.attached` row
means the DLL was injected and did not attach. Each chunk the DLL uploads carries its process id in the `X-Cimmeria-Client-Pid` header (TG-PIPE-02), so a relaunch under the same session is distinguishable.

## Failure modes

| Scenario | Behavior |
|---|---|
| `/auth/dev-session` returns 503 (kill switch) | The DLL is left out and the game launches with the client patches only; the status is `session_unavailable`. |
| `/auth/dev-session` network failure, or the handshake takes over 8 s | Same as kill switch. |
| The server hands back an upload address the endpoint policy refuses | The DLL is left out; the status is `endpoint_refused`. |
| The build has no verified telemetry DLL, or a `lab-bridge` one | Opting in fails with `platform_unavailable` and saves nothing; a `lab-bridge` build is refused at staging, bundle resolution and Play admission. Play is unaffected. |
| DLL session outlives its 8 h token | The DLL cannot refresh a token itself. Past the session file's `expires_at_ms` by its own clock it holds uploads and sends only one probe at once and then one per 10 min, since its clock may run ahead of the server's; a probe the server accepts restores normal uploads. A 401 or 403 makes it re-read `current-session.json`: a different token there is used at once, otherwise it holds off with backoff (flush interval, doubling, capped at 10 min) and re-reads at each step. It writes one `client.telemetry.upload_paused` event when it stops and one `client.telemetry.upload_resumed` when a POST goes through again. Neither launcher rewrites the file when it refreshes its own token yet, so in practice the DLL's uploads end at its first token's expiry; the launcher's own uploads refresh and continue. Before 2026-10-12 the DLL retried a refused token every flush, indefinitely. |
| DLL chunk POST 5xx, other 4xx, or network error | The batch is kept; the next attempt waits on the same backoff, or the answer's `Retry-After` (whole seconds) when that is longer, capped at 10 min. While uploads are held the batch is kept under about twice its 1,000-event cap (oldest half dropped, counted as `upload_dropped_total`). |
| Chunk POST 401 | `ChunkError::TokenRejected` → next tick fires `refresh_if_due`. |
| Chunk POST 503 | `ChunkError::KillSwitch { retry_after_secs }` honored. Events stay on disk. |
| Launcher killed mid-session | Game keeps running (no Job Object). Events stay in `telemetry-queue.jsonl` in the launcher's state directory and go with the next session's first flush (`recover_pending_on_startup`). |
| Game crashes | Same as clean exit from the runner's perspective — `child.wait()` returns. Final flush + bundle upload still fire. |
| Disk full mid-enqueue | Enqueue surfaces the IO error; the event is lost on the stack. Telemetry is supplementary — never load-bearing. |

## Token format

```text
payload = base64url(JSON {iss, sub, sid, iat, exp, scope, kind?, acct?, mid?})
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

`mid` is the request's `machine_id`, which every launcher already sends:
a truncated SHA-256 of the Windows `MachineGuid` (of the host name on other
platforms and in the desktop launcher), so the raw `MachineGuid` never
leaves the machine. It is omitted when the request's `machine_id` is blank.
`acct` is the game account named in the mint request's optional
`account_name` field, absent when the request has no name or a blank one.
No launcher sends one today, because the launcher mints before the game
logs in. A name over 20 bytes (the game login's own cap) or with a control
character is a 400. Both are signed labels, not proof of identity, and a
refresh keeps both.

The mint and every refresh record the stream in
`cimmeria_auth::auth::client_telemetry` (TG-NET-18, owner decision D-TG14)
under its `mid`, its `acct` (ASCII case ignored) and the quota-key address the
request came from: behind a loopback proxy, the forwarded caller (D-LTL4).
The registry keeps that address in memory only and never logs or writes it.
The `World entry complete` row joins on the game
session's address:

| `client_telemetry` | When | Extra fields |
| --- | --- | --- |
| `present` | A live stream names the session's account, or exactly one machine has a live stream from the session's address | `client_telemetry_session_id` (the `session_id` on the stream's replayed rows), `client_machine_id` |
| `ambiguous` | Several machines have live streams from the address, such as a shared NAT | `client_telemetry_candidates` |
| `absent` | Streams have been recorded since boot, none matches | none |
| `unknown` | No stream has been recorded since boot | none |

Anyone can mint a token under any machine id or account name, so
`present` and `client_telemetry_session_id` are a launcher's claim, not
proof. The registry is in-process and holds at most 4096 streams per
index; a stream lapses at its token's `exp`, and a stale refresh never
replaces a newer stream for the same machine.

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
`verify_bearer` in `crates/admin-api/src/routes/telemetry/upload_gate.rs`
refuses a token without it. The scope is checked at ingest rather
than assumed from the mint path, so narrowing what a token may do
stays a one-line change on the server.

**Quotas.** Mint and refresh are counted per quota-key address (the socket
peer, or the forwarded caller behind a loopback proxy; see below), and mint
additionally per `install_id`, over a fixed window. Refresh verifies
the token before charging its counter, and counts tokens that fail
verification on a separate, tighter counter, so junk sent from an
address shared with real launchers cannot lock them out. Over quota
returns 429 with a `Retry-After` the launcher's back-off path already
honours. Defaults and env-var names are in
[telemetry.md](../operations/telemetry.md#mint-and-refresh-quotas).

**Upload limits.** Because a token costs nothing to mint, the upload
routes cannot trust a token holder either. Each upload passes the kill
switch, the token, a per-session and a per-address rate quota and a
concurrency slot (server-wide, and one per address) before its body is
read, and the body has a deadline. The compressed size is refused past
its cap (413). The expansion budgets, expanded bytes and rows for a
chunk and files, expanded bytes and lines for a bundle, truncate
instead: what fits is replayed and the answer is a 200 with
`truncated: true`. Launchers released before these limits post their
whole queue as one chunk and re-queue it on any error, and bundle every
past session's logs, so a refusal there would be retried forever. Past the per-session event
budget, priority rows have an allowance of their own rather than a
bypass. Client strings are length-capped before they reach a log row.
Values and refusal rows: [telemetry.md § Upload size and rate
limits](../operations/telemetry.md#upload-size-and-rate-limits).

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
  because a single host is routinely handed a whole /64. An
  IPv4-mapped IPv6 peer (`::ffff:a.b.c.d`, as seen on a dual-stack
  listener) is keyed as its IPv4 address.
- **Caller metadata is checked before it is logged.** `machine_id`,
  `branch`, `git_sha` and `launcher_version` reach the INFO mint line,
  and the plain `fmt` log sinks do not escape them, so a value with a
  control character (or longer than 256 bytes) is refused with 400.
  Both routes also cap the request body at 8 KiB, because the JSON
  body is parsed before any quota is charged.

**The quota key behind a proxy.** Behind a reverse proxy every request
arrives from the proxy, so a plain peer-address key degenerates to a global
cap. The server therefore picks the key with `rate_limit_ip`
(`routes/dev_session/forwarded_for.rs`): when the socket peer is loopback,
the key is the rightmost comma entry of the last `X-Forwarded-For` header, if
it parses as a bare IP; otherwise it is the peer. Headers from a non-loopback
peer are ignored, so a caller who reaches the port directly cannot choose its
quota key, and the rightmost entry is the one the proxy appended, not one the
caller wrote. The result is only an `ip_key` for the quota counters and the
per-address upload slot. It is never logged, persisted, returned or put in a
span (the in-memory client-telemetry registry holds it for the stream join,
D-LTL4), and it is not hashed into anything either. This is the C18 rule and
owner decisions D-LT19 and D-LT18; the statement of the no-address rule is
[telemetry.md § No address in any log](../operations/telemetry.md#no-address-in-any-log).

What stays shared: a NAT or office egress, a proxy that is not on loopback,
and a container that sees its proxy as the bridge gateway rather than as
loopback (whether the colo's does is OPS-LT2, unverified). Operators there
raise the per-address quotas; trusting the header past loopback is a
separate owner decision (D-LT20) that is not taken.

**Refusal rows.** A refused mint or refresh writes one throttled row per
`(route, reason)` per 60 seconds, with a `reason` from
`dev_session_quota_exceeded`, `dev_session_kill_switch`,
`dev_session_no_upload_endpoint`, `dev_session_secret_unusable`,
`dev_session_bad_token`, `dev_session_expired`,
`dev_session_lifetime_exceeded` and `dev_session_bad_request`, and a
`suppressed` count. A row carries no peer address and nothing derived from
one. The 503 status covers the kill switch and a missing upload endpoint,
and only those. Levels and filters: [telemetry-upload-endpoint.md](../operations/telemetry-upload-endpoint.md#dev-session-mint-and-refresh).

**Bounded refresh chain.** `refresh` preserves the original `iat` and
refuses once the session passes `CIMMERIA_TELEMETRY_MAX_SESSION_SECS`
(24 h by default), clamping the last token's `exp` to the deadline
rather than overshooting it. Without this, each refresh granted a
fresh full TTL and one leaked token could be walked forward forever.
At the cap the launcher sees a 401 on refresh and mints a fresh
session, the same path it already takes for an expired token. Every
refused refresh logs its `reason` (`dev_session_lifetime_exceeded` with
`elapsed` and `cap` at INFO, `dev_session_expired` with `exp` and `now`,
`dev_session_bad_token`, and `dev_session_kill_switch` at WARN), so a
stream that ends after a refresh has a row naming the refusal.

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

### Applied launcher content identity

The existing `client.launcher.install` channel also carries
`event=applied_content_identity`, `phase=applied|play_admitted`, the retained
signed installed `manifest_sha256`, fully applied `overlay_ids`, and separate
`incomplete_overlay_ids`. The launcher reads the live ledger with exact
`@sgw_game` keys. Catalog offers and staging do not emit this event; Play
admission does not prove the game loaded a particular overlay.
The launcher-row filter retains a 64-character lowercase hex
`manifest_sha256` only for that exact target, event and phase. Other long hex
identifiers still receive the generic privacy scrub. Existing row, field,
size and consent limits apply.
