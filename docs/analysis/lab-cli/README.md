# Lab CLI

> Type: ledger. Audience: the coordinator, packet workers and reviewers.
> Opened 2026-10-10 against `main`. Prefix `LC-`. Follows the lab parallel
> clients campaign ([../lab-parallel-clients/README.md](../lab-parallel-clients/README.md), #1312).
> Packet specs: [work-packets.md](work-packets.md).
>
> **Campaign status (2026-10-10): code complete, UAT pending.** LC-01 to
> LC-05 merged; LC-06 (docs) is the last PR; LC-07, the live UAT, waits
> on the owner. D-LC1 to D-LC5 accepted; D-LC5 amended to allow `-Force`.

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
| `lab install -From <worktree> [-SkipBuild]` | Builds the lab through the PowerShell lane (no bash) and installs it. Refreshes the CLI's own installed copy. |
| `lab instances [init \| status]` | `instances.ps1`, run from the installed copy. |
| `lab clients stop [<instance> \| all] [-Force]` | Closes lab clients whose instance has no lease. Refuses a leased one, naming the holder, unless `-Force` is given with a named instance. |
| `lab doctor` | Checks the setup, one PASS, WARN or FAIL line each. |
| `lab setup [-From <checkout>] [-NoPath] [-Yes]` | Installs the CLI copy and the shim, and offers to add the `bin` folder to the user `PATH`. |
| `lab version`, `lab help` | Version, and help listing each command's first help line. |

`lab` is `%LOCALAPPDATA%\cimmeria-lab\bin\lab.cmd`, which runs the installed
copy of the scripts in `%LOCALAPPDATA%\cimmeria-lab\cli\`. It never runs a
checkout's copy. `lab setup` writes the shim and adds the `bin` folder to the
user `PATH` once. Operating detail: [The `lab` command](../../guides/live-research-lab.md#the-lab-command).

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
| LC-01 | Daemon `GET /status` | packet-coder (Rust) | D-LC2 | Integrated (#1334) |
| LC-02 | CLI skeleton: dispatcher, common library, `status`, `start`/`stop`/`restart`, `version`, `help` | packet-coder | D-LC1, contract | Integrated (#1336) |
| LC-03 | `logs` and `env` | packet-coder | LC-02 | Integrated (#1338) |
| LC-04 | `install` without bash, and `setup` (shim, installed copy, PATH) | packet-coder | D-LC3, D-LC4 | Integrated (#1335) |
| LC-05 | `instances`, `clients stop`, `doctor` | packet-coder | LC-02, LC-01 | Integrated (#1337) |
| LC-06 | Docs and memory | documentation-writer | LC-01 to LC-05 | InReview (this PR, branch `docs/lab-cli-docs`) |
| LC-07 | Live UAT of every command on this machine | coordinator | all | UATPending (needs the owner: `lab setup` edits the user `PATH`) |

Waves:

1. LC-01, LC-02 and LC-04 run in parallel (different files). LC-02 builds against the `/status` contract and works without it, reporting "status endpoint unavailable".
2. LC-03 and LC-05, each in its own files under `tools/lab/cli/`.
3. LC-06, then LC-07.

## Review outcomes

Where the merged code differs from [work-packets.md](work-packets.md). The
code and the [operating guide](../../guides/live-research-lab.md#the-lab-command)
are the reference; the packet specs are not updated.

- **Dispatcher (LC-02).** It skips `common`, `test-*` and `*-lib` files.
  Named flags pass through to the command, and a command that falls off the
  end exits 0 (`$global:LASTEXITCODE` is reset first). `$Rest` is assigned
  directly: an if-expression unrolled a one-element array to a string, and
  `lab instances status` splatted one character at a time.
- **`CIMMERIA_LAB_HOME` (LC-02).** A test-only override. `daemon.ps1`
  ignores it, so with it set, `lab status` reads a different `labd.pid` than
  `lab restart` acts on.
- **`lab logs` (LC-03).** A continuation line (no leading timestamp) is
  kept or dropped with the entry above it, so a level word inside a
  backtrace never ranks it alone.
- **`lab env` (LC-03).** Masking is wider than the spec: keys containing
  `TOKEN`, `SECRET`, `PASSWORD`, `KEY`, `AUTH` or `CREDENTIAL`, a URL's
  userinfo and host, 64-hex and bearer tokens; a stray line shows
  `<unparsed line>`. Keys match ignoring case, as the daemon reads them. A
  value starting with `-` is written `-Value:-x`; a value with edge spaces is
  refused. `labd.env` is replaced by a move after the timestamped backup.
- **`lab install` (LC-04).** `-From` is required. It builds through
  `lane.ps1`, then copies the CLI with the worktree's own `install-lib.ps1`
  in a child `pwsh`, so a newer file list applies. A reinstall removes
  command files the checkout no longer has, and `VERSION` is written last.
- **`lab setup` (LC-04).** From the installed copy it needs `-From`. It asks
  `[y/N]` before adding `<LabHome>\bin` to the user `PATH` unless `-Yes`
  (D-LC3). It writes `HKCU\Environment` directly to keep `REG_EXPAND_SZ`
  (`SetEnvironmentVariable(..., 'User')` writes `REG_SZ`) and broadcasts
  `WM_SETTINGCHANGE`. The shim holds the real path, not `%LOCALAPPDATA%`.
- **`lab clients stop` (LC-05).** `-Force` only with a named instance. Exit
  codes 0, 1 (daemon down), 2 (usage), 3 (a leased client refused) and 4 (a
  client could not be stopped); the spec had no 4.
- **`lab doctor` (LC-05).** The last check compares the copy's `tools/lab`
  with `origin/main` by content, not ancestry: squash merges mean a
  worktree sha never becomes an ancestor of `origin/main`.

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
