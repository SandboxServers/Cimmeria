# Launcher summary dashboard and saved view (SigNoz fixtures)

> Type: reference. Audience: operators who import or read the launcher-summary SigNoz objects, and anyone changing the rows they query.
> Updated: 2026-10-04 (launcher observability, track O). Companions: [launcher-summary design](../../architecture/launcher-summary-telemetry.md), [telemetry operations](../telemetry.md), [SigNoz deployment](../signoz-deployment.md), [target catalog](../../architecture/observability-target-catalog.md), [NPC AI views](npc-ai-views.md) (the same file shapes, exported from a live SigNoz), [implementation assignment](../../analysis/playtests/2026-10-03-macos-wine/worknotes/observability-implementation-assignment.md).

The desktop launcher can report, with the player's consent, how each install, runtime setup, repair, uninstall or launch attempt ended. The server turns each accepted report into typed log rows. This page describes the two SigNoz objects that read those rows:

| File | What it is |
|---|---|
| [launcher-journey.dashboard.json](launcher-journey.dashboard.json) | Dashboard `Cimmeria — Launcher journey` (SigNoz dashboard `v5` JSON), four count tables |
| [launcher-summary.view.json](launcher-summary.view.json) | Saved Logs Explorer view `Launcher summary — ingest batches` |

## Status: fixtures only, not validated against a live SigNoz

Neither object exists in any SigNoz, so this page lists no ids. Nothing here was imported, run or checked against a live SigNoz:

- The JSON copies the shape of [npc-ai-health.dashboard.json](npc-ai-health.dashboard.json) and [black-market.view.json](black-market.view.json), which were exported from the colo SigNoz. Whether SigNoz accepts these two files, and whether the tables render as intended, is untested.
- The launcher ships with no summary endpoint configured, and the public route is not deployed. No server has received a summary, so no index holds a row for these queries to match.
- What is tested, offline, is that every key and literal the queries name matches what the server writes. See [The offline guard](#the-offline-guard).

## Who is counted

The dashboard counts attempts from opted-in launchers whose summary the server successfully received. It counts nothing else. Read every number with that in mind:

- **It is not an all-player funnel.** A player who never opted in, a launcher with no endpoint, and a summary that was dropped before it arrived (queue overflow, 24-hour expiry, a server that was down) are all missing.
- **It is not an install success rate.** The share of `succeeded` among received attempts says nothing about the attempts that were not received.
- **It is not a login or world-entry metric.** A launch counts as `succeeded` when the game process the launcher watched exited with code 0. It never means a login or a world entry: the launcher does not know whether the player logged in or reached a world.
- **`unknown` is its own outcome.** It means the launcher lost sight of the attempt: it crashed or was closed mid-attempt, or it found a state it had to reconcile. Do not add it to `succeeded` or to `failed`.
- **The rows are self-reported.** The token that authorises a summary needs no credential, so a summary can be forged within the per-address quota. Do not alert on these counts or set a target from them.
- **A resent summary can be counted twice across a server restart.** The server drops a summary it has already accepted, but it remembers accepted ids in memory only. A launcher that resends after the server restarted is accepted again.

## The rows

The server writes three kinds of row, all at INFO. The Rust log `target` is the `scope_name` column in SigNoz, and the row's message is the `body`.

| `event` | `scope_name` | `service.name` | One row per | Fields |
|---|---|---|---|---|
| `launcher_summary` | `launcher.summary` | `cimmeria-client` | accepted summary: one attempt, or one pre-admission failure (see [below](#what-a-launcher_summary-row-stands-for)) | `event_id`, `attempt_id`, `operation`, `phase`, `outcome`, `error_code`, `duration_ms`, `duration_bucket`, `retry_count`, `launcher_version`, `os`, `arch`, `schema_version`, `cimmeria.session_kind` |
| `launcher_phase` | `launcher.summary` | `cimmeria-client` | timed phase of an accepted summary | `attempt_id`, `operation`, `phase`, `duration_ms`, `duration_bucket`, `launcher_version`, `os`, `arch`, `schema_version`, `cimmeria.session_kind` |
| `launcher_summary_batch` | `launcher.ingest` | `cimmeria-server` | request that reached validation | `accepted`, `duplicate`, `rejected`, `client_dropped_overflow`, `client_dropped_expired`, `client_dropped_rejected` |

The first two are what a player's machine reported about itself, so they go to the `cimmeria-client` index. The batch row is the server's own account of a request and stays in `cimmeria-server`.

An optional value that is absent is an absent field, never a placeholder: `error_code` is present only when `outcome = 'failed'`, and `duration_ms` and `duration_bucket` only when the launcher timed the attempt (never for a launch; see [Reading the duration panels](#reading-the-duration-panels)).

### What a `launcher_summary` row stands for

Most rows are one admitted attempt that the launcher watched to its end. These rows need more care:

- **`phase = none`** means the attempt's end was reported by a later launcher process: the launcher was closed or crashed during the attempt, and the next run reported it. No phase and no timing was observed, so the row has neither.
- **A pre-admission row** has one of the phases `platform_check`, `compatibility_check`, `catalog_fetch`, `manifest_verify`, `destination_check` or `admission`, and no duration. The command failed before the launcher admitted an attempt. A repeat of the same failure while the row is still waiting to be sent is folded into it, so the row stands for `retry_count + 1` identical failed commands, and every panel counts it as one failed row.
- **`admission` with `local_io` or `state_invalid`** means before or at admission. The launcher takes the phase from the kind of error, and these two errors also come from steps that run before admission is tried.
- **`outcome = unknown`** means the launcher lost sight of the attempt. The row carries no attempt duration and no timing for the phase that was open when observation was lost, because the launcher did not see either end.

Two more things to know when reading `phase` and `retry_count`:

- `launch_not_started` can appear with `phase = running`. The launcher enters `running` just before it starts the game process, so a launch that never started can end there.
- `retry_count` is an at-least-once approximation: a request the launcher has to retry repeats it. Read it as "at least this many repeats were reported".

The values are closed sets:

| Field | Values |
|---|---|
| `operation` | `install`, `prepare_runtime`, `repair`, `uninstall`, `launch` |
| `outcome` | `succeeded`, `failed`, `cancelled`, `unknown` |
| `phase` on a `launcher_summary` row (where the attempt ended) | `none` (the end was reported by a later launcher process); a step before the attempt was admitted: `platform_check`, `compatibility_check`, `catalog_fetch`, `manifest_verify`, `destination_check`, `admission`; or the last timed phase entered |
| `phase` on a `launcher_phase` row (the timed phases) | `starting`, `running`, `download`, `extraction` |
| `error_code` | `unspecified`, `platform_unavailable`, `launcher_too_old`, `invalid_directory`, `manifest_unavailable`, `manifest_invalid`, `signing_key_unavailable`, `state_invalid`, `local_io`, `destination_unavailable`, `install_failed`, `content_invalid`, `rosetta_required`, `runtime_unavailable`, `prerequisite_failed`, `launch_not_started`, `launch_early_exit`, `launch_exit_nonzero` |
| `duration_bucket` | `lt_1s`, `lt_10s`, `lt_1m`, `lt_5m`, `lt_30m`, `ge_30m`. A duration on a bound falls in the higher band: exactly 1 s is `lt_10s` |
| `os`, `arch` | `windows`, `macos`, `linux`; `x86_64`, `aarch64` |

The source of truth for the rows is [rows.rs](../../../crates/admin-api/src/routes/telemetry/launcher_summary/rows.rs); the wire contract is in the module documentation of [launcher_summary/mod.rs](../../../crates/admin-api/src/routes/telemetry/launcher_summary/mod.rs).

## Dashboard

| Field | Value |
|---|---|
| Title | `Cimmeria — Launcher journey` |
| Export | [launcher-journey.dashboard.json](launcher-journey.dashboard.json) |
| Tags | `cimmeria`, `launcher`, `launcher-summary` |

Every panel is a table whose only aggregation is `count()`. Every filter starts with `service.name = 'cimmeria-client' AND scope_name = 'launcher.summary' AND`, which the table below leaves out.

| Panel id | Title | Question it answers | Filter (after the common clause) | Group by |
|---|---|---|---|---|
| `attempts-by-version-os-outcome` | Attempts by launcher version, OS and outcome | How did received attempts end, per launcher build and platform? | `event = 'launcher_summary'` | `launcher_version`, `os`, `outcome` |
| `failures-by-operation-phase-error` | Failures by operation, phase and error code | Where do failed attempts fail, and with which code? | `event = 'launcher_summary' AND outcome = 'failed'` | `operation`, `phase`, `error_code` |
| `phase-durations` | Phase durations by operation and phase (sample counts) | How long does each timed phase take? | `event = 'launcher_phase'` | `operation`, `phase`, `duration_bucket` |
| `attempt-durations` | Attempt durations by operation | How long does a whole attempt take? Launches carry no duration | `event = 'launcher_summary'` | `operation`, `duration_bucket` |

### Reading the duration panels

- **The count is the sample size.** The duration panels show how many rows fell in each bucket, not a mean or a percentile. A bucket with three rows is three observations.
- **Timings exist only for attempts the launcher watched from admission.** An attempt reported after a launcher restart has no `duration_ms` and no phase rows, and a failure before the attempt was admitted (a platform, compatibility, catalog, manifest or destination check) has no duration either. The launcher never estimates a timing it did not measure.
- **A launch is timed only while it prepares.** A launch reports the duration of `starting` (the preparation before the game process is started) and no other timing: it has no `running` phase row and no `duration_ms`. Both would measure the length of the play session, not the launch, so the launcher does not report them. The attempt panel therefore shows every launch under the empty bucket.
- **Install phases accumulate.** An install downloads and unpacks the seed and then each patch. `download` is the total across the seed and every patch, and so is `extraction`. `extraction` also includes content verification and promotion after the last unpack, because the launcher enters no further timed phase after it.
- **A failed or cancelled attempt's last phase is in the phase panel.** It ends at the failure or the cancellation, so it is shorter than the same phase of an attempt that completed. The `launcher_phase` row has no `outcome` field; join on `attempt_id` to tell the two apart.
- **An `unknown` attempt has no duration and no timing for the phase that was open.** The launcher lost sight of the attempt, so it saw neither end.
- **An empty `duration_bucket` in the attempt panel** is the count of attempts without a duration: launches, `unknown` attempts, failures before admission and attempts reported after a launcher restart. It is expected, and it tells you how much of the cohort the buckets cover.
- **`duration_bucket` is derived by the server** from the reported milliseconds, so the panels need no numeric aggregation.

### Why attempts and phases have different event names

One attempt produces one `launcher_summary` row and up to four `launcher_phase` rows, all under the same `scope_name`. If the phase rows shared the summary's `event`, a count of attempts would count an attempt once for the summary and once more for each timed phase.

The two names keep the counts apart: a panel that counts attempts filters on `event = 'launcher_summary'`, and the phase panel filters on `event = 'launcher_phase'`. A filter on `scope_name = 'launcher.summary'` alone mixes the two, so never count on the scope alone. To follow one attempt across both, use `attempt_id`, which both rows carry.

## Saved view

| Field | Value |
|---|---|
| Name | `Launcher summary — ingest batches` |
| Export | [launcher-summary.view.json](launcher-summary.view.json) |
| Category, tags | `launcher`; `launcher`, `launcher-summary` |
| Filter | `service.name = 'cimmeria-server' AND scope_name = 'launcher.ingest' AND event = 'launcher_summary_batch'` |
| Columns | `accepted`, `duplicate`, `rejected`, `client_dropped_overflow`, `client_dropped_expired`, `client_dropped_rejected`, `body` |

It answers "is the ingest healthy?" with one row per request:

- `accepted`, `duplicate` and `rejected` are the server's verdicts on that request's summaries. A steady stream of `duplicate` means launchers are resending summaries the server already has (a lost response). `rejected` means a launcher sent a summary the server's rules refuse, which points at a launcher and server that disagree about the contract.
- `client_dropped_*` are what the launcher says it discarded before sending: `overflow` (its queue was full), `expired` (older than 24 hours) and `rejected` (the server refused an earlier batch). These are the attempts the dashboard is missing.
- `client_dropped_*` are at-least-once approximations. The launcher clears a counter only when the server answers the request that carried it, so a request it has to retry repeats the same numbers on a second batch row. Do not add them up across rows as if each row reported new drops.

A request refused before its summaries are validated (kill switch, quota, a bad token, a malformed body or envelope) writes no batch row.

## Importing

Import after the server has ingested its first rows, not before. SigNoz rejects a filter on a key it has never ingested (`key ... not found`; see the note in [npc-ai-views.md](npc-ai-views.md#saved-logs-explorer-views)), and several keys here exist nowhere else: `launcher_version`, `duration_bucket`, `error_code` and the batch counters. Whether a group-by on an unknown key is refused in the same way is untested. To be safe, wait until these rows exist:

| Object | Needs |
|---|---|
| Saved view | one `launcher_summary_batch` row (the first request that reaches validation) |
| Attempt panels | one `launcher_summary` row; for the failure panel, one with `outcome = 'failed'`, which is the only kind that carries `error_code` |
| Duration panels | one summary that carries timings, which also writes the first `launcher_phase` rows |

Then:

1. **Dashboard.** In the SigNoz UI open **Dashboards → New dashboard → Import JSON** and paste the file, or pass its `title`, `description`, `tags`, `layout` and `widgets` to the SigNoz MCP tool `signoz_create_dashboard`. SigNoz assigns its own query ids on import.
2. **View.** Pass the file's content to the SigNoz MCP tool `signoz_create_view`, or paste the filter into the Logs Explorer search bar, add the columns, and use **Save view** with the name and category above.
3. Record the ids SigNoz assigns in this page, as [npc-ai-views.md](npc-ai-views.md) does, and correct anything the import changed.

If SigNoz refuses a file, fix the file and this page together and rerun the guard below.

## The offline guard

`crates/admin-api/src/routes/telemetry/launcher_summary/fixture_tests/` reads both JSON files in a unit test. It needs no SigNoz and no network:

```bash
bash tools/build-lane/lane.sh cargo nextest run -p cimmeria-admin-api fixture_tests
```

It posts the launcher's golden request through the real ingest, captures the rows, and checks the fixtures against them:

- Every key in a filter, a group-by, an aggregation or a column list is a field of the rows that query's `event` selects, or one of `service.name`, `scope_name`, `severity_text`, `body`. A field renamed in `rows.rs` fails the test until the fixture follows.
- Every literal compared with a closed-set field is a member of that set, `scope_name` is the row's real target, and `service.name` is the index the row is exported to.
- Every aggregation is `count()`.
- The five views above are present by title, each counts the `event` its title promises, and `event` is fixed with a single `=` (no `OR`, no `IN`), so no attempt count can take in phase rows.
- The dashboard description still carries the statements under [Who is counted](#who-is-counted).
- Each panel description still carries its reading notes: what a launch `succeeded` means, what a pre-admission row and `phase = none` stand for, and what the duration panels leave out (`PANEL_CAVEATS` in `fixture_tests/mod.rs`).
- A query the guard cannot read (an unparseable expression, a ClickHouse or metrics panel, an unknown key) fails the test instead of being skipped.

To add or change a panel, edit the JSON and the `VIEWS` table in `fixture_tests/mod.rs` together; the table records which `event` each title counts. A new panel also needs a `PANEL_CAVEATS` entry. The guard proves the fixtures agree with the server. It does not prove SigNoz accepts them.
