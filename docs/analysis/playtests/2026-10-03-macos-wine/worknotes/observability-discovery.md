# Launcher observability discovery handoff (track O)

> **Type:** Reference (discovery handoff; nothing here is implemented)
> **Audience:** The Codex integration owner and the fresh track O implementation session
> **Companions:** [Requirements](../launcher-implementation-plan.md), [delegation plan](../launcher-delegation-plan.md)
> **Last updated:** 2026-10-04
> **Evidence:** Read-only source audit at `0b10d869c869793ab506dbf9215ddb91714a244b`. No builds, edits, application control or live probes.

The consent preference already exists in the Tauri desktop workspace, but
nothing consumes it. The server has no launcher-only scope, no deduplication
and no per-event acknowledgement. This document proposes the contract that
closes those gaps and the prompt for the session that implements it.

**Method.** Five read-only readers mapped the code. A draft contract was then
attacked by six reviewers: the `network-security-auth`,
`server-authority-enforcer` and `testing-validation-engineer` advisors, two
fact-checkers and a requirements critic. The contract below is the revised
version.

**Not reviewed.** The journal-observer hook in section 2 was adopted after that
review. It was checked against `operations.rs` and `storage/mod.rs`, but no
reviewer saw it as the primary design.

Path prefixes: `D/` = `crates/launcher/desktop/`, `E/` = `D/engine/src/`,
`S/` = `D/shell/src/`, `F/` = `D/frontend/`, `A/` = `crates/admin-api/src/routes/`.

## Drift since the analysis base

The integration branch moved from `0b10d869c` to
`5900500846d1d890a96564251494311fdb9ce708` (21 commits) while this discovery
ran. Every citation below is from `0b10d869c`. A diff of the two commits shows:

| Area | State at `590050084` | Effect on this handoff |
|---|---|---|
| `crates/admin-api`, `crates/server` | Unchanged | Sections 3 and the server tests stand as written |
| `E/operations.rs`, `E/storage/atomic.rs`, `E/commands.rs` | Unchanged | The journal-observer design and the queue helpers stand |
| Launch | Now implemented: `E/storage/launch/`, `S/host/launch/`; admission goes through `Operations::begin` (`E/storage/launch/mod.rs:150`) | "No desktop Launch implementation" is stale. The journal observer covers launch with no extra hook. The launch phases and error codes marked reserved now have a producer; map them from the real launch result types |
| Repair | Shell host now exists: `S/host/repair/` | "No Repair IPC" is stale |
| `E/storage/mod.rs` | `open` now runs `recover_legacy_import()`; `state_root()` is no longer `cfg`-gated; new `launch` and `migration` modules | Line numbers shifted; the queue load in `open` must sit beside the new recovery call |
| `S/host.rs`, `S/main.rs`, `S/host/install/{mod,contract}.rs` | Edited for Play and Repair | Line numbers shifted; the hook points still exist |
| `D/engine/Cargo.toml` | Gained a `test-support` feature | No dependency the contract relies on changed |
| Consent copy | "This build sends nothing" still present in `F/ui/index.html` and `F/src/view.ts:103` | Decision 2 stands |

The implementation session must revalidate the desktop-side citations against
its own base before editing.

## Decisions the coordinator must record first

1. **Public route.** The new ingest route would be served on the public login
   listener (8081) as soon as the server is deployed.
   `crates/admin-api/src/login_port.rs:6-13` records a decision covering "only
   these four routes", so this needs the maintainer's explicit yes.
2. **No production endpoint in this packet.** The exporter endpoint is compiled
   in as `None`, so nothing is collected or sent and the existing copy "This
   build sends nothing" stays true. A real endpoint must be `https` with a
   publicly trusted certificate, because the engine's reqwest uses bundled
   webpki roots, not the OS store.
3. **No installation correlator in schema v1.** Operators can count attempts
   and outcomes, not distinct installs.
4. **Engine hook location.** The single hook lives in `E/storage/mod.rs`, which
   the delegation plan reserves for the coordinator.

## 1. Reusable code and verified gaps

| Reusable | Where | How |
|---|---|---|
| Consent preference, default off, revisioned atomic save, UI checkbox, restart UAT | `E/storage/mod.rs:54-70,182-237`, `E/commands.rs:8`, `F/src/workflows.ts:54`, `F/uat.mjs:53-79` | As-is; it is the opt-in |
| Every admission and state change passes one commit | `E/operations.rs:133-169,258-273` → `FileJournal::commit` at `E/storage/mod.rs:87-96` | Single hook point |
| Atomic bounded state files (64 KiB) | `E/storage/atomic.rs`, `read` at `E/storage/mod.rs:266-287` | `pub(super)`, so the new module must be a child of `storage` |
| Closed result enums | `install_worker::Outcome` (`E/storage/install_worker/mod.rs:17`), `JobError` (`S/host/install/contract.rs:77`), `CatalogError` (`E/catalog/mod.rs:14`) | Allowlist sources |
| Bounded response read | `E/catalog/mod.rs:76-100` | Copy the idiom |
| reqwest (rustls), uuid v4, `CancellationToken`, wiremock dev-dependency | `D/engine/Cargo.toml` | No manifest change needed |
| HMAC token, scope check, quota tables, 429/503 with `Retry-After` | `A/dev_session/{token,quota,handlers}.rs` | Add a scope and a kind |
| Injected-state handler idiom | `mint_inner` at `A/dev_session/handlers.rs:198-205` | Model for the new ingest |
| Log-capture test layer | `A/telemetry/replay_tests.rs:57-63` | Copy; it is private and synchronous |
| SigNoz builder fixtures, all `count()` | `docs/operations/signoz/black-market.view.json`, `npc-ai-health.dashboard.json` | Format template |

Not reusable:

- **`crates/launcher/src/telemetry/*`**: the crate is bin-only and in another
  workspace. Its queue deletes before upload (`queue.rs:85-110`) with a 100 MiB
  cap and no event ids. Its install result is an open field bag carrying error
  text and paths.
- **The existing `TelemetryEvent` stream**: reusing it would put summaries
  behind the game `telemetry.write` scope, and an unknown variant rejects the
  whole chunk (`A/telemetry/replay.rs:84`).

Verified gaps:

- **Desktop:** no summary schema, queue or exporter exists. `Operation` has no
  timestamps (`E/operations.rs:39`). There is no clock abstraction, and tokio
  `test-util` is off.
- **Failure before game:** the egui launcher's install result uploads only in a
  later game session (`crates/launcher/src/telemetry/install_result.rs:10-15`).
- **Operation id:** `Operation.id` is minted in the webview
  (`F/src/install-view.ts:17`), so it must not be exported.
- **Install failure detail:** every `InstallError` collapses to
  `Outcome::InstallFailed` (`E/storage/install_worker/mod.rs:279-295`). No
  retry or byte totals exist at the terminal.
- **Pre-admission failures** never reach the journal (`S/main.rs:68-108`), and
  `NativeHost::store`/`with_state` are private (`S/host.rs:67-99`).
- **Server auth:** one scope only (`A/dev_session/token.rs:29`,
  `handlers.rs:247`). The mint charges the shared per-IP bucket before it reads
  the kind (`handlers.rs:214-230`).
- **Server ingest:** no dedup (`A/telemetry/mod.rs:13`) and counts-only
  responses (`A/telemetry/dto.rs:78`). `kill_switch_active` is `pub(super)`
  (`handlers.rs:404`) and is not checked on upload routes.
- **Tests and fixtures:** admin-api has no dev-dependencies. No test reads the
  SigNoz fixtures. The two `CLIENT_TARGETS` tests loop over the list, so they
  guard nothing.

## 2. Summary schema and producer/consent interfaces

**Wire v1:** `POST {base}/telemetry/launcher-summary`, bearer token, JSON, body
at most 32 KiB, 1 to 32 summaries.

```json
{ "schema_version": 1,
  "client_dropped": { "overflow": 0, "expired": 0, "rejected": 0 },
  "summaries": [ {
    "event_id": "uuid-v4",      "attempt_id": "uuid-v4",
    "operation": "install",     "phase": "content_verify",
    "outcome": "failed",        "error_code": "content_invalid",
    "duration_ms": 81234,       "retry_count": 0,
    "launcher_version": "0.1.0", "os": "windows", "arch": "x86_64"
  } ] }
```

- **Ids:** both are minted in the engine and never come from the webview.
  `event_id` is the dedup key. `attempt_id` is created when the attempt is
  admitted. `Operation.id` stays a local lookup key.
- **Closed enums:**
  - `operation`: `install | prepare_runtime | repair | uninstall | launch`
  - `outcome`: `succeeded | failed | cancelled | unknown`
  - `phase`: `none | platform_check | catalog_fetch | manifest_verify | destination_check | runtime_setup | prerequisite_setup | download | extraction | content_verify`
  - `os`: `windows | macos | linux`; `arch`: `x86_64 | aarch64`
- **Launch values:** phases `launch_prepare | helper_start | inject | process_observe`,
  the `launch_*` error codes, and `outcome = unknown` (lost launch observation
  only). They were reserved with no producer at the analysis base; see the
  drift table. For `operation = launch`, `succeeded` means the game process
  started and was observed, never login.
- **Bounds:** `error_code` is present only when `outcome = failed`.
  `duration_ms` is optional (absent after a restart), 0 to 604,800,000.
  `retry_count` is 0 to 100. Each `client_dropped` counter is 0 to 65,535 and
  is a delta since the last 200. `launcher_version` is three integers 0 to 999,
  from the Tauri product version.
- **Rows:** terminal rows only, one per attempt.
- **Cut from v1:** phase-boundary rows, `age_s`, byte and item totals, and the
  manifest release id. Row time is therefore receipt time, up to 24 hours late.

Proposed mapping; the implementer verifies each variant name against code:

| Source | phase | error_code |
|---|---|---|
| Journal terminal with no detail | `none` | `unspecified` (if failed) |
| `Outcome::DestinationUnavailable` | `destination_check` | `destination_unavailable` |
| `Outcome::InstallFailed` | last progress seen: `download`, `extraction` or `none` | `install_failed` |
| `Outcome::ContentInvalid` | `content_verify` | `content_invalid` |
| `Outcome::RosettaRequired` / `RuntimeUnavailable` | `runtime_setup` | `rosetta_required` / `runtime_unavailable` |
| Prerequisite `Outcome::Failed` | `prerequisite_setup` | `prerequisite_failed` |
| `JobError::PlatformUnavailable` | `platform_check` | `platform_unavailable` |
| `JobError::ManifestUnavailable` | `catalog_fetch` | `manifest_unavailable` |
| `JobError::InvalidManifest` / `SigningKeyUnavailable` | `manifest_verify` | `manifest_invalid` / `signing_key_unavailable` |
| `JobError::InvalidDirectory` | `destination_check` | `invalid_directory` |

**Response 200:** `{ "results": ["accepted" | "duplicate" | "rejected", …] }`,
positional. `accepted` means validated, first sighting in this server process,
and handed to the log pipeline; it is not a storage acknowledgement.

| Step | Status | Client action |
|---|---|---|
| Mint | 400, 404, 405, 415, 422 | Server lacks the summary kind: stop for this process run, keep entries |
| Ingest | 200 with a valid `results` array | Remove accepted, duplicate and rejected entries; zero the drop counters |
| Ingest | 200 with wrong length or unknown value, or oversized response | Transient; remove nothing |
| Ingest | 400, 413 | Permanent: remove the batch, count rejected |
| Ingest | 404, 405 | Stop for this process run, keep entries |
| Either | 429, 503 | Wait min(`Retry-After`, 60 s), within the retry budget |
| Either | Anything else, including 401, 3xx, timeout | Transient within the retry budget; never delete |

**Producer (Rust, new module `E/storage/launcher_summary/`):**

```rust
pub struct SummaryConfig { pub launcher_version: (u16, u16, u16), pub endpoint: Option<SummaryEndpoint> }
impl DesktopState {
    pub fn configure_summaries(&mut self, config: SummaryConfig);
    pub fn summary_detail(&mut self, operation_id: Uuid, phase: SummaryPhase, code: Option<SummaryErrorCode>);
    pub fn summary_pre_admission_failure(&mut self, operation: SummaryOperation, phase: SummaryPhase, code: SummaryErrorCode);
}
```

- **Automatic begin and terminal.** An observer on `FileJournal::commit` starts
  tracking when a new operation is committed and emits the terminal row when a
  terminal state is committed. Outcome and operation kind come from the
  committed snapshot, never from a caller.
- **Detail is optional.** `summary_detail` is called before the terminal commit
  by sites that know more, for example `install_worker::publish`.
- **Types.** Arguments are closed enums, integers and UUIDs only, with a `Copy`
  bound, so no string or path can reach a summary.
- **Failure isolation.** Every summary call returns `()`, runs inside
  `catch_unwind`, and never sets `preferences_uncertain`. Any queue read error
  means "empty queue" and never fails `DesktopState::open`.
- **Gate.** `consent && !export_blocked && !requires_reopen()`, checked at
  creation, at enqueue and immediately before each POST.
- **Eligibility.** An attempt is tracked only if the gate was open at admission.
- **Opt-out.** Set `export_blocked` and cancel the in-flight request before
  writing preferences. Then replace the queue with an empty one at a new
  generation. If the write fails in any way, the gate stays closed for the rest
  of the run.
- **Opt-in.** Write an empty queue at a new generation before writing
  preferences; if that fails, consent stays off.
- **Queue.** One file `launcher-summaries.json`, rewritten whole with
  `atomic::write`. At most 64 entries and 48 KiB, 24-hour TTL, drop-oldest with
  aggregate counters. Tracking is a single optional entry.
- **Pre-admission coalescing.** Identical pre-admission failures increment
  `retry_count` on the queued entry and never evict an admitted attempt's
  terminal.
- **Exporter.** One task, started from `NativeHost::store()` after the first
  successful open, only when the endpoint is `Some`. It holds a `Weak` state
  handle and does its lock sections in `spawn_blocking`.
- **Cycle.** The token is a local of one cycle. Timeouts are 2 s; at most two
  retries. Responses are read bounded (8 KiB mint, 4 KiB ingest).
- **Test seam.** `run_cycle() -> CycleOutcome` with injected tuning, sleeper,
  clock and id source.
- **Endpoint.** `SummaryEndpoint::parse` accepts `https://`, or `http://` to
  loopback, and nothing else. Redirects are off, and the mint response's
  `upload_endpoint` is ignored.
- **Mint body.** `install_id` is a fresh random UUID, never persisted.
  `machine_id`, `branch` and `git_sha` are `""`. `session_kind` is
  `"launcher_summary"`.

## 3. Minimal authentication and ingestion changes

Auth (`A/dev_session/`):

- **`token.rs`:** add `SCOPE_LAUNCHER_SUMMARY_WRITE = "launcher_summary.write"`
  and `SESSION_KIND_LAUNCHER_SUMMARY`. `TokenClaims` is unchanged.
- **`handlers.rs`, summary kind:**
  - scope is only the summary scope;
  - `sub` is a server constant;
  - `machine_id`, `branch`, `git_sha` and `tags` must be empty, and
    `launcher_version` must be the version triple, else 400;
  - the mint is charged to a new `mint_summary_ip` table, selected before the
    charge, and skips the per-install charge.
- **`handlers.rs`, visibility:** `kill_switch_active` becomes `pub(crate)`.
- **`mod.rs`:** re-export the constants and `kill_switch_active`.
- **Separation is by scope.** The existing `verify_bearer` already refuses a
  token without `telemetry.write`.
- **Deliberately not added:** a shorter TTL and a refresh refusal. The security
  advisor prefers a scope-based refresh refusal (about four lines); it is
  optional hardening.
- **Trust statement:** the mint stays credential-free, so every summary is
  self-reported and forgeable within quota. Rows must never drive server state,
  alerts or SLOs.

Ingest (`A/telemetry/launcher_summary/`):

- **Handler shape.** A synchronous
  `ingest_inner(state, policy, peer_ip, headers, body, now, now_unix)` with
  injected dedup and quota; the axum handler is a thin wrapper taking
  `HeaderMap`, `ConnectInfo` and `Bytes`.
- **Order.** Kill switch (503) → per-IP quota (429) → bearer with summary scope
  → parse → validate → dedup → emit → respond.
- **Quota.** One new knob, `CIMMERIA_TELEMETRY_SUMMARY_QUOTA_PER_IP`, default
  120 per hour, 0 disables.
- **Validation.** Element count is checked before any element is typed. Each
  element is typed separately, so one bad element is rejected alone. Ids are
  `String` in the DTO, parsed with `Uuid::parse_str`, nil rejected, and emitted
  in canonical form.
- **Errors.** Bodies are static text; serde error text is never returned or
  logged.
- **Dedup.** A fixed FIFO of the last 16,384 accepted `event_id`s, in memory.
  When full it evicts the oldest and never refuses. Verdicts are decided under
  the lock and rows are emitted after releasing it.
- **Rows.** One INFO record per accepted summary at target `launcher.summary`
  with `event = "launcher_summary"`. It carries the typed fields plus a
  server-derived `duration_bucket`
  (`lt_1s | lt_10s | lt_1m | lt_5m | lt_30m | ge_30m`). It carries no
  `install_id`, `session_id` or peer address.
- **Health row.** One INFO batch row per request at `launcher.ingest`, with
  server-counted accepted, duplicate and rejected totals plus the
  `client_dropped_*` values.
- **Index.** Add `launcher.summary` to `CLIENT_TARGETS`
  (`crates/server/src/otel.rs:167`) so self-reported rows land in
  `cimmeria-client`.
- **Mounting.** Expose a separate `launcher_summary_routes()` and merge it
  explicitly in `A/mod.rs` and `login_port.rs`, so the public mount is one
  reviewed line.

Operator fixtures, all `count()` with `service.name = 'cimmeria-client'`:

1. Attempts by `launcher_version`, `os`, `outcome`.
2. Failures by `operation`, `phase`, `error_code`.
3. Attempts by `operation`, `duration_bucket` (attempt duration, not phase
   duration).
4. A saved list view over the batch row.

The dashboard description states that the cohort is opted-in, received attempts
only.

## 4. File ownership

O owns (new):

- `E/storage/launcher_summary/{mod,schema,projection,queue,export}.rs`, its
  tests, and `fixtures/{request-all,response-mixed,mint-request}.json`
- `S/host/summary.rs` (the `JobError` mapping and a `NativeHost` method for
  pre-admission failures)
- `A/telemetry/launcher_summary/{mod,dto,handlers,dedup,rows,tests,fixture_tests}.rs`
- `docs/operations/signoz/launcher-journey.dashboard.json`,
  `launcher-summary.view.json`, `launcher-summary-views.md`
- `docs/architecture/launcher-summary-telemetry.md`,
  `D/docs/launcher-summaries.md`
- `docs/analysis/playtests/2026-10-03-macos-wine/worknotes/observability.md`

O edits (telemetry docs): `docs/operations/telemetry.md` (endpoint table and
line 467), `docs/architecture/dev-session-telemetry.md`,
`docs/architecture/observability-target-catalog.md`, `docs/tools/admin-api.md`.

Shared files needing a coordinator ownership record (isolated integration
commit):

| File | Edit |
|---|---|
| `E/storage/mod.rs` | `mod` line, journal observer, `DesktopState` field, gate in `save_preferences_with`, queue load in `open` |
| `E/lib.rs` | Re-export |
| `E/storage/install_worker/mod.rs` | `summary_detail` in `publish`; last-progress capture in `spawn_worker` |
| `E/storage/runtime_setup/mod.rs`, `E/mac_wine/prerequisites/mod.rs` | Optional `summary_detail` calls |
| `S/host.rs` | `mod summary;`, config builder, exporter start in `store()` |
| `S/main.rs` | Pass `SummaryConfig`; one wrapper call for pre-admission failures |
| `A/telemetry/mod.rs`, `A/telemetry/handlers.rs` | `mod` line, `pub use`; `verify_bearer_scoped` |
| `A/dev_session/{token,handlers,mod}.rs`, `session_kind_tests.rs` | Section 3 auth edits and tests |
| `A/mod.rs`, `crates/admin-api/src/login_port.rs` | Explicit route merge; "four routes" doc and test |
| `crates/server/src/otel.rs`, `logging/parity_tests/guard.rs`, `logging/client_index_tests.rs` | `CLIENT_TARGETS` entry and literal assertions |
| `crates/server/src/main.rs` | Env table row and "four routes" comment; open PR #617 also edits this file |
| `.gitattributes` | `eol=lf` for the golden fixtures |
| `docs/readme.md`, `D/README.md` (link only), campaign ledger | Indexes |

Out of this packet: all `F/` files, the egui launcher, and Repair/Launch detail
hooks (they get begin and terminal rows from the journal for free).

## 5. Acceptance tests

Every negative test awaits `run_cycle()` directly, asserts its typed outcome,
and shares a positive control that differs only in the varied condition.

Engine (tempdir state, wiremock, injected clock, ids and sleeper):

1. **Failure before game.** A real install worker failure with consent on
   produces exactly one terminal entry that reaches the mock ingest with no
   launch code invoked. A shell test drives `select_install_release` with a
   failing fetch and asserts one `manifest_unavailable` entry. Removing the
   journal observer must fail it.
2. **Terminal table.** Install success, content-invalid, cancel, reconcile,
   uninstall and runtime setup each give one row, and none on a repeated
   terminal.
3. **Default off and exporter disabled.** No queue file; outcomes
   `SkippedNoConsent` and `SkippedNoEndpoint`.
4. **Opt-out races, sequenced by the mock responder (not a barrier):**
   - opt-out inside the mint response → zero ingest POSTs;
   - opt-out during the injected backoff sleep → exactly one POST;
   - opt-out inside the POST response → queue not recreated;
   - opt-out then opt-in → only the new attempt is delivered.
5. **Opt-out write failures.** `Io` and `PersistenceUncertain` (via the
   `AfterReplace` checkpoint) each give zero further POSTs in the run. A failed
   opt-in write leaves consent off.
6. **Restart.** Entries survive reopen. A resend after a lost acknowledgement
   carries the same `event_id` and is removed on `duplicate`. A terminal
   committed before a crash is emitted at open.
7. **Endpoint outage.** Refused connection, timeout, 500, 429 and 503 give
   exact request counts and recorded sleeps. Install results are identical to a
   run with no exporter. A 302 on either step sends nothing further and retains
   the queue.
8. **Old server.** Mint 400 and ingest 404 stop the run with entries retained.
9. **Hostile response.** Oversized bodies, a short `results` array, an unknown
   result string and a token containing CR/LF each leave the queue intact and
   the task alive.
10. **Broken queue file.** Truncated JSON, 65 KiB, `schema_version: 99` and a
    directory at that path: `open` succeeds and install works.
11. **Budgets.** The worst-case entry is at most 1 KiB. The 65th entry drops
    the oldest. Exactly 24 hours is kept and one second more is dropped. 200
    identical pre-admission failures leave one entry.
12. **Leak test.** Seed markers in the install directory name, state root and
    manifest URL; assert absence in the queue bytes, both request bodies and
    the URLs.
13. **Consent isolation.** Requests go only to the mint and summary paths; two
    mints carry different `install_id`s. A source scan restricts readers of
    `launcher_summary_consent`. Track L owns the test that the launch plan is
    identical with consent on and off.
14. **Endpoint policy.** Non-loopback `http://` and other schemes are refused
    at construction.
15. **Lock release.** Dropping the host with an idle exporter releases
    `launcher.lock`.

Server (`ingest_inner` with fresh state inside the capture layer; loopback only
for route exposure and the body limit):

16. **Scope, through `mint_inner`.** A summary mint decodes to exactly the
    summary scope. That token fails `verify_bearer`. Player and lab tokens fail
    the summary route. Non-empty identifiers are 400.
17. **Quota isolation.** Exhausting the summary mint table leaves player mints
    working, and the reverse.
18. **Order.** Kill switch with no bearer gives 503. Over quota with a garbage
    bearer gives 429. Malformed JSON with no bearer gives 401.
19. **Validation.** 33 elements, unknown field or enum, a value above
    `i64::MAX`, nil and non-canonical UUIDs, non-ASCII digits, and exact
    body-limit boundaries.
20. **Dedup.** The same id twice in a request, across requests and from two
    threads gives one row. After capacity plus one, the first id is accepted
    again and nothing is ever refused.
21. **No echo.** A marker with a newline in enum, field-name and body positions
    appears in no log record on any target and no response body.
22. **Row shape.** Exact key-set equality on the summary row and the batch row;
    level and discriminator pinned.
23. **Index.** `launcher.summary` routes to the client index and
    `launcher.ingest` to the server index. Removing the `CLIENT_TARGETS` entry
    must fail it.
24. **Route exposure** on the login-port router; admin routes still 404.

Cross-workspace contract and fixtures:

25. **Golden contract.** `request-all.json` holds one row per valid
    combination. The engine asserts it produces that body, compared as JSON
    values. Admin-api asserts every element is `accepted` and that its real
    response equals `response-mixed.json`.
26. **Operator fixtures.** Every filter and group-by key is an emitted key or a
    known intrinsic. Every aggregation is `count()`. The cohort caveat is
    present. A deliberately misspelt key fails the check.

## 6. Validation by platform

- **Platform-independent logic, run on the macOS and Windows legs of
  `launcher-desktop.yml`** (there is no Linux engine leg): schema, projection,
  queue, gate, exporter, and the journal, install-worker and runtime-setup
  hooks. These run under `cargo test` in one process, so no test may mutate env
  or statics without a lock.
- **Server tests** run on ubuntu under nextest.
- **Golden-contract gap.** A server-only change runs only the server half of
  the golden contract, because `launcher-desktop.yml` has no
  `crates/admin-api/**` path filter.
- **Windows-specific:**
  - atomic replace durability and `launcher.lock` behaviour with the extra
    state file;
  - whether `app_data_dir()` resolves to a roaming profile, and so whether the
    queue roams;
  - the native install backend and `Resume` path;
  - the final Windows-native build.
- **macOS-only:** `mac_wine` prerequisite terminals (`finish_not_dispatched`),
  the Wine install path's callers, and the unix directory-fsync path.
- **Not validated anywhere in this packet:** a real endpoint, the packaged
  webview, and the consent copy change. These belong to the endpoint rollout.

Known limits:

- An attempt that crashes before its terminal, or never exports within 24
  hours, is invisible.
- A failure to open the state is invisible, because consent is unknown.
- The window between the last consent recheck and the socket write cannot be
  closed.

Findings outside this track, for issues:

- The egui launcher zips `sessions/current-session.json`, which holds the
  bearer token, into debug bundles (`crates/launcher/src/logs.rs:58-68`).
- `docs/architecture/dev-session-telemetry.md:413-419` claims server-side dedup
  that the code does not have.

## 7. Copy-ready prompt for the implementation session

Prepend the shared prompt from the
[delegation plan](../launcher-delegation-plan.md) with base, worktree and
branch filled in, then:

```text
Implement launcher-summary observability through local ingestion, exactly per
docs/analysis/playtests/2026-10-03-macos-wine/worknotes/observability-discovery.md
(sections 2-6) and the coordinator's recorded decisions on: the public
login-port mount, the unset production endpoint, no installation correlator,
and the journal-observer hook in engine/src/storage/mod.rs.

That document was audited at 0b10d869c. Read its "Drift since the analysis
base" table first and revalidate every desktop-side citation against your own
base before editing; Launch and Repair now exist.

Scope: the Tauri desktop workspace (crates/launcher/desktop) and crates/admin-api
plus the listed crates/server lines. Do not touch the egui launcher
(crates/launcher/src) or any frontend file. Do not add dependencies or edit any
Cargo manifest, lockfile or workspace-hack; ids, jitter, timeouts and clocks are
injected through the existing *_with idiom.

Deliver, in this order:
1. Engine: schema and projection types (closed enums, Copy, no strings), the
   whole-file acknowledged queue, the consent gate with fail-closed opt-out, the
   journal observer, and run_cycle() with injected tuning. Endpoint stays None in
   production; the exporter is not started and nothing is recorded.
2. Server: summary scope and session kind on the existing mint with its own
   per-IP table, ingest_inner with injected dedup and quota, typed INFO rows at
   launcher.summary, the batch row at launcher.ingest, the CLIENT_TARGETS entry
   with literal routing assertions.
3. Golden fixtures shared by both halves, the SigNoz dashboard and view fixtures
   with their offline test, and the docs named in section 4.
4. An integration commit containing only the shared files listed in section 4,
   with its SHA reported separately. Keep your shell code in S/host/summary.rs
   so the diff to S/host.rs and S/main.rs is one mod line, one builder call and
   one wrapper call.

Acceptance is section 5. Every negative test awaits run_cycle() and has a
positive control; every guard names the revert that fails it. A queue or an
endpoint alone is not completion: the seeded install failure must reach the
local mock ingest without any launch code, and the same golden body must be
accepted by the real admin-api handler and matched by the query fixtures.

Constraints: every compiling cargo call goes through tools/build-lane/lane.sh,
including the desktop workspace. Engine and shell tests run on macOS or native
Windows, not WSL; Windows artifacts build natively on Windows. Never build the
egui launcher on macOS. wiremock and loopback only: no SigNoz or observability
MCP, no VPN, no live endpoint. Self-contained startup validation stays last.
Record honestly which hooks are delivered and which await another track. If
code contradicts the contract, return one bounded blocker handoff and stop.
Write worknotes/observability.md, ship as a draft PR retargeted to
prototype/launcher-packaging-proof, report once, and stop.
```
