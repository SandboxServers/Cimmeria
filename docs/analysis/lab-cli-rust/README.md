# Lab CLI in Rust

> Type: ledger. Audience: the coordinator, packet workers and reviewers.
> Opened 2026-10-11 against `main`. Prefix `LX-`. Follows the
> [lab CLI campaign](../lab-cli/README.md), whose decision D-LC1
> (PowerShell, one file per command) this campaign replaces.
> Packet specs: [work-packets.md](work-packets.md) (rules, contract,
> LX-01 to LX-09) and [work-packets-ports.md](work-packets-ports.md)
> (LX-10 to LX-20).
>
> **Campaign status (2026-10-11): planned.** Nothing built.

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
| D-LX2 | Strangler migration. `lab.exe` knows every command from day one; a command not yet ported runs the installed PowerShell dispatcher (`<lab home>\cli\lab.ps1`) with the original arguments and passes its exit code on. Each port packet deletes the PowerShell it replaces. | Accepted 2026-10-11 | Every packet ships something usable; there is never a second copy of a command to keep in step. |
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
| LX-01 | Crate, argument rewriting, UI kit, root help, PowerShell fallback, `version` | M | 1 | none | Ready |
| LX-02 | Command registry: every command's args, page and fallback stub | M | 2 | LX-01 | BlockedDependency |
| LX-03 | Install `lab.exe`; `setup` drops the shim | S | 2 | LX-01 | BlockedDependency |
| LX-04 | Lab paths, `labd.env`, token, `/status` client | M | 2 | LX-01 | BlockedDependency |
| LX-05 | MCP client and lease wait | M | 2 | LX-01 | BlockedDependency |
| LX-06 | `status`, `logs` | M | 3 | LX-02, LX-03, LX-04 | BlockedDependency |
| LX-07 | `env` | S | 3 | LX-02, LX-03, LX-04 | BlockedDependency |
| LX-08 | `doctor` | M | 3 | LX-02, LX-03, LX-04 | BlockedDependency |
| LX-09 | `clients stop` | M | 3 | LX-02, LX-03, LX-04 | BlockedDependency |
| LX-10 | `record` | M | 3 | LX-02 to LX-05 | BlockedDependency |
| LX-11 | `golden` | M | 3 | LX-02 to LX-05 | BlockedDependency |
| LX-12 | `calibrate` | M | 3 | LX-02 to LX-05 | BlockedDependency |
| LX-13 | `watch`: model, `--once`, `--json`, known issues, batch files | M | 3 | LX-02 to LX-04 | BlockedDependency |
| LX-14 | `watch`: live view | S | 4 | LX-13 | BlockedDependency |
| LX-15 | `uat`: arguments, plan, pre-flight, `--plan-only` | M | 4 | LX-05, LX-13; lab-chaos CH-08 (#1651) merged | BlockedDependency |
| LX-16 | `uat`: lanes, brake, timeouts, batch files, progress, summary | L | 5 | LX-15 | BlockedDependency |
| LX-17 | `uat`: `--diff-golden`, `--heal`, `--json`, exit codes; delete the uat PowerShell | M | 6 | LX-16, LX-12 | BlockedDependency |
| LX-18 | Native wrappers, `completions`, delete the dispatcher | S | 7 | all ports | BlockedDependency |
| LX-19 | Docs close-out | M | 7 | LX-18 | BlockedDependency |
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

None yet.
