# Lab CLI in Rust: work packets

> Type: packet specs. Ledger: [README.md](README.md). This file: dispatch
> rules, the contract, LX-01 to LX-09. Ports and close-out:
> [work-packets-ports.md](work-packets-ports.md).

## Dispatch rules

**Roles (D-LX8).** `packet-coder` (Haiku) implements a packet. When it
stops short of the end, it writes `worknotes/LX-NN.md` (done, left, the
next step) and leaves its work uncommitted; the coordinator starts a fresh
`packet-coder` with the continuation brief. `packet-review-fixer` (Sonnet)
then reviews the packet's commits and fixes what it finds. The coordinator
never edits code: it creates worktrees, dispatches, ships and merges.

**Briefs are short.** The spec lives here; a brief points at it.

Coder brief:

```text
Worktree: <absolute worktree path>
Packet: LX-NN. Read "## Contract" and "## LX-NN" in
<worktree>\docs\analysis\lab-cli-rust\work-packets.md (or
work-packets-ports.md); Grep for the heading and Read that range only.
Commit message (use it verbatim):
<subject>

<trailers>
```

Continuation brief: the same, plus `Continue LX-NN: read
<worktree>\docs\analysis\lab-cli-rust\worknotes\LX-NN.md and
git -C <worktree> diff; do the steps it lists as left.`

Review-fixer brief: worktree, `Range: origin/main..HEAD`, the packet
heading and file, and the fix commit message
(`fix(lab-cli): LX-NN review fixes` plus trailers).

**Reports.** One message at the end, results only: files, each check's
final `status=` line, commit hashes, deviations. No progress messages.
The coordinator reads the report and the lane summary, never the
transcript.

**Worker rules.** PowerShell only (no bash, WSL or Git Bash). Every
compiling `cargo` call through
`pwsh -NoProfile -File <worktree>\tools\build-lane\lane.ps1 cargo ...`, run
from the worktree. `git -C <worktree>` with absolute paths. Never
`git push`, `git stash`, `gh pr`, `git worktree prune`. Markdown under
`docs/` stays CRLF.

**Checks, in order, for every Rust packet** (from the worktree):

```powershell
pwsh -NoProfile -File tools\build-lane\lane.ps1 cargo fmt -p cimmeria-lab-cli
pwsh -NoProfile -File tools\build-lane\lane.ps1 cargo clippy -p cimmeria-lab-cli --all-targets -- -D warnings
pwsh -NoProfile -File tools\build-lane\lane.ps1 cargo nextest run -p cimmeria-lab-cli
```

A packet that edits PowerShell also runs the test scripts it names, as
`pwsh -NoProfile -File tools\lab\cli\test-<name>.ps1`.

**Coordinator loop per packet:** `mk-worktree.ps1 lab-cli-rust/lx-NN-<slug>
lx-NN` → coder → (continuation coders) → review-fixer →
`python tools/build-lane/ship.py pr -C <worktree> -m <subject>` →
`ship.py merge <PR> --retire lx-NN` → ledger row. Ship one PR per packet.

## Contract

Everything packets build against. A packet changes it only by saying so.

### Crate layout

```text
crates/lab-cli/
  Cargo.toml            package cimmeria-lab-cli, [[bin]] name = "lab", path = "src/main.rs"
  src/main.rs           fn main() -> ExitCode { cimmeria_lab_cli::main_with(std::env::args_os()) }
  src/lib.rs            pub fn main_with(args) -> ExitCode; module declarations
  src/argv.rs           PowerShell-style flag rewriting
  src/fallback.rs       running the PowerShell dispatcher
  src/help.rs           the root help screen and suggestions
  src/ui/mod.rs         Ui, colours, glyphs, error output
  src/ui/table.rs       Table
  src/daemon/mod.rs     pub mod paths; pub mod env_file; pub mod token; pub mod status; pub mod mcp;
  src/daemon/*.rs       one file each (LX-01 creates them with only a //! line)
  src/commands/mod.rs   Command enum, dispatch, the command table
  src/commands/<name>.rs        one per command (a folder commands/<name>/ when it needs 3+ files)
  tests/*.rs            integration tests
```

`foo/mod.rs` style, no `helpers.rs` or `utils.rs`, files under 500 lines
where a seam exists and never over 700.

### Dependencies (LX-01 adds all of them, so later packets never touch `Cargo.toml`)

```toml
[dependencies]
clap = { version = "4", features = ["derive", "wrap_help", "env"] }
clap_complete = "4"
anstyle = "1"
anstream = "0.6"
crossterm = "0.29"
serde = { workspace = true }
serde_json = { workspace = true }
reqwest = { workspace = true, features = ["blocking"] }
chrono = { workspace = true }
regex = "1"

[target.'cfg(windows)'.dependencies]
windows-sys = { version = "0.61", features = ["Win32_Foundation", "Win32_System_Registry", "Win32_System_Threading", "Win32_UI_WindowsAndMessaging", "Win32_System_Diagnostics_ToolHelp"] }

[dev-dependencies]
tempfile = "3"
```

Copy the `[package]` fields (edition, version, license, `publish`) and the
`cimmeria-workspace-hack` line from `crates/lab-mcp/Cargo.toml`. Add
`"crates/lab-cli"` to both `members` and `default-members` in the root
`Cargo.toml`. Then run, through the lane:
`cargo hakari generate` and `cargo hakari manage-deps --yes`.

### Arguments (D-LX4)

`argv::rewrite(args: Vec<OsString>) -> Vec<OsString>` runs on the
arguments after the program name, before clap:

1. A token matching `^-[A-Za-z][A-Za-z0-9]+$` (one dash, two or more
   characters) becomes `--` plus its kebab-case lowercase form:
   `-Rows` → `--rows`, `-PlanOnly` → `--plan-only`,
   `-RunsPerLease` → `--runs-per-lease`, `-DiffGolden` → `--diff-golden`,
   `-Json` → `--json`. Kebab rule: insert `-` before an upper-case letter
   that follows a lower-case letter or digit, then lowercase.
2. A token `-Name:value` (PowerShell's colon form) becomes
   `--name=value`: `-Value:-x` → `--value=-x`.
3. Everything else is untouched: `-h`, `--json`, `FS-01,FS-02`, `-5`,
   `C:\path`, and every token after a bare `--`.
4. `-?` and `/?` become `--help`.

List-valued flags (`--rows`, `--chaos` where it is a list) use
`value_delimiter = ','` and also accept several values.

Fallback (D-LX2) forwards the **original** tokens, not the rewritten ones.

### Commands, groups and summaries

`commands::COMMANDS: &[CommandInfo]`, in this order, drives the root help
(`CommandInfo { name: &'static str, group: Group, summary: &'static str }`).
Groups print in this order: `Run and record UAT`, `Daemon`,
`Instances and clients`, `Install and configure`.

| Group | Name | Summary |
|---|---|---|
| Run and record UAT | `uat` | Run a UAT spec on the lab with no agent |
| | `watch` | Watch a running lab uat batch, read-only |
| | `record` | Record a person playing a lab instance and draft a spec from it |
| | `calibrate` | Recalibrate a row's poses by hover and propose a spec diff |
| | `golden` | Record or show a golden fingerprint |
| Daemon | `status` | Show the daemon and each instance: account, client, port, lease, seed |
| | `start` | Start the lab daemon (the CimmeriaLabDaemon scheduled task) |
| | `stop` | Stop the lab daemon and close its lab clients |
| | `restart` | Restart the lab daemon, picking up a newer cimmeria-lab.exe |
| | `logs` | Show the daemon log, filtered by instance or level |
| | `doctor` | Check the install, the daemon and each instance |
| Instances and clients | `instances` | Show each instance's account, profile and client, or write account files |
| | `clients` | Close the game client of one lab instance, or of all of them |
| Install and configure | `env` | Show, get, set or unset labd.env entries (secrets masked) |
| | `install` | Build the lab from a worktree and install it |
| | `setup` | Refresh the installed CLI and add it to the user PATH |
| | `local-setup` | One-shot setup for a lab against a local server |
| | `version` | Print the CLI and daemon versions |
| | `completions` | Print a shell completion script (LX-18) |

### Look and feel (D-LX5)

`Ui::detect()` decides once per run:

- `color`: stdout is a terminal, `NO_COLOR` is unset or empty, and
  `TERM` is not `dumb`.
- `fancy` (glyphs, box characters, live redraws): the same as `color`.
- `width`: `crossterm::terminal::size()` columns when stdout is a
  terminal, else `None` (no truncation).

**Plain mode** (`fancy == false`) prints exactly what the PowerShell
command printed, line for line, so scripts and the ported PowerShell tests
still hold. **Fancy mode** uses this style:

| Element | Style |
|---|---|
| Section heading | bold yellow (`Daemon`, `Usage`, `Examples`) |
| Command names, paths a user types | cyan |
| Secondary text, hints, ages | dim |
| Success | green `✓`; failure red `✗`; warning yellow `!`; running green `●`; stopped red `○`; in progress cyan `▶` |
| Errors | stderr, `error:` in bold red, then the message; an optional next line, indented two spaces, `hint: <text>` dim |
| Table headers | dim, upper case; two spaces between columns; the last column cut to the width with `…` |

`Ui` API (LX-01 builds it; every command uses it, no command writes ANSI
codes itself):

```rust
pub struct Ui { pub color: bool, pub fancy: bool, pub width: Option<usize> }
pub enum Tone { Heading, Cmd, Dim, Ok, Warn, Fail, Bold }
pub enum Mark { Pass, Warn, Fail, Running, Stopped, Active }
impl Ui {
    pub fn detect() -> Self;
    pub fn plain() -> Self;                                   // for tests
    pub fn paint(&self, tone: Tone, text: &str) -> String;     // text unchanged when !color
    pub fn mark(&self, mark: Mark) -> &'static str;            // "✓" .. when fancy, "" when plain
    pub fn error(&self, message: &str, hint: Option<&str>);    // to stderr
}
pub struct Table { /* headers, rows */ }
impl Table {
    pub fn new(headers: &[&str]) -> Self;
    pub fn row(&mut self, cells: Vec<String>);                 // cells may carry paint()
    pub fn render(&self, ui: &Ui) -> String;                   // pads by visible width (ANSI stripped)
}
```

Mock-ups the ports follow in fancy mode (plain mode is the PowerShell
text):

```text
$ lab status
lab daemon  ● running  pid 41236 · up 2h 3m · 0.1.0 (6ab7ffe)

INSTANCE  ACCOUNT  CLIENT  PORT  LEASE                           PROFILE
default   lab1     12044   8790  free                            seeded
p2        lab2     -       8791  claude-a1 (FS UAT) · 12m left   seeded
p3        lab3     -       -     free                            not seeded

$ lab doctor
✓ scheduled task CimmeriaLabDaemon exists
✓ daemon answers /status
! instance p3: profile not seeded
    hint: lab instances init
✗ installed cimmeria-lab.exe is the daemon's copy
    hint: lab restart
10 checks · 8 passed · 1 warning · 1 failed

$ lab uat first-session --rows FS-01,FS-02 --leases 2 --runs-per-lease 3
▶ first-session · 2 rows · 2 lanes × 3 runs · batch-20261011-013906
  default 1/3  ✓ pass  2/2 rows  4m12s
  p2 1/3       ✗ fail  FS-02: Login_PasswordEdit holds 0 ch…  [#1343]  1m02s
```

### Exit codes and JSON (D-LX5)

Each command keeps the exit codes in the
[guide's command table](../../guides/live-research-lab.md#commands) and its
PowerShell `.DESCRIPTION`. `--json` output keeps the PowerShell keys, key
order, caps and size budgets; build it with `serde_json::json!` or
`#[derive(Serialize)]` structs whose field order matches, and leave out
the keys the PowerShell leaves out. Usage errors exit 2 before anything
contacts the daemon.

### Daemon endpoint types (LX-04 builds these; later packets only use them)

```rust
// daemon/paths.rs
pub fn lab_home() -> PathBuf;            // CIMMERIA_LAB_HOME, else %LOCALAPPDATA%\cimmeria-lab (tests only override)
pub fn labd_env_path() -> PathBuf;       // <home>\labd.env
pub fn labd_pid_path() -> PathBuf;       // <home>\labd.pid
pub fn labd_log_path() -> PathBuf;       // <home>\labd.log
pub fn uat_root() -> PathBuf;            // port Get-UatRoot (uat-lib.ps1)
pub fn default_specs_dir() -> Option<PathBuf>; // port Get-DefaultSpecsDir
pub fn profile_root() -> PathBuf;        // port Get-ProfileRoot (common.ps1)
pub fn installed_cli_dir() -> PathBuf;   // <home>\cli
pub struct DaemonInfo { pub pid: Option<u32>, pub bind: String } // labd.pid; bind defaults to 127.0.0.1:8779
pub fn daemon_info() -> DaemonInfo;
// daemon/token.rs
pub fn daemon_token() -> Option<String>; // port Get-LabToken: user env (HKCU\Environment), then process env
// daemon/status.rs
pub struct Endpoint { pub base_url: String, pub token: Option<String> } // base_url = "http://<bind>"
impl Endpoint { pub fn discover() -> Self; }
pub fn fetch(ep: &Endpoint) -> Option<StatusReply>;   // GET /status, bearer, 5 s timeout; None on any failure or wrong shape
#[derive(Deserialize)] pub struct StatusReply { pub daemon: DaemonRow, pub instances: Vec<InstanceRow> }
#[derive(Deserialize)] pub struct DaemonRow { pub pid: u32, pub version: String, pub started_at: Option<String>, pub uptime_s: Option<u64> }
#[derive(Deserialize)] pub struct InstanceRow {
    pub instance: String, pub account: Option<String>, pub bridge_port: Option<u16>,
    pub client_pid: Option<u32>, pub lease: Option<serde_json::Value>, pub uat: Option<serde_json::Value>,
}
// daemon/env_file.rs
pub enum EnvLine { Pair { key: String, value: String }, Comment(String), Blank, Unparsed(String) }
pub fn read(path: &Path) -> io::Result<Vec<EnvLine>>;
pub fn get<'a>(lines: &'a [EnvLine], key: &str) -> Option<&'a str>;       // key matched ignoring case
pub fn set(lines: &mut Vec<EnvLine>, key: &str, value: &str);             // replace in place, else append
pub fn unset(lines: &mut Vec<EnvLine>, key: &str) -> bool;
pub fn write_with_backup(path: &Path, lines: &[EnvLine]) -> io::Result<PathBuf>; // labd.env.bak-<yyyyMMdd-HHmmss>[-N], then replace by a move
pub fn mask(key: &str, value: &str) -> String;                            // port Get-MaskedEnvValue / Hide-LabdEnvText
// daemon/mcp.rs (LX-05)
pub struct McpClient { /* reqwest blocking client, endpoint, session id */ }
pub enum McpError { Unreachable(String), Http(u16, String), Tool(String), Protocol(String) }
impl McpClient {
    pub fn connect(ep: &Endpoint) -> Result<Self, McpError>;                  // initialize (2025-06-18) + notifications/initialized
    pub fn call_tool(&mut self, name: &str, args: serde_json::Value, timeout: Duration) -> Result<serde_json::Value, McpError>;
    pub fn close(self);                                                       // DELETE, errors ignored
}
pub fn wait_lease_free(ep: &Endpoint, instance: &str, max: Duration) -> bool; // port Wait-UatLeaseFree
pub fn hide_lease_ids(text: &str) -> String;                                  // port Hide-LeaseIds
```

### Command shape (LX-02 builds the registry; ports fill in `run`)

```rust
// commands/status.rs (every command file looks like this)
#[derive(clap::Args, Debug)]
#[command(about = "...", long_about = "...", after_help = "Examples:\n  lab status\n  ...")]
pub struct Args { /* flags */ }
pub fn run(args: &Args, ctx: &Ctx) -> u8 { fallback::run(ctx) } // a port replaces the body
// commands/mod.rs
pub struct Ctx { pub ui: Ui, pub command: &'static str, pub raw: Vec<OsString> } // raw = original tokens after the command
```

### Tests

- Port every `Check` of the command's PowerShell test file to a `#[test]`
  (plain mode), one-to-one, naming the old check in a comment. A packet
  lists its test file; the review-fixer counts them.
- Daemon calls are tested against a stub HTTP server on `127.0.0.1:0`,
  built from `std::net::TcpListener` in `tests/support/mod.rs` (LX-04
  creates it: `StubServer::start(routes: Vec<(method, path, status, body)>) -> (StubServer, Endpoint)`).
- Tests set `CIMMERIA_LAB_HOME` to a `tempfile::TempDir`, never the real
  lab home. Tests that change the process environment take
  `support::ENV_LOCK` (a `std::sync::Mutex<()>`): nextest gives each test
  its own process, but `cargo test` runs them as threads of one.
  Integration tests that run the binary pass the environment to the
  child (`Command::env`) instead of changing their own.
- A Windows-only test is `#[cfg(windows)]`; CI's Linux runners skip it.

### Docs each port owes

In `docs/guides/live-research-lab.md` § The `lab` command: the command's
row and paragraph switch to the `--flag` form, and keep saying that the
PowerShell form still works. Nothing else per packet; LX-19 does the rest.

## LX-01: Crate, argument rewriting, UI kit, root help, fallback, `version`

**Branch** `lab-cli-rust/lx-01-crate`, worktree `lx-01`.
**Commit** `feat(lab-cli): LX-01 lab.exe skeleton with grouped help and PowerShell fallback`.

Files: everything under `crates/lab-cli/` named in § Crate layout, root
`Cargo.toml`, the hakari files `cargo hakari` changes, and one row in
`crates/README.md`.

1. Create the crate per § Crate layout and § Dependencies. `daemon/*.rs`
   files hold one `//!` line each.
2. `argv.rs`: `pub fn rewrite(args: Vec<OsString>) -> Vec<OsString>` per
   § Arguments. Non-UTF-8 tokens pass through untouched.
3. `ui/mod.rs`, `ui/table.rs`: the § Look and feel API. Visible width
   strips `\x1b[...m` sequences. `Ui::error` writes
   `error: <message>` (bold red `error:` when `color`) and, indented two
   spaces, `hint: <hint>`.
4. `commands/mod.rs`: `Ctx`, `Group`, `CommandInfo`, the `COMMANDS` table
   from § Commands, and a clap `Cli` with `#[command(name = "lab",
   disable_help_subcommand = true)]`, `version` as the only real
   subcommand, and `#[command(external_subcommand)] Other(Vec<OsString>)`
   for everything else. `commands/version.rs`: prints
   `lab <CARGO_PKG_VERSION> (<git sha or "dev">)` and, when the daemon
   answers, `daemon <version>`. LX-01 has no status client yet: print
   only the CLI line and leave a `// LX-06: daemon version` comment.
   The git sha comes from `option_env!("CIMMERIA_LAB_CLI_SHA")`; LX-03's
   install sets it.
5. `help.rs`: `pub fn root_help(ui: &Ui) -> String` renders:

   ```text
   lab - drive the Cimmeria Live Research Lab

   Usage
     lab <command> [options]
     lab help <command>      full help for one command (or: lab <command> -h)

   Run and record UAT
     uat          Run a UAT spec on the lab with no agent
     ...one line per COMMANDS entry, grouped, names padded to the longest + 2...

   Examples
     lab status                                          what is running, and who holds each lease
     lab doctor                                          check the setup
     lab logs --follow                                   follow the daemon log
     lab uat first-session --rows FS-01,FS-02 --plan-only  plan a UAT run without driving anything

   lab <version> - docs/guides/live-research-lab.md#the-lab-command
   ```

   Headings and names styled per § Look and feel; summaries cut to
   `ui.width` with `…` in fancy mode. `pub fn suggest(name: &str) ->
   Vec<&'static str>`: names that start with `name` (two or more
   characters), else names within edit distance 2.
6. `lib.rs` `main_with`: rewrite the arguments. No arguments, `help`,
   `--help` or `-h` → print `root_help`, exit 0. `help <name>`: a known
   name runs `lab <name> --help` through the fallback for now (LX-02
   switches it to clap), an unknown one is the unknown-command error.
   An unknown command: `ui.error("unknown command '<x>'", Some("did you
   mean: a, b?  run: lab help"))` (the hint names only the run part when
   there is no suggestion), exit 2. A known command other than `version`
   → `fallback::run`.
7. `fallback.rs`: `pub fn run(ctx: &Ctx) -> u8` and
   `pub fn dispatcher_path() -> Option<PathBuf>`: `CIMMERIA_LAB_CLI_DIR`
   when set, else `<exe dir>\..\cli`, and it must contain `lab.ps1`.
   Runs `pwsh -NoProfile -File <dir>\lab.ps1 <command> <raw...>` with
   inherited stdio and returns its exit code (255 when it has none). No
   dispatcher: `ui.error("the PowerShell half of the lab CLI is not
   installed", Some("run: lab setup -From <checkout>"))`, exit 1.
   `pub fn command_line(dir: &Path, ctx: &Ctx) -> Vec<OsString>` builds
   the argument list, so a test can check it without `pwsh`.
8. `crates/README.md`: add a `lab-cli` row next to the `lab` row in the
   hand-written crate table (not inside a `gen:` or `crate-graph` block):
   "`lab.exe`, the lab command-line tool; replaces `tools/lab/lab.ps1`
   command by command (lab-cli-rust campaign)".

Tests (unit, in each module):

- `argv`: every example in § Arguments, including `-Value:-x`, `-h`,
  `-?`, a Windows path, a negative number and a token after `--`.
- `ui`: `paint` returns text unchanged when `color` is false;
  `Table::render` aligns columns with painted cells (visible width);
  the last column is cut to the width with `…`.
- `help`: `root_help(&Ui::plain())` lists every `COMMANDS` name under its
  group, in group order; `suggest("stauts")` contains `status` and
  `start`; `suggest("gold")` is `["golden"]`; `suggest("zzzz")` is empty.
- `fallback::command_line` puts `-NoProfile -File <dir>\lab.ps1 <command>`
  first and the raw tokens unchanged after it (`-Rows` stays `-Rows`).
- `tests/cli.rs` (runs the built binary through
  `env!("CARGO_BIN_EXE_lab")`): no arguments exits 0 and prints `Usage`;
  `lab stauts` exits 2 with `did you mean` on stderr; `lab version` exits
  0; `lab status` with `CIMMERIA_LAB_CLI_DIR` pointing at an empty temp
  folder exits 1 with `not installed` on stderr.

Each of these fails with the step it covers reverted.

## LX-02: Command registry

**Branch** `lab-cli-rust/lx-02-registry`, worktree `lx-02`.
**Commit** `feat(lab-cli): LX-02 clap args and help pages for every lab command`.

Files: `crates/lab-cli/src/commands/*.rs` (one new file per command in
§ Commands, except `completions`), `commands/mod.rs`, `lib.rs`.

1. For each command, create `commands/<name>.rs` per § Command shape
   (`local-setup` is `local_setup.rs`). Read the PowerShell file's
   `param()` block and `.DESCRIPTION`/`.EXAMPLE`
   (`tools/lab/cli/<name>.ps1`) and write:
   - one clap field per PowerShell parameter, same meaning: `[switch]` →
     `bool` flag; `[int]` → `Option<u32>` or `u32` with the same default;
     `[string[]]` → `Vec<String>` with `value_delimiter = ','`;
     `ValidateSet` → `value_parser = ["a", "b"]`; a `Position = N`
     parameter → positional; `ValidateRange` → `value_parser =
     clap::value_parser!(u32).range(a..=b)`. Skip `[Parameter(DontShow)]`
     and test-seam parameters except `watch`'s `--status-file` and
     `--log-file`, which become `#[arg(hide = true)]`.
   - `about` = the § Commands summary; `long_about` = the `.DESCRIPTION`
     text, reflowed, with implementation notes (function names, which
     file dot-sources what, test seams) left out; `after_help` =
     `Examples:` plus the `.EXAMPLE` lines rewritten as `lab ...` with
     `--flag` names.
   - `pub fn run(args: &Args, ctx: &Ctx) -> u8 { let _ = args; fallback::run(ctx) }`.
   - `instances` takes `#[arg(trailing_var_arg = true, allow_hyphen_values = true)] rest: Vec<String>`;
     `start`, `install`, `setup` and `local-setup` use their own params as
     above.
2. `commands/mod.rs`: replace `Other` with one clap subcommand per
   command (`Command::Status(status::Args)`, ...) and `pub fn dispatch(cmd:
   &Command, ctx: &Ctx) -> u8`. Keep the unknown-command path of LX-01
   (clap's own "unrecognized subcommand" must not show: parse with
   `try_get_matches_from` and map an `InvalidSubcommand` to LX-01's error,
   exit 2). Other clap errors print clap's message and exit 2.
3. `lib.rs`: `help <name>` and `<name> -h/--help` print clap's page for
   that command, styled with `clap::builder::Styles` matching § Look and
   feel (headings bold yellow, literals cyan, placeholders dim). The root
   screen stays `help::root_help`.
4. Clap must accept the args the PowerShell commands accept today: for
   each command, the `.EXAMPLE` lines, rewritten by `argv::rewrite`,
   must parse.

Tests:

- One unit test per command: every `.EXAMPLE` line from its PowerShell
  file, rewritten, parses with `try_get_matches_from` (fails if a flag is
  missing or misnamed).
- `COMMANDS` and the clap subcommands list the same names (a test walks
  `Cli::command().get_subcommands()`).
- `tests/cli.rs`: `lab help uat` and `lab uat -h` exit 0 and print
  `--rows`; `lab uat --bogus` exits 2; `lab status --help` prints
  `Examples:`.

## LX-03: Install `lab.exe`; `setup` drops the shim

**Branch** `lab-cli-rust/lx-03-install`, worktree `lx-03`.
**Commit** `feat(lab): LX-03 install lab.exe beside cimmeria-lab.exe`.

Files: `tools/lab/install.ps1`, `tools/lab/cli/install-lib.ps1`,
`tools/lab/cli/setup.ps1`, `tools/lab/cli/test-install-lib.ps1`.

1. `install.ps1`: the host build that builds `-p cimmeria-lab` also builds
   `-p cimmeria-lab-cli` (same lane call, two `-p`), with the environment
   variable `CIMMERIA_LAB_CLI_SHA` set to `git -C <worktree> rev-parse
   --short HEAD` for that call. Copy `<target>\release\lab.exe` to
   `<lab home>\bin\lab.exe` the same way it copies `cimmeria-lab.exe`
   (rename the running file first, then copy; reuse that code, don't
   duplicate it: make it a function taking the file name).
2. `install-lib.ps1` / `setup.ps1`: after writing or refreshing the CLI
   copy, when `<lab home>\bin\lab.exe` exists, delete `bin\lab.cmd` and
   print `lab.exe installed; the lab.cmd shim is no longer needed`. When
   `lab.exe` does not exist, keep writing the shim as today.
3. `lab.exe` finds the dispatcher at `<exe dir>\..\cli\lab.ps1`, which is
   where `Install-LabCli` already puts it; check, don't move it.

Tests (`test-install-lib.ps1`, in its temp lab home): with a fake
`bin\lab.exe` present, the setup step removes `bin\lab.cmd`; without it,
`lab.cmd` is written. Run `pwsh -NoProfile -File
tools\lab\cli\test-install-lib.ps1`. No Rust checks.

Docs: the guide's "Set up the `lab` command" and "Where things live"
gain `bin\lab.exe` and lose the shim once `lab.exe` is installed.

## LX-04: Lab paths, `labd.env`, token, `/status` client

**Branch** `lab-cli-rust/lx-04-daemon-client`, worktree `lx-04`.
**Commit** `feat(lab-cli): LX-04 lab paths, labd.env and the /status client`.

Files: `crates/lab-cli/src/daemon/{paths,env_file,token,status}.rs`,
`crates/lab-cli/tests/support/mod.rs`, `crates/lab-cli/tests/daemon.rs`.

Implement § Daemon endpoint types except `mcp.rs`, porting from
`tools/lab/cli/common.ps1` (`Get-LabHome`, `Get-LabdEnvPath`,
`Get-LabToken`, `Get-DaemonInfo`, `Get-LabStatus`, `Get-ProfileRoot`),
`tools/lab/labd-lib.ps1` (`Read-LabdEnvFile`, `Write-LabdEnvFile`: UTF-8
without BOM, CRLF), `tools/lab/cli/env.ps1` (`Hide-LabdEnvText`,
`Get-MaskedEnvValue`, `Save-LabdEnv` backup naming) and
`tools/lab/cli/uat-lib.ps1` (`Get-UatRoot`, `Get-DefaultSpecsDir`).

- `status::fetch` checks the shape the way `Get-LabStatus` does
  (a `daemon` object and an `instances` array) and returns `None`
  otherwise. Before deserializing `StatusReply`, read
  `crates/lab/src/daemon/status.rs` and `crates/lab/src/lease/mod.rs`
  around `status()` to confirm field names; every field the daemon may
  omit or send as `null` is `Option` with `#[serde(default)]`.
- `token::daemon_token` on Windows reads `CIMMERIA_LAB_DAEMON_TOKEN` from
  `HKCU\Environment` with `RegGetValueW` (`RRF_RT_REG_SZ |
  RRF_RT_REG_EXPAND_SZ`), then the process environment; elsewhere only the
  process environment. An empty value is `None`.
- `tests/support/mod.rs`: `StubServer` per § Tests, answering one request
  per connection with `Content-Length`, and `ENV_LOCK`.

Tests: port the `common.ps1` checks of `test-common.ps1` (lab home
override, default lab home, `labd.env` beats the environment for the
profile root, `Get-DaemonInfo` reads `labd.pid`, `Get-LabStatus` is null
when nothing listens) and the masking and backup checks of
`test-logs-env.ps1`; plus `fetch` against the stub: 200 with the right
shape → `Some`; 401 → `None`; wrong shape → `None`; the request carried
`Authorization: Bearer <token>`.

## LX-05: MCP client and lease wait

**Branch** `lab-cli-rust/lx-05-mcp`, worktree `lx-05`.
**Commit** `feat(lab-cli): LX-05 MCP client for the lab daemon`.

Files: `crates/lab-cli/src/daemon/mcp.rs`, `crates/lab-cli/tests/mcp.rs`.
Uses `Endpoint` from LX-04: when LX-04 has not merged yet, add only the
`Endpoint` struct and `discover()` to `status.rs` exactly as § Daemon
endpoint types shows, and say so in the report.

Port `Get-LabMcpUrl`, `Read-McpBody`, `New-McpSession`,
`Send-McpRequest`, `Invoke-McpTool`, `Close-McpSession`,
`Wait-UatLeaseFree` and `Hide-LeaseIds` from `tools/lab/cli/uat-lib.ps1`
(read lines 40-250). Reference implementation of the same protocol in
Rust: `crates/lab/src/timeline/mcp_session.rs` (headers `Mcp-Session-Id`,
`Accept: application/json, text/event-stream`). Behaviour:

- `connect`: POST `initialize` (protocol `2025-06-18`, client name
  `lab-cli`, version `CARGO_PKG_VERSION`), keep the `Mcp-Session-Id`
  header, POST `notifications/initialized`.
- `call_tool`: POST `tools/call`; the body is JSON or SSE (use the last
  `data:` line); `result.isError == true` → `McpError::Tool(<text>)`;
  else parse `result.content[0].text` as JSON (a non-JSON text is
  returned as a JSON string). HTTP 401/403 → `Http`; connection refused
  or timeout → `Unreachable`.
- every request carries `Authorization: Bearer <token>` when there is one.

Tests against the stub server: a JSON reply; an SSE reply with two
`data:` lines (the last wins); `isError` → `Tool`; a non-JSON text; 401;
nothing listening → `Unreachable`; the session id from `initialize` is
sent on `tools/call`; `hide_lease_ids` redacts the same samples
`test-uat.ps1` uses for `Hide-LeaseIds`.

## LX-06: `status` and `logs`

**Branch** `lab-cli-rust/lx-06-status-logs`, worktree `lx-06`.
**Commit** `feat(lab-cli): LX-06 native lab status and lab logs`.

Files: `crates/lab-cli/src/commands/{status,logs,version}.rs`, new
`crates/lab-cli/src/log_filter.rs` (declared in `lib.rs`), tests; delete
`tools/lab/cli/status.ps1`, `tools/lab/cli/logs.ps1`; in
`tools/lab/cli/test-logs-env.ps1` delete only the `logs` checks.

- `status`: port `status.ps1` (`Get-LeaseText`, `Get-SeedText`, the daemon
  line, exit 1 when the daemon is down while still listing the
  `labd.env` instances with the seed column). Plain mode prints the
  PowerShell text; fancy mode prints the § Look and feel mock-up with a
  `Table`. Lease text: `free`, or `<owner> (<purpose>) · <ago> left`,
  never a lease id.
- `version`: add the daemon line from `status::fetch` (the
  `// LX-06` comment).
- `logs`: port `logs.ps1` and `Select-LabLogLine` (`common.ps1`) into
  `log_filter.rs` (`--lines`, `--instance`, `--level`, continuation lines
  kept with their entry, bearer and 64-hex tokens shown as `<redacted>`).
  `--follow` polls the file every 500 ms from its end, prints new
  complete lines through the same filter, and stops on Ctrl+C (exit 0).
  Fancy mode colours the level word (ERROR red, WARN yellow, INFO default,
  DEBUG/TRACE dim) and dims the timestamp.

Tests: port the `logs` checks from `test-logs-env.ps1` one-to-one; a
`status` plain-mode render from a `StatusReply` fixture (two instances,
one leased) matches the PowerShell format; daemon down → exit 1 and the
instances from `labd.env`.

## LX-07: `env`

**Branch** `lab-cli-rust/lx-07-env`, worktree `lx-07`.
**Commit** `feat(lab-cli): LX-07 native lab env`.

Files: `crates/lab-cli/src/commands/env.rs`, tests; delete
`tools/lab/cli/env.ps1`; delete `tools/lab/cli/test-logs-env.ps1` when
LX-06 has merged (else only its `env` checks).

Port `env.ps1` on top of `env_file`: verbs `get`, `set`, `unset`, none
(print all lines masked); key rule `^[A-Z][A-Z0-9_]*$`; a value with
leading or trailing spaces is refused (exit 2); `set` and `unset` print
the backup path and the hint `run: lab restart`. Exit codes per the
guide's table. Fancy mode: keys cyan, masked values dim, comments dim.

Tests: the `env` checks from `test-logs-env.ps1`, one-to-one, plus
`lab env set SOME_FLAGS -Value:-x` through `argv::rewrite` stores `-x`.

## LX-08: `doctor`

**Branch** `lab-cli-rust/lx-08-doctor`, worktree `lx-08`.
**Commit** `feat(lab-cli): LX-08 native lab doctor`.

Files: `crates/lab-cli/src/commands/doctor/{mod,checks,system}.rs`,
tests; delete `tools/lab/cli/doctor.ps1`; in `tools/lab/cli/test-ops.ps1`
delete only the doctor checks.

Port every check of `doctor.ps1` (`Get-TaskCheck` .. `Get-VersionCheck`)
as `fn(&Inputs) -> Check` in `checks.rs`, where `Inputs` holds everything
read from the system, so checks are pure. `system.rs` gathers `Inputs`:
the scheduled task (`schtasks /Query /TN CimmeriaLabDaemon`, exit code
only), `status::fetch`, the token, `labd.env`, account files, profile
seeds, running `SGW.exe` pids (Windows: `CreateToolhelp32Snapshot`;
elsewhere none), SHA-256 of `bin\cimmeria-lab.exe` and
`labd\cimmeria-lab.exe` (add nothing to `Cargo.toml`: compare bytes when
both sizes match instead of hashing), and the `tools/lab` content check
(`git diff --quiet <VERSION> origin/main -- tools/lab`, only inside a
Cimmeria checkout). Output: plain mode is the PowerShell `PASS/WARN/FAIL`
lines; fancy mode is the mock-up (glyph, text, indented dim hint, a
count line). Exit 1 when any check fails.

Tests: port the doctor checks from `test-ops.ps1` one-to-one against
`Inputs` fixtures.

## LX-09: `clients stop`

**Branch** `lab-cli-rust/lx-09-clients`, worktree `lx-09`.
**Commit** `feat(lab-cli): LX-09 native lab clients stop`.

Files: `crates/lab-cli/src/commands/clients/{mod,targets,win}.rs`,
tests; delete `tools/lab/cli/clients.ps1`, and `tools/lab/cli/test-ops.ps1`
when LX-08 has merged (else only its `clients` checks).

- `targets.rs`: port `Select-StopTargets`, `Get-LeaseHeld`,
  `Get-LeaseWho` as pure functions over `StatusReply` (D-LC5: a leased
  instance is refused naming the holder; `--force` only with a named
  instance; `all --force` exits 2 before the daemon is asked).
- `win.rs` (`cfg(windows)`): stop one pid only when its image name is
  `SGW.exe` (`QueryFullProcessImageNameW`); post `WM_CLOSE` to its
  top-level windows (`EnumWindows` + `GetWindowThreadProcessId`); wait up
  to 8 s (`WaitForSingleObject`); then `TerminateProcess`. Non-Windows:
  a stub that returns "Windows only".
- Exit codes 0/1/2/3/4 per the guide. Fancy output: one line per instance,
  `✓ stopped p2 (pid 1234)`, `✗ refused p3: leased by <owner> (<purpose>)`.

Tests: port the `clients` checks from `test-ops.ps1` one-to-one against
`targets.rs`; `all --force` exits 2 with no daemon running.
