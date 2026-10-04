# Launcher summary telemetry

> **Type:** Reference
> **Audience:** Engineers changing the summary mint, the ingest route, its rows or the desktop exporter, and the maintainer deciding whether to activate it
> **Last updated:** 2026-10-04
> **Companions:** [desktop-side contract](../../crates/launcher/desktop/docs/launcher-summaries.md), [telemetry operations](../operations/telemetry.md), [dashboard and saved view](../operations/signoz/launcher-summary-views.md), [dev-session telemetry](dev-session-telemetry.md), [target catalog](observability-target-catalog.md#launchersummary), [implementation assignment](../analysis/playtests/2026-10-03-macos-wine/worknotes/observability-implementation-assignment.md)

The desktop launcher can report, with the player's consent, how each install, runtime-setup, repair, uninstall or launch attempt ended. A report is one row per attempt: closed enums, a few bounded integers and two random ids. The server validates a batch of those rows, drops the ones it has already accepted, and writes one typed log row per accepted summary.

**Status: code only. Nothing here is deployed.** No distributed launcher build has a summary endpoint, so no launcher collects or sends a summary. The route exists in the server code on both listeners, and serving it publicly is an open maintainer decision (see [Public-activation gate](#public-activation-gate)). No part of this was run against a real collector or a real SigNoz; the proof is local fixtures and loopback tests.

## Purpose and non-goals

The rows answer three questions about the attempts the server received:

- How did attempts end, per launcher build and platform?
- Where do failed attempts fail, and with which closed error code?
- How long do the phases the launcher timed take?

They do not answer anything else, and the design keeps it that way:

- **Not an all-player funnel.** Only opted-in launchers with a configured endpoint report, and only the summaries that arrived are counted. [Known limits](#known-limits) lists what is invisible.
- **Not an install success rate.** The share of `succeeded` among received rows says nothing about the attempts that were not received.
- **Not a login or world-entry metric.** A launch is `succeeded` when the game process the launcher watched exited with code 0. The launcher does not know whether the player logged in.
- **Not a play-session timer.** A launch row carries the launcher's own preparation time and no total duration, because the total would be the length of the play session.
- **Not game telemetry.** It has its own consent, its own token scope, its own route and its own rows. It shares the mint endpoint and the HMAC secret with [dev-session telemetry](dev-session-telemetry.md) and nothing else.

## Trust

The summary mint needs no credential, like the player mint. So every summary is **self-reported and forgeable within the quotas**: anyone who can reach the mint can post rows that look like a launcher's.

Rows from this pipeline must never drive server state, an alert or an SLO. They are for counting what opted-in launchers say happened.

What bounds a forger is not authentication:

- Every value is a closed enum, a bounded integer or a parsed UUID, so a forger chooses among the same values a launcher can send and can put no text of their own in a row.
- The mint and the ingest are each limited per peer address ([Auth](#auth), [Ingest order](#ingest-order-and-validation)).
- A request is at most 64 KiB and 32 summaries, and the dedup set has a fixed size.
- A summary token has one scope, so it works on the summary route and nowhere else.

## Where the code lives

| Piece | Path |
|---|---|
| Summary arm of the mint | `crates/admin-api/src/routes/dev_session/summary_mint.rs`, entered from `mint_inner` in `dev_session/handlers.rs` |
| Scope and session-kind constants | `crates/admin-api/src/routes/dev_session/token.rs` |
| Ingest route | `crates/admin-api/src/routes/telemetry/launcher_summary/`: `mod.rs` (router, body cap), `handlers.rs` (order of checks), `dto.rs` (wire types, validation, error bodies), `dedup.rs`, `rows.rs` |
| Admin-listener mount | `api_routes` in `crates/admin-api/src/routes/mod.rs` |
| Public login-listener mount | `login_port_telemetry_router` in `crates/admin-api/src/login_port.rs` |
| Index routing | `CLIENT_TARGETS` in `crates/server/src/otel.rs` |
| Producer and exporter | `crates/launcher/desktop/engine/src/storage/launcher_summary/` ([desktop-side contract](../../crates/launcher/desktop/docs/launcher-summaries.md)) |
| Golden wire fixtures | `crates/launcher/desktop/engine/src/storage/launcher_summary/fixtures/` |
| SigNoz fixtures and their guard | `docs/operations/signoz/launcher-journey.dashboard.json`, `launcher-summary.view.json`, and `launcher_summary/fixture_tests/` |

## Wire contract (schema version 1)

### Endpoints

| Path | Method | Auth | Body cap |
|---|---|---|---|
| `/api/auth/dev-session` with `"session_kind": "launcher_summary"` | POST | none; limited per peer address | 8 KiB |
| `/api/telemetry/launcher-summary` | POST | `Authorization: Bearer <token>` with scope `launcher_summary.write` | 64 KiB (65,536 bytes) |

Both paths are the same on the admin listener and on the public login listener. The launcher holds a base URL and appends `auth/dev-session` and `telemetry/launcher-summary` to it (`endpoint.rs`), so a base of `https://<host>/api/` produces the two paths above.

### Mint request and response

The request is the common dev-session body, and for a summary session it is exactly [`mint-request.json`](../../crates/launcher/desktop/engine/src/storage/launcher_summary/fixtures/mint-request.json):

```json
{ "install_id": "00000000-0000-4000-8000-0000000000aa",
  "machine_id": "", "branch": "", "git_sha": "",
  "launcher_version": "0.1.0", "tags": [],
  "session_kind": "launcher_summary" }
```

| Field | Rule for a summary session |
|---|---|
| `session_kind` | Exactly `launcher_summary`. The summary arm is chosen by string equality before any quota is charged |
| `install_id` | Passes the common check (1 to 128 bytes of ASCII alphanumerics, `-` or `_`) and is then discarded. The exporter sends a fresh random UUID with every mint and never stores it |
| `machine_id`, `branch`, `git_sha` | Must be empty strings |
| `tags` | Must be empty |
| `launcher_version` | Exactly three dot-separated components of 1 to 3 ASCII digits |

The identifier fields must be empty so that a build which sends them is refused instead of looking accepted.

The response is the common `DevSessionResponse` (`session_id`, `token`, `expires_at_ms`, `upload_endpoint`, `chunk_max_bytes`, `flush_interval_ms`). The exporter reads `token` and ignores the rest, including `upload_endpoint`: its URLs come only from its own configuration.

| Mint status | When |
|---|---|
| 200 | Token issued |
| 400 | A non-empty identifier field or `tags`, a malformed `launcher_version`, a malformed `install_id`, or an unknown `session_kind` |
| 413 | Body over 8 KiB |
| 415, 422 | The `Json` extractor refused the body (no JSON content type, a missing field) before the handler ran. No quota is charged |
| 429 + `Retry-After` | Over the summary mint allowance |
| 500 | `CIMMERIA_TELEMETRY_HMAC_SECRET` is unset or shorter than 32 bytes |
| 503 + `Retry-After: 60` | Kill switch |

### Ingest request

```json
{ "schema_version": 1,
  "client_dropped": { "overflow": 0, "expired": 0, "rejected": 0 },
  "summaries": [ {
    "event_id": "uuid", "attempt_id": "uuid",
    "operation": "install", "phase": "download", "outcome": "failed",
    "error_code": "install_failed",
    "duration_ms": 81234, "retry_count": 0,
    "phases": [ { "phase": "starting", "duration_ms": 12 },
                { "phase": "download", "duration_ms": 81000 } ],
    "launcher_version": "0.1.0", "os": "windows", "arch": "x86_64" } ] }
```

The body is plain JSON, not compressed. Every object is closed: an unknown key at the top level or in `client_dropped` fails the request, and an unknown key in an element rejects that element.

Envelope rules. Breaking one is a 400 for the whole request:

| Field | Rule |
|---|---|
| `schema_version` | The integer 1 |
| `client_dropped` | An object with exactly `overflow`, `expired` and `rejected`, each an integer 0 to 65,535 |
| `summaries` | An array of 1 to 32 elements. The count is checked before any element is typed |

Element rules. Breaking one rejects that element alone:

| Field | Rule |
|---|---|
| `event_id`, `attempt_id` | Required strings that parse as a UUID in any spelling `Uuid::parse_str` reads, and are not nil. The server dedups on and emits the parsed value in lower-case hyphenated form, so two spellings of one id are one id |
| `operation`, `phase`, `outcome`, `os`, `arch` | Required; a member of the closed set below |
| `error_code` | Required when `outcome` is `failed`, forbidden otherwise |
| `duration_ms` | Optional integer 0 to 604,800,000 (seven days) |
| `retry_count` | Required integer 0 to 100 |
| `phases` | Optional array of at most 32 entries, each `{ "phase": <timed phase>, "duration_ms": 0 to 604,800,000 }`, with no phase listed twice |
| `launcher_version` | Required; three dot-separated components of 1 to 3 ASCII digits |

An optional key that is present must hold a value: an explicit `null` rejects the element. The server enforces no operation-by-phase matrix; the closed sets already keep arbitrary text out.

### Closed sets

These lists are exhaustive. The launcher's `request-all.json` fixture covers every value, and both sides test that it does.

| Field | Values |
|---|---|
| `operation` | `install`, `prepare_runtime`, `repair`, `uninstall`, `launch` |
| `phase` (where the attempt ended) | `none`, `platform_check`, `compatibility_check`, `catalog_fetch`, `manifest_verify`, `destination_check`, `admission`, `starting`, `running`, `download`, `extraction` |
| timed phases (the only values inside `phases[]`) | `starting`, `running`, `download`, `extraction` |
| `outcome` | `succeeded`, `failed`, `cancelled`, `unknown` |
| `error_code` | `unspecified`, `platform_unavailable`, `launcher_too_old`, `invalid_directory`, `manifest_unavailable`, `manifest_invalid`, `signing_key_unavailable`, `state_invalid`, `local_io`, `destination_unavailable`, `install_failed`, `content_invalid`, `rosetta_required`, `runtime_unavailable`, `prerequisite_failed`, `launch_not_started`, `launch_early_exit`, `launch_exit_nonzero` |
| `os` | `windows`, `macos`, `linux` |
| `arch` | `x86_64`, `aarch64` |
| result | `accepted`, `duplicate`, `rejected` |

### Ingest response and statuses

A request that passes the envelope checks gets `200 { "results": [ ... ] }`, one result per element of `summaries`, in the same order:

- `accepted`: valid, and the first sighting of that `event_id` by this server process. The row was handed to the log pipeline. It is not a storage acknowledgement.
- `duplicate`: valid, but that `event_id` was already accepted. Inside one request, a repeated id is `accepted` and then `duplicate`.
- `rejected`: the element broke a rule. The server does not say which, because the only honest description would quote the element.

| Status | When | What the exporter does |
|---|---|---|
| 200 | The envelope is valid | Removes every answered row from its queue |
| 400 | The body is not a JSON object, a top-level key is missing or unknown, `schema_version` is not 1, `client_dropped` is invalid, or `summaries` is not an array of 1 to 32 | Drops the batch for good and counts it |
| 401 | No `Authorization: Bearer` header, or a token that fails to decode, fails its signature, has expired or lacks the summary scope | Mints again and retries, within its retry budget |
| 413 | Body over 64 KiB | Drops the batch for good and counts it |
| 429 + `Retry-After` | Over the per-address ingest allowance | Waits, then retries |
| 500 | The HMAC secret is unusable | Retries |
| 503 + `Retry-After: 60` | Kill switch | Waits, then retries |

A token that does not decode is a 401 here, never a 400: the exporter treats a 400 from this route as "this batch can never be delivered" and deletes it. Every refusal body is static text. No response repeats anything the caller sent, and no serde error text leaves the server (`SummaryError` in `dto.rs`). The exporter's full handling is in the [desktop-side contract](../../crates/launcher/desktop/docs/launcher-summaries.md#exporter-cycle-and-outcomes).

## Auth

**Scope.** A summary token carries exactly one scope, `launcher_summary.write` (`SCOPE_LAUNCHER_SUMMARY_WRITE`). The summary route requires it, and the two upload routes require `telemetry.write`, so neither kind of token works on the other's routes. `verify_bearer_scoped` in `telemetry/handlers.rs` does the check for both.

**Session kind.** The token's `kind` claim is `launcher_summary` (`SESSION_KIND_LAUNCHER_SUMMARY`), and so is its `sub`. A player token's `sub` is the caller's `install_id`; a summary token's is a server constant, so the token carries nothing the caller chose and cannot tie a row to an installation. The `sid` is a server-minted UUID.

**Lifetime.** The token is an ordinary dev-session token: the common 8-hour TTL, and the refresh route extends it like any other, keeping its scope and kind. What separates it from a player token is its scope, not its lifetime. The exporter never refreshes. It mints once per delivery attempt and never stores the token.

**Mint quota.** Summary mints are counted per peer address on a table of their own (`Tables::mint_summary_ip`, reported as `mint/summary_ip`), so summary mints and player or lab mints cannot spend each other's allowance. The limit is `CIMMERIA_TELEMETRY_MINT_QUOTA_PER_IP`, over the shared window. Two differences from the player mint:

- The allowance is charged before any field is validated, so a malformed summary mint spends it.
- There is no per-`install_id` charge, because the exporter's `install_id` is random per mint.

**What the mint logs.** A successful summary mint writes one INFO row, `Minted launcher-summary token`, with `session_id`, `session_kind`, `launcher_version` (re-formatted from the parsed integers) and `exp`. It writes no `install_id` and no DEBUG identifiers row. A refused mint is logged like any other dev-session refusal (`log_refusal` in `dev_session/handlers.rs`): the over-quota WARN and the bad-request DEBUG row carry the peer address.

**Kill switch.** `CIMMERIA_TELEMETRY_KILL_SWITCH=1` makes the mint, the refresh and the summary route answer 503 with `Retry-After: 60`.

## Ingest order and validation

`ingest_inner` in `launcher_summary/handlers.rs` runs these steps in this order, and `tests/order.rs` pins the order:

1. **Kill switch.** 503.
2. **Per-address quota.** `CIMMERIA_TELEMETRY_SUMMARY_QUOTA_PER_IP` requests per peer address per window (default 120; `0` disables; the window is `CIMMERIA_TELEMETRY_QUOTA_WINDOW_SECS`). Over it, 429.
3. **Bearer token with the summary scope.** 401. The claims are then discarded: the token proves the caller went through the summary mint and nothing more.
4. **Envelope.** 400.
5. **Each element, alone.** A failing element becomes `rejected`; the others are unaffected.
6. **Dedup.** Every verdict of the request is decided under one lock acquisition.
7. **Rows.** One summary row and its phase rows per accepted summary, then one batch row.
8. **Response.**

The cheap refusals come first so that a caller without a token, or over its allowance, costs neither an HMAC nor a JSON parse. The cost of that order is in [Known limits](#known-limits): a request with no token still spends the address's allowance.

The 64 KiB cap is a `DefaultBodyLimit` on the route, so an oversized body is a 413 before the handler runs. The handler takes the body as bytes, not through `Json<T>`: the extractor would answer a malformed body with serde's error text before the kill switch, the quota or the token had been checked.

A refused request writes no row of this module's.

## Dedup

The launcher keeps a summary until the server answers for it, so a lost response means the same `event_id` arrives again. `dedup.rs` holds the last 16,384 accepted `event_id`s in memory, in arrival order:

- A valid element whose id is in the set is `duplicate`. Otherwise it is `accepted` and its id is remembered.
- When the set is full, the oldest id is forgotten. A summary is never refused for lack of room, and a caller minting ids in a loop costs a fixed amount of memory.
- A rejected element's id is never remembered.
- The whole request is judged under one lock, so two requests carrying the same id cannot both see it as new.
- The set lives for the process. A server restart forgets it, and a summary resent after one is accepted again. Delivery is therefore **at-least-once**.

16,384 ids is 256 launchers' full queues (a launcher holds at most 64 unsent summaries), in under 1 MiB.

## Emitted rows

`rows.rs` writes three kinds of row, all at INFO, all built from validated values only. The Rust `target` is the `scope_name` column in SigNoz.

| `event` | Target | SigNoz `service.name` | One row per | Fields |
|---|---|---|---|---|
| `launcher_summary` | `launcher.summary` | `cimmeria-client` | accepted summary | `event_id`, `attempt_id`, `operation`, `phase`, `outcome`, `error_code`, `duration_ms`, `duration_bucket`, `retry_count`, `launcher_version`, `os`, `arch`, `schema_version`, `cimmeria.session_kind` |
| `launcher_phase` | `launcher.summary` | `cimmeria-client` | entry of an accepted summary's `phases` | `attempt_id`, `operation`, `phase`, `duration_ms`, `duration_bucket`, `launcher_version`, `os`, `arch`, `schema_version`, `cimmeria.session_kind` |
| `launcher_summary_batch` | `launcher.ingest` | `cimmeria-server` | request that reached validation | `accepted`, `duplicate`, `rejected`, `client_dropped_overflow`, `client_dropped_expired`, `client_dropped_rejected` |

- **Index routing.** `launcher.summary` is in `CLIENT_TARGETS` (`crates/server/src/otel.rs`), so the summary and phase rows go to the `cimmeria-client` index: they are a player's machine's account of itself. `launcher.ingest` is the server's own account of a request and stays in `cimmeria-server`. Two tests name the targets literally and fail if the entry is removed: `launcher_summary_rows_land_in_the_client_index_and_batch_rows_in_the_server_index` (`crates/server/src/logging/parity_tests/guard.rs`) and `launcher_summary_is_a_client_target_and_its_batch_row_is_not` (`crates/server/src/logging/client_index_tests.rs`).
- **Separate event names.** A phase row has its own `event`, so a count of attempts (`event = 'launcher_summary'`) never counts a timed phase as an attempt. Join the two on `attempt_id`.
- **Absent means absent.** An optional value that is missing is a missing field, never a placeholder: `error_code` appears only on a failed row, and `duration_ms` and `duration_bucket` only when the launcher timed the attempt.
- **`duration_bucket`** is derived by the server so a dashboard can chart durations with `count()` alone: `lt_1s`, `lt_10s`, `lt_1m`, `lt_5m`, `lt_30m`, `ge_30m`. A duration on a bound falls in the higher band.
- **`cimmeria.session_kind`** is the constant `launcher_summary` on summary and phase rows.
- **`launcher_version`** is written from the parsed integers, and the ids from the parsed UUIDs.

What one row stands for (a launch row's single `starting` phase, an `unknown` row, `phase = none`, a pre-admission row and its `retry_count`) is decided by the launcher and described in the [desktop-side contract](../../crates/launcher/desktop/docs/launcher-summaries.md#what-a-row-says). The reading notes for operators are in [launcher-summary-views.md](../operations/signoz/launcher-summary-views.md#what-a-launcher_summary-row-stands-for).

## Privacy

A summary or batch row never contains:

- an account name, a character name or anything the player typed;
- an installation id, a machine id, or any id that persists across attempts. `event_id` is minted per row and `attempt_id` per attempt, both by the engine, and schema version 1 has no installation correlator;
- the journal's operation id, which the webview supplies and which is never exported;
- the token's session (`sid`) or subject (`sub`), or the peer address;
- a path, a URL, a file name or an error text;
- the length of a play session.

How the code keeps it that way:

- **On the launcher,** every exported value is a closed enum, a bounded integer or an engine-minted UUID (`schema.rs`), and error detail is mapped to a closed code from the launcher's own result records.
- **On the server,** no string the client sent survives validation. Enums are emitted through `as_str`, ids and the version are re-formatted from their parsed values, serde errors are discarded where they are produced, and the token's claims are dropped after the scope check.

The tests that pin this:

| Test | File |
|---|---|
| `a_marker_in_an_element_reaches_no_row_and_no_response`, `a_marker_in_the_envelope_reaches_no_row_and_no_response`, `a_marker_as_the_token_reaches_no_row_and_no_response`, with the control `the_capture_would_see_an_echo` | `launcher_summary/tests/no_echo.rs` |
| `no_row_carries_the_session_the_subject_or_the_peer` | `launcher_summary/tests/rows.rs` |
| `a_summary_mint_logs_no_install_id_and_no_caller_string` | `dev_session/summary_mint_tests.rs` |
| `a_query_string_reaches_no_span_no_event_and_no_response` | `crates/admin-api/src/login_port.rs` |
| `no_path_url_or_operation_id_reaches_the_queue_or_a_request_body` | `crates/launcher/desktop/engine/src/storage/install_worker/summary_tests.rs` |

These claims cover the rows this module writes and the login listener's request span. They do not cover everything a listener may log about a connection; see the request-span entries under [Known limits](#known-limits).

## Golden fixtures: the cross-workspace contract

The launcher's engine and the server live in different cargo workspaces, so neither can import the other's types. Five JSON files in `crates/launcher/desktop/engine/src/storage/launcher_summary/fixtures/` are the contract instead. Both sides test against the same files, so neither can drift alone. The server reaches them with `include_str!` relative to `CARGO_MANIFEST_DIR`.

| File | What it is | Engine side | Server side |
|---|---|---|---|
| `mint-request.json` | The exact mint body of a summary session | The engine's `MintRequest` serializes to it | The mint accepts it and issues only the summary scope |
| `request-all.json` | One batch covering every enum value and numeric bound, every element valid | Round-trips through the strict wire types; every enum variant occurs in it | Every element is `accepted` and emitted as sent; the fixture covers every value of every server enum |
| `request-mixed.json`, `response-mixed.json` | Three summaries whose results are `accepted`, `duplicate`, `rejected` | The exporter applies `response-mixed.json` | The real response to the request equals the response file |
| `request-install-failure.json` | The body the engine really sends for one failed install, recorded from a real install worker with injected ids and clock | A real install worker's failure, delivered by the exporter to a loopback mock, produces this body, with the test's own `os` and `arch` (`install_worker/export_tests.rs`) | The ingest accepts it and emits it as recorded |

The last row is the source-to-ingest proof: the engine test shows a real worker produces the file, and the server test shows the real ingest accepts it, with no hand-written body in between. No test runs the two halves in one process.

Compare the files as JSON values, never as bytes: a Windows checkout may convert line endings. A change to a fixture must pass the engine tests and the admin-api tests in the same PR. The other fixture tests are in `launcher_summary/tests/golden.rs` on each side.

## Known limits

- **Invisible attempts.** A launcher without consent, a build without an endpoint (every distributed build today), an attempt admitted while the gate was closed, a row evicted from a full queue, a row older than 24 hours, and a batch the server refused for good never produce a `launcher_summary` row. The batch row's `client_dropped_*` counters are the only trace of the last three.
- **At-least-once delivery.** The launcher resends until it gets an answer. The server drops a resend it remembers, so a duplicate row needs a server restart, or more than 16,384 accepted ids, between the two deliveries.
- **In-memory dedup.** The set is per process and lost on restart. Nothing persists it.
- **Approximate counters.** `client_dropped_*` and a pre-admission row's `retry_count` are at-least-once approximations: a request the launcher has to retry repeats the same counters, and the server does not deduplicate them. Do not sum them across batch rows as if each reported new drops.
- **The request span on the admin listener records the full URI.** The admin router (`build_router` in `crates/admin-api/src/lib.rs`) uses tower-http's default span, so a caller-chosen query string is a field of the span around every row of that request. The login listener's span (`request_span` in `login_port.rs`) records the path alone. The admin listener is private; this matters if it is ever exposed.
- **The request span still records the caller's method and path** on both listeners. A request whose path or method matches no route gets a 404 or 405 and writes no rows, so that text appears once per request, not once per row.
- **Tokenless requests are counted by the ingest quota.** The quota is charged before the token is checked. Anyone behind the same address as real launchers (a NAT, a tunnel) can use the allowance up without a token and turn those launchers' posts into 429s until the window ends. The refresh route avoids this by verifying first; doing the same here would change the pinned order.
- **Behind a proxy the per-address limits are global.** No forwarded-for header is read, as for the other dev-session quotas.
- **The summary token is not short-lived.** It has the common 8-hour TTL and can be refreshed up to the session cap. Its scope is the boundary.
- **A restart can misattribute one outcome.** If a process that never configured summaries (an older launcher, a tool) reconciled a tracked operation to a terminal state between two runs, the next run reports that terminal as observed (`tracker.rs`).
- **The SigNoz fixtures are unimported.** Whether SigNoz accepts the dashboard and view JSON is untested ([launcher-summary-views.md](../operations/signoz/launcher-summary-views.md#status-fixtures-only-not-validated-against-a-live-signoz)).

## Public-activation gate

Two things have to be decided by the maintainer, explicitly, before any summary leaves a player's machine. Neither is decided.

1. **The login-listener mount.** The recorded decision (@Cadacious, 2026-09-29) puts four telemetry routes on the public login port. `login_port_telemetry_router` now merges `/api/telemetry/launcher-summary` beside them as a fifth, which is outside that decision. The merge is its own commit so it can be accepted or dropped alone. A build carrying it must not be deployed until the maintainer says yes.
2. **A production endpoint.** The shell composes the exporter with `endpoint: None` (`crates/launcher/desktop/shell/src/host/summary.rs`), and there is no environment override. Shipping an endpoint is a rollout change with its own checklist: [what a rollout packet must change](../../crates/launcher/desktop/docs/launcher-summaries.md#what-a-rollout-packet-must-change).

Until both are decided, the route exists and no shipped launcher has an endpoint, so the launcher's consent copy, "This build sends nothing", stays true.
