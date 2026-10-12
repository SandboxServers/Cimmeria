# Lab CLI in Rust

> Type: ledger. Audience: the coordinator, packet workers and reviewers.
> Opened 2026-10-11 against `main`. Prefix `LX-`. Follows the
> [lab CLI campaign](../lab-cli/README.md), whose decision D-LC1
> (PowerShell, one file per command) this campaign replaces.
> Packet specs: [work-packets.md](work-packets.md) (rules, contract,
> LX-01 to LX-09) and [work-packets-ports.md](work-packets-ports.md)
> (LX-10 to LX-20).
>
> **Campaign status (2026-10-12): code complete; LX-20 live check owed.** Everything is built on one local branch. The user
> asked for local work and one PR at the end (2026-10-11), so packets merge
> into `lab-cli-rust/integration` (worktree `lcr`), not `main`; #1654
> (LX-01) was closed and folded in. Every packet has a step-by-step spec in
> [specs/](specs/), written by a Sonnet expander before a Haiku coder starts.

## Why

The `lab` command grew from a few wrappers into 44 PowerShell files and
about 7,800 lines, 2,300 of them home-made test scripts. Its biggest parts
are real programs: an MCP client over streamable HTTP (`uat-lib.ps1`), a
lane scheduler with brakes and timeouts, a live terminal dashboard
(`watch`), JSON compaction under size budgets. PowerShell traps cost
review rounds on every lab campaign (a one-element array unrolled to a
string, `$LASTEXITCODE` under strict mode, `-Value:-x`), the help screen
was a raw dump, and the JSON shapes it reads are defined in Rust on the
daemon side and re-parsed by hand.

The rest of the repo, the lane, CI and the lab daemon (`crates/lab`) are
Rust. The CLI moves there, as one `lab.exe`, and gets a proper look:
grouped help, coloured tables, glyphs, per-command pages, typo
suggestions and shell completions.

## Decisions

| ID | Decision | Status | Reason |
|---|---|---|---|
| D-LX1 | The CLI becomes a Rust binary `lab` in a new crate `crates/lab-cli` (package `cimmeria-lab-cli`). It replaces D-LC1. | Accepted by the user 2026-10-11; owner to confirm before LX-06 merges | Size and logic outgrew PowerShell; one toolchain. A small crate rebuilds in seconds, which answers D-LC1's rebuild objection. |
| D-LX2 | Strangler migration. `lab.exe` knows every command from day one; a command not yet ported runs the installed PowerShell dispatcher (`<lab home>\cli\lab.ps1`) with the original arguments and passes its exit code on. Each port packet deletes the PowerShell it replaces. | Accepted 2026-10-11; fully retired at LX-18 (only the seven script wrappers still start PowerShell, and there is no generic fallback) | Every packet ships something usable; there is never a second copy of a command to keep in step. |
| D-LX3 | `crates/lab-cli` builds on every OS and is a `default-member`, so CI builds, lints and tests it. Windows-only code (registry, windows, processes) sits behind `cfg(windows)` with a non-Windows stub that returns "Windows only". It does not depend on `cimmeria-lab`. | Accepted 2026-10-11 | `cimmeria-lab` is excluded from CI; the CLI must not be. The daemon's JSON is read through the CLI's own small serde types. |
| D-LX4 | Flags are clap long flags (`--rows`). PowerShell-style flags still work: `-Rows`, `-PlanOnly`, `-Value:-x` are rewritten before clap sees them (contract § Arguments). | Accepted 2026-10-11 | Docs, skills, specs and muscle memory use the PowerShell form. |
| D-LX5 | Output parity. Exit codes and `--json` output (keys, key order, caps and size budgets) are exactly the PowerShell command's. Plain output (stdout not a terminal, or `NO_COLOR`) keeps the PowerShell line formats. Terminal output is restyled per contract § Look and feel. | Accepted 2026-10-11 | Agents and scripts parse the JSON and the plain lines; people read the terminal. |
| D-LX6 | Stays PowerShell: `daemon.ps1` and `labd-lib.ps1` (the scheduled task, Windows PowerShell 5.1), `instances.ps1`, `tools/lab/install.ps1`, and the `install`, `setup` and `local-setup` commands. `lab.exe` runs them as child processes and gives them native help entries. | Accepted 2026-10-11 | Windows glue and bootstrapping: `setup` must work before `lab.exe` exists. |
| D-LX7 | `lab.exe` lives at `<lab home>\bin\lab.exe` next to `cimmeria-lab.exe`. `lab install` builds and copies it; `lab setup` deletes the `lab.cmd` shim once `lab.exe` is there (`PATHEXT` finds `.exe` before `.cmd`). D-LC4 (run an installed copy, never a checkout) holds. | Accepted 2026-10-11 | One install path, no shim. |
| D-LX8 | Work split: `packet-coder` (Haiku) implements; when it runs out of context, a fresh `packet-coder` continues from `worknotes/<packet>.md` and the uncommitted diff. `packet-review-fixer` (Sonnet) reviews and fixes every packet. The coordinator writes no code: it dispatches, ships and merges. | Accepted 2026-10-11 | The user's rule for this campaign. |

## Packets

Size: S about 200 lines of Rust with tests, M about 400, L about 700 (L
expects one continuation).

| ID | Packet | Size | Wave | Depends on | Status |
|---|---|---|---|---|---|
| LX-01 | Crate, argument rewriting, UI kit, root help, PowerShell fallback, `version` | M | 1 | none | Integrated (local) |
| LX-02 | Command registry: every command's args, page and fallback stub | M | 2 | LX-01 | Integrated (local) |
| LX-03 | Install `lab.exe`; `setup` drops the shim | S | 2 | LX-01 | Integrated (local) |
| LX-04 | Lab paths, `labd.env`, token, `/status` client | M | 2 | LX-01 | Integrated (local) |
| LX-05 | MCP client and lease wait | M | 2 | LX-01 | Integrated (local) |
| LX-06 | `status`, `logs` | M | 3 | LX-02, LX-03, LX-04 | Integrated (local) |
| LX-07 | `env` | S | 3 | LX-02, LX-03, LX-04 | Integrated (local) |
| LX-08 | `doctor` | M | 3 | LX-02, LX-03, LX-04 | Integrated (local) |
| LX-09 | `clients stop` | M | 3 | LX-02, LX-03, LX-04 | Integrated (local) |
| LX-10 | `record` | M | 3 | LX-02 to LX-05 | Integrated (local) |
| LX-11 | `golden` | M | 3 | LX-02 to LX-05 | Integrated (local) |
| LX-12 | `calibrate` | M | 3 | LX-02 to LX-05 | Integrated (local) |
| LX-13 | `watch`: model, `--once`, `--json`, known issues, batch files | M | 3 | LX-02 to LX-04 | Integrated (local) |
| LX-14 | `watch`: live view | S | 4 | LX-13 | Integrated (local) |
| LX-15 | `uat`: arguments, plan, pre-flight, `--plan-only` | M | 4 | LX-05, LX-13; lab-chaos CH-08 (#1651) merged | Integrated (local) |
| LX-16 | `uat`: lanes, brake, timeouts, batch files, progress, summary | L | 5 | LX-15 | Integrated (local) |
| LX-17 | `uat`: `--diff-golden`, `--heal`, `--json`, exit codes; delete the uat PowerShell | M | 6 | LX-16, LX-12 | Integrated (local) |
| LX-18 | Native wrappers, `completions`, delete the dispatcher | S | 7 | all ports | Integrated (local) |
| LX-19 | Docs close-out | M | 7 | LX-18 | Integrated (local) |
| LX-20 | Live check of every command on the workstation | S | 8 | LX-19, the user's OK for lab use | BlockedDependency |

Wave 3 has eight packets; run at most four at once (four lane slots, and
the lab workstation starves under load).

## Interaction with other campaigns

- **lab-chaos.** CH-08 (#1651) edits `uat.ps1` and `uat-lib.ps1`. LX-15
  starts only after it merges and ports the merged code. CH-09 and later
  packets that change `lab uat` target the Rust command once LX-17 merges;
  note it in the lab-chaos ledger at LX-17's merge.
- **lab-record, lab-golden, lab-watch.** Code-complete; their remaining
  packets are live checks and docs. A port packet ports `origin/main` as of
  its dispatch. Before merging a port, the coordinator checks
  `git log <dispatch base>..origin/main -- tools/lab/cli/<command>*` and
  sends any later change to the review-fixer.

## Review outcomes

Where the merged code differs from the packet text. The specs in
[specs/](specs/) and the code are the reference.

- **Agents.** `packet-review-fixer` (Sonnet; reviews and fixes) was added
  for this campaign. Every Haiku commit gets one, including follow-ups made
  after a review and merge-conflict resolutions (a gap review on
  2026-10-12 covered the ones that had slipped through).
- **A new crate must be classified** in
  `crates/server/src/logging/target_scan_tests.rs` (LX-01 failed the merge
  queue on it).
- **Fallback (LX-02).** The PowerShell dispatcher gets PowerShell-form flags:
  `--runs-per-lease 5` becomes `-RunsPerLease 5`, `--value=-x` becomes
  `-Value:-x`, `--x=` becomes `-X` plus an empty token. List flags take one
  comma-separated value per occurrence. PowerShell's implicit positional
  binding (`lab logs 5`) is not supported.
- **JSON key order.** The workspace `serde_json` has no `preserve_order`, so
  objects the CLI builds are `#[derive(Serialize)]` structs in PowerShell
  order; daemon answers pass through as `Value` (they are already sorted).
- **`labd.env` (LX-04).** Untouched lines are written back byte-identical
  (`EnvLine::Pair` keeps `raw`); every line is written with CRLF, as the
  PowerShell writer did.
- **Tests never reach a real daemon (LX-15).** With `CIMMERIA_LAB_HOME` set
  (tests only) the token comes from the process environment, never HKCU; every
  test that runs the binary uses `tests/support`'s `isolated_lab_home()`,
  whose `labd.pid` names a closed port.
- **`doctor` (LX-08)** compares the two `cimmeria-lab.exe` copies by bytes,
  not SHA-256. `test-ops.ps1` and `test-logs-env.ps1` are deleted (no checks
  left after the ports).
- **`calibrate` (LX-12).** `changed` fields print in the daemon's key order
  (sorted), as the PowerShell did; the spec's fixed order was dropped.
  `--apply` refuses `..` paths and counts files by both diff headers.
- **`uat` (LX-15, LX-16).** Only `--plan-only` was native in LX-15; LX-16
  makes a real run native; `--json`, `--diff-golden`, `--heal` and `--chaos`
  stay on PowerShell until LX-17. Row ids match exactly (the PowerShell matched
  them ignoring case, against its own message).
- **Follow-ups F-LX1 to F-LX9** (found in reviews) are fixed in LX-18a:
  IPv6 masking, doctor hints and plurals, cell-width tables, absolute
  `--specs-dir`, calibrate's no-instance wording, `logs --follow` on a
  replaced file, the live view's panic restore, a deterministic Windows stop
  test (10/10), stale references.
- **F-LX10 to F-LX12 (LX-18c).** Found in the LX-15 to LX-17 reviews.
  Ctrl+C during the 10-minute lease wait after a run timeout ends the wait
  at once, and a wait that Ctrl+C cut short does not brake the lane
  (F-LX10). `clip_text` counts UTF-16 units, as PowerShell's `Limit-UatText`
  did, so the text budgets cut in the same place (F-LX11). Six end-to-end
  `lab uat` tests run the real binary against a scripted stub daemon
  (F-LX12, `tests/uat_e2e.rs`). Review fixes: the lease wait cuts its
  slices from the clock, not from a running sum, so a sleep that overshoots
  (about 15 ms on Windows) no longer stretches the poll cadence or the
  wait's maximum; the two-lane batch test compares the progress lines
  without their real elapsed times.
- **F-LX13 (f8bbd67ad).** A flaky two-lane batch test compared real elapsed
  time; it now strips the duration suffix before comparing.
- **`uat` is fully native (LX-17).** `--json`, `--diff-golden`, `--heal`
  and `--chaos` run in `lab.exe`; the `uat` PowerShell and its tests are
  deleted. Review fixes made the output match PowerShell's: `--json` writes
  U+0085, U+2028 and U+2029 as `\u` escapes like `ConvertTo-Json -Compress`
  (a `serde_json` `Formatter`), golden-divergence groups and the
  `seed=<digits>` mask compare ignoring case as PowerShell's dictionaries and
  `-replace` did, a trailing separator does not change a file name (as
  `Split-Path -Leaf`), and a closed pipe (`| head`) no longer panics the
  heal pass.
- **Wrappers, completions and no dispatcher (LX-18).** `start`, `stop`,
  `restart`, `instances`, `install`, `setup` and `local-setup` are
  `lab.exe` subcommands that run their script through
  `fallback::run_script` (flags in PowerShell form, the script's exit code,
  255 when it has none, exit 1 with a message when `pwsh` or the script is
  missing). `lab completions <powershell|bash|zsh|fish>` prints a script, and
  `--install` adds one line to `$PROFILE.CurrentUserAllHosts` once.
  `tools/lab/lab.ps1` and the per-command scripts are deleted; `lab setup`
  deletes an old `bin\lab.cmd`; `tools/lab/install.ps1` and `lab install`
  build and install `lab.exe`.
- **Docs (LX-19).** The guide's "The `lab` command" is written for
  `lab.exe`; the automated-UAT, unified-UAT, contributor-setup and drafts
  documents use `--flag` forms (the PowerShell spellings are said to work);
  D-LC1 in the [lab CLI ledger](../lab-cli/README.md) is marked superseded;
  the unified UAT guide gains a "Lab CLI" section with LX-20's steps (LX-U1
  to LX-U16). The tools catalog's stale rows leave at the next
  `regen-docs` run.
