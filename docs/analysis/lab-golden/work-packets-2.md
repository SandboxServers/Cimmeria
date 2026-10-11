# Lab golden runs: work packets, part 2

> GD-08 to GD-11. The lane rules, Rust rules and GD-01 to GD-07 are in
> [work-packets.md](work-packets.md); the shared names are in
> [contract.md](contract.md); the ledger is [README.md](README.md).

## GD-08 lab golden record

**Implementer:** packet-coder. **Size:** M. **Wave:** 5. **Depends on:** GD-07.
**Branch:** `lab-golden/gd08-record-cli`. **Worktree:** `gd08`.
**Subject:** `feat(lab): GD-08 lab golden record and show`

Why: the owner's five-run rule as one command. README D-GD6, F16, F17.

Files:

1. `tools/lab/cli/uat-lib.ps1`: move `Get-LabMcpUrl`, `Get-DefaultSpecsDir`
   and `Get-UatRoot` here from `uat.ps1` unchanged (golden.ps1 needs them).
2. `tools/lab/cli/uat.ps1`: a `-Fingerprint` switch (not in the usage line;
   doc: "used by lab golden") that sets `$baseArgs.fingerprint = $true`.
3. New `tools/lab/cli/golden-lib.ps1` (pure, dot-sourced):
   - `Test-GoldenArguments($Action, $Section, $Runs, $Leases, $ServerVersion)`:
     error text or `$null`. Rules: action `record` or `show`; section
     given; `-Runs` 5 to 20; `-Leases` 1 to 5 and `Runs % Leases -eq 0`;
     `-ServerVersion` matches `^[0-9a-f]{7,40}$` for `record`.
   - `Get-GoldenRunDirs($Records)`: the `RunDir` of every record, in order,
     `$null` when any record is not `Ok` or has no `RunDir`.
   - `ConvertTo-GoldenExit($Reply)`: `ok` 0; `exists` 2; `disagree` 6;
     `not_passing` 1; `cannot_record` 7; anything else 7.
   - `ConvertTo-GoldenCompactJson($Reply, $Exit)`: no nulls; under 300
     characters for a success.
4. New `tools/lab/cli/golden.ps1` (`.SYNOPSIS` first line:
   `Record or show a golden fingerprint: lab golden record <section> -Rows ... [-Runs 5] -ServerVersion <sha> [-SpecsDir <dir>] [-Rebless] [-Json]`).
   `-SpecsDir` works as in `uat.ps1` (default `Get-DefaultSpecsDir`); it is passed to the child `uat.ps1` and as `specs_dir` to `lab_golden_record`, so lab-record (LR-13) can bless a draft from `docs/guides/uat-specs/drafts/`.
   `record`:
   1. Check arguments (exit 2). Refuse when
      `<specs>/golden/<section>.json` exists and `-Rebless` is not set
      (exit 2, "a golden exists; re-blessing needs -Rebless and five fresh runs").
   2. Run the batch as a child process so its `-Json` line is captured:
      `pwsh -NoProfile -File (Join-Path $PSScriptRoot 'uat.ps1') $Section -Rows ($Rows -join ',') -Leases $Leases -RunsPerLease ($Runs / $Leases) -ServerVersion $ServerVersion -SpecsDir $specs -Fingerprint -MaxConsecutiveFailures 1 -Json`.
      Exit 2 or 3 from it passes through; 1 or 4 is exit 1 with its first
      failure (D-GD6: every run must pass).
   3. `Read-UatRecords` on `<batch>\batch.jsonl`, `Get-GoldenRunDirs`.
   4. `lab_golden_record` over `New-McpSession` / `Invoke-McpTool`, then
      `ConvertTo-GoldenExit`.
   5. Print one line (`golden first-session: 7 rows, 312 events, 21456 bytes from 5 runs; <path>`)
      or, under `-Json`, the compact JSON.
   `show <section>`: read the committed file; print section, rows, total
   events, build, evidence run ids (or compact JSON). No daemon.
5. New `tools/lab/cli/test-golden.ps1`, in the style of `test-uat.ps1`:
   argument rules, run-dir extraction (a failed record gives `$null`), the
   exit mapping for every code, compact JSON length and no nulls.

Tests: `pwsh -NoProfile -File tools/lab/cli/test-golden.ps1` and
`pwsh -NoProfile -File tools/lab/cli/test-uat.ps1` (the moved functions).
Type: TESTING.md type 1 (pure PowerShell). Each fails when its rule is
removed.

Checks: the two test scripts; no Rust changes.

Docs owed: the `lab golden` lines in the `lab` command section of
`docs/guides/live-research-lab.md` (§ The `lab` command, ### Commands: usage, exit codes 0, 1, 2,
3, 6, 7, the "don't call lab_timeline on the lab character while
recording" note), and the `lab/` row in `tools/README.md`.

Reviewer focus: the child-process capture (a bare `&` call loses
`[Console]::Out` writes); no lease ids or token in output; `-Runs` below 5
is impossible.

## GD-09 lab uat -DiffGolden

**Implementer:** packet-coder. **Size:** S. **Wave:** 6. **Depends on:** GD-08.
**Branch:** `lab-golden/gd09-diff-cli`. **Worktree:** `gd09`.
**Subject:** `feat(lab): GD-09 lab uat -DiffGolden with one-line divergences`

Why: README D-GD7.

Files:

1. `tools/lab/cli/uat.ps1`: a `-DiffGolden` switch (in the usage line).
   Before the plan: exit 2 when `<specs>/golden/<section>.json` is missing
   ("no golden for <section>; record one with lab golden record"). It sets
   `$baseArgs.fingerprint = $true`. In the lane loop, after a run with a
   `RunDir`, call `lab_golden_diff { run_dir, section, specs_dir }` on the
   same session pattern and set `$rec.Golden` to `match`, the `first` line
   (through `Hide-LeaseIds`, cut to 300 characters) or `error: <msg>`. The
   progress line ends with `; golden: <that>`.
2. `tools/lab/cli/uat-lib.ps1`:
   - `Merge-UatResults`: when any record has `Golden`, add
     `golden = [ordered]@{ match = n; diverged = n; groups = @(...) }`
     (divergence lines grouped with counts and up to 5 run refs, like
     `failures`).
   - `Get-UatExitCode`: 5 when the summary is otherwise ok and
     `golden.diverged -gt 0`.
   - `ConvertTo-UatCompactJson`: `golden = { match, diverged, first }`
     (`first` the most common line), omitted without `-DiffGolden`.
   - `Format-UatSummary`: a "Golden" block (counts, then each line with its
     count).
3. `tools/lab/cli/test-uat.ps1`: cases for the three functions.

Tests (type 1, PowerShell): `golden_match_and_divergence_are_counted`;
`a_divergence_on_passing_runs_exits_5` (fails if exit stays 0);
`compact_json_with_golden_stays_under_500_chars` for a 5-run batch with two
distinct divergences; a batch without `Golden` fields is unchanged.

Checks: `pwsh -NoProfile -File tools/lab/cli/test-uat.ps1`.

Docs owed: the `-DiffGolden` flag and exit code 5 in
`docs/guides/live-research-lab.md` § Commands and in the `lab/` row of
`tools/README.md`.

Reviewer focus: rows' verdicts and the `ok` flag are unchanged by a
divergence; an error from the diff tool is reported, not treated as a match.

## GD-10 Live acceptance

**Implementer:** the coordinator, after the user approves lab use. **Size:**
S. **Wave:** 7. **Depends on:** GD-09, D-GD6.
**Branch:** `lab-golden/gd10-first-session-golden`. **Worktree:** `gd10`.
**Subject:** `test(lab): GD-10 golden fingerprint for first-session FS-01 to FS-P5`

Why: the campaign's live acceptance (handoff § 3).

Ask the user first: the lab must be free, and the run takes one instance for
about 25 minutes. Then, from the worktree, against a server build whose SHA
is known:

1. `lab golden record first-session -Rows FS-01,FS-02,FS-P1,FS-P2,FS-P3,FS-P4,FS-P5 -Runs 5 -ServerVersion <sha>`.
2. On exit 6 (disagree), read the line. If the differing family is timing
   noise (an ambient NPC message, a periodic update), add it to that row's
   `[row.golden]` `ignore` or `allow_unordered` in `first-session.toml` with
   a comment quoting the line, and run step 1 again (five fresh runs). Never
   ignore a dialog, mission, step, objective, item or chat family. Stop
   after three rounds and report the lines to the user.
3. On exit 1 with FS-P2 tagged #1341, rerun once; twice in a row, stop and
   report.
4. On success: `lab uat first-session -Rows <same> -DiffGolden` once; it
   must report `golden: match`.
5. Commit `first-session.toml` (variance changes only) and
   `docs/guides/uat-specs/golden/first-session.json`. The lab tests'
   `committed_goldens_parse_and_fit` must pass in the `lab` workflow.

Record in `worknotes/GD-10.md`: the batch folder, the five run ids, each
variance entry added and the line that justified it, and the golden's size.

Docs owed: none beyond the worknote (GD-11 folds it in).

## GD-11 Close-out

**Implementer:** documentation-writer. **Size:** S. **Wave:** 8. **Depends on:** all.
**Branch:** `lab-golden/gd11-closeout`. **Worktree:** `gd11`.
**Subject:** `docs(lab): GD-11 golden runs close-out`

Files:

1. `docs/guides/automated-uat.md`: a `## Golden runs` section (what a
   fingerprint holds, the five-run rule, record, diff, re-blessing, the
   variance knobs from GD-01, reading a divergence line), linked from the
   contents and from `### Golden variance`.
2. `docs/guides/unified-uat.md`: in the first-session section, the step
   "after a server change, `lab uat first-session -Rows ... -DiffGolden`;
   `golden: match` expected", and the owner UAT still to run, if any.
3. `docs/architecture/live-research-lab.md`: one paragraph under the UAT
   consequences: golden fingerprints, D-GD1 and D-GD2.
4. `docs/project-status.md` and `docs/gap-analysis*`: only where they
   already describe the lab or UAT automation.
5. This ledger: every packet's status, review outcomes, the final
   decisions.
6. `.claude/agent-memory/main-session/`: one reference memory (the
   fingerprint rules and the variance that FS needed), indexed.

Then retire every worker worktree
(`pwsh -NoProfile -File tools/build-lane/rm-worktree.ps1 --merged`) and post a
handoff to the campaign's board subcategory.

Checks: `pwsh -NoProfile -File tools/lint-md.ps1` on the touched files.
