# Launcher summary telemetry

> **Type:** Reference
> **Audience:** Engineers changing the ingest route, its rows or the desktop exporter, and the maintainer deciding whether to activate it
> **Last updated:** 2026-10-04
> **Companions:** [desktop-side contract](../../crates/launcher/desktop/docs/launcher-summaries.md), [telemetry operations](../operations/telemetry.md), [dashboard and saved view](../operations/signoz/launcher-summary-views.md), [dev-session telemetry](dev-session-telemetry.md), [target catalog](observability-target-catalog.md#launchersummary), [implementation assignment](../analysis/playtests/2026-10-03-macos-wine/worknotes/observability-implementation-assignment.md)

The desktop launcher can report, with the player's consent, how each install, runtime-setup, repair, uninstall or launch attempt ended. A report is one row per attempt: closed enums, a few bounded integers and two random ids. The server validates a batch of those rows, drops the ones it has already accepted, and writes one typed log row per accepted summary.

The upload is anonymous. There is no token, no mint step and no `Authorization` header: the launcher sends one `POST` and the server judges the body. Two things stand where a credential would: the route accepts only the exact schema-1 payload and refuses everything else, and each peer address gets a low number of requests per window (12 an hour by default).

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
- **Not game telemetry.** It has its own consent, its own route and its own rows, and it uses no dev-session token. What it shares with [dev-session telemetry](dev-session-telemetry.md) is the kill switch, the quota window setting and the quota-table code, and nothing else.

## Trust

The route is anonymous, so anyone can post correctly shaped rows within the rate limit. Rows are self-reported; they are useful for spotting failure patterns and must never drive server state, alerts, success-rate claims or SLOs. The strict schema means nothing but closed enum values, bounded integers, UUIDs and a version triple can ever be stored.

What bounds a stranger is not authentication:

- **The payload rule.** Only the schema-1 JSON envelope gets past the handler. A request that is anything else is refused whole, before a row is written ([Whole-request refusals](#whole-request-refusals)).
- **Closed values.** Every stored value is a closed enum, a bounded integer, a parsed UUID or a parsed version triple. A stranger chooses among the same values a launcher can send and can put no text of their own in a row.
- **The rate limit.** Each peer address gets 12 requests per window by default ([Anonymous access and the rate limit](#anonymous-access-and-the-rate-limit)). A request is at most 64 KiB and 32 summaries, so by default one address can add at most 384 summary rows per window.
- **Fixed memory.** The dedup set has a fixed size, and the quota table is the fixed-size one the dev-session mint uses.

None of this tells a real launcher from a script that sends the same bytes. That is the accepted cost of the design, and the reason for the rule above.

## Where the code lives

| Piece | Path |
|---|---|
| Ingest route | `crates/admin-api/src/routes/telemetry/launcher_summary/`: `mod.rs` (router, body cap), `handlers.rs` (order of checks, the body read, the default allowance), `envelope.rs` (the one pass over the body: envelope rules, repeated keys), `dto.rs` (wire types, the content-type rule, element validation, error bodies), `dedup.rs`, `rows.rs` |
| Kill switch and quota table (shared with the mint) | `kill_switch_active` and `env_u32` in `crates/admin-api/src/routes/dev_session/handlers.rs`, `WindowTable` in `dev_session/quota.rs` |
| Admin-listener mount | `api_routes` in `crates/admin-api/src/routes/mod.rs` |
| Public login-listener mount | `login_port_telemetry_router` in `crates/admin-api/src/login_port.rs` |
| Index routing | `CLIENT_TARGETS` in `crates/server/src/otel.rs` |
| Producer and exporter | `crates/launcher/desktop/engine/src/storage/launcher_summary/` ([desktop-side contract](../../crates/launcher/desktop/docs/launcher-summaries.md)) |
| Golden wire fixtures | `crates/launcher/desktop/engine/src/storage/launcher_summary/fixtures/` |
| SigNoz fixtures and their guard | `docs/operations/signoz/launcher-journey.dashboard.json`, `launcher-summary.view.json`, and `launcher_summary/fixture_tests/` |

## Wire contract (schema version 1)

### Endpoint

| Path | Method | Auth | Body cap |
|---|---|---|---|
| `/api/telemetry/launcher-summary` | POST | none; limited per peer address | 64 KiB (65,536 bytes) |

The path is the same on the admin listener and on the public login listener. The launcher holds a base URL and appends `telemetry/launcher-summary` to it (`endpoint.rs`), so a base of `https://<host>/api/` produces the path above. That is the only URL the exporter ever requests.

### Request headers

| Header | Rule |
|---|---|
| `Content-Type` | Exactly one header, `application/json` in any letter case. A `charset` parameter is allowed and its value is not read. Anything else is a 415: no header, two headers, another media type, a `+json` type, any other parameter, a trailing `;` |
| `Authorization` | Not used. The launcher sends none, and the handler never reads one, so a request with a token (a real dev-session token included) is treated exactly like one without |
| `Content-Encoding` | Not used. The route inflates nothing, so a gzip body is a 400 whatever this header says |

The exporter sends `Content-Type: application/json` and the batch, and nothing else of its own: no cookie and no identifier of the sender (`exchange.rs`).

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

The body is plain UTF-8 JSON, not compressed. Every object is closed: an unknown key at the top level or in `client_dropped` fails the request, and an unknown key in an element rejects that element.

No key may be written twice. `serde_json` alone would keep the later value and report nothing, while another reader of the same bytes might keep the first. A repeat at the top level or anywhere in `client_dropped` fails the request; a repeat anywhere inside an element (the element itself, or an entry of its `phases`) rejects that element. Keys are compared after JSON unescaping (`envelope.rs`, `tests/repeated_keys.rs`).

Each structure must be a JSON object. serde's derived structs would also read a positional array (`[0,0,0]` for `client_dropped`, twelve values in a row for a summary, `["starting", 12]` for a `phases` entry); `typed_object` in `dto.rs` refuses those before typing.

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

### Ingest response

A request that passes the envelope checks gets `200 { "results": [ ... ] }`, one result per element of `summaries`, in the same order:

- `accepted`: valid, and the first sighting of that `event_id` by this server process. The row was handed to the log pipeline. It is not a storage acknowledgement.
- `duplicate`: valid, but that `event_id` was already accepted. Inside one request, a repeated id is `accepted` and then `duplicate`.
- `rejected`: the element broke a rule. The server does not say which, because the only honest description would quote the element.

One invalid element never fails its neighbours. A game-telemetry event placed between two valid summaries is `rejected` in its own position, and the two beside it are `accepted` and written (`a_game_telemetry_event_as_an_element_is_rejected_alone` in `tests/rejected.rs`).

### Whole-request refusals

Anything that is not the payload is refused as a whole. No row is written and no id is remembered. The rows below are in the order the server answers them.

| Status | When | Body |
|---|---|---|
| 503 + `Retry-After: 60` | `CIMMERIA_TELEMETRY_KILL_SWITCH=1` | `Kill switch active — telemetry ingest is paused` |
| 429 + `Retry-After` | The peer address has used its allowance. `Retry-After` is the rest of the window plus one second | `summary/ip quota exceeded — retry in Ns` |
| 415 | `Content-Type` breaks the rule under [Request headers](#request-headers) | `Content-Type must be application/json` |
| 400 | The URI has a query string, an empty one (`?`) included | `Query string not allowed` |
| 413 | The body is over 64 KiB | `Body is over 64 KiB` |
| 400 | The body is not the envelope: see the list below | A fixed sentence, such as `Body is not a JSON object` or `Repeated key` |

The first four are answered before any of the body is read.

What ends as a 400, each sent as `application/json`:

- an empty body, whitespace, a form body, XML, or anything else that does not parse as one JSON value, such as the envelope followed by more text;
- JSON whose top level is not an object: an array (the valid envelope wrapped in `[ ]` included), a string, a number, `null`;
- JSON nested past the 128 levels `serde_json` reads (`deep_nesting_is_a_400` in `tests/envelope.rs`);
- a key written twice at the top level or inside `client_dropped`;
- a missing or unknown top-level key;
- a `schema_version` that is not the integer 1;
- an invalid `client_dropped`;
- `summaries` that is not an array of 1 to 32 elements. An array of 30,000 two-byte elements fits in 64 KiB; it is refused by its count, and only its first 33 elements are ever built;
- a gzip body, with or without `Content-Encoding: gzip`;
- an NDJSON body or a single game-telemetry event, which the chunk upload would accept (`the_game_telemetry_line_is_a_real_upload_event` proves the test's line is a real one).

`SummaryError` in `dto.rs` has exactly those five refusals (503, 429, 415, 413, 400), and the handler makes all of them: no extractor and no router layer answers for it. Every body is the server's text. The 400, 413, 415 and 503 bodies are fixed sentences, and the 429 body names the server's own counter and the seconds left. No response repeats anything the caller sent, and no serde error text leaves the server.

### What the exporter does with each status

| Status | What the exporter does |
|---|---|
| 200 with one known result per row | Removes every answered row from its queue |
| 400, 413, 415, 422 | The server will never take this body: drops the batch for good and counts its rows as rejected |
| 404, 405 | The server has no summary route: stops for the rest of the process run and keeps the rows |
| 429 | Ends the cycle at once, with no retry and no wait, and keeps the rows. The allowance is low, and a retry would only spend more of it |
| 503 | Waits `Retry-After`, capped at 60 s, then retries within its budget of two retries |
| Anything else, a timeout or a refused connection | Transient: retries within the same budget and never deletes |

The exporter makes one request per attempt, the `POST`. Its full handling is in the [desktop-side contract](../../crates/launcher/desktop/docs/launcher-summaries.md#exporter-cycle-and-outcomes).

## Anonymous access and the rate limit

**No token.** The route takes no credential of any kind. `ingest_inner` receives the whole request and reads one header from it, `Content-Type`. `tests/anonymous.rs` pins it: the golden requests are accepted with no `Authorization` header and with no HMAC secret configured, and a header that is sent anyway (garbage, or a real player or lab token) changes no verdict, no refusal and no row, and is no way past the quota or the kill switch.

**No summary session kind.** The dev-session mint has no part in this flow. A mint request with `"session_kind": "launcher_summary"` is refused like any other unknown kind, with the same 400 and the same body (`launcher_summary_is_refused_as_an_unknown_session_kind` in `dev_session/session_kind_tests.rs`). The value `launcher_summary` survives in one place only: as the `cimmeria.session_kind` label on the rows ([Emitted rows](#emitted-rows)), where it tells these rows from the `player` and `lab` rows in the same index. It names no session.

**The rate limit.** `CIMMERIA_TELEMETRY_SUMMARY_QUOTA_PER_IP` requests per peer address per window. The default is 12 (`DEFAULT_SUMMARY_PER_IP` in `handlers.rs`), `0` disables the limit, and a value that does not parse falls back to the default. The window is `CIMMERIA_TELEMETRY_QUOTA_WINDOW_SECS` (default 3,600 s), shared with the mint quotas, and starts at the address's first counted request.

- **Every request shape counts,** the refused ones too. The allowance is charged before anything the caller sent is looked at and before the body is read, so a wrong content type, a query string, a malformed body or a body over 64 KiB spends it like an accepted request (`malformed_requests_spend_the_allowance` and `oversized_requests_spend_the_allowance` in `tests/quota.rs`).
- **One refusal does not count.** A request under the kill switch is answered before the quota is charged.
- **An IPv4-mapped address is counted as its IPv4 address.** On a dual-stack listener an IPv4 peer arrives as `::ffff:a.b.c.d`. The quota table folds IPv6 to its /64, and every mapped address is in the same one, so `peer_key` in `handlers.rs` takes the canonical form first: the mapped and the plain form of one address share an allowance, and two different mapped addresses do not (`an_ipv4_mapped_peer_is_counted_as_its_ipv4_address`).
- **Why 12.** A launcher sends one request per export cycle, and a cycle runs when the launcher starts, when a tracked attempt ends and when a failure before admission is queued. That is a few requests an hour, so a low limit leaves a single launcher room, and it is the only thing between the route and anyone who can reach the port.
- **The allowance belongs to the address, not to a machine.** Everyone behind one NAT, reverse proxy or tunnel shares it, and no forwarded-for header is read. Behind a shared address the default is too low for more than a few launchers, and an operator there has to raise it. See [Known limits](#known-limits).

**Kill switch.** `CIMMERIA_TELEMETRY_KILL_SWITCH=1` makes the route answer 503 with `Retry-After: 60`, as it does for the dev-session mint and refresh.

## Ingest order and validation

`ingest_inner` in `launcher_summary/handlers.rs` runs these steps in this order, and `tests/order.rs` pins the order:

1. **Kill switch.** 503.
2. **Per-address quota.** 429.
3. **Content type.** 415.
4. **Query string.** 400.
5. **Body read, up to 64 KiB.** 413 past it.
6. **Envelope.** 400.
7. **Each element, alone.** A failing element becomes `rejected`; the others are unaffected.
8. **Dedup.** Every verdict of the request is decided under one lock acquisition.
9. **Rows.** One summary row and its phase rows per accepted summary, then one batch row.
10. **Response.** 200.

The quota comes before everything that reads the request, so a caller over its allowance costs no body buffering and no JSON parse. The cost of that order is in [Known limits](#known-limits): junk requests spend the address's allowance.

The handler takes the raw request, body unread, and reads the body itself at step 5 (`read_body` in `handlers.rs`). A `Bytes` or `Json<T>` extractor, or a `DefaultBodyLimit` layer in front of one, would buffer the body and answer an oversized or malformed one before the kill switch or the quota had been checked. So a request over 64 KiB is charged like any other, and a caller who is paused or over quota is answered with nothing buffered: `a_paused_ingest_answers_before_the_body_arrives` in `tests/routes.rs` sends the headers of a 64 KiB request and no body, and gets its 503. The cap is exact, 65,536 bytes are read and 65,537 are a 413 (`tests/body.rs`), and there is no shortcut on `Content-Length`, so a length-prefixed body and a chunked one take the same path.

The query string is refused because of where it would be logged. No code reads it, but the admin listener's request span records the whole URI and every row sits inside that span, so the request is refused before the first row ([Known limits](#known-limits)).

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
- the peer address, or anything from a header the caller sent;
- a path, a URL, a file name or an error text;
- the length of a play session.

How the code keeps it that way:

- **On the launcher,** every exported value is a closed enum, a bounded integer or an engine-minted UUID (`schema.rs`), and error detail is mapped to a closed code from the launcher's own result records. The request carries no token, cookie or sender identifier, and nothing a server answers is kept or sent back.
- **On the server,** no string the client sent survives validation. Enums are emitted through `as_str`, ids and the version are re-formatted from their parsed values, and serde errors are discarded where they are produced. An `Authorization` header is never read, so nothing from one can be logged.

The tests that pin this:

| Test | File |
|---|---|
| `a_marker_in_an_element_reaches_no_row_and_no_response`, `a_marker_in_the_envelope_reaches_no_row_and_no_response`, `a_marker_as_the_authorization_header_reaches_no_row_and_no_response`, `a_marker_as_the_content_type_reaches_no_row_and_no_response`, with the control `the_capture_would_see_an_echo` | `launcher_summary/tests/no_echo.rs` |
| `no_row_carries_the_peer_or_a_token_the_caller_sent` | `launcher_summary/tests/rows.rs` |
| `an_authorization_header_changes_no_verdict_and_reaches_no_row`, `an_authorization_header_changes_no_refusal` | `launcher_summary/tests/anonymous.rs` |
| `every_request_is_anonymous_and_nothing_a_server_issues_is_kept` | `crates/launcher/desktop/engine/src/storage/launcher_summary/tests/exporter/delivery.rs` |
| `a_query_string_reaches_no_span_no_event_and_no_response` | `crates/admin-api/src/login_port.rs` |
| `a_query_string_is_a_400_and_writes_no_row` | `launcher_summary/tests/order.rs` |
| `the_admin_router_refuses_a_query_string_before_any_row` | `launcher_summary/tests/routes.rs` |
| `no_path_url_or_operation_id_reaches_the_queue_or_a_request_body` | `crates/launcher/desktop/engine/src/storage/install_worker/summary_tests.rs` |

These claims cover the rows this module writes and the login listener's request span. They do not cover everything a listener may log about a connection; see the request-span entries under [Known limits](#known-limits).

## Golden fixtures: the cross-workspace contract

The launcher's engine and the server live in different cargo workspaces, so neither can import the other's types. Four JSON files in `crates/launcher/desktop/engine/src/storage/launcher_summary/fixtures/` are the contract instead. Both sides test against the same files, so neither can drift alone. The server reaches them with `include_str!` relative to `CARGO_MANIFEST_DIR`.

| File | What it is | Engine side | Server side |
|---|---|---|---|
| `request-all.json` | One batch covering every enum value and numeric bound, every element valid | Round-trips through the strict wire types; every enum variant occurs in it | Every element is `accepted` and emitted as sent; the fixture covers every value of every server enum |
| `request-mixed.json`, `response-mixed.json` | Three summaries whose results are `accepted`, `duplicate`, `rejected` | The exporter applies `response-mixed.json` | The real response to the request equals the response file |
| `request-install-failure.json` | The body the engine really sends for one failed install, recorded from a real install worker with injected ids and clock | A real install worker's failure, delivered by the exporter to a loopback mock, produces this body, with the test's own `os` and `arch` (`install_worker/export_tests.rs`) | The ingest accepts it and emits it as recorded |

The last row is the source-to-ingest proof: the engine test shows a real worker produces the file, and the server test shows the real ingest accepts it, with no hand-written body in between. No test runs the two halves in one process.

Compare the files as JSON values, never as bytes: a Windows checkout may convert line endings. A change to a fixture must pass the engine tests and the admin-api tests in the same PR. The other fixture tests are in `launcher_summary/tests/golden.rs` on each side. There is no mint fixture, because there is no mint step.

## Known limits

- **Invisible attempts.** A launcher without consent, a build without an endpoint (every distributed build today), an attempt admitted while the gate was closed, a row evicted from a full queue, a row older than 24 hours, and a batch the server refused for good never produce a `launcher_summary` row. The batch row's `client_dropped_*` counters are the only trace of the last three.
- **At-least-once delivery.** The launcher resends until it gets an answer. The server drops a resend it remembers, so a duplicate row needs a server restart, or more than 16,384 accepted ids, between the two deliveries.
- **In-memory dedup.** The set is per process and lost on restart. Nothing persists it.
- **Approximate counters.** `client_dropped_*` and a pre-admission row's `retry_count` are at-least-once approximations: a request the launcher has to retry repeats the same counters, and the server does not deduplicate them. Do not sum them across batch rows as if each reported new drops.
- **The request span on the admin listener records the full URI.** The admin router (`build_router` in `crates/admin-api/src/lib.rs`) uses tower-http's default span, and that is unchanged. The handler refuses a query string before it writes a row, so a query string is still a field of that request's span, beside the 400, but never of a span that holds a row of this module. The query is the only part of the request target the handler checks: an absolute-form target (`POST http://host/api/telemetry/launcher-summary HTTP/1.1`) is served like the bare path, and on the admin listener its host is then in the span's `uri` field beside the rows. The login listener's span (`request_span` in `login_port.rs`) records the path alone.
- **The request span still records the caller's method and path** on both listeners. A request whose path or method matches no route gets a 404 or 405 and writes no rows, so that text appears once per request, not once per row.
- **Forged rows cannot be told from real ones.** The route is anonymous, and a script that sends the launcher's bytes is a launcher as far as the server can see. The rate limit bounds how many rows one address adds, not how many addresses add them.
- **Junk spends a shared allowance.** Every request is counted, the refused and the oversized ones too. Anyone behind the same address as real launchers (a NAT, a tunnel) can use the allowance up with 12 requests of any content and turn those launchers' posts into 429s until the window ends.
- **A shared address shares one allowance of 12.** No forwarded-for header is read, as for the dev-session quotas. Behind a reverse proxy or a tunnel every launcher arrives from one address, so the default limits the whole deployment to 12 requests per window until the operator raises `CIMMERIA_TELEMETRY_SUMMARY_QUOTA_PER_IP`.
- **A rate-limited launcher waits, and a row can age out while it does.** A 429 ends the exporter's cycle with the rows still queued, and nothing retries until the next trigger (the launcher starting, or another attempt ending). A row that is still queued after 24 hours expires on the launcher and is only counted in `client_dropped.expired`.
- **A retried request spends allowance again.** A cycle makes up to three POSTs when the server answers with a transient failure, and each one that reaches the handler is counted.
- **An admitted request is buffered up to the cap.** A request inside the allowance with the right content type and no query string has up to 64 KiB of its body read into memory before it is parsed, and an oversized one is read up to the cap before its 413. The quota bounds how often one address can do that.
- **A restart can misattribute one outcome.** If a process that never configured summaries (an older launcher, a tool) reconciled a tracked operation to a terminal state between two runs, the next run reports that terminal as observed (`tracker.rs`).
- **The SigNoz fixtures are unimported.** Whether SigNoz accepts the dashboard and view JSON is untested ([launcher-summary-views.md](../operations/signoz/launcher-summary-views.md#status-fixtures-only-not-validated-against-a-live-signoz)).

## Public-activation gate

**Decided (owner, 2026-10-04):** the upload is anonymous, accepts only the strictly structured schema-1 payload, and has a low per-address rate limit. That settles how the route is protected, and it is what this page describes. It does not activate anything.

Two things still have to be decided by the maintainer, explicitly, before any summary leaves a player's machine. Neither is decided.

1. **The login-listener mount.** The recorded decision (@Cadacious, 2026-09-29) puts four telemetry routes on the public login port. `login_port_telemetry_router` now merges `/api/telemetry/launcher-summary` beside them as a fifth, which is outside that decision. The merge is its own commit so it can be accepted or dropped alone. What it would expose is the anonymous route above: on that port nothing stands between the internet and the handler but the payload rule and the rate limit. A build carrying it must not be deployed until the maintainer says yes.
2. **A production endpoint.** The shell composes the exporter with `endpoint: None` (`crates/launcher/desktop/shell/src/host/summary.rs`), and there is no environment override. Shipping an endpoint is a rollout change with its own checklist: [what a rollout packet must change](../../crates/launcher/desktop/docs/launcher-summaries.md#what-a-rollout-packet-must-change).

Until both are decided, the route exists and no shipped launcher has an endpoint, so the launcher's consent copy, "This build sends nothing", stays true.
