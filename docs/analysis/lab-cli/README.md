# Lab CLI

> Type: ledger. Audience: the coordinator, packet workers and reviewers.
> Opened 2026-10-10 against `main`. Prefix `LC-`. Follows the lab parallel
> clients campaign ([../lab-parallel-clients/README.md](../lab-parallel-clients/README.md), #1312).
> Packet specs: [work-packets.md](work-packets.md).
>
> **Campaign status (2026-10-10): wave 1 dispatched.** D-LC1 to D-LC5
> accepted; D-LC5 amended to allow `-Force`.

## Why

The lab daemon now hosts up to five clients, but working with it on this
machine still means:

- knowing which of three scripts (`daemon.ps1`, `instances.ps1`,
  `install.ps1`) does what;
- running them from a checkout that may be on an old branch (2026-10-10: the
  main checkout's `instances.ps1` predated LP-08 and reported every profile as
  "not seeded");
- calling an MCP tool to see who holds which lease;
- editing `labd.env` by hand;
- and `install.ps1` still builds through Git Bash, which this machine's rules
  forbid.

The owner asked for something "easier to work with and more natively
available and managed". A Windows service was ruled out: services run in
Session 0, with no desktop for `SGW.exe`. The scheduled task stays. This
campaign puts one `lab` command in front of it.

## What the command does

| Command | Does |
|---|---|
| `lab status` | Daemon pid, build commit and uptime. Per instance: account, client pid, bridge port, lease holder (owner, purpose, expiry; never an id) and profile seed state. |
| `lab start` / `stop` / `restart` | The scheduled task, through `daemon.ps1`. |
| `lab logs [-Follow] [-Instance p2] [-Level warn] [-Lines 50]` | `labd.log`, filtered by the per-instance `instance=` field the spans carry. |
| `lab env [get KEY \| set KEY VALUE \| unset KEY]` | Reads or edits `labd.env` with a timestamped backup. A change prints the restart hint. Token values are masked on print. |
| `lab install [-From <worktree>]` | Builds the lab through the PowerShell lane (no bash) and installs it. Refreshes the CLI's own installed copy. |
| `lab instances [init \| status]` | `instances.ps1`, run from the installed copy. |
| `lab clients stop [<instance> \| all] [-Force]` | Closes lab clients whose instance has no lease. Refuses a leased one, naming the holder, unless `-Force` is given with a named instance. |
| `lab doctor` | Checks the setup, one PASS, WARN or FAIL line each. |
| `lab version`, `lab help` | Version, and help listing each command's first help line. |

`lab` is `%LOCALAPPDATA%\cimmeria-lab\bin\lab.cmd`, which runs the installed
copy of the scripts in `%LOCALAPPDATA%\cimmeria-lab\cli\`. It never runs a
checkout's copy. `lab setup` writes the shim and adds the `bin` folder to the
user `PATH` once.

## Decisions

| ID | Decision | Status | Reason |
|---|---|---|---|
| D-LC1 | PowerShell 7 scripts under `tools/lab/cli/`, one file per command, found by the dispatcher `tools/lab/lab.ps1`. | Accepted 2026-10-10 | All existing lab tooling is PowerShell. One file per command lets packets run in parallel without touching a shared file, and Haiku handles it well. A Rust subcommand of `cimmeria-lab.exe` would add a rebuild to every CLI change. |
| D-LC2 | The daemon gains `GET /status`, behind the same bearer token as `/mcp`, returning the JSON in the contract. | Accepted 2026-10-10 | `lab status` would otherwise need an MCP streamable-HTTP client in PowerShell (initialize, session id, SSE parsing): fragile, and the wrong job for a small model. The endpoint is read-only and shows no lease ids. |
| D-LC3 | `lab setup` adds `%LOCALAPPDATA%\cimmeria-lab\bin` to the **user** `PATH` (registry `HKCU\Environment`), once, after asking. | Accepted 2026-10-10 | It is a machine-level change. Without it, the command is `pwsh <path>\lab.ps1`. |
| D-LC4 | The CLI runs from an installed copy (`%LOCALAPPDATA%\cimmeria-lab\cli\`), refreshed by `lab install` and `lab setup`. | Accepted 2026-10-10 | The same reason `daemon.ps1` copies itself to `labd\`: a checkout on an old branch must not drive the lab. |
| D-LC5 | `lab clients stop` closes only clients whose instance holds no lease. `-Force` overrides that for one named instance (never with `all`), and says the holder loses the client. | Accepted 2026-10-10, amended | Killing a leased client takes it from under another agent, and its watchdog relaunches it, so the default refuses. The owner wanted an override for stuck leases; scoping it to one named instance keeps it deliberate. |

## Packets

| ID | Packet | Worker | Depends on | Status |
|---|---|---|---|---|
| LC-01 | Daemon `GET /status` | packet-coder (Rust) | D-LC2 | Writing |
| LC-02 | CLI skeleton: dispatcher, common library, `status`, `start`/`stop`/`restart`, `version`, `help` | packet-coder | D-LC1, contract | Writing |
| LC-03 | `logs` and `env` | packet-coder | LC-02 | BlockedDependency |
| LC-04 | `install` without bash, and `setup` (shim, installed copy, PATH) | packet-coder | D-LC3, D-LC4 | Writing |
| LC-05 | `instances`, `clients stop`, `doctor` | packet-coder | LC-02, LC-01 | BlockedDependency |
| LC-06 | Docs and memory | documentation-writer | LC-01 to LC-05 | BlockedDependency |
| LC-07 | Live UAT of every command on this machine | coordinator | all | BlockedDependency |

Waves:

1. LC-01, LC-02 and LC-04 run in parallel (different files). LC-02 builds against the `/status` contract and works without it, reporting "status endpoint unavailable".
2. LC-03 and LC-05, each in its own files under `tools/lab/cli/`.
3. LC-06, then LC-07.

## Dispatch rules

- **Workers.** `packet-coder` (Haiku), one packet each, in its own worktree
  (`pwsh -NoProfile -File tools/build-lane/mk-worktree.ps1 <branch> <name>`).
- **Review.** Each finished packet gets a Sonnet `packet-reviewer`. Findings
  go back to the same coder over `SendMessage`, and the reviewer re-checks the
  fix commit. A coder past about 85k context, or on its third round, hands
  over to a fresh worker or the coordinator.
- **Shell and merge.** PowerShell only: no bash, WSL or Git Bash, and no
  direct `cargo`. Ship with
  `python tools/build-lane/ship.py pr -C <worktree> -m <msg>`. Merge once the
  build-proving CI jobs pass, or per the owner's current merge rule.
</content>
</invoke>
