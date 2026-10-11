# Lab CLI in Rust: ports and close-out

> Type: packet specs, LX-10 to LX-20. Dispatch rules and the contract
> every packet builds against are in [work-packets.md](work-packets.md);
> read its "## Contract" first. Ledger: [README.md](README.md).

Common to every port below:

- Port `origin/main`'s PowerShell as of your worktree's base. Read the
  command file whole and its `-lib` file by function (Grep `^function`).
- The command's `run` replaces the fallback body in
  `crates/lab-cli/src/commands/<name>.rs`; split into
  `commands/<name>/` with `mod.rs` re-exporting `Args` and `run` when it
  passes about 400 lines.
- Delete the command's `.ps1` and its test file. Delete a `-lib.ps1` only
  when no remaining `.ps1` in `tools/lab/` dot-sources it (Grep for its
  file name first); say which you kept and why.
- Port each `Check` of the test file to a `#[test]`, one-to-one, naming
  the old check in a comment.
- Checks: the three lane commands in the dispatch rules.
- Docs: the command's row and paragraph in
  `docs/guides/live-research-lab.md` § The `lab` command use `--flag`
  names (CRLF).

## LX-10: `record`

**Branch** `lab-cli-rust/lx-10-record`, worktree `lx-10`.
**Commit** `feat(lab-cli): LX-10 native lab record`.

Source: `tools/lab/cli/record.ps1`, `record-lib.ps1` (all of it).
Tests: `test-record.ps1` (70 checks).

- Verbs `start <section>`, `status`, `stop`, `transform <id or folder>`
  over `McpClient::call_tool` with `lab_record_start`, `lab_record_status`,
  `lab_record_stop`, `lab_record_transform`. Argument checks
  (`Test-RecordArguments`) run before the daemon is asked (exit 2).
- The state file `<lab home>\record-<instance>.json`
  (`Save-RecordState`/`Read-RecordState`), deleted only after a
  successful `stop`.
- Exit codes 0 ok, 1 tool failed, 2 usage or refused, 3 no daemon, 4
  instance busy. `--json` per `ConvertTo-RecordCompactJson`.
- Fancy mode: `start` prints `● recording p2 · section first-session ·
  stops by itself in 30m`; `status` prints a small key/value block
  (instance, age, ticks, anchors); `stop` prints `✓ draft written:
  <path>` in cyan.

## LX-11: `golden`

**Branch** `lab-cli-rust/lx-11-golden`, worktree `lx-11`.
**Commit** `feat(lab-cli): LX-11 native lab golden`.

Source: `golden.ps1`, `golden-lib.ps1`. Tests: `test-golden.ps1` (84).

- `record`: run the batch by spawning this program:
  `std::env::current_exe()` with `uat <section> --fingerprint
  --progress-to-stderr --json` plus the rows, runs and leases arguments
  `Get-GoldenBatchArgs` builds. Stream the child's stderr to ours as it
  arrives; parse its stdout JSON (`ConvertFrom-GoldenBatchOutput`). This
  works whether `uat` is native or still PowerShell. Then
  `lab_golden_record` over MCP.
- `show`: reads `<specs>/golden/<section>.json` with no daemon
  (`Get-GoldenSummary`/`Format-GoldenSummary`).
- Exit codes 0/1/2/3/6/7 per the guide; `--json` per
  `ConvertTo-GoldenCompactJson`.
- Fancy mode: `show` prints a key/value block with the section as a bold
  heading; `record` ends with `✓ golden first-session: 7 rows, 312 events
  from 5 runs` and the path in cyan.

## LX-12: `calibrate`

**Branch** `lab-cli-rust/lx-12-calibrate`, worktree `lx-12`.
**Commit** `feat(lab-cli): LX-12 native lab calibrate`.

Source: `calibrate.ps1`, `calibrate-lib.ps1`. Tests: `test-calibrate.ps1`
(47).

- Put the heal functions `Get-UatHealFileName` and `Invoke-UatHeal` in
  `commands/calibrate/heal.rs` as `pub` items: LX-17 calls them. They
  take the MCP call and the lease wait as closures, as the PowerShell
  takes scriptblocks, so tests pass stubs.
- `--apply`: `git apply` in this checkout only; the dirty-spec check runs
  before the sweep and again before applying (`Test-CalibrateSpecDirty`,
  `Invoke-CalibrateApply`, `Get-CalibrateDiffTarget`).
- Keep `calibrate-lib.ps1`: `uat.ps1` still dot-sources it until LX-17.
- Fancy mode per pose: `✓ frost  score 8312 · confirmed 3/3 · bearing
  322.5 → 330.0`, `· frost  unchanged`, `✗ frost  no proposal: <reason>`;
  the diff printed with `+` lines green and `-` lines red.

## LX-13: `watch` model, `--once`, `--json`

**Branch** `lab-cli-rust/lx-13-watch-model`, worktree `lx-13`.
**Commit** `feat(lab-cli): LX-13 native lab watch snapshot and json`.

Source: `watch.ps1`, `watch-lib.ps1`, and from `uat-lib.ps1`
`Get-UatKnownIssue`, `Read-UatRecords`, `Read-UatPlan`. Tests:
`test-watch.ps1` (72) and its fixtures under `tools/lab/cli/fixtures/watch/`
(move them to `crates/lab-cli/tests/fixtures/watch/`).

New shared modules (declared in `lib.rs`), which LX-15 to LX-17 reuse:

- `known_issues.rs`: the issue table and `fn tag(reason: &str) ->
  Option<&'static str>` (`#1341`, `#1343`).
- `batch/mod.rs`: `Plan`, `RunRecord` serde types for `plan.json` and
  `batch.jsonl` (field names exactly as written today), `read_plan`,
  `read_records`, `find_latest_batch`.

`commands/watch/{mod,snapshot,text,json}.rs`: `snapshot.rs` builds a
`Snapshot` from the batch folder, a `StatusReply` and the log tail
(`Get-LabWatchSnapshot`); `text.rs` renders it (plain = the PowerShell
frame); `json.rs` renders the one-line JSON (`ConvertTo-LabWatchJson`,
same keys and budgets). This packet ships `--once` and `--json`; without
them it prints one snapshot too (LX-14 adds the live view).
`--status-file` and `--log-file` stay hidden test seams.

Delete `watch.ps1`, `watch-lib.ps1`, `test-watch.ps1`. Keep `uat-lib.ps1`.

## LX-14: `watch` live view

**Branch** `lab-cli-rust/lx-14-watch-live`, worktree `lx-14`.
**Commit** `feat(lab-cli): LX-14 lab watch live view`.

Files: `crates/lab-cli/src/commands/watch/live.rs`, `watch/mod.rs`.

- Fancy mode without `--once`/`--json`: crossterm alternate screen,
  hidden cursor, redraw every `--interval-seconds` (1 to 60, default 2),
  quit on `q`, `Esc` or Ctrl+C (raw mode reads keys with
  `event::poll`), and always restore the screen and cursor (a guard
  struct whose `Drop` leaves the alternate screen, so a panic restores it
  too). Plain mode (redirected) prints one snapshot and exits, as today.
- Frame layout, fancy: a header line (batch, section, state as a coloured
  word: `live` green, `done` default, `interrupted` red), a progress bar
  `███████░░░░ 4/6` sized to the width, the instance `Table`, the last
  failure with its issue tag, the footer. A frame taller than the window
  is cut with a last line `… <n> more lines`.
- Exit 0 on quit.

Tests: the frame renderer at widths 60, 100 and 160 keeps every line
within the width; the guard restores on drop (call it on a fake writer
and check the escape sequences written).

## LX-15: `uat` arguments, plan, pre-flight, `--plan-only`

**Starts after lab-chaos CH-08 (#1651) merges.** Port that merged code.
**Branch** `lab-cli-rust/lx-15-uat-plan`, worktree `lx-15`.
**Commit** `feat(lab-cli): LX-15 lab uat arguments, plan and pre-flight`.

Source: `uat.ps1` (up to the lane loop) and from `uat-lib.ps1`
`Test-UatArguments`, `ConvertTo-UatRowIds`, `Format-UatChaosHeader`,
`Set-UatChaos`, `Add-UatChaosHeader`, `Get-UatPlan`, `Write-UatPlan`.
Tests: the matching checks of `test-uat.ps1` (list them in the report).

`commands/uat/{mod,args,plan,preflight}.rs`:

- argument checks first, exit 2 before the daemon is asked (including a
  row id the section does not have: exact and case sensitive);
- pre-flight: the daemon answers, the lanes' instances are free, no
  `SGW.exe` outside the lab (reuse `doctor/system.rs`'s pid scan; LX-08
  made it `pub(crate)`, or make it so), the plan-only pass of the rows via
  `lab_uat_run` with `plan_only`; failure exits 3;
- `--plan-only` prints the plan and exits 0. Writes `plan.json` through
  `batch::Plan`.
- A real run (no `--plan-only`) still runs through `fallback::run` until
  LX-16. Keep `uat.ps1`.

## LX-16: `uat` lanes, brake, timeouts, batch files, progress, summary

**Branch** `lab-cli-rust/lx-16-uat-lanes`, worktree `lx-16`. Size L: a
continuation is expected; write the worknote before 85k.
**Commit** `feat(lab-cli): LX-16 lab uat lanes, brake and batch summary`.

Source: the lane loop in `Invoke-LabUat` (`uat.ps1`) and from
`uat-lib.ps1` `Add-UatRecord`, `Get-UatRunVerdict`, `Test-UatBrake`,
`Test-UatStopLane`, `Merge-UatResults`, `Format-UatSummary`,
`Write-UatBatchSummary`, `Format-UatProgressLine`, `Get-UatProgressMode`,
`Write-UatProgress`, and `Wait-UatLeaseFree` (already in `mcp.rs`).

`commands/uat/{lanes,record,summary,progress}.rs`:

- one thread per lane (`std::thread::scope`), lanes started
  `--stagger-seconds` apart; each run takes its own `McpClient`, calls
  `lab_uat_run` with a timeout of `--run-timeout-minutes`, appends a line
  to `batch.jsonl` under a `Mutex` (`Started`/`Ended` UTC), applies the
  brake (`--max-consecutive-failures`, 0 never) and the 10-minute lease
  wait after a timeout;
- Ctrl+C (`ctrlc`-free: a `static AtomicBool` set from a
  `SetConsoleCtrlHandler` on Windows; on other OSes the default
  behaviour is fine) starts no new run and still writes the summary;
- `batch.json` and `summary.md` exactly as today; lease ids redacted with
  `hide_lease_ids`;
- progress: plain mode prints the PowerShell progress lines (stdout, or
  stderr under `--progress-to-stderr`); fancy mode prints the § Look and
  feel mock-up lines, then the summary as a per-row and per-lane `Table`
  with pass rates coloured (100 % green, 0 % red, else yellow) and the
  failure groups with their known-issue tags.

A real run now runs natively. Keep `uat.ps1` for `--diff-golden` and
`--heal` (fall back when either is given) until LX-17.

Tests: the lane, brake, verdict, merge, summary and progress checks of
`test-uat.ps1`, one-to-one, with the MCP call behind a closure so tests
pass a stub (as the PowerShell passes a scriptblock).

## LX-17: `uat` golden diff, heal, `--json`, exit codes

**Branch** `lab-cli-rust/lx-17-uat-golden-heal`, worktree `lx-17`.
**Commit** `feat(lab-cli): LX-17 lab uat golden diff, heal and json; retire the uat PowerShell`.

Source: from `uat-lib.ps1` `Get-UatGoldenValue`, `Get-UatRunGolden`,
`Test-UatGoldenFile`, `Merge-UatGolden`, `Get-UatExitCode`,
`Limit-UatText`, `ConvertTo-UatCompactJson`, `Invoke-UatHealPass`,
`Get-UatHealHints`; heal through LX-12's `commands::calibrate::heal`.

`commands/uat/{golden,heal,json}.rs`; exit codes 0/1/2/3/4/5/8 with 8
winning, as `Get-UatExitCode`. Remove the LX-16 fallback for
`--diff-golden`/`--heal`.

Delete `uat.ps1`, `uat-lib.ps1`, `test-uat.ps1`, and
`calibrate-lib.ps1` if nothing dot-sources it any more. Run
`Grep "uat-lib|calibrate-lib" tools/` and fix any remaining reference.
Add a line to `docs/analysis/lab-chaos/README.md` § Review outcomes (or
its notes section): "From LX-17 on, `lab uat` is Rust
(`crates/lab-cli/src/commands/uat/`); chaos packets change it there."

Tests: the remaining `test-uat.ps1` checks (golden, heal, JSON budget,
exit codes), one-to-one.

## LX-18: Native wrappers, `completions`, delete the dispatcher

**Branch** `lab-cli-rust/lx-18-cleanup`, worktree `lx-18`.
**Commit** `feat(lab-cli): LX-18 lab completions; retire the PowerShell dispatcher`.

- `start`, `stop`, `restart`: run `pwsh -NoProfile -File <cli dir>\daemon.ps1
  <verb> [-Force]` directly; `instances`: `<cli dir>\instances.ps1`;
  `install`, `setup`, `local-setup`: `<cli dir>\cli\<name>.ps1`. Pass the
  original tokens; return the child's exit code. `fallback.rs` becomes
  this one helper (`run_script(path, raw)`); the generic dispatcher path
  goes.
- `completions <shell>` (`powershell`, `bash`, `zsh`, `fish`) via
  `clap_complete`, plus `lab completions powershell --install`, which
  appends `lab completions powershell | Out-String | Invoke-Expression`
  to `$PROFILE.CurrentUserAllHosts` once (it checks for the line first)
  and prints what it did.
- Delete `tools/lab/lab.ps1`, `tools/lab/cli/help-lib.ps1` if present,
  the `start/stop/restart/instances.ps1` wrappers under `tools/lab/cli/`,
  and whatever of `common.ps1` nothing uses (Grep each function). Update
  `install-lib.ps1`'s copy list and `test-install-lib.ps1` to match,
  and `test-common.ps1` (delete it if nothing it tests remains).

Tests: `completions powershell` output contains `Register-ArgumentCompleter`
and every command name; the install step is idempotent against a temp
profile path (`--profile-path` hidden test seam).

## LX-19: Docs close-out

**Agent** `documentation-writer`. **Branch** `lab-cli-rust/lx-19-docs`,
worktree `lx-19`. **Commit** `docs(lab-cli): LX-19 lab.exe in the guides; close the campaign`.

- `docs/guides/live-research-lab.md` § The `lab` command: rewrite for
  `lab.exe` (setup, where things live, `--flag` forms with a note that
  PowerShell forms still work, completions, `NO_COLOR`).
- `docs/analysis/lab-cli/README.md`: D-LC1 marked superseded by D-LX1.
- `.claude/skills/lab-uat/` and any skill or agent text that runs
  `pwsh tools/lab/lab.ps1` or names a deleted `.ps1`: point at `lab`.
  Run `python tools/agent-skills/sync.py`.
- `tools/README.md` and `crates/README.md` prose (not generated blocks).
- `docs/project-status.md` and `docs/gap-analysis.md`, once, if they
  mention the lab CLI.
- This ledger: statuses, review outcomes, a resume note; the unified UAT
  guide (`docs/guides/unified-uat.md`) gains LX-20's steps if LX-20 is
  still owed.
- Main-session project memory: one reference memory on `lab.exe`
  (layout, fallback, flag rewriting), committed here.

## LX-20: Live check

**Coordinator**, with the user's OK before any lab client is driven.
From a fresh `lab install -From <main checkout>` on the workstation, run
and record in this ledger: `lab`, `lab help uat`, `lab stauts`,
`lab version`, `lab status`, `lab doctor`, `lab env`, `lab logs --lines 5
--instance p2`, `lab logs --follow` (Ctrl+C), `lab watch --once`,
`lab golden show first-session`, `lab completions powershell --install`
in a new terminal (tab completes `lab u` → `lab uat`), then with the
user's OK `lab uat first-session --rows FS-01,FS-02,FS-P1 --plan-only`
and one real run with `lab watch` in a second terminal, and
`lab clients stop <instance>` on an idle client. Plain-mode spot check:
each of `status`, `doctor`, `watch --once` piped to `Out-File` matches the
PowerShell format.
