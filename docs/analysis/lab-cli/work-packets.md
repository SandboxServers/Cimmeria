# Lab CLI: work packets

> Type: work packets. Audience: `packet-coder` workers (Haiku) and
> `packet-reviewer` reviewers (Sonnet). Ledger and decisions: [README.md](README.md).
>
> PowerShell only. Rust checks go through the lane, in this order:
>
> ```powershell
> pwsh -NoProfile -File tools/build-lane/lane.ps1 cargo fmt -p cimmeria-lab
> pwsh -NoProfile -File tools/build-lane/lane.ps1 cargo clippy -p cimmeria-lab --all-targets -- -D warnings
> pwsh -NoProfile -File tools/build-lane/lane.ps1 cargo test -p cimmeria-lab
> ```
>
> PowerShell files are CRLF, start with a `<# .SYNOPSIS .DESCRIPTION .EXAMPLE #>` block,
> and set `Set-StrictMode -Version Latest` and `$ErrorActionPreference = 'Stop'`.
> Each script must run under `pwsh` 7. Never print a token or a lease id.

## Contents

- [Contract](#contract)
- [LC-01 Daemon GET /status](#lc-01-daemon-get-status)
- [LC-02 CLI skeleton](#lc-02-cli-skeleton)
- [LC-03 logs and env](#lc-03-logs-and-env)
- [LC-04 install without bash, and setup](#lc-04-install-without-bash-and-setup)
- [LC-05 instances, clients stop, doctor](#lc-05-instances-clients-stop-doctor)
- [LC-06 Docs and memory](#lc-06-docs-and-memory)
- [LC-07 Live UAT](#lc-07-live-uat)

## Contract

### `GET /status` (LC-01 serves it, LC-02 and LC-05 read it)

The request carries the same `Authorization: Bearer <CIMMERIA_LAB_DAEMON_TOKEN>`
as `/mcp`. The response is `200`, `application/json`:

```json
{
  "daemon": { "pid": 49056, "version": "0.1.0", "started_at": "2026-10-10T20:58:29Z", "uptime_s": 1234 },
  "instances": [
    { "instance": "default", "account": "lab", "bridge_port": 8770, "client_pid": 49448,
      "lease": { "held": true, "lease": { "owner": "lp07-lab", "purpose": "...", "expires_at": "...", "remaining_s": 1785 } } },
    { "instance": "p2", "account": "lab2", "bridge_port": 8771, "client_pid": null, "lease": { "held": false } }
  ]
}
```

- `lease` is exactly `LeaseBook::status()` for that instance. It never
  carries a lease id; LC-01's test asserts that.
- `account` and `client_pid` are `null` when unknown.

### CLI layout (LC-02 creates it, the other packets add files)

```text
tools/lab/lab.ps1            dispatcher: lab <command> [args...]
tools/lab/cli/common.ps1     dot-sourced library: paths, token, labd.env, /status
tools/lab/cli/<command>.ps1  one file per command, run with the remaining args
tools/lab/cli/test-common.ps1  tests for common.ps1 (no Pester), exits non-zero on failure
```

The dispatcher finds commands by file name. A file's first `.SYNOPSIS` line
is its help line. Adding a command never edits `lab.ps1`.

`common.ps1` exports these functions (LC-02 writes them; the others use them
and add none):

| Function | Returns |
|---|---|
| `Get-LabHome` | `%LOCALAPPDATA%\cimmeria-lab` |
| `Get-LabdEnvPath` | `<LabHome>\labd.env` |
| `Get-LabdEnv` | ordered map of labd.env, via `labd-lib.ps1`'s `Read-LabdEnvFile` (dot-source `..\labd-lib.ps1`) |
| `Get-LabToken` | `CIMMERIA_LAB_DAEMON_TOKEN` from the user environment, else the process; `$null` if unset |
| `Get-DaemonInfo` | the `labd.pid` JSON as an object (`pid`, `bind`, ...), or `$null` |
| `Get-LabStatus` | the `/status` JSON as an object, or `$null` when the daemon or the endpoint is down (no throw). It uses `Invoke-RestMethod -Uri "http://<bind>/status" -Headers @{Authorization="Bearer <token>"} -TimeoutSec 5`, with the bind from `Get-DaemonInfo`, else `127.0.0.1:8779`. |
| `Get-ProfileRoot` | the same resolution as `instances.ps1`'s `Get-ProfileRoot` (labd.env, then env, then `<LOCALAPPDATA>\cimmeria-lab\instances`) |
| `Format-Ago([int]$seconds)` | `"12s"`, `"5m"`, `"2h 3m"` |
| `Write-LabTable($rows, [string[]]$columns)` | aligned columns on stdout, through `Format-Table -AutoSize \| Out-String -Width 200` |

## LC-01 Daemon GET /status

**Worktree:** `pwsh -NoProfile -File tools/build-lane/mk-worktree.ps1 feat/lab-cli-status lc01`.
**Subject:** `feat(lab): LC-01 daemon GET /status for the lab CLI`

Files:

1. `crates/lab/src/server/instances.rs`: on `impl LabServer`, add
   `pub fn instances(&self) -> &Instances { &self.instances }` with a doc comment.
2. `crates/lab/src/supervisor/mod.rs`: next to `label()`, add
   `/// This instance's bridge port.` `pub fn port(&self) -> u16 { self.config.port }`.
3. New `crates/lab/src/daemon/status.rs`:

   ```rust
   //! `GET /status`: a read-only JSON summary of the daemon and every hosted
   //! lab instance, for the `lab` CLI (docs/analysis/lab-cli/). Behind the
   //! same bearer gate as `/mcp`; never carries a lease id.

   /// When this daemon process started (set once in `build_router`).
   pub struct Started { pub at_ms: i64, pub pid: u32 }

   /// One instance row (pure, unit-tested).
   pub fn instance_row(label: &str, account: Option<String>, bridge_port: u16,
       client_pid: Option<u32>, lease: serde_json::Value) -> serde_json::Value

   /// The whole body (pure, unit-tested).
   pub fn body(started: &Started, now_ms: i64, rows: Vec<serde_json::Value>) -> serde_json::Value

   /// The axum handler.
   pub async fn handler(State((server, started)): State<(LabServer, Arc<Started>)>) -> axum::Json<serde_json::Value>
   ```

   `body` sets `daemon.version` to `env!("CARGO_PKG_VERSION")`,
   `started_at` with `crate::lease::rfc3339(at_ms)`, and
   `uptime_s = (now_ms - at_ms).max(0) / 1000`. The handler iterates
   `server.instances().iter()`. For each instance: `h.label`,
   `h.supervisor.account_name()`, `h.supervisor.port()`,
   `h.supervisor.client_pid().await`, `h.supervisor.leases().status()`. Use
   the same `now_ms` the crate already uses (Grep `fn now_ms`). It must make
   no bridge call: do NOT call `Supervisor::status()`.
4. `crates/lab/src/daemon/mod.rs`: add `pub mod status;`. In `build_router`,
   create `let started = Arc::new(status::Started { at_ms: <now_ms>, pid: std::process::id() });`
   and add `.route("/status", axum::routing::get(status::handler).with_state((server_for_status, started)))`
   before the `.layer(...)`, so the bearer layer covers it. Clone `server`
   for it before it moves into the MCP closure. Add `/status` to the module
   doc's list.
5. Tests:
   - In `status.rs`, `mod tests`: `instance_row` shape; `body` uptime and
     version.
   - In `crates/lab/src/daemon/http_tests.rs`, two `#[tokio::test]` (copy the
     existing `spawn_daemon`/`headers` helpers; `spawn_daemon` returns the
     `/mcp` URL, so swap the path for `/status`):
     - `status_needs_the_bearer`: no token gives 401, and the right token
       gives 200;
     - `status_lists_instances_without_lease_ids`: acquire a lease on the
       test server's supervisor book, GET `/status`, and assert one instance
       with `lease.held == true`, and that the body text does not contain the
       acquired lease id.

Checks: the three lane commands. `http_tests.rs` is about 210 lines: keep
it under 500.

Reviewer focus: the bearer layer covers `/status`; no bridge call (no hang
on a stuck client); no lease id anywhere; the `Host` and `Origin` guards that
`/mcp` has (does `/status` need them? It is GET-only and token-gated, so say
whether DNS rebinding matters with a bearer).

## LC-02 CLI skeleton

**Worktree:** `pwsh -NoProfile -File tools/build-lane/mk-worktree.ps1 feat/lab-cli-skeleton lc02`.
**Subject:** `feat(lab): LC-02 lab CLI skeleton: dispatcher, common library, status, start/stop/restart`

Files (all new unless named):

1. `tools/lab/lab.ps1`: `param([Parameter(Position=0)][string]$Command = 'help', [Parameter(ValueFromRemainingArguments)][string[]]$Rest)`.
   Commands are the base names of `$PSScriptRoot\cli\*.ps1` except
   `common` and `test-*`.
   - `help`, or no command: list `<name>  <first .SYNOPSIS line>`, sorted.
     Read the first non-blank line after `.SYNOPSIS` in each file.
   - An unknown command: write `unknown command '<x>'; run: lab help` to
     stderr and exit 2.
   - Otherwise `& "$PSScriptRoot\cli\$Command.ps1" @Rest` and
     `exit $LASTEXITCODE`, treating `$null` as 0.
2. `tools/lab/cli/common.ps1`: every function in the contract table,
   functions only, each with a one-line comment.
3. `tools/lab/cli/status.ps1`:
   - `Get-LabStatus`. On `$null`, print
     `daemon: not running (or status endpoint unavailable)`, then still print
     the instances from labd.env's `CIMMERIA_LAB_INSTANCES` with the seed
     column only, and exit 1.
   - Otherwise print `daemon: pid <pid>, up <Format-Ago uptime_s>, version <v>`.
   - Then a table: `INSTANCE ACCOUNT CLIENT PORT LEASE SEED`.
     - CLIENT is the pid, or `-`.
     - LEASE is `<owner> (<purpose>, <Format-Ago remaining_s> left)`, or `free`.
     - SEED is `seeded` when
       `<Get-ProfileRoot>\<instance>\profile\Documents\My Games\Firesky\SGWGame`
       exists, else `-`.
4. `tools/lab/cli/start.ps1`, `stop.ps1`, `restart.ps1`: each runs
   `pwsh -NoProfile -File "$PSScriptRoot\..\daemon.ps1" <start|stop|restart>`
   and passes its exit code on. Find `daemon.ps1` relative to the CLI's own
   folder, so the installed copy works. LC-04 installs `daemon.ps1` beside
   it, so use `Join-Path $PSScriptRoot '..\daemon.ps1'`.
5. `tools/lab/cli/version.ps1`: prints `lab CLI <git short sha of the copy, from cli\VERSION if present, else 'dev'>`,
   then the daemon version from `/status` or `daemon: not running`.
6. `tools/lab/cli/test-common.ps1`, in the same style as
   `tools/lab/test-labd-lib.ps1`:
   - `Format-Ago` at 0, 59, 61 and 7380 s;
   - `Get-ProfileRoot` with the labd.env value, the env value and the default
     (point `LOCALAPPDATA` and the labd.env path at a temp folder);
   - `Get-LabStatus` returns `$null`, not a throw, when nothing listens:
     temporarily point it at a closed port by writing a temp `labd.pid` with
     bind `127.0.0.1:9` under a temp LabHome. `Get-LabHome` honours a
     `$env:CIMMERIA_LAB_HOME` override for exactly this; document it.

Checks:
- `pwsh -NoProfile -File tools/lab/cli/test-common.ps1` exits 0.
- `pwsh -NoProfile -File tools/lab/lab.ps1 help` lists `restart`, `start`,
  `status`, `stop` and `version`.
- `pwsh -NoProfile -File tools/lab/lab.ps1 status` runs. It is live: the
  daemon may be up or down, and either output is fine, but it must not throw.
- `pwsh -NoProfile -File tools/lab/lab.ps1 nope` exits 2.

Reviewer focus:
- no token is printed, including in error text from `Invoke-RestMethod`;
- strict-mode errors on missing JSON properties (an old daemon with no
  `/status`, or `account: null`);
- relative paths that break when the CLI runs from the installed copy;
- the exit codes.

## LC-03 logs and env

**Depends on:** LC-02 merged. **Worktree:** `feat/lab-cli-logs-env`, `lc03`.
**Subject:** `feat(lab): LC-03 lab logs and lab env`

Files:

1. `tools/lab/cli/logs.ps1`: parameters `[int]$Lines = 50`, `[switch]$Follow`,
   `[string]$Instance`, `[ValidateSet('trace','debug','info','warn','error')][string]$Level`.
   - The file is `<LabHome>\labd.log`.
   - `-Instance x` keeps lines that contain `instance="x"` or
     `instance=Some("x")`, ignoring case.
   - `-Level` keeps lines whose level token, the second whitespace field
     (`INFO`, `WARN`, ...), is at or above the level.
   - It prints the last `$Lines` matching lines; `-Follow` uses
     `Get-Content -Tail $Lines -Wait`, with the same filter applied in the
     pipeline.
   - It masks anything that looks like a bearer token or a 64-hex token:
     `(?i)bearer\s+\S+` and `\b[0-9a-f]{64}\b` become `<redacted>`.
   - Put the filter in a pure function `Select-LabLogLine` in `logs.ps1`, so
     it can be tested.
2. `tools/lab/cli/env.ps1`: `lab env` with no argument prints every labd.env
   line.
   - It masks values whose key contains `TOKEN`, `SECRET` or `PASSWORD`, and
     shows a host in a URL as `<host>`. That last rule is display only.
   - `lab env get KEY` prints the value, masked by the same rule.
   - `lab env set KEY VALUE` and `lab env unset KEY` first copy labd.env to
     `labd.env.bak-<yyyyMMdd-HHmmss>`. Then they rewrite it with
     `labd-lib.ps1`'s `Write-LabdEnvFile`, keeping comments and line order,
     replacing the key's line in place or appending it, and print
     `restart the daemon for this to apply: lab restart`.
   - KEY must match `^[A-Z][A-Z0-9_]*$`; otherwise exit 2.
3. `tools/lab/cli/test-logs-env.ps1`: tests for `Select-LabLogLine` (the
   instance, level and redaction cases), and for `env set` / `env unset` on a
   temp labd.env through `CIMMERIA_LAB_HOME`: a backup is written, the order
   is kept, and an unknown key is appended.

Checks: `pwsh -NoProfile -File tools/lab/cli/test-logs-env.ps1` exits 0;
`lab.ps1 logs -Lines 5` and `lab.ps1 env` run against the live files
without printing a token.

Reviewer focus: tokens never printed (the env masking and the log
redaction); `set` keeps comments; a concurrent daemon reading labd.env (it
reads only at start).

## LC-04 install without bash, and setup

**Depends on:** D-LC3 and D-LC4. **Worktree:** `feat/lab-cli-install`, `lc04`.
**Subject:** `feat(lab): LC-04 lab install through the PowerShell lane, and lab setup`

Files:

1. `tools/lab/install.ps1`: replace the Git Bash build loop (the
   `$bash = Join-Path $env:ProgramFiles 'Git\bin\bash.exe'` block and its
   `& $bash tools/build-lane/lane.sh @b`) with:

   ```powershell
   & pwsh -NoProfile -File (Join-Path $Worktree 'tools\build-lane\lane.ps1') @b
   ```

   run from `Push-Location $Worktree` as before. Change the
   `lane.sh` existence check to `lane.ps1`, and every printed
   `bash tools/build-lane/lane.sh` to `pwsh tools/build-lane/lane.ps1`. The
   header comment at the top now says the lane is `lane.ps1`.
2. `tools/lab/cli/install.ps1`: `lab install [-From <worktree path>] [-SkipBuild]`.
   - Without `-From`, it refuses with a message naming the option: a lab
     build must come from a worktree the caller chose.
   - It runs `<From>\tools\lab\install.ps1 -Worktree <From> -SkipBuild:$SkipBuild`,
     with `-InstallDir` from labd.env's `CIMMERIA_LAB_INSTALL_DIR`.
   - Then it calls `Install-LabCli <From>` (below), so the CLI copy matches
     the build.
3. `tools/lab/cli/setup.ps1`: `lab setup [-NoPath]`. It calls
   `Install-LabCli <repo root of this script>`, then writes the shim
   `<LabHome>\bin\lab.cmd`:

   ```bat
   @echo off
   pwsh -NoProfile -File "%LOCALAPPDATA%\cimmeria-lab\cli\lab.ps1" %*
   ```

   Unless `-NoPath`, it adds `<LabHome>\bin` to the user PATH if it is
   missing, and prints that a new terminal sees it. Read and write the user
   PATH with `[Environment]::GetEnvironmentVariable('Path','User')` and
   `SetEnvironmentVariable(...,'User')`, and compare entries
   case-insensitively and without trailing slashes.
4. `Install-LabCli` goes in a new `tools/lab/cli/install-lib.ps1`, which
   `install.ps1` and `setup.ps1` dot-source.
   - It copies `<root>\tools\lab\lab.ps1`, `<root>\tools\lab\cli\*.ps1`,
     `daemon.ps1`, `instances.ps1` and `labd-lib.ps1` into
     `<LabHome>\cli\`.
   - Inside that folder the layout is `lab.ps1`, `cli\*.ps1`, `daemon.ps1`,
     `instances.ps1` and `labd-lib.ps1`, so `cli\..\daemon.ps1` resolves.
   - It writes `<LabHome>\cli\cli\VERSION` holding
     `git -C <root> rev-parse --short HEAD`.
   - It refuses when `<root>\tools\lab\lab.ps1` is missing.
5. `tools/lab/cli/test-install-lib.ps1`: `Install-LabCli` into a temp
   `CIMMERIA_LAB_HOME` from the worktree. It checks that the layout exists,
   that `VERSION` holds the sha, and that a second run overwrites the files.

Checks:
- `pwsh -NoProfile -File tools/lab/cli/test-install-lib.ps1` exits 0;
- `pwsh -NoProfile -File tools/lab/install.ps1 -DryRun -Worktree <this worktree>`
  prints `pwsh tools/build-lane/lane.ps1` lines, with no `bash`;
- `git grep -n "bash" tools/lab/install.ps1` matches only comments that say
  bash is gone, if any.

Do NOT run `setup` without `-NoPath` (it edits the user PATH, and only the
owner runs that).

Reviewer focus:
- PATH editing: no duplicate entries, and the registry value keeps its type
  (`REG_EXPAND_SZ`). Does `SetEnvironmentVariable` change the type? Say
  whether it does on Windows, and if so suggest the
  `Microsoft.Win32.Registry` route.
- `install.ps1`'s refusal while `SGW.exe` runs is unchanged.
- The installed copy resolves `labd-lib.ps1` and `daemon.ps1`.

## LC-05 instances, clients stop, doctor

**Depends on:** LC-01 and LC-02 merged. **Worktree:** `feat/lab-cli-ops`, `lc05`.
**Subject:** `feat(lab): LC-05 lab instances, lab clients stop, lab doctor`

Files:

1. `tools/lab/cli/instances.ps1`: forwards to
   `pwsh -NoProfile -File "$PSScriptRoot\..\instances.ps1" @args`.
2. `tools/lab/cli/clients.ps1`: `lab clients stop [<instance>|all] [-Force]` (D-LC5).
   - `-Force` with `all` (or with no instance) is refused: print
     `-Force needs a named instance` to stderr and exit 2.
   - `Get-LabStatus`; if `$null`, exit 1 with "daemon not running".
   - For each targeted instance with a `client_pid`:
     - when `lease.held` and no `-Force`, print `p2: leased to <owner> (<purpose>); not stopped (use -Force to override)` and count a refusal;
     - when `lease.held` and `-Force`, print
       `p2: leased to <owner> (<purpose>); stopping anyway (-Force). The holder loses the client and the watchdog may relaunch it.`
       and stop it as below;
     - to stop: if the process name is `SGW`, call `CloseMainWindow()`; after
       8 s, `Stop-Process -Force` if it is still alive; print `p2: stopped pid N`.
   - An instance without a client prints `p2: no client`.
   - Exit 0 if no refusals, else 3.
   - Pure helper, for tests: `Select-StopTargets($status, $which, [bool]$force)`
     returns `@{ stop = @(...); forced = @(...); refuse = @(...); none = @(...) }`
     (`forced` is the leased instances stopped because of `-Force`).
3. `tools/lab/cli/doctor.ps1`: one line per check,
   `PASS|WARN|FAIL  <check>  <detail>`. Exit 1 if any check fails.

   | Check | Result |
   |---|---|
   | The scheduled task `CimmeriaLabDaemon` exists | FAIL if not |
   | The daemon answers `/status` | FAIL if not |
   | `CIMMERIA_LAB_DAEMON_TOKEN` is set (never print it) | FAIL if not |
   | labd.env has `CIMMERIA_LAB_INSTALL_DIR` and `SGW.exe` exists under it | FAIL if not |
   | The profile root is absolute and not inside the install dir | FAIL if not |
   | Each instance's account file exists (paths as in `instances.ps1`) | WARN if missing |
   | Each instance's profile is seeded | WARN if not |
   | No `SGW.exe` runs that `/status` does not list | WARN, naming pids |
   | Installed `cimmeria-lab.exe` is the one the daemon runs (`<LabHome>\bin` vs `<LabHome>\labd`, file hash) | WARN if they differ, "run lab restart" |
   | The CLI copy's `VERSION` is an ancestor of `origin/main` (when run inside a git checkout) | WARN, "run lab setup" |

   The checks are functions returning `[pscustomobject]@{ Status; Check; Detail }`,
   so tests can call them with fakes.
4. `tools/lab/cli/test-ops.ps1`: `Select-StopTargets` with a fake status of
   one leased, one free with a client and one free without a client, both
   without `-Force` (the leased one is refused) and with `-Force` on the
   leased instance by name (it lands in `forced`); `-Force` with `all` exits
   2; and the profile-root doctor check with an inside and an outside root.

Checks: `test-ops.ps1` exits 0. `lab.ps1 doctor` runs live and prints lines.
Do NOT run `clients stop` live, because it closes game clients; only the
pure tests.

Reviewer focus:
- a leased client is never killed without `-Force`, and `-Force` never
  applies to `all`;
- pid reuse: check `ProcessName` before acting;
- strict mode on a `/status` without `client_pid`;
- the doctor never prints a token.

## LC-06 Docs and memory

**Worker:** `documentation-writer`. **Depends on:** LC-01 to LC-05 merged.
**Subject:** `docs(lab): the lab command (LC-06)`

- `docs/guides/live-research-lab.md`: a new section "The `lab` command",
  covering setup, every command with one example, and where the copy and the
  shim live. Point "Install or update the lab" at `lab install`.
- `docs/architecture/live-research-lab.md`: `/status` in the daemon section.
- `tools/lab/daemon.ps1` `.DESCRIPTION`: one line pointing at `lab`.
- `.claude/agent-memory/main-session/`: one reference memory, plus its
  `MEMORY.md` line.
- The ledger: statuses and close-out.

## LC-07 Live UAT

**Coordinator.** Ask the owner before touching the lab. Then, on this machine:

1. `lab setup` (the owner approves the PATH change), then open a new terminal.
2. `lab status`, `lab version`, `lab doctor` (all PASS or an explained WARN).
3. `lab env get CIMMERIA_LAB_INSTANCES`, then `lab env set` / `lab env unset`
   a dummy key, checking the backup file.
4. `lab logs -Lines 20 -Instance p2`, then `lab restart`, then `lab status`.
5. `lab install -From <a worktree on main>`, then `lab doctor` (binary check PASS).
6. With one lab-driver holding p2: `lab clients stop p2` refuses, naming the
   holder. `lab clients stop all -Force` exits 2. After release, it stops
   the client. (`-Force` on a leased client is checked only with the
   owner's say-so, and only on a lease the coordinator holds.)

Record the results in the ledger.
</content>
</invoke>
