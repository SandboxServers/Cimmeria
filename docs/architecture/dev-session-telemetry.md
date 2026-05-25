# Dev-Session Telemetry — Architecture

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
- v1 trusts any caller of `/api/auth/dev-session`. Tokens are scoped
  to `telemetry.write` and single-session — the worst an attacker can
  do is upload garbage telemetry. Account-bound auth is a v2 concern.

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
| `crates/launcher/src/telemetry/process_watch.rs` | `spawn_blocking child.wait()` — game-exit signal without burning an async worker. |
| `crates/launcher/src/telemetry/runner.rs` | Per-session loop: tail → enqueue → flush → on-exit bundle. |
| `crates/launcher/src/telemetry/mod.rs` | `Telemetry` orchestrator (`start_session` / `enqueue` / `flush` / `refresh_if_due` / `upload_bundle`). |
| `crates/admin-api/src/routes/dev_session.rs` | Server-side `/api/auth/dev-session` + `/refresh` endpoints (mint + verify). |
| `crates/admin-api/src/routes/telemetry.rs` | Server-side `/api/telemetry/upload-{chunk,bundle}` ingest. Validates the HMAC token, decompresses gzip(NDJSON) or unzips bundle, replays each event through `tracing::*` so the OTLP layer ships it to SigNoz. |

## Session lifecycle

```
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

## Failure modes

| Scenario | Behavior |
|---|---|
| `/auth/dev-session` returns 503 (kill switch) | Launcher logs warn, falls back to `launch_atera_debug` (no telemetry). Game launches. |
| `/auth/dev-session` network failure | Same as kill switch — game launches without telemetry. |
| Chunk POST 401 | `ChunkError::TokenRejected` → next tick fires `refresh_if_due`. |
| Chunk POST 503 | `ChunkError::KillSwitch { retry_after_secs }` honored. Events stay on disk. |
| Launcher killed mid-session | Game keeps running (no Job Object). Events stay in `telemetry-queue.jsonl`. Next launch drains them via `recover_pending_on_startup`. |
| Game crashes | Same as clean exit from the runner's perspective — `child.wait()` returns. Final flush + bundle upload still fire. |
| Disk full mid-enqueue | Enqueue surfaces the IO error; the event is lost on the stack. Telemetry is supplementary — never load-bearing. |

## Token format

```text
payload = base64url(JSON {iss, sub, sid, iat, exp, scope})
sig     = base64url(HMAC-SHA256(secret, payload))
token   = payload || "." || sig
```

`iss` = `"cimmeria-server"`. `sub` = install_id. `sid` = session_id.
`exp` − `iat` = 8 hours. `scope` = `["telemetry.write"]`.

8-byte URL-safe base64-no-pad on both segments. Constant-time
signature comparison via `Hmac::verify_slice` defends against timing
oracles.

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
