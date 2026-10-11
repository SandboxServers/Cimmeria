# Lab watch

> Type: ledger. Audience: the coordinator, packet workers and reviewers.
> Opened 2026-10-10 against `main` @ `6996c9403`. Prefix `LW-`. Effort 6 of
> the [lab roadmap handoff](../lab-roadmap/handoff.md). Packet specs:
> [work-packets.md](work-packets.md). Earlier lab ledgers it builds on:
> [lab-cli](../lab-cli/README.md) (`lab status`, `GET /status`) and
> [lab-parallel-clients](../lab-parallel-clients/README.md) (lanes).
>
> **Campaign status (2026-10-10): planned, nothing built.** LW-01 and LW-02
> can start now. The live acceptance (LW-05) needs the user's OK (D-LW7).

## Purpose

`lab watch` is a read-only terminal dashboard for a running `lab uat` batch,
with no inference and no lab lease:

- one panel row per lab instance: client pid, lease (owner and purpose, never
  the id), current run and row, the lane's last progress line, and the age of
  the last screenshot;
- a batch panel that tails `uat-runs\batch-*\batch.jsonl`: pass and fail
  counts, the latest failure signature, known issues tagged (#1341, #1342,
  #1343);
- `-Once` prints one compact snapshot instead of the live view, and
  `-Once -Json` one JSON object for agents (under 500 characters for a
  two-lane batch).

Out of scope: any write to the lab, MCP tool calls, a new daemon endpoint
beyond one field on `/status` (D-LW1), golden-run divergence (a follow-up
once [lab-golden](../lab-golden/README.md) lands), and progress for the
hosted p2 of a two-player row (F13).

## What was found

Against `main` @ `6996c9403`, plus one real batch on this workstation
(`batch-20261010-193906`, two lanes, one run each).

| # | Finding | Packets |
|---|---|---|
| F1 | `batch.jsonl` gets a line only when a run finishes (`Add-UatRecord`, `tools/lab/cli/uat-lib.ps1:170`; called at `uat.ps1:212`). There is no start record and no timestamp on a record. The plan (lanes, runs per lease, rows) is written only at the end, in `batch.json` (`uat.ps1:238`). A watcher cannot tell how many runs a batch will have, or whether a batch without `batch.json` is still running or was killed. | D-LW5, LW-02 |
| F2 | A record is `Lane, Instance, Index, Ok, Rows[{section, row, result, reasons?}], Verdict{Ok, Row, Result, Reason, Issue}, RunDir, Error, Seconds, Braked` (`uat.ps1:188`, confirmed in the real batch). `Index = 0` with `Error = "lane: ..."` is a lane that died outside a run (`uat.ps1:225`). `reasons` is absent when empty (the daemon's compact shaping). | LW-03 |
| F3 | The per-run progress line (`[p2 1/3] FAIL FS-P2: ... [known #1341] (130s)`) is only `Write-Host` inside the `lab uat` process (`uat.ps1:213`). Everything in it is in the record, so it can be rebuilt from `batch.jsonl`. | LW-02 |
| F4 | `/status` gives per instance `instance, account, bridge_port, client_pid, lease` (`crates/lab/src/daemon/status.rs:21`, handler at `:51`). `lease` is `LeaseBook::status_at` (`crates/lab/src/lease/mod.rs:466`): `held`, `lease{owner, purpose, since, renewed_at, expires_at, remaining_s, ttl_s, took_over_from}`, `recent`, `rules`. It never has a lease id (`lease/tests.rs:164`, `daemon/http_tests.rs:201`). Nothing about UAT runs. | LW-01 |
| F5 | The current row is known only inside `Runner::run_all` (`crates/lab/src/uat/runner/mod.rs:230`). The runner logs nothing per row (no `tracing` call under `uat/`). `run.json`'s `rows` changes only when a row finishes (`finish_row`, `mod.rs:279`), and the CLI learns the run directory only when `lab_uat_run` returns (`server/uat.rs:507`). The run's own lease purpose names the requested rows, not the current one (`run_purpose`, `server/uat.rs:316`). So the current row needs the daemon to say it. | D-LW1, LW-01 |
| F6 | A run directory names its account (`RunManifest.account`, `mod.rs:190`) but not its instance, so matching run directories to instances from the CLI would mean joining on `/status`'s `account`. With LW-01's `run_dir` on `/status` it is direct. | LW-01 |
| F7 | During a `lab_uat_run`, screenshots go into the run directory: `rows\<section>\<row>\final.png` at each row's end (`runner/actions.rs:361`), and nothing else for the `first-session` rows (seen in run `2026-10-11-unknown-cfzzhpjy`). A screenshot an agent takes outside a run is saved to the shared `%LOCALAPPDATA%\cimmeria-lab\screenshots\<tool>-<stamp>.png` (`server/compact.rs:87`), with no instance in the name. So "last screenshot age" is the newest `*.png` under the run directory's `rows\`, and `-` outside a run. | LW-03 |
| F8 | The known-issue table has only #1341 (`uat-lib.ps1:25`: row `^FS-[PS]2$`, reason `movie-over`). #1343's failure reads `Login_PasswordEdit holds 0 characters after typing` (issue text), on any row's login. #1342 has no run-reason signature: it shows as an `SGW.exe` that `/status` doesn't list (the check `uat.ps1:126` and `lab doctor` already make), and the watchdog logs `relaunched after crash; logging back in new_pid=N` at INFO (seen in `labd.log`). #1343's retry logs `lab_login: credential typing missed; typing both again`. | D-LW4, LW-02, LW-03 |
| F9 | `Select-LabLogLine` lives in `logs.ps1`, which has a `param()` block (`logs.ps1:23`). Dot-sourcing it from another command rebinds that command's `$Instance`, `$Level` and `$Lines`. It belongs in `common.ps1`. | LW-02 |
| F10 | `Get-UatRoot` (where batches live) is defined in `uat.ps1:83`, a command, not in a library, so another command cannot reuse it. | LW-02 |
| F11 | `lab.ps1` splats its arguments to the command (`lab.ps1:28`, `:65`), so switches are PowerShell switches: `-Once`, not `--once`. `--once` would arrive as a positional string. The handoff's `--once` means `-Once`. | contract |
| F12 | CI runs every `tools/lab/**/test-*.ps1` in the `scripts` job of `.github/workflows/lab.yml`, so a new `test-watch.ps1` needs no workflow edit. Setup copies `cli\*.ps1` except `test-*` and is not recursive (`install-lib.ps1:125`), and the dispatcher lists only `cli\*.ps1` (`lab.ps1:32`), so fixtures under `tools/lab/cli/fixtures/` are neither installed nor commands. | LW-03 |
| F13 | A two-player row leases the hosted p2 with purpose `... (p2)` (`server/uat.rs:360`, `:424`) but runs no runner of its own, so LW-01's progress appears on the run's own instance only. The p2 row still shows its lease. | follow-up |
| F14 | `lab_uat_run` runs on the routed instance: `call_tool` picks `target` from the `instance` argument and calls the tool on it (`server/handler.rs:95`, `:117`), so `self.supervisor` inside `lab_uat_run` is the lane's own supervisor. That is where LW-01 records progress. | LW-01 |
| F15 | The existing compact-JSON precedent is `ConvertTo-UatCompactJson` with a test that a realistic batch stays under 500 characters (`test-uat.ps1:88`). | LW-03 |
| F16 | `Read-UatRecords` (`uat-lib.ps1:180`) throws on a line that is not JSON. A watcher reading while a lane appends can see a half-written last line, so the watch's reader must skip it rather than throw. | LW-03 |

## Decisions

| ID | Status | Decision | Reason |
|---|---|---|---|
| D-LW1 | PROPOSED (coordinator; the handoff allows it when the current row needs it) | **`/status` gains one field per instance, `uat`:** the instance's running `lab_uat_run` (run id, run directory, rows done and total, the current section and row with its start time, the last finished row and result), or `null`. The runner keeps it in a `ProgressCell` on the instance's `Supervisor`, set at each row start and finish and cleared by a guard when the run ends on any path. No new endpoint, no lease data in it. | F5, F6. Every CLI-only way to get the current row (parsing the lease purpose, scanning run directories by account) is a guess that breaks on two lanes with the same section. The runner already knows; the field costs one mutex and changes nothing else on `/status`. |
| D-LW2 | PROPOSED (coordinator). Recommended: **(a)** | **The live renderer.** (a) Plain PowerShell 7: a pure function builds the frame as lines; the loop switches to the alternate screen buffer (`ESC[?1049h`), hides the cursor, and each tick moves home (`ESC[H`), writes each line cut to the window width with `ESC[K`, then `ESC[J`; `finally` restores the screen and cursor, so Ctrl+C leaves the terminal clean. With output redirected it behaves as `-Once`. (b) The PwshSpectreConsole module. (c) A Rust TUI (ratatui) in `crates/lab`. | (a) needs nothing installed, keeps every rule in the testable pure function, and Windows Terminal and conhost on Windows 11 both honour these sequences. (b) is a new module dependency with its own version pinning, absent on CI and other machines. (c) needs a new binary built and installed for a few panels whose data the PowerShell libraries already read. |
| D-LW3 | PROPOSED (coordinator) | **Read-only and redacted.** `lab watch` reads `GET /status`, the batch folder, the run directory's file times and the tail of `labd.log`, and lists `SGW` processes. It takes no lease and calls no MCP tool. Every string it prints from a lease, a record or a log line goes through `Hide-LeaseIds`; log lines go through `Select-LabLogLine`'s token masking. | The handoff's "read-only, no inference". Lease purposes are free text an agent writes, so a lease id can turn up in one. |
| D-LW4 | PROPOSED (coordinator) | **One known-issue table.** `$script:UatKnownIssues` in `uat-lib.ps1` stays the only list, behind a new `Get-UatKnownIssue`; LW-02 adds #1343. The watch re-classifies every failed record from the table (a record's own `Issue` is used only when the table finds nothing), so an issue added later also tags old batches. #1342 is not a run signature: the watch shows an `SGW.exe` that no instance owns as `orphan SGW.exe: <pid> (#1342?)`, and counts the watchdog's relaunch lines per instance since the batch started. #1343's retry lines are counted as `login retries`, even when the retry worked. | F8. One table means `lab uat`'s summary and `lab watch` never disagree on what is known. |
| D-LW5 | PROPOSED (coordinator) | **Batch files for watchers.** `lab uat` writes `plan.json` (section, rows, lanes, runs per lease, start time, its own pid) when the batch folder is made, and every record gains `Started` and `Ended` (UTC ISO 8601). A batch is `done` when `batch.json` exists, `live` when `plan.json`'s pid is alive, `interrupted` when it isn't, and `unknown` for a batch from before LW-02 (no `plan.json`, no `batch.json`). `lab watch` follows the newest `batch-*` folder by name, or `-Batch <folder>`. | F1. The pid check costs nothing and tells a killed batch from a slow one. Folder names are timestamps (`batch-yyyyMMdd-HHmmss`), so the name order is the time order and file times don't matter. |
| D-LW6 | PROPOSED (coordinator) | **The JSON budget.** `-Once -Json` prints one object with no nulls or empty values: a two-lane batch is under 500 characters, and the worst five-lane case under 700. The failure reason is cut at 72 characters, each lane is one short string (`"2/3 FS-P3 42s shot 5s"`), orphans are capped at 3 pids, and relaunches and login retries are single totals. Failures use `lab uat`'s shape, `{"ok":false,"exit":N,"error":"..."}`. | The handoff's output budget and acceptance line. Per-lane objects would not fit five lanes. |
| D-LW7 | **BlockedDecision** (user) | **The live acceptance (LW-05).** It reinstalls the lab daemon from `main` (`lab install`, `lab restart`, which closes the lab clients) and runs a `-Leases 2 -RunsPerLease 3` batch on the shared lab. | The lab is shared: ask before any lab use, and wait until it's free. |

## Packets

| ID | Packet | Implementer | Size | Wave | Depends on | Status |
|---|---|---|---|---|---|---|
| LW-01 | Daemon: per-instance UAT progress on `/status` | packet-coder | M | 1 | D-LW1 | Ready |
| LW-02 | `lab uat`: `plan.json`, record times, shared progress line and known issues, two moves into libraries | packet-coder | S | 1 | D-LW4, D-LW5 | Ready |
| LW-03 | `watch-lib.ps1`: snapshot model, text and JSON, recorded fixtures, `test-watch.ps1` | packet-coder | M | 2 | LW-02 (and LW-01's contract, not its code) | BlockedDependency |
| LW-04 | `lab watch` command, live redraw, docs | packet-coder | S | 3 | LW-03 | BlockedDependency |
| LW-05 | Live acceptance: a `-Leases 2 -RunsPerLease 3` batch watched | coordinator | S | 4 | LW-01 to LW-04 merged, D-LW7 | BlockedDecision |
| LW-06 | Close-out: ledger, UAT guide, memory, board | documentation-writer | S | 5 | LW-05 | BlockedDependency |

Size: S under about 40k tokens, M 40k to 70k.

Waves (packets in one wave touch disjoint files):

1. LW-01 (`crates/lab/`, `docs/architecture/live-research-lab.md`) and LW-02
   (`tools/lab/cli/common.ps1`, `logs.ps1`, `uat-lib.ps1`, `uat.ps1`, their
   tests, the guide's `lab uat` paragraph).
2. LW-03 (new `watch-lib.ps1`, `test-watch.ps1`, `fixtures/watch/`).
3. LW-04 (new `watch.ps1`, the guide's command table and `lab watch`
   paragraph, one sentence in `automated-uat.md`).
4. LW-05, when the user says the lab is free.
5. LW-06.

## Dispatch rules

- **Workers.** One packet each, in its own worktree:
  `pwsh -NoProfile -File tools/build-lane/mk-worktree.ps1 lab-watch/<packet>-<slug> <worktree>`.
  `packet-coder` (Haiku) for LW-01 to LW-04; `documentation-writer` for
  LW-06. The brief carries the worktree path, the packet's section of
  [work-packets.md](work-packets.md), the Contract section, and the commit
  subject with the attribution lines. No packet needs a test database.
- **Review.** Each finished packet gets a Sonnet `packet-reviewer` on its
  commit range. Fixes go to a fresh worker or the coordinator, never back to
  the implementer.
- **Shell.** PowerShell only: no bash, WSL or Git Bash, no direct `cargo`, no
  `git worktree prune`, no `git stash`. Every compiling command goes through
  `pwsh -NoProfile -File tools/build-lane/lane.ps1`.
- **The lab.** No packet but LW-05 touches the lab: no lease, no client, no
  `lab install`, `lab restart` or `lab uat`, and no `cimmeria-lab` or
  `lab-server` MCP tool. Tests read recorded fixtures only.
- **Ship.** `python tools/build-lane/ship.py pr -C <worktree> -m <msg>`, then
  `python tools/build-lane/ship.py merge <PR> --retire <worktree>` once the
  `lab` workflow's jobs pass (`ship.py` waits for them through `CONDITIONAL`).
  Update this table and write `worknotes/<packet>.md` when anything is left
  over.
- **Shared files.** Only LW-06 edits `docs/guides/unified-uat.md`,
  `docs/project-status.md` and memory indexes.

## Follow-ups (not packets)

- **Golden divergence column.** When [lab-golden](../lab-golden/README.md)'s
  `lab uat --diff-golden` writes its first divergence into the run record,
  the batch panel shows it next to the failure signature.
- **Chaos profile.** When [lab-chaos](../lab-chaos/README.md) adds
  `--chaos <profile>`, `plan.json` should carry the profile and the header
  show it.
- **p2 progress for two-player rows** (F13): the runner could mark the hosted
  p2's cell as `driven by <instance>`.
- **Screenshot attribution outside runs** (F7): an instance label in saved
  screenshot names would let the panel age agent-driven captures too.

## Review outcomes

None yet. Where merged code differs from the packet specs, record it here;
the code is then the reference, not the spec.
