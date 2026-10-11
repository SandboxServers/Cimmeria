# Lab watch: work packets

> Type: work packets. Audience: `packet-coder` workers (Haiku),
> `documentation-writer` for LW-06, and `packet-reviewer` reviewers (Sonnet).
> Ledger, findings (F1 to F16) and decisions (D-LW1 to D-LW7):
> [README.md](README.md).
>
> PowerShell only. Rust checks go through the lane, in this order, from the
> worktree root:
>
> ```powershell
> pwsh -NoProfile -File tools/build-lane/lane.ps1 cargo fmt -p cimmeria-lab
> pwsh -NoProfile -File tools/build-lane/lane.ps1 cargo clippy -p cimmeria-lab --all-targets -- -D warnings
> pwsh -NoProfile -File tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-lab
> ```
>
> The `tools/lab` PowerShell tests compile nothing, so they run directly,
> exactly as CI's `scripts` job runs them:
>
> ```powershell
> Get-ChildItem tools/lab -Recurse -Filter 'test-*.ps1' | Sort-Object FullName | ForEach-Object {
>     pwsh -NoProfile -File $_.FullName
>     if ($LASTEXITCODE) { throw "$($_.Name) exited $LASTEXITCODE" }
> }
> ```
>
> PowerShell files are CRLF, start with a `<# .SYNOPSIS .DESCRIPTION .EXAMPLE #>`
> block, and set `Set-StrictMode -Version Latest` and
> `$ErrorActionPreference = 'Stop'`. They run under `pwsh` 7. Libraries
> (`*-lib.ps1`, `common.ps1`) define functions only. Never print a token or a
> lease id. Rust rules: no `unwrap()` outside tests (a poisoned mutex is
> `lock().unwrap_or_else(|e| e.into_inner())`), comments say why, test
> modules last, no file over 700 lines.
>
> **No packet here uses the lab** except LW-05: no lease, no client, no
> `lab install`, `lab restart` or `lab uat`, and no `cimmeria-lab` or
> `lab-server` MCP tool. A test that would need the live lab reads a
> recorded fixture instead.

## Contents

- [Contract](#contract)
- [LW-01 Daemon: UAT progress on /status](#lw-01-daemon-uat-progress-on-status)
- [LW-02 lab uat: batch files for watchers](#lw-02-lab-uat-batch-files-for-watchers)
- [LW-03 watch-lib: snapshot, text, JSON, fixtures](#lw-03-watch-lib-snapshot-text-json-fixtures)
- [LW-04 The lab watch command](#lw-04-the-lab-watch-command)
- [LW-05 Live acceptance](#lw-05-live-acceptance)
- [LW-06 Close-out](#lw-06-close-out)

## Contract

Parallel packets build against these names. A packet that needs to change
one stops and tells the coordinator.

### `/status` per-instance `uat` (LW-01 serves it, LW-03 and LW-04 read it)

Every instance row gains `uat`, after `lease`. It is `null` when the
instance runs no `lab_uat_run` (and during a `plan_only` call):

```json
{ "instance": "p2", "account": "lab2", "bridge_port": 8771, "client_pid": 51200,
  "lease": { "held": true, "lease": { "owner": "lab_uat_run", "purpose": "UAT run, sections first-session, rows FS-01 FS-02 FS-P1 FS-P2 FS-P3 FS-P4 FS-P5", "remaining_s": 1700 } },
  "uat": { "run_id": "cfzzhprq", "run_dir": "C:\\Users\\x\\AppData\\Local\\cimmeria-lab\\uat-runs\\2026-10-11-unknown-cfzzhprq",
           "started_at": "2026-10-11T01:39:30.120Z", "rows_total": 7, "rows_done": 3,
           "section": "first-session", "current_row": "FS-P2", "row_index": 4,
           "row_started_at": "2026-10-11T01:40:41.500Z",
           "last_row": "FS-P1", "last_result": "PASS" } }
```

- `row_index` is 1-based among the rows this run selected; `current_row`,
  `row_index` and `row_started_at` are `null` between a row's finish and the
  next row's start, and after the last row.
- Times are `runner::utc_of` strings (RFC 3339, milliseconds, `Z`).
- A daemon from before LW-01 has no `uat` key at all. Readers treat a
  missing key as "unknown" and `null` as "idle".

Rust, new file `crates/lab/src/uat/progress.rs`:

```rust
/// One instance's running `lab_uat_run`, as `/status` shows it (D-LW1).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RunProgress {
    pub run_id: String,
    pub run_dir: String,
    pub started_at: String,
    pub rows_total: usize,
    pub rows_done: usize,
    pub section: Option<String>,
    pub current_row: Option<String>,
    pub row_index: Option<usize>,
    pub row_started_at: Option<String>,
    pub last_row: Option<String>,
    pub last_result: Option<String>,
}

/// Shared between an instance's `Supervisor` (read by `/status`) and the
/// runner of that instance's current `lab_uat_run` (writes).
#[derive(Debug, Clone, Default)]
pub struct ProgressCell(std::sync::Arc<std::sync::Mutex<Option<RunProgress>>>);

impl ProgressCell {
    /// Starts a run's progress; the guard clears it when dropped.
    pub fn begin(&self, run_id: &str, run_dir: &str, rows_total: usize, now_ms: i64) -> ProgressGuard;
    /// Sets section, current_row, row_index (= rows_done + 1) and row_started_at.
    pub fn row_started(&self, section: &str, row: &str, now_ms: i64);
    /// rows_done += 1; last_row/last_result set; current_row, row_index, row_started_at cleared.
    pub fn row_finished(&self, row: &str, result: &str);
    pub fn snapshot(&self) -> Option<RunProgress>;
    /// `serde_json::to_value(snapshot)`, or `Value::Null` when idle.
    pub fn to_json(&self) -> serde_json::Value;
}

/// Clears the cell on drop, so an early return, an error or a lost lease
/// never leaves a stale "running" row on `/status`.
#[must_use]
pub struct ProgressGuard { cell: ProgressCell }
impl Drop for ProgressGuard { fn drop(&mut self) { /* *lock = None */ } }

impl<'a, I: ToolInvoker> Runner<'a, I> {
    pub fn with_progress(self, cell: ProgressCell) -> Self;
    /// None for a plan_only run or when no cell was given.
    pub(crate) fn begin_progress(&self) -> Option<ProgressGuard>;
    pub(crate) fn progress_row_started(&self, section: &str, row: &str);
    pub(crate) fn progress_row_finished(&self, ev: &RowEvidence);
}
```

`row_started` and `row_finished` on an idle cell (no `begin`) do nothing.

### Batch files (LW-02 writes them, LW-03 reads them)

`<uat root>\batch-<yyyyMMdd-HHmmss>\plan.json`, written once, right after
the folder is made and before any run starts:

```json
{ "schema": 1, "section": "first-session",
  "rows": ["FS-01", "FS-02", "FS-P1", "FS-P2", "FS-P3", "FS-P4", "FS-P5"],
  "lanes": [ { "lane": 1, "instance": "default" }, { "lane": 2, "instance": "p2" } ],
  "runs_per_lease": 3, "started": "2026-10-11T01:39:06.0000000Z", "pid": 41236 }
```

`batch.jsonl` records keep every field they have today (F2) and gain two,
UTC ISO 8601 (`(Get-Date).ToUniversalTime().ToString('o')`):

```text
Started   when the run's lab_uat_run call began (a lane-death record: when it was written)
Ended     just before the record is appended
```

`batch.json` and `summary.md` are unchanged.

### Library functions

| File | Function | Packet | What it does |
|---|---|---|---|
| `common.ps1` | `Select-LabLogLine` | LW-02 (moved from `logs.ps1`, unchanged) | the log filter and token mask |
| `uat-lib.ps1` | `Get-UatRoot` | LW-02 (moved from `uat.ps1`, unchanged; needs `common.ps1`) | the folder batches and runs live in |
| `uat-lib.ps1` | `Get-UatKnownIssue([string]$Row, [string]$Reason)` | LW-02 | the first `$script:UatKnownIssues` entry whose `Rows` and `Reason` patterns match, as its `Issue` (`'#1341'`), else `$null` |
| `uat-lib.ps1` | `Format-UatProgressLine($Record, [int]$Total)` | LW-02 | the exact progress line `lab uat` prints today (below) |
| `uat-lib.ps1` | `Write-UatPlan([string]$Path, $Plan)` / `Read-UatPlan([string]$Path)` | LW-02 | `plan.json` out (UTF-8, no BOM) and in (`$null` when missing or not JSON) |
| `watch-lib.ps1` | the `*-LabWatch*` functions | LW-03 | see LW-03 |

`Format-UatProgressLine` returns, for a record `$r` and the lane's runs per
lease `$n`, exactly what `uat.ps1:214-216` builds today:

```text
[default 1/3] PASS (114s)
[p2 1/3] FAIL FS-P2: clause movie-over failed: client_entity_find/count gte 1 [known #1341] (130s)
[p2 2/3] ERROR lab_uat_run failed: bridge down (3s)
```

### `lab watch` (LW-04 is the command, LW-03 the logic)

```text
lab watch [-Batch <folder>] [-Once] [-Json] [-IntervalSeconds 2]
```

- `-Json` implies `-Once`. With output redirected, the live view prints one
  `-Once` snapshot instead.
- Exit codes: 0 a snapshot was printed, or the live view ended with Ctrl+C;
  1 the daemon is down and there is no batch to show; 2 usage (an
  `-IntervalSeconds` outside 1 to 60, or a `-Batch` folder with neither
  `plan.json` nor `batch.jsonl`).
- Hidden test seam: `-StatusFile <path>` reads the `/status` JSON from a file
  instead of the daemon, and `-LogFile <path>` reads that file instead of
  `labd.log`. Both are for tests and for reading a saved snapshot.

The `-Once` text (the live view draws the same lines each tick, with the
time in the header and `Ctrl+C to quit`):

```text
batch-20261011-013906  first-session  live  runs 3/6  passed 1  failed 2 (2 known)
last failure  default#2  FS-01 BLOCKED: setup failed: lab_login: Login_PasswordEdit holds 0 ch...  [#1343]

INSTANCE  CLIENT  LEASE                                      RUN  ROW          SHOT  LAST
default   49448   lab_uat_run (UAT run, sections first-se...)  3/3  FS-P3 42s    5s    [default 2/3] FAIL FS-01: setup failed: lab_login: ... (88s)
p2        51200   lab_uat_run (UAT run, sections first-se...)  2/3  FS-02 3s     -     [p2 1/3] FAIL FS-P2: clause movie-over failed [known #1341] (130s)
p3        -       free                                       -    -            -     -

orphan SGW.exe: 63904 (#1342?)  relaunches: p2 1  login retries: 1 (#1343)
```

- LEASE is `free`, or `<owner> (<purpose>)` with the purpose cut to 40
  characters plus `...`, through `Hide-LeaseIds`. Never `remaining_s`.
- RUN is `<i>/<n>`: while the batch is live and the lane has runs left and
  no brake, `i` is its record count + 1; otherwise its record count.
  A braked lane reads `<count>/<n> braked`. An instance not in the batch
  shows `-`.
- ROW is `<current_row> <seconds since row_started_at>s` from `/status`
  `uat`, `-` when `uat` is `null`, `?` when the daemon has no `uat` key (the
  footer then says `daemon has no run progress: lab install, then lab restart`).
- SHOT is the age of the newest `*.png` under `<run_dir>\rows\`, run_dir
  from `/status` `uat` (else the lane's last record's `RunDir`), or `-`.
- LAST is `Format-UatProgressLine` of the lane's last record.
- Every line is cut to the window width (120 in tests).

`-Once -Json` (one line; keys in this order; a key whose value would be
null, empty or zero is left out, except `passed` and `failed`):

```json
{"ok":true,"daemon":"up","batch":"batch-20261011-013906","state":"live","section":"first-session","runs":"3/6","passed":1,"failed":2,"known":2,"last_failure":{"run":"default#2","why":"FS-01 BLOCKED: setup failed: lab_login: Login_PasswordEdit holds 0 ch...","issue":"#1343"},"lanes":{"default":"3/3 FS-P3 42s shot 5s","p2":"2/3 FS-02 3s"},"orphans":[63904],"relaunches":1,"login_retries":1}
```

- `daemon` is `up` or `down`. `state` is `live`, `done`, `interrupted`,
  `unknown` or `none` (no batch: then no `batch`, `section`, `runs`, counts
  or `lanes`).
- `runs` is `<records>/<lanes x runs_per_lease>`, or `<records>` without a
  plan. `known` counts failed records with an issue tag.
- `why` is the record's `"<row> <result>: <reason>"` (or `"error: <error>"`),
  through `Hide-LeaseIds`, cut to 72 characters with `...`.
- A lane's value is `"<RUN> <row> <row_s>s shot <shot_s>s"`, leaving out the
  parts that are `-`.
- `orphans`: at most 3 pids. `relaunches` and `login_retries`: totals since
  the batch started.
- Budget (D-LW6): a two-lane batch under 500 characters, the worst five-lane
  case under 700.

## LW-01 Daemon: UAT progress on /status

**Implementer:** packet-coder. **Size:** M. **Wave:** 1. **Depends on:** D-LW1.
**Branch:** `lab-watch/lw01-status-progress`. **Worktree:** `lw01`.
**Subject:** `feat(lab): LW-01 per-instance UAT run progress on /status`

Why: README F4, F5, F6, F14.

Files:

1. New `crates/lab/src/uat/progress.rs` (contract). Module doc: what the
   cell is for (`lab watch`, docs/analysis/lab-watch/), that it carries no
   lease data, and that the guard clears it on every exit path. Times via
   `crate::uat::runner::utc_of(now_ms)`. The `impl Runner` block holds
   `with_progress`, `begin_progress`, `progress_row_started` and
   `progress_row_finished`:
   - `begin_progress`: `None` when `self.req.plan_only` or
     `self.progress.is_none()`; else `cell.begin(&self.manifest.run_id,
     &self.run.root.display().to_string(), total, now_ms())`, where `total`
     counts the rows `run_all` will run: every row of every section in
     `self.req.sections` that passes the same `self.req.rows` filter
     `run_all` applies (`mod.rs:235`).
   - `progress_row_finished(ev)`: `cell.row_finished(&ev.row_id, ev.result.as_str())`.
2. `crates/lab/src/uat/mod.rs`: `pub mod progress;` (alphabetical) and a
   doc bullet: "[`progress`] — the running row, for `GET /status`".
3. `crates/lab/src/uat/runner/mod.rs`:
   - field `pub(crate) progress: Option<crate::uat::progress::ProgressCell>`
     after `revoked`, with a doc comment; `progress: None` in `new`.
   - in `run_all`, after `let sections = ...`:
     `let _progress = self.begin_progress();` with the comment
     `// Cleared when run_all returns, on every path (D-LW1).`
   - around each row: `self.progress_row_started(&loaded.spec.section.id, &row.id);`
     before `run_row`, and `self.progress_row_finished(&ev);` after
     `finish_row(&ev)?`.
   - `#[cfg(test)] mod progress_tests;` beside the other test modules.
4. `crates/lab/src/supervisor/mod.rs`: field
   `uat_progress: crate::uat::progress::ProgressCell` (doc: "this instance's
   running `lab_uat_run`, for `/status`"), `uat_progress: Default::default()`
   in `new`, and `pub fn uat_progress(&self) -> &crate::uat::progress::ProgressCell`
   next to `leases()`.
5. `crates/lab/src/server/uat.rs`, in `lab_uat_run` where the runner is
   built (`:462`): add `.with_progress(self.supervisor.uat_progress().clone())`
   after `.with_p2(second)`. One line; the file is at 660 lines, so add
   nothing else there.
6. `crates/lab/src/daemon/status.rs`: `instance_row` gains a last parameter
   `uat: Value` and puts `"uat": uat` after `"lease"`; the handler passes
   `h.supervisor.uat_progress().to_json()`. Update the module doc (the row
   now carries the running UAT row). Grep `instance_row(` for other callers.

Tests:

- In `progress.rs` (unit, TESTING.md type 1):
  - `begin_then_rows_report_the_current_row_and_clear_on_drop`: `begin("r1",
    "C:\\runs\\r1", 2, 1_000)`, `row_started("s", "A", 2_000)` → snapshot
    `current_row == Some("A")`, `row_index == Some(1)`, `rows_done == 0`;
    `row_finished("A", "PASS")` → `current_row None`, `rows_done 1`,
    `last_row Some("A")`, `last_result Some("PASS")`; `row_started("s", "B",
    3_000)` → `row_index Some(2)`; drop the guard → `snapshot()` is `None`
    and `to_json()` is `Value::Null`. Fails if the guard does not clear or
    `row_index` is not `rows_done + 1`.
  - `an_idle_cell_ignores_row_updates`: `row_started` and `row_finished`
    with no `begin` leave `snapshot()` `None`.
- New `crates/lab/src/uat/runner/progress_tests.rs` (unit over the existing
  `Fake`, type 1). Copy `ROWS` from `revocation_tests.rs` without its
  `{ wait_ms = 60000 }` step (rows R1 `.help`, R2 `.who`). A `Spy<'a> {
  inner: &'a Fake, cell: ProgressCell, seen: Mutex<Vec<(String,
  Option<RunProgress>)>> }` implements `ToolInvoker` by delegating to
  `inner` and, on each `call`, pushing `(args["text"] as a string or "",
  cell.snapshot())` first.
  - `progress_names_the_row_being_driven_and_clears_after_the_run`: run
    `Runner::new(&spy, None, request(&tmp, ROWS)).unwrap().with_progress(cell.clone()).run_all()`.
    The entry whose text is `.help` has `current_row Some("R1")`,
    `row_index Some(1)`, `rows_total 2`; the `.who` entry has
    `current_row Some("R2")`, `rows_done 1`, `last_result Some("PASS")`;
    after `run_all`, `cell.snapshot()` is `None`. Fails if `run_all` drops
    either hook or the guard.
  - `a_plan_only_run_never_begins_progress`: build the runner with
    `req.plan_only = true` and `.with_progress(cell.clone())`, and assert
    `runner.begin_progress().is_none()` before calling `run_all`. Fails if
    `begin_progress` ignores `plan_only` (a plan would then show a run on
    `/status` that drives nothing).
- In `status.rs`: extend `instance_row_has_the_contract_shape` with
  `row["uat"].is_null()` for a `Value::Null` argument, and add
  `instance_row_carries_the_uat_progress` (a `json!({ "current_row": "FS-P3" })`
  argument comes back at `row["uat"]["current_row"]`).
- In `crates/lab/src/daemon/http_tests.rs` (integration over the real
  router), new `status_reports_the_current_uat_row`: take
  `server.instances().first().supervisor.clone()`, `let g =
  sup.uat_progress().begin("cfzzhprq", "C:\\runs\\x", 7, 1_000)`,
  `row_started("first-session", "FS-P3", 2_000)`, spawn the daemon, GET
  `/status` with the token: `rows[0]["uat"]["current_row"] == "FS-P3"` and
  `rows[0]["uat"]["rows_total"] == 7`, and the body has no `lease-`
  followed by hex. Then `drop(g)`, GET again: `rows[0]["uat"]` is null.
  Fails if the handler passes `Value::Null` or reads another instance's
  cell. Also add `assert!(rows[0]["uat"].is_null())` to
  `status_lists_instances_without_lease_ids`.

Checks: the three lane commands in the header.

Docs owed (doc-update map, "Live research lab" row):
`docs/architecture/live-research-lab.md`, the `GET /status` bullet (`:277`):
add one sentence that each instance row also carries `uat`, its running
`lab_uat_run` (run id, run directory, rows done and total, the current row
and when it started), or `null`, for `lab watch` (D-LW1 of the
[lab watch ledger](../analysis/lab-watch/README.md)).

Reviewer focus: the guard is alive for the whole of `run_all` (a `let _ =`
would drop it at once); `plan_only` never sets the cell; the cell is the
routed instance's (F14); no lease data in `RunProgress`; `server/uat.rs`
stays under 700 lines.

## LW-02 lab uat: batch files for watchers

**Implementer:** packet-coder. **Size:** S. **Wave:** 1. **Depends on:** D-LW4, D-LW5.
**Branch:** `lab-watch/lw02-batch-files`. **Worktree:** `lw02`.
**Subject:** `feat(lab-cli): LW-02 plan.json, record times and a shared progress line for lab watch`

Why: README F1, F3, F8, F9, F10.

Files:

1. `tools/lab/cli/common.ps1`: move `Select-LabLogLine` here from
   `logs.ps1:35-67`, unchanged, with its comment. Add to the `.SYNOPSIS`
   that it holds the log filter.
2. `tools/lab/cli/logs.ps1`: delete the function; it already dot-sources
   `common.ps1`. Its `.DESCRIPTION` keeps describing the filter.
3. `tools/lab/cli/uat-lib.ps1`:
   - move `Get-UatRoot` here from `uat.ps1:81-89`, unchanged. Its comment
     gains "Needs common.ps1".
   - add the #1343 entry to `$script:UatKnownIssues`, after #1341:
     `@{ Issue = '#1343'; Rows = '.'; Reason = 'Login_PasswordEdit holds 0 characters'; Note = 'login password box read back empty on a slow-booting client' }`.
   - `Get-UatKnownIssue([string]$Row, [string]$Reason)` (contract), and
     `Get-UatRunVerdict` uses it instead of its own loop (`uat-lib.ps1:205`).
   - `Format-UatProgressLine($Record, [int]$Total)` (contract): the body of
     `uat.ps1:214-216`, reading the record through `Get-UatField` (records
     are hashtables in the lane and `pscustomobject`s when read back).
   - `Write-UatPlan` and `Read-UatPlan` (contract). `Write-UatPlan` uses
     `ConvertTo-Json -Depth 6` and `[IO.File]::WriteAllText` with
     `UTF8Encoding($false)`; `Read-UatPlan` returns `$null` on a missing
     file or a parse error.
4. `tools/lab/cli/uat.ps1`:
   - delete `Get-UatRoot` (now in the library).
   - move `$started = Get-Date` above `$batchDir`, and after `$jsonl` is set
     call `Write-UatPlan (Join-Path $batchDir 'plan.json')` with the
     contract's object: `schema = 1`, `section = $Section`, `rows = $rowIds`,
     `lanes` from `$plan.Lanes` as `[ordered]@{ lane; instance }`,
     `runs_per_lease = $RunsPerLease`,
     `started = $started.ToUniversalTime().ToString('o')`, `pid = $PID`.
   - in the lane loop, `$rec` gains `Started = $t0.ToUniversalTime().ToString('o')`
     and `Ended = $null`; set `$rec.Ended = (Get-Date).ToUniversalTime().ToString('o')`
     just before `Add-UatRecord`. The lane-death record gets both as now.
   - the progress `Write-Host` uses
     `Format-UatProgressLine $rec $n`; the two follow-up lines (lease did
     not clear, brake) stay.
   - `.DESCRIPTION`: "Every batch writes plan.json first, then batch.jsonl
     (one line per run, as each finishes), then batch.json and summary.md".
5. `tools/lab/cli/test-uat.ps1`, new checks (unit, type 1, no daemon):
   - `Format-UatProgressLine` gives the three contract lines exactly, for a
     PASS record, a FAIL record tagged #1341, and an ERROR record. Fails if
     the wording drifts from what `lab uat` prints.
   - `Get-UatRunVerdict` on a `BLOCKED` FS-01 whose reason contains
     `Login_PasswordEdit holds 0 characters after typing, expected 4` is
     tagged `#1343`; `Get-UatKnownIssue 'FS-P3' 'clause aim failed'` is
     `$null`. Fails without the new entry.
   - `Write-UatPlan` then `Read-UatPlan` round-trips `section`, two lanes
     and `pid`; `Read-UatPlan` on a missing path and on a file holding `{`
     returns `$null`.
   - A record with `Started` and `Ended` still aggregates through
     `Merge-UatResults` exactly as one without them.
6. `tools/lab/cli/test-common.ps1`: one check that `Select-LabLogLine` is
   defined after dot-sourcing `common.ps1` alone. Fails if the move is
   reverted. (`test-logs-env.ps1` keeps passing unchanged: it dot-sources
   `logs.ps1`, which dot-sources `common.ps1`.)

Checks: the PowerShell test loop in the header. No Rust.

Docs owed: `docs/guides/live-research-lab.md`, the `lab uat` "Files" bullet
(`:1016`): `plan.json` first (section, rows, lanes, runs per lease, start
time and the `lab uat` pid), and `Started`/`Ended` on each `batch.jsonl`
line. The known-issues sentence (`:1014`) adds `#1343` for "the login
password box read back empty".

Reviewer focus: the progress line is byte-identical to today's; `plan.json`
is written before the first lane starts (so a watcher never sees runs
without a plan); `$using:` values in the parallel block still resolve; no
behaviour change in the summary or the exit codes.

## LW-03 watch-lib: snapshot, text, JSON, fixtures

**Implementer:** packet-coder. **Size:** M. **Wave:** 2. **Depends on:** LW-02 merged.
**Branch:** `lab-watch/lw03-watch-lib`. **Worktree:** `lw03`.
**Subject:** `feat(lab-cli): LW-03 lab watch snapshot library with recorded fixtures`

Why: README F2, F7, F8, F12, F15, F16; D-LW3 to D-LW6. Builds against
LW-01's `/status` contract through fixtures, so it does not wait for LW-01.

Files:

1. New `tools/lab/cli/watch-lib.ps1`. Header: "Library for `lab watch`:
   pure functions over a `/status` object, a batch folder, `labd.log` lines
   and `SGW` pids. Dot-source it after common.ps1 and uat-lib.ps1. Never
   print a token or a lease id." Functions:

   | Function | Returns |
   |---|---|
   | `Find-LabWatchBatch([string]$UatRoot)` | the full path of the `batch-*` folder with the greatest name, or `$null` |
   | `Read-LabWatchRecords([string]$Path)` | the `batch.jsonl` records; a line that does not parse is skipped, never thrown (F16) |
   | `Read-LabWatchBatch([string]$Dir, [scriptblock]$IsAlive)` | `[ordered]@{ Dir; Name; Plan; Records; State }` with State per D-LW5; `$IsAlive` takes a pid and returns a bool (default: `Get-Process -Id` succeeds) |
   | `Get-LabWatchIssue($Record)` | `Get-UatKnownIssue` on the record's `Verdict.Row` and `Verdict.Reason`, else the record's own `Verdict.Issue`; `$null` for an `Error` record |
   | `Get-LabWatchBatchPanel($Batch)` | `[ordered]@{ section; runs; total; passed; failed; known; last_failure = @{ run; why; issue } or $null; lanes = [ordered]@{ <instance> = @{ count; total; braked; last } } }`; `last_failure` is the last failed record in file order; `run` is `"<instance>#<index>"` |
   | `Get-LabWatchShotAge([string]$RunDir, [datetime]$Now)` | whole seconds since the newest `*.png` under `$RunDir\rows` (recursive), or `$null` |
   | `Get-LabWatchLogCounts([string[]]$Lines, [string]$Since)` | `@{ relaunches = [ordered]@{ <instance> = n }; login_retries = n }`, from lines whose leading timestamp is at or after `$Since` (string compare of the ISO prefix); a relaunch is a line containing `relaunched after crash`, its instance the first `instance="<x>"` on the line; a login retry contains `credential typing missed` |
   | `Get-LabWatchSnapshot` | the whole model (below) |
   | `Format-LabWatchText($Snapshot, [int]$Width = 120)` | the `-Once` lines (contract), each at most `$Width` characters |
   | `ConvertTo-LabWatchJson($Snapshot)` | the `-Once -Json` string (contract) |

   `Get-LabWatchSnapshot` takes named parameters `-Status` (the `/status`
   object or `$null`), `-BatchDir` (or `$null`), `-LogLines`, `-SgwPids`
   (`[int[]]`), `-Now` (`[datetime]`, UTC) and `-IsAlive`, and returns:

   ```text
   [ordered]@{
     daemon    = 'up' | 'down'
     old_daemon = $true when /status rows have no 'uat' key
     batch     = Get-LabWatchBatchPanel output plus Name and State, or $null
     instances = one row per /status instance (else per plan lane):
                 @{ instance; client_pid; lease; run; row; row_s; shot_s; last }
     orphans   = SGW pids not among /status client_pids (none when the daemon is down)
     logs      = Get-LabWatchLogCounts since the plan's start (or $null without a plan)
   }
   ```

   Rules: lease text and every string from a record or the log go through
   `Hide-LeaseIds` (D-LW3); the lease purpose is cut to 40 characters plus
   `...`; RUN, ROW and SHOT as in the contract; `row_s` is
   `$Now - row_started_at` in whole seconds.
2. New fixtures in `tools/lab/cli/fixtures/watch/` (recorded shapes:
   `batch.jsonl` lines copy the real record of F2, with run directories
   rewritten to `C:\lab\uat-runs\...`; the log lines copy real `labd.log`
   lines; nothing from a real machine's paths, accounts or tokens):
   - `status-two-lanes.json`: `default` (client 49448) and `p2` (51200) each
     leased by `lab_uat_run` with the full first-session purpose, `p2`'s
     purpose ending in a space and `lease-0123456789abcdef0123456789abcdef` to prove
     redaction; `uat` blocks (`default` at FS-P3, `row_started_at` 42 s
     before the tests' `$Now`; `p2` at FS-02, 3 s); `p3` free with
     `"uat": null`.
   - `status-old-daemon.json`: the same instances with no `uat` key.
   - `batch-20261011-013906/plan.json` and `batch.jsonl`: the contract's
     plan (2 lanes x 3 runs, pid 41236), and three records: `default#1`
     PASS; `p2#1` FS-P2 FAIL `clause movie-over failed: client_entity_find/count gte 1`
     with `Issue = "#1341"`; `default#2` FS-01 BLOCKED `setup failed:
     lab_login: Login_PasswordEdit holds 0 characters after typing, expected 4`
     with `Issue = null` (re-classified to #1343), then a half-written
     fourth line `{"Lane":2,"Instance":"p2","Ind`.
   - `batch-20261010-193906/batch.jsonl`: the real two-record PASS batch
     (pre-LW-02 shape: no `Started`, no `plan.json`), plus `batch.json`.
   - `labd-tail.log`: six lines in `labd.log`'s format, including
     `2026-10-11T01:44:04.474224Z  INFO serve_inner:lab_call{instance="p2"}:lab_watchdog{instance="p2" pid=57012}: cimmeria_lab::supervisor::watchdog: relaunched after crash; logging back in new_pid=77860`,
     one relaunch line timestamped before the plan's start (must not
     count), and one `lab_login: credential typing missed; typing both again`
     line for `default`.
3. New `tools/lab/cli/test-watch.ps1` (no Pester, no daemon, exits non-zero
   on any failed check; same `Check` helper as `test-uat.ps1`). It
   dot-sources `common.ps1`, `uat-lib.ps1` and `watch-lib.ps1`, copies the
   fixtures to a temp folder, and uses a fixed
   `$Now = [datetime]'2026-10-11T01:41:23Z'` (UTC). Checks (unit, type 1):
   - `Find-LabWatchBatch` picks `batch-20261011-013906` over
     `batch-20261010-193906` (name order, after setting the older folder's
     `LastWriteTime` to be the newer: fails if it sorts by time).
   - `Read-LabWatchRecords` returns 3 records and skips the half line.
   - `Read-LabWatchBatch` states: `live` with `{ $true }`, `interrupted`
     with `{ $false }`, `done` once a `batch.json` is written into the temp
     copy, and the old batch reads `done` (it has `batch.json`) with a
     `$null` plan; delete its `batch.json` and it reads `unknown`.
   - Panel: `runs 3`, `total 6`, `passed 1`, `failed 2`, `known 2` (fails
     if only the record's own `Issue` is trusted), `last_failure.run ==
     'default#2'`, `issue '#1343'`.
   - Instances from `status-two-lanes.json`: RUN `default` `3/3` (two
     records, live, `+1`), `p2` `2/3`, `p3` `-`; ROW `FS-P3 42s` and
     `FS-02 3s`; no output string anywhere in the snapshot or its text or
     JSON contains `0123456789abcdef` (fails without `Hide-LeaseIds`); the
     lease text is cut with `...`.
   - The same with `status-old-daemon.json`: ROW `?` and `old_daemon`.
   - A braked lane (add a record with `Braked = true`) reads `<n>/3 braked`.
   - `Get-LabWatchShotAge`: a temp run dir with `rows\s\A\final.png` whose
     `LastWriteTimeUtc` is `$Now - 5 s` gives 5; a dir without `rows` gives
     `$null`.
   - `Get-LabWatchLogCounts` on `labd-tail.log` since the plan start:
     `relaunches.p2 == 1` (the earlier line excluded), `login_retries == 1`.
   - Orphans: `-SgwPids 49448, 63904` with the two-lane status gives
     `63904` only; with `-Status $null`, none.
   - JSON: the two-lane snapshot parses, has no `null`, and is under 500
     characters; a built worst case (5 lanes, `20/20` runs, a 600-character
     reason, 5 orphans, relaunches on every instance) is under 700 and
     shows 3 orphans; `-Status $null` with no batch gives
     `{"ok":true,"daemon":"down","state":"none"}`.
   - Text: every line of `Format-LabWatchText $snap 120` is at most 120
     characters; it has the `INSTANCE` header and a `p3 ... free` row and
     the footer's `orphan SGW.exe: 63904 (#1342?)`.
   - The pre-LW-02 batch's real records aggregate with no error (strict
     mode on records without `Started`).

Checks: the PowerShell test loop in the header.

Docs owed: none in this packet (LW-04 documents the command).

Reviewer focus: every function is pure (no `Get-Process`, no network, no
`Get-Date` inside; `$Now` and `$IsAlive` come in); strict mode on optional
fields (`reasons`, `Started`, `uat`); the JSON leaves out empty keys rather
than writing `null`; nothing in the fixtures comes from a real home folder.

## LW-04 The lab watch command

**Implementer:** packet-coder. **Size:** S. **Wave:** 3. **Depends on:** LW-03 merged.
**Branch:** `lab-watch/lw04-watch-command`. **Worktree:** `lw04`.
**Subject:** `feat(lab-cli): LW-04 lab watch, a live read-only view of a lab uat batch`

Why: the command around LW-03's library; D-LW2.

Files:

1. New `tools/lab/cli/watch.ps1` (`#requires -Version 7`). `.SYNOPSIS`
   first line (it is the `lab help` line): `Watch a running lab uat batch,
   read-only: lab watch [-Batch <folder>] [-Once] [-Json] [-IntervalSeconds 2].`
   `.DESCRIPTION`: the panels, D-LW3's read-only rule, the exit codes, the
   test seams. `param([string]$Batch, [switch]$Once, [switch]$Json,
   [int]$IntervalSeconds = 2, [string]$StatusFile, [string]$LogFile)`.
   Dot-sources `common.ps1`, `uat-lib.ps1`, `watch-lib.ps1`.
   - `Get-LabWatchInputs`: status from `$StatusFile`
     (`Get-Content -Raw | ConvertFrom-Json`) or `Get-LabStatus`; the batch
     folder from `$Batch` or `Find-LabWatchBatch (Get-UatRoot)`; log lines
     from `$LogFile` or `labd.log` under `Get-LabHome`, with
     `Get-Content -Tail 4000` (absent file: none); SGW pids from
     `Get-Process SGW -ErrorAction SilentlyContinue`; `$Now` =
     `(Get-Date).ToUniversalTime()`.
   - usage checks first (exit 2, `lab uat`'s error shape under `-Json`):
     `-IntervalSeconds` 1 to 60; a `-Batch` folder must exist and hold
     `plan.json` or `batch.jsonl`.
   - `-Once`, `-Json`, or `[Console]::IsOutputRedirected`: one snapshot,
     printed with `[Console]::Out.WriteLine` (JSON) or the text lines; exit
     1 when the daemon is down and there is no batch, else 0.
   - the live view (D-LW2): write `ESC[?1049h` and `ESC[?25l`, then loop:
     build the snapshot, prepend a header line
     `lab watch  <local time HH:mm:ss>  every <n>s  Ctrl+C to quit`, write
     `ESC[H`, each line cut to `[Console]::WindowWidth - 1` followed by
     `ESC[K`, then `ESC[J`; `Start-Sleep -Seconds $IntervalSeconds`. A
     `finally` writes `ESC[?25h` and `ESC[?1049l`. A failure inside one
     tick prints its first line (through `Hide-LeaseIds`) on the frame and
     the loop goes on.
2. `tools/lab/cli/test-watch.ps1`: add command cases (run
   `pwsh -NoProfile -File watch.ps1 ...` with `CIMMERIA_LAB_HOME` pointing
   at a temp folder, restored afterwards; integration, type 1):
   - `-Once -Json -Batch <fixture copy> -StatusFile <status-two-lanes.json> -LogFile <labd-tail.log>`
     exits 0 and prints one line that parses, has `"state"` and `"lanes"`,
     and is under 500 characters (with the fixture's pid not alive the
     state is `interrupted`; assert it is one of the five states).
   - `-IntervalSeconds 0` exits 2; under `-Json` it prints
     `{"ok":false,"exit":2,...}`.
   - `-Batch <empty temp folder>` exits 2.
   - Redirected output without `-Once` (the test captures stdout) prints
     one snapshot and exits instead of looping (run it with a 20 s timeout
     through `Start-Process -Wait` or a job, and fail on timeout).
3. `docs/guides/live-research-lab.md`:
   - the Commands table (`:984`): a row after `lab uat`:
     `lab watch [-Batch <folder>] [-Once] [-Json] [-IntervalSeconds 2]` |
     a live, read-only view of a running `lab uat` batch; see below | `0;
     1 daemon down and no batch; 2 usage`.
   - a `**lab watch.**` paragraph after the `lab uat` block: what each panel
     shows and where it comes from (`/status` and its `uat` field, the batch
     folder's `plan.json` and `batch.jsonl`, the run directory's
     screenshots, `labd.log`), the batch states, `-Once` and `-Json` with
     the contract's example and budget, the known-issue tags (#1341 and
     #1343 by signature, #1342 as an orphan `SGW.exe` plus relaunch counts),
     the screenshot caveat (F7: one `final.png` per row for the
     first-session rows; `-` outside a run), and that it takes no lease.
     Example: `lab watch` in one terminal while
     `lab uat first-session -Rows ... -Leases 2 -RunsPerLease 3` runs in
     another.
4. `docs/guides/automated-uat.md`, "Run without an agent: `lab uat`"
   (`:59`): one sentence that `lab watch` shows a running batch live, and
   `lab watch -Once -Json` gives an agent a snapshot without waiting for the
   batch to end.

Checks: the PowerShell test loop in the header. Also run `lab help` from
the worktree (`pwsh -NoProfile -File tools/lab/lab.ps1 help`) and confirm
the `watch` line; it reads no lab state.

Docs owed: the guide and `automated-uat.md` rows above (doc-update map,
"Live research lab" row).

Reviewer focus: the terminal is restored on Ctrl+C and on an error
(`finally`); nothing in the command takes a lease or calls `/mcp`; the
redirected path never enters the loop; the test seams are documented as
seams; no `--once` anywhere in the docs (F11).

## LW-05 Live acceptance

**Implementer:** coordinator. **Size:** S. **Wave:** 4.
**Depends on:** LW-01 to LW-04 merged; D-LW7 (the user's OK, and the lab
free). **Status until then:** BlockedDecision.
**Branch:** `lab-watch/lw05-acceptance` (ledger and worknote only).
**Worktree:** `lw05`. **Subject:** `docs(lab-watch): LW-05 live acceptance`

The handoff's acceptance: watching a `-Leases 2 -RunsPerLease 3` batch shows
both lanes progressing, and `-Once -Json` stays under 500 characters.

Steps, after the user says the lab is free and agrees to a daemon restart:

1. `lab install -From <main checkout>` then `lab restart` (this closes the
   lab clients). `lab doctor` passes; `lab status` lists `default` and
   `p2`.
2. In a background PowerShell job:
   `lab uat first-session -Rows FS-01,FS-02,FS-P1,FS-P2,FS-P3,FS-P4,FS-P5 -Leases 2 -RunsPerLease 3 -Json`.
3. Every 60 s until the job ends, `lab watch -Once -Json`, each output
   appended to `worknotes/LW-05.md` with its time.
4. Ask the user to look at the live view (`lab watch` in a terminal) for a
   minute while the batch runs, and to Ctrl+C it; record what they saw.

Pass when:

- across the snapshots, both `lanes.default` and `lanes.p2` change their run
  index and their row, and at least one snapshot shows both lanes on a row
  at the same time;
- every snapshot is under 500 characters and none contains `lease-`
  followed by hex;
- the last snapshot after the job ends reads `state` `done` and `runs`
  `6/6`, with `passed` and `failed` equal to the `lab uat -Json` summary;
- a failure, if any, carries the same issue tag in both outputs;
- the terminal is normal after the user's Ctrl+C.

Record the result in this ledger's packet table and
`docs/guides/unified-uat.md` (through LW-06). A failed criterion is a bug
packet, not a retry.

## LW-06 Close-out

**Implementer:** documentation-writer. **Size:** S. **Wave:** 5.
**Depends on:** LW-05. **Branch:** `lab-watch/lw06-close-out`.
**Worktree:** `lw06`. **Subject:** `docs(lab-watch): LW-06 close-out`

1. This ledger: every packet's final status, the review outcomes, and the
   campaign status line.
2. `docs/guides/unified-uat.md`: the `lab watch` live-view check as an
   owner step if LW-05's user check is still open, else its result.
3. `docs/project-status.md` (and `docs/gap-analysis*` only if they list lab
   tooling): one line for `lab watch`, in the close-out only.
4. A project memory under `.claude/agent-memory/main-session/` only for a
   non-obvious fact LW-05 turned up (for example a terminal that ignores the
   alternate screen), with its `MEMORY.md` line.
5. A handoff post on the board's lab-watch subcategory, and retire every
   `lw0*` worktree (`pwsh tools/build-lane/rm-worktree.ps1 --merged`).

Docs owed: the rows above; `docs/readme.md` only if a doc was added or
renamed.
