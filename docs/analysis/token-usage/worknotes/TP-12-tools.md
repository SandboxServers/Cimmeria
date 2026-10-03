# TP-12 worknote: scheduled jobs and the TP-05b follow-ups

> Type: worknote. Tools half of packet TP-12 of [#957](https://github.com/SandboxServers/Cimmeria/issues/957); ledger [README.md](../README.md). Branch `feat/token-profile-scheduled`. Written 2026-10-03 by the TP-12 tools worker. The ledger row, the guide and the status docs belong to the docs worker and the coordinator.

## Done

- **TP-05b follow-ups, re-applied.** The two CodeRabbit fixes that the TP-05b worker wrote after its PR merged were lost with its worktree. Its edit script survived and was re-applied to current `main` unchanged:
  - `validate/score.py`: a labelled request's weight that its attribution rows don't account for (no rows, or rows summing under 1) counts as unattributed. Before, a request with no rows dropped out of the totals and a request with half its weight placed scored as fully placed. The ingest refuses both (`attribution_imbalance`), so the real-data numbers in [TP-05b.md](TP-05b.md) do not change; this guards a hand-edited or partial database.
  - `validate/cli.py`: an `--out` it can't write exits 2 with `status=error` instead of a traceback.
  - The edit script carried no tests, so `validate/test_validate.py` is new: with either fix reverted, its test fails (checked: 2 failures and 1 error with both reverted, and each fix's test fails alone).
- **`validate --max-wrong-share`**, new: exit 4 when one method misplaces more than that share of a set's USD, so the weekly job can fail on the attribution.md 10% rule instead of only printing it.
- **The backfill result** is in [TP-05b.md § Backfill](TP-05b.md#backfill): 426 PRs merged since 2026-09-13, 404 comments created, 2 already current, 20 with no attributed request, 0 errors or gate refusals.
- **Scheduled jobs**, `tools/token-profile/scheduled/`:
  - `jobs.py` (run as `python tools/token-profile/scheduled weekly|pr-sweep`): the two jobs, a lock so they don't write the database at once, each step a child process with its own log, and a summary that names each failed step and what its exit code means.
  - `weekly.ps1` and `pr-sweep.ps1`: find Python and run `jobs.py`, passing every option through.
  - `register-tasks.ps1`: registers or updates the two Task Scheduler tasks for the current user, `-WhatIf` prints, `-Unregister` removes. Not run; the coordinator runs it after the merge.
  - `test_jobs.py`: windows, the lock, step order and arguments, exit codes, with a fake runner. 14 tests; 205 token-profile tests pass in all.
- README: a [Scheduled jobs](../../../../tools/token-profile/README.md#scheduled-jobs) section and the validate options.

## Smoke runs (2026-10-03, on a copy of the database)

- `weekly`, full: ingest, reconcile (profiler 7.7% under the cost-state over the 7-day window), validate and report all passed in 43 s. The output folder held the reports, `reconcile.md`, `attribution-check.json`, each step's log and `weekly-summary.txt`.
- `weekly --days 1 --skip-ingest`: reconcile was out of tolerance on the one-day window; the job still ran validate and report and exited 1 naming reconcile. Short windows have few sessions, so expect this below about a week.
- `pr-sweep --dry-run --days 1 --rate 60/min --skip-ingest`: 19 PRs printed, nothing posted, 88 s.
- `pr-sweep.ps1` with no Python on PATH: exit 2 and a line in `<home>\logs\scheduled.log`.
- `register-tasks.ps1 -WhatIf`: refuses from a worktree whose scripts aren't on the main checkout yet; with that check pointed at the worktree, it printed both tasks. `-Unregister -WhatIf` reported both as not registered.

## Decisions taken here

- **Logic in Python, thin `.ps1` wrappers**, so the window and exit-code logic is unit-tested and the jobs run the same from Git Bash.
- **The sweep reuses `pr_stats --backfill`** with `--restart` and its own state file, rather than new code: the backfill already rate-limits, stops after three `gh` errors in a row, and posts through the idempotent upsert. `--restart` makes each run look at every PR in the window, so a comment made stale by a later ingest is edited, and a current one is left alone (`unchanged`). A comment's stamp holds the profiler commit, so a profiler change pulled into the main checkout edits every comment in the window once.
- **Windows are whole UTC days**, matching the reports' UTC timestamps: the weekly report covers the 7 UTC days before the run's UTC date, the sweep PRs merged since 3 UTC days before it. A Monday 08:00 run in UTC-5 covers Monday to Sunday UTC.
- **A failed step doesn't stop the rest**, so an out-of-tolerance reconcile still leaves a report to read.
- **Task triggers in local time.** `New-ScheduledTaskTrigger` writes its start as UTC, which pins a task to UTC and moves it an hour at each daylight-saving change; the script rewrites it without an offset.
- **Tasks run the main checkout's scripts**, and registration refuses otherwise: a worktree is deleted when its PR merges.

## For the guide (`docs/guides/token-profiling.md`)

- Script names: `tools/token-profile/scheduled/weekly.ps1`, `pr-sweep.ps1`, `register-tasks.ps1`; the logic is `scheduled/jobs.py`.
- Register after merging and pulling `main`, from the main checkout: `pwsh tools/token-profile/scheduled/register-tasks.ps1 -WhatIf`, then without `-WhatIf`. Remove with `-Unregister`. Task names `\Cimmeria\TokenProfile-Weekly` (Mondays 08:00 local) and `\Cimmeria\TokenProfile-PrSweep` (daily 07:30 local); they run only while the user is logged on, store no password, and start late after the machine was off.
- Run by hand: `pwsh tools/token-profile/scheduled/weekly.ps1 [--days 7] [--until YYYY-MM-DD] [--labels <csv>] [--skip-ingest]` and `pwsh tools/token-profile/scheduled/pr-sweep.ps1 [--days 3] [--rate 6/min] [--dry-run] [--skip-ingest]`. `--home`, `--db` and `--repo` also pass through.
- Defaults: job home `TOKEN_PROFILE_HOME`, else `%LOCALAPPDATA%\cimmeria-token-profile`; database `TOKEN_PROFILE_DB`, else `~/token-profile.sqlite` (the same default as `pr_stats`); Python `TOKEN_PROFILE_PYTHON`, else `python`, else `py -3`; labels `TOKEN_PROFILE_LABELS`, else none; repo the main checkout.
- Where output goes: weekly to `<home>\reports\<YYYY-MM-DD>\` (`token-report.md`/`.json`, `reconcile.md`/`.json`, `attribution-check.json`, a log per step, `weekly-summary.txt`); the sweep to `<home>\logs\pr-sweep-<YYYY-MM-DD>.log`, `-ingest.log` and `-summary.txt`, state in `<home>\pr-sweep-state.json`. Nothing is written into the repo, and nothing there should be committed: the ingest log holds local paths.
- Exit codes: 0 all passed, 1 a step failed (the last line names it), 2 usage error, no Python, or `<home>\job.lock` held (taken over after six hours).
- The weekly validate fails at a 10% wrong share for one method (`--max-wrong-share 0.10`); with no labels it scores the authors set only.
- Logs and reports are never pruned; they are a few MB a year.

## Not done

- Nothing posts the weekly report anywhere; it stays local. Publishing it (a Discussion, an artifact) needs an owner decision on where.
- The tasks are not registered; the coordinator runs `register-tasks.ps1` after the merge.
