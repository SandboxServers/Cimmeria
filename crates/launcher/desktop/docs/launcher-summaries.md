# Launcher summaries contract

> **Type:** Reference
> **Audience:** Launcher contributors, and whoever prepares the rollout packet
> **Last updated:** 2026-10-04
> **Companions:** [Desktop workspace](../README.md), [design and wire contract](../../../../docs/architecture/launcher-summary-telemetry.md), [operator notes](../../../../docs/operations/telemetry.md), [dashboard reading notes](../../../../docs/operations/signoz/launcher-summary-views.md), [launch](launch.md), [implementation assignment](../../../../docs/analysis/playtests/2026-10-03-macos-wine/worknotes/observability-implementation-assignment.md)

**Status: the engine component, its queue, its exporter and the shell glue are
connected. The endpoint is `None` in every distributed build, so this build
collects nothing and sends nothing. Endpoint configuration, consent copy and
frontend UAT are open rollout gates.** Nothing here ran against a real server.

A launcher summary is one terminal row per install, runtime-setup, repair,
uninstall or launch attempt: closed enums, bounded integers and two engine-minted
UUIDs. This page is the desktop side. The wire contract, the server's rules and
the rows it writes are in the
[design reference](../../../../docs/architecture/launcher-summary-telemetry.md).

## Module map

Everything lives in `engine/src/storage/launcher_summary/`, a child of
`storage`:

| File | What it holds |
|---|---|
| `mod.rs` | The public surface, the `DesktopState` entry points, lazy finalization and the error-code mapping |
| `schema.rs` | Wire contract v1: closed enums with `ALL` lists, bounded integer types, strict serde |
| `endpoint.rs` | `SummaryEndpoint`: where summaries may be sent |
| `tracker.rs` | The gate, in-memory attempt tracking, the journal observer and restart reconciliation |
| `attempt.rs` | One tracked attempt: its timed phases, and what a row may say about time |
| `queue.rs` | The on-disk queue file |
| `consent.rs` | The consent transitions around the preferences write |
| `batch.rs` | The exporter's seam into the state: take a batch, check it is still current, apply verdicts |
| `export.rs` | The exporter task, `run_cycle` and its outcomes |
| `exchange.rs` | The two HTTP requests of one delivery attempt |
| `fixtures/` | The golden wire fixtures shared with the server |
| `tests/` | The suite; see [Tests](#tests-and-where-they-run) |

Three files outside the folder take part:

- `engine/src/storage/mod.rs`: `FileJournal::commit` calls the observer, and
  `operations_mut` and `save_preferences_with` finalize and handle consent.
- `engine/src/storage/install_worker/mod.rs`: a watcher task enters `download`
  and `extraction`, and `publish` finalizes right after the terminal commit.
- `shell/src/host/summary.rs`: composition (`endpoint: None`), the product
  version, and the closed codes for a command that failed before admission.
  `shell/src/host.rs` calls `summary::start` once, right after the state is
  first opened, and `shell/src/main.rs` notes a failed `install_command` or
  `launch_command`.

## Configuration and the gate

`launcher_summary::start(state, SummaryConfig { launcher_version, endpoint })`
configures the component. With `endpoint: None` the component is inert: nothing
is tracked, recorded or sent, and any queue file an earlier run left is removed.

`SummaryEndpoint::parse` is the only constructor. It accepts `https://` anywhere
and `http://` only to `localhost` or a literal loopback address, and refuses a
URL with credentials, a query or a fragment. The value comes from native
configuration only: never from a response, the webview or the environment.

One function decides whether anything may be tracked, recorded or sent
(`Tracker::gate`). The gate is open only when all four hold:

1. an endpoint is configured;
2. `preferences.launcher_summary_consent` is true;
3. no opt-out is in doubt (`export_blocked` is clear);
4. the state does not require a reopen.

The gate is asked at admission, when a row is enqueued, when a batch is taken,
and again before every request and before verdicts are applied.

**Eligibility is decided at admission.** Only the commit that admits an
operation (state `Starting`) starts tracking, and only with the gate open. An
attempt admitted while the gate was closed, before configuration or without an
endpoint stays unreported for its whole life, even if consent is given a moment
later.

Consent is the existing `launcher_summary_consent` preference, default off. It
is independent of game and DLL telemetry: launch, install, repair and migration
code never read it. A source scan
(`only_the_allowed_files_mention_the_summary_consent_flag`) fails when a new
non-test file in the engine or the shell mentions the flag.

## Journal observer and lazy finalization

The component has one hook. `FileJournal::commit` calls
`launcher_summary::observe_commit` after `operation.json` was replaced
successfully. A failed commit observes nothing. Because every workflow commits
through the journal, every admission and terminal is seen without a hook in each
workflow.

The observer does only cheap, infallible work under the tracker's own mutex
(lock order: state mutex outside, tracker inside):

| Committed state | What the observer does |
|---|---|
| `Starting`, new operation, gate open | Mints an `attempt_id`, opens the timed phase `starting`, and writes a tracking entry to the queue file so a crash is later reported as `unknown` |
| `Running`, tracked | Enters the timed phase `running` |
| `Succeeded`, `Failed`, `Cancelled`, tracked | Stores a pending end and wakes the exporter |
| `ReconciliationRequired`, tracked | Stores a pending end with outcome `unknown` |
| `CancelRequested`, or any state of an untracked operation | Nothing |

The journal's operation id comes from the webview. It is kept only to match the
journal and is never exported; `attempt_id` and `event_id` are minted by the
engine.

The install worker adds two phases. `watch_phases` follows the worker's
progress and calls `summary_phase(id, TimedPhase::Download)` or
`TimedPhase::Extraction` whenever the kind of progress changes. It takes the
state lock for that one call, holds no strong state handle, and ends when the
worker drops its progress sink.

**Finalization is lazy.** Turning a pending end into a queue entry needs the
whole `DesktopState`, because the error code is read from the launcher's own
result records. So the observer only records the end, and
`finalize_summaries` builds the row later. It is idempotent and runs at the
start of `operations_mut`, a preferences save, every summary entry point and
the exporter's batch take, and right after the install worker's terminal
commit, while the result record is still that attempt's. One queue write stores
the row and clears the tracking entry together.

The error code of a failed attempt:

| Operation | Source | Code |
|---|---|---|
| Install | `install_outcome()` | `destination_unavailable`, `install_failed`, `content_invalid`, `rosetta_required` or `runtime_unavailable`; anything else is `unspecified` |
| Launch | `launch_observation()` | `launch_not_started`; a non-zero exit is `launch_early_exit` when it was early and `launch_exit_nonzero` otherwise; anything else is `unspecified` |
| Prepare runtime | none | `prerequisite_failed` |
| Repair, uninstall | none | `unspecified` |

Detail is read only while the journal's current operation is still that failed
attempt. If a second attempt ended before the first was finalized, the first is
queued with `unspecified`: both rows are kept, and no detail is guessed.

## What a row says

`Live::finish` in `attempt.rs` decides what a row may say about time.

- **Phases are totals.** Entering a phase closes the one before it, and a phase
  entered again adds to the same entry. An install moves between `download` and
  `extraction` for the seed and every patch, so each entry is the total over all
  of them. Nothing reports progress after the last unpack, so `extraction` also
  covers content verification and promotion.
- **A watched attempt** carries `duration_ms` (end minus admission), `phases`
  (what this process timed), and `phase` (the last timed phase entered).
  Durations saturate at seven days.
- **A launch row carries `starting` only.** It has no `running` entry and no
  `duration_ms`. The journal commits `Running` before the game host is spawned
  and the terminal when the game process exits, so both would be the length of
  the play session, which is game activity and outside this consent. `starting`
  is the launcher's own preparation. The row's `phase` still says where the
  attempt ended, so a launch that never started can end at `running` with
  `launch_not_started`.
- **Launch `succeeded`** means the game process the launcher watched exited with
  code 0. It never means a login or a world entry.
- **A lost observation is one `unknown` row.** It has no `duration_ms` and no
  entry for the phase that was open, because nobody saw either end. Phases that
  had already closed are kept; an attempt lost in its first phase has no
  `phases` key. After the `unknown` row the attempt is no longer tracked, so a
  later reconciled terminal adds nothing.
- **After a restart nothing is timed.** At configuration the engine settles the
  tracking entry a previous process left. If the journal still shows that
  operation, the row takes the journal's terminal state as its outcome, or
  `unknown` when the operation is not terminal, with `phase = none`, no
  `duration_ms` and no `phases`. If the journal shows a different operation,
  or the gate is closed, the tracking entry is dropped and nothing is reported.
  Timings are never invented.
- **A state that requires a reopen** is left alone. Tracking stays on disk and
  the next process reports the attempt, without timings.
- **`retry_count` is 0** for every admitted attempt.

## Pre-admission failures

A command can fail before the journal admits an operation. The shell reports
those through `NativeHost::note_command_failure`, which maps the `JobError`
alone to a closed pair. No string from the webview is involved.

| `JobError` | `phase` | `error_code` |
|---|---|---|
| `PlatformUnavailable` | `platform_check` | `platform_unavailable` |
| `LauncherTooOld` | `compatibility_check` | `launcher_too_old` |
| `InvalidDirectory` | `destination_check` | `invalid_directory` |
| `ManifestUnavailable` | `catalog_fetch` | `manifest_unavailable` |
| `InvalidManifest` | `manifest_verify` | `manifest_invalid` |
| `SigningKeyUnavailable` | `manifest_verify` | `signing_key_unavailable` |
| `CorruptState` | `admission` | `state_invalid` |
| `Io` | `admission` | `local_io` |

Every other `JobError` (a stale revision, a busy launcher, an unknown
operation) is a contract answer to the webview, not an attempt's end, and
records nothing. Only Install, PrepareRuntime, Repair, Uninstall and Play name
an attempt. The shell never opens the state to record a failure.

`summary_pre_admission_failure` then does this, with the gate open:

- It ignores a failure that names the tracked operation or the journal's
  current one. That attempt reports through the journal.
- An identical failure (same operation, phase and code) that is still queued
  only raises that row's `retry_count`, which stops at 100. Two hundred
  identical failures are one row.
- Otherwise it appends one `failed` row with fresh ids, no `duration_ms` and no
  `phases`.
- When the queue is full, a pre-admission row never evicts an admitted
  attempt's row. It replaces the oldest pre-admission row, or is itself dropped
  when there is none. Either way `dropped.overflow` goes up by one.

## Consent transitions

`save_preferences_with` calls `summary_consent_requested` before the preferences
write and `summary_consent_written` after it (`consent.rs`).

**Opt-out (`true` to `false`).** Before the write, the engine:

1. sets `export_blocked`, which closes the gate;
2. cancels the batch's token, which aborts a mint or POST in flight and ends a
   backoff wait;
3. forgets the tracked attempt and removes its tracking entry.

After the write returns `Ok` or `PersistenceUncertain`, it replaces the queue
with an empty one at `generation + 1`. If the write fails any other way, the
gate stays closed for the rest of the process run: `export_blocked` stays set,
and only a later successful opt-in clears it. A panic inside the first step
still sets `export_blocked`.

Bytes the server had already received cannot be recalled. Their answer is never
read or applied.

**Opt-in (`false` to `true`).** The engine first writes an empty queue at
`generation + 1`. If that write fails, the save returns `StorageError::Io` and
preferences are untouched, so consent stays off. Then preferences are written as
before. An operation already running at opt-in is not reported, and nothing
recorded before the opt-in can be sent after it.

**Unchanged consent** has no queue effect.

**Startup.** `DesktopState::open` does not read the queue file and behaves the
same whether or not one exists. The queue is loaded at `configure_summaries`; if
consent is off at that point, the file is removed.

**Without an endpoint,** a consent change writes no queue file. An opt-in or a
withdrawal removes a file an earlier configured run left, best effort, and can
never fail the preferences save.

The `generation` number is what makes a withdrawal safe against an upload in
flight: a batch remembers the generation it was taken at, and a batch whose
generation is no longer the queue's is neither sent nor applied.

## Queue file

One file, `launcher-summaries.json`, in the state root, rewritten whole with the
same `atomic::write` the other state files use (64 KiB cap).

```text
{ schema_version: 1, generation, dropped: { overflow, expired, rejected },
  tracking: { local_operation_id, attempt_id, kind } | null,
  entries: [ { created_unix_s, pre_admission, summary } ] }   // oldest first
```

| Bound | Value |
|---|---|
| Entries | At most 64. An admitted attempt's row always gets in; the oldest entry makes room and `dropped.overflow` goes up |
| Age | 24 hours. An entry is kept at exactly 24 hours and dropped one second later, counted in `dropped.expired`. A creation time ahead of the clock counts as age zero |
| Counters | `u16`, saturating |
| Entry size | A worst-case entry is 592 bytes, pinned exactly, and a full queue fits the 64 KiB cap (`worst_case_entry_and_a_full_queue_fit_their_budgets`) |

The queue is disposable. Any problem reading it (corrupt JSON, an unknown
schema version, more than 64 entries, an oversized file, not a regular file, an
I/O error) means "empty", and never reaches the launcher's own state. A failed
write is counted in `SummaryFaults::queue_writes` and tried again at the next
entry point. Until it succeeds, a restart could report the tracked attempt a
second time.

`dropped` holds what was lost since the last answered upload. It is sent with
every batch and cleared only by an answered `200`.

## Exporter cycle and outcomes

`start` spawns one task per state, and only when an endpoint is configured and a
tokio runtime is running. The task holds a weak state handle, so dropping the
state ends the task and releases the state directory. It runs a cycle at start,
then whenever a tracked attempt ends or a new pre-admission row is queued, and
repeats at once while a cycle delivered, because more rows may be waiting than
one batch holds.

One cycle (`run_cycle` in `export.rs`):

1. **Take a batch**, under the state lock on a blocking thread: finalize pending
   rows, expire old ones, check the gate, and copy the oldest rows (at most 32
   and at most 48 KiB serialized) with the queue's generation and drop counters.
   The rows stay queued.
2. **Mint**: `POST {base}/auth/dev-session` with a fresh random `install_id`.
   Only `token` is read from the answer.
3. **Check again** that the gate is open, the endpoint is the configured one and
   the generation is unchanged.
4. **Post**: `POST {base}/telemetry/launcher-summary` with the bearer token.
5. **Apply**, under the lock, only if the batch is still current: remove every
   answered row and clear the counters that were sent.

The state mutex is never held across a request or a sleep.

| Setting | Production value (`ExportTuning::PRODUCTION`) |
|---|---|
| Timeout per request, body included | 2 s for the mint, 2 s for the post |
| Retries per cycle | 2, so at most 3 attempts. Each retry mints again |
| Wait on 429 or 503 | `Retry-After`, capped at 60 s; 60 s when the header is absent or unreadable |
| Wait after another transient failure | Jittered backoff, 250 ms to 2 s |
| Response read limit | 8 KiB for the mint, 4 KiB for the ingest |
| Redirects | Never followed |

The token is a local of one attempt. It must be non-empty, at most 4,096 bytes
and a legal header value; it is marked sensitive, and it is never stored, cached
across attempts or logged. Response bodies are parsed into closed types only.

What a status does to the cycle:

| Step | Answer | Action |
|---|---|---|
| Mint | 200 with a usable token | Go on |
| Mint | 400, 404, 405, 415, 422 | The server has no summary session kind: `StoppedForRun`. Rows stay queued |
| Post | 200 with one known verdict per row | Remove the `accepted`, `duplicate` and `rejected` rows; add the rejected count to `dropped.rejected` |
| Post | 200 with the wrong length, an unknown verdict, an unparseable or oversized body | Transient. Remove nothing |
| Post | 400, 413 | Permanent: remove the batch and add its size to `dropped.rejected`. The counters it carried are sent again |
| Post | 404, 405 | `StoppedForRun`. Rows stay queued |
| Either | 429, 503 | Wait as above, within the retry budget |
| Either | Anything else: 401, 3xx, 403, 5xx, a timeout, a refused connection, an unusable token | Transient within the retry budget. Never delete |

How a cycle ends (`CycleOutcome`):

| Outcome | Meaning |
|---|---|
| `SkippedNoEndpoint` | No endpoint is configured, or not the one this exporter was started for |
| `SkippedNoConsent` | The gate is closed: no consent, an opt-out in doubt, or a state that must be reopened |
| `Empty` | Nothing is queued |
| `Delivered { accepted, duplicate, rejected }` | The server answered and every answered row left the queue. A body refused for good counts all its rows as rejected |
| `ConsentWithdrawn` | The gate closed or the queue was replaced mid-cycle. Nothing was applied |
| `StoppedForRun` | The server has no summary routes. The task ends and makes no further request in this process run |
| `GaveUp` | The retry budget ran out. The rows wait for the next trigger |
| `StateGone` | The state was dropped. The task ends |

Two counts are at-least-once approximations by design in v1 (`batch.rs`):
`client_dropped`, because a body the server took but whose answer was lost is
sent again with the same counters; and a pre-admission row's `retry_count`,
because a repeat that arrives while the row is in flight is never reported.

## Failure isolation

No entry point can fail or change the result of the launcher's own work. Each
returns `()`, wraps its body in `catch_unwind` while the caller's state guard is
held, and counts what it swallowed in `SummaryFaults` (`queue_writes`,
`panics`), which is local only. The component never sets
`preferences_uncertain`, and an export error leaves install, Play, recovery and
preferences as they would be without it. The one place it can return an error
is the opt-in, which refuses to turn consent on over a queue it could not empty.

## Tests and where they run

| File under `launcher_summary/tests/` | What it covers |
|---|---|
| `journal.rs` | One row per attempt with its closed code, admission eligibility, lost observation, a failed commit, no endpoint |
| `timings.rs` | Phase totals, the launch row's single `starting` phase, lost observations, saturation |
| `launch_rows.rs` | Launch outcomes and codes through the real launch worker |
| `pre_admission.rs` | Coalescing, the `retry_count` ceiling, eviction rules |
| `restart.rs` | Tracking on disk, rows without timings after a restart, the startup scrub |
| `consent.rs` | Opt-out, a failed or uncertain opt-out write, opt-in, the inert component |
| `queue_file.rs` | Size budgets, overflow, expiry, a broken queue file changing no launcher result |
| `batch_seam.rs` | Batch size, verdicts, generation checks |
| `isolation.rs` | Contained panics, a fault during a consent change, the consent-flag source scan |
| `golden.rs` | The shared fixtures and every rule the server enforces |
| `exporter/` | Delivery, outages and retries, hostile answers, races with consent changes, an opt-out aborting a held request, the task's lifetime, the production tuning |

Outside the folder: `endpoint.rs` has its own tests,
`install_worker/summary_tests.rs` covers the real install worker and the phase
watcher, `install_worker/export_tests.rs` takes a real failed install through
the exporter to a loopback mock of the two routes, and
`shell/src/host/summary.rs` tests the version parse, the `JobError` mapping and
that noting a failure never opens the state.

Every exporter test uses wiremock on loopback. No test contacts a real
endpoint.

Where they run:

- **Engine suite.** The `desktop launcher` workflow
  (`.github/workflows/launcher-desktop.yml`) runs
  `cargo test -p cimmeria-launcher-engine` on `windows-latest` and
  `macos-latest`. Locally it runs on Linux through the lane:

  ```bash
  bash tools/build-lane/lane.sh cargo test --locked \
    --manifest-path crates/launcher/desktop/Cargo.toml \
    -p cimmeria-launcher-engine --target-dir target/desktop launcher_summary
  ```

- **Shell glue.** `cimmeria-launcher-desktop` does not build on the Linux
  development host (no GTK or WebKit), so `host/summary.rs`, the
  `summary::start` call and the two command wrappers are first compiled, linted
  and tested by the same workflow's shell steps on Windows and macOS.
- **Server half.** The ingest tests are in `cimmeria-admin-api` and run with the
  main workspace.

## What a rollout packet must change

This packet ships the mechanism switched off. Turning it on is a separate
packet, after the maintainer's decision recorded as open in the
[public-activation gate](../../../../docs/architecture/launcher-summary-telemetry.md#public-activation-gate).
It has to change at least these:

1. **Endpoint configuration.** `shell/src/host/summary.rs` passes
   `endpoint: None`. A rollout supplies a `SummaryEndpoint` from native
   configuration. There is deliberately no environment override. Today
   `SummaryEndpoint::parse` refuses plain `http://` to anything but loopback,
   so the base must be an `https://` address. Sending to the plain-HTTP login
   port instead needs a change to that policy as well as the maintainer's
   decision on the mount.
2. **Consent copy.** The frontend confirms a consent change with "Diagnostics
   choice saved. This build sends nothing." (`frontend/src/view.ts`). That
   sentence becomes false the moment an endpoint is configured. The copy must
   say what is sent (the fields in the wire contract) and what is not, before
   any build with an endpoint ships. The preferences paragraph in the
   [desktop README](../README.md) needs the same correction.
3. **Frontend UAT.** A frontend change needs the REPL-style logic UAT the repo
   requires ([AGENTS.md](../../../../AGENTS.md)), covering opt-in, opt-out and
   the copy for both states.
4. **Server side.** Deploy a server that serves the route where the endpoint
   points, size `CIMMERIA_TELEMETRY_SUMMARY_QUOTA_PER_IP` and the mint allowance
   for the address launchers arrive from, and import the SigNoz fixtures only
   after the first rows exist
   ([importing](../../../../docs/operations/signoz/launcher-summary-views.md#importing)).
5. **Native evidence.** Record the Windows and macOS runs of both suites for the
   revision that ships.

Finer launch phases (host started, process started) are not timed. Adding them
needs edits under `engine/src/storage/launch/` and is not part of a rollout.
