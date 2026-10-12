---
title: Set up the Live Research Lab on your own machine
type: how-to
audience: outside contributors with a fresh clone and their own Stargate Worlds client, using Claude Code or OpenAI Codex
last_updated: 2026-10-11
companion_docs:
  - live-research-lab.md
  - automated-uat.md
  - getting-started.md
  - re-toolchain-setup.md
  - ../architecture/live-research-lab.md
  - ../../.claude/skills/lab-uat/SKILL.md
---

# Set up the Live Research Lab on your own machine

> Type: how-to. Audience: a contributor with a fresh clone, their own game client, and Claude Code or Codex. You need none of the owner's infrastructure.
> Updated: 2026-10-11. Companions: [Live Research Lab rulebook](live-research-lab.md), [automated UAT](automated-uat.md), [getting started](getting-started.md).

This guide takes you from a fresh clone to a passing `lab_uat_run` row against a server on your own machine. When you finish, your agent can start the game client, log in, read the client's Lua state and run UAT rows, and the server's lab endpoint answers its `server_*` reads.

The [rulebook](live-research-lab.md) explains what the lab is and how to use it well. This page only gets it running.

**Read [What's verified and what isn't](#whats-verified-and-what-isnt-as-of-2026-10-11) first.** Nobody has followed this guide from a fresh clone on another machine yet; you may be the first.

## What's verified and what isn't (as of 2026-10-11)

"Verified" below means a campaign ledger records a live pass. Everything else is unproven, whatever the code or a test suggests.

**Verified working** (on the owner's workstation, against the colo server):

- The shared daemon, the `lab` command and `lab doctor` (10 of 10 PASS), `lab install` through the PowerShell build lane, and `lab restart` closing the old daemon's clients: [lab-cli LC-07](../analysis/lab-cli/README.md#lc-07-live-uat-2026-10-10), 2026-10-10.
- Five lab clients in one daemon, each under its own lease, all in the world at once: [lab-parallel-clients LP-07](../analysis/lab-parallel-clients/README.md#lp-07-result-2026-10-10-colo), second run, 2026-10-10. Two of the five hit a bridge dispatch timeout at `enter_world` and passed on retry.
- The `first-session` Praxis rows FS-01 to FS-P5, hand-calibrated, five clean runs in a row: [unified UAT § First session smoke](unified-uat.md#first-session-smoke), 2026-10-10.

**Built but not yet proven live:**

- **This whole guide.** The local-server path has never been run end to end from a fresh clone on another machine. The owner pointed the lab at a local server once (2026-10-05), recorded only in project memory.
- **`lab local-setup`.** Tested only in a sandbox (process-scope variables, a temp install dir), never against a real client install.
- **The i686 step of `install.ps1`.** The "target missing, add it" branch has never run: the owner's machine already had the target.
- **Codex.** `.codex/config.toml.example` is untested. Its keys come from Codex's documentation, and whether Codex reads a project-level `.codex/config.toml` is unconfirmed.
- **`server: "Local"` in `lab-account.json`.** That `lab_login` falls back to the `Test` shard is read from the code, not observed.
- **Pending live acceptances:** SV-15, the spec-vocabulary acceptance ([worknote](../analysis/lab-spec-vocab/worknotes/SV-15.md); paused after a loaded workstation starved the client); FX-12, row fixtures ([ledger](../analysis/lab-fixtures/README.md)); GD-10, the first golden ([ledger](../analysis/lab-golden/README.md); waits on SV-15); LW-05, `lab watch` ([ledger](../analysis/lab-watch/README.md); waits on a decision). Until they pass, treat fixtures, goldens, calibration and `lab watch` as unproven on a live client.

**Known gaps and limits:**

- `-Chaos` is not live: until [lab-chaos](../analysis/lab-chaos/README.md) CH-08 wires the runner, a real `lab uat` run refuses it (exit 2) and only `-PlanOnly` accepts it. `lab record` is not built yet ([lab-record](../analysis/lab-record/README.md)).
- One agent drives at a time; the lease enforces it.
- Ask the user before any lab use. The client takes focus and the lab's input hooks swallow your own clicks while it runs.
- A loaded machine starves the client: SV-15's third attempt failed at 91 % CPU with every build-lane slot busy. Run the lab on an idle machine.
- A screensaver stops the client rendering and looks like a crash loop ([The display](live-research-lab.md#the-display-screensaver-and-d3d)).
- Account names and passwords must stay inside letters, digits, space and `-_/.`.
- At most five clients per daemon.
- Open lab issues: #1341 (FS-P2 "Corporal Frost is known"), #1342 (orphaned `SGW.exe`), #1343 (the login password box reads back empty).

## What you don't need

The owner's setup runs the lab against a colo server over a VPN, with SigNoz and a key vault. A local lab needs none of that:

- **No colo, no WireGuard VPN.** Your server runs on `127.0.0.1`.
- **No SigNoz.** The lab still runs without it. Row clauses that read SigNoz come back UNVERIFIED until you attest them, and the two rows this guide runs read none.
- **No Azure Key Vault, no signing keys, no launcher manifest.** You generate every secret below yourself, and they never leave your machine.
- **No Dev Drive.** The build lane uses one when it exists and `target\` when it doesn't.
- **No AteraLoader.** The lab injects its own DLL.

## Ask before you drive

The lab drives a real game window on your desktop, and its input hooks swallow your own mouse clicks while it runs. An agent asks you, and waits for a yes, before it takes the lab lease, starts or restarts the client, or changes the server the lab points at. This is the first rule of the [`lab-uat` skill](../../.claude/skills/lab-uat/SKILL.md). Tell your agent the same if your harness doesn't load the skill.

## Prerequisites

| You need | Why | Get it |
|---|---|---|
| Windows 10 or 11 | The client and the lab are Windows-only | — |
| PowerShell 7 | Every lab script is `pwsh` | `winget install Microsoft.PowerShell` |
| Rust through rustup | Builds the server and the lab; the lab install adds the 32-bit target itself | <https://rustup.rs> |
| Your own Stargate Worlds client | The repo doesn't ship it | [`game/sgw/README.md`](../../game/sgw/README.md) |
| Claude Code or the Codex CLI | Runs the agent that calls the lab tools | — |

In this guide, **the install dir** is the client folder that holds `Binaries\SGW.exe` and `SGWGame\`. In a stock install it is the `Working` folder, for example `D:\Games\Stargate Worlds-QA\Working`.

## The secrets

Four values matter, and the local-setup helper (step 2) makes three of them. None of them is shared with anyone else, and none goes in a tracked file.

| Name | What it's for | Made by | Where it lives | Read by |
|---|---|---|---|---|
| `CIMMERIA_LAB_DAEMON_TOKEN` | Bearer token for the lab daemon's `/mcp` and `/status` | `daemon.ps1 install`, when it is missing (32 random bytes as hex) | Your user environment | The daemon; Claude Code through `tools/lab/labd-headers.ps1`; Codex through `bearer_token_env_var`; the `lab` command |
| `CIMMERIA_LAB_MCP_TOKEN` | Bearer token for the server's lab endpoint (`server_*` tools) | The local-setup helper or you, step 2 (32 random bytes) | Your user environment, which the server you start from a new terminal inherits. Never in `labd.env` or `.mcp.json` | `cimmeria-server`; the daemon (for `lab_timeline` and UAT server clauses); the `lab-server` MCP entry |
| `CIMMERIA_TELEMETRY_HMAC_SECRET` | Signs the telemetry token `lab_client_start` mints from your server | The local-setup helper or you, step 2 (64 random bytes; at least 32) | Your user environment, which the server inherits; nothing else needs it | `cimmeria-server` |
| `lab-account.json` | The account and character the lab logs in with | The local-setup helper or you, step 4 | `<install dir>\Binaries\sessions\lab-account.json` | The supervisor (`lab_login`, and the relogin after a crash) |

Two more values look like secrets, but you never set them:

- `CIMMERIA_LAB_TOKEN` is the token between the supervisor and the bridge DLL. The supervisor makes a fresh one at every launch and writes it to `current-session.json`. You'd set it only to attach to a client the lab didn't start.
- `CIMMERIA_LAB_MCP_BIND` is the address the server's lab endpoint listens on (the helper sets `127.0.0.1:8451`). It's not secret, but without it the endpoint stays off.

## Steps

### 1. Bootstrap the repo

From the repo root:

```powershell
pwsh setup.ps1 -NoLaunch
```

This builds the server, starts PostgreSQL on port 5433 and loads the seed data. The seed includes the lab accounts `lab` and `lab2` to `lab5`, all with the password `test`. The [getting-started tutorial](getting-started.md) explains each phase.

### 2. Run the local-setup helper

The helper makes the secrets, writes `lab-account.json` and, if you ask, adds your server to the client's login list:

```powershell
pwsh tools/lab/cli/local-setup.ps1 -InstallDir 'D:\Games\Stargate Worlds-QA\Working' -PatchLoginInternal
```

Once the `lab` command is set up (step 6), the same thing is `lab local-setup`. Its flags:

| Flag | What it does |
|---|---|
| `-InstallDir <folder>` | The install dir. Default: `CIMMERIA_LAB_INSTALL_DIR`, else `labd.env` |
| `-PatchLoginInternal` | Adds the `Local` login row to `LoginInternal.lua` (step 3). Without it, the helper prints the line and where it goes |
| `-Worktree <checkout>` | The checkout whose pinned toolchain gets the i686 target |
| `-EnvScope User\|Process` | Where the variables go; `Process` is for the helper's tests only |

It sets these user environment variables, keeping any that already hold a valid value, and never prints a value:

| Variable | Value |
|---|---|
| `CIMMERIA_LAB_MCP_TOKEN` | 32 random bytes as hex, from .NET's cryptographic random number generator |
| `CIMMERIA_TELEMETRY_HMAC_SECRET` | 64 random bytes as hex |
| `CIMMERIA_LAB_MCP_BIND` | `127.0.0.1:8451` |
| `CIMMERIA_LAB_MCP_URL` | `http://127.0.0.1:8451/mcp` |
| `CIMMERIA_LAB_INSTALL_DIR` | Your install dir |

It ends with a "Left to do" list, which is the rest of this guide.

**Open a new terminal** afterwards, and restart Claude Code or Codex: a running process doesn't see a user variable set after it started, and that includes the server you start in step 5.

To make the two secrets by hand instead, this one-liner needs no `openssl`:

```powershell
[Environment]::SetEnvironmentVariable('CIMMERIA_LAB_MCP_TOKEN', [Convert]::ToHexString([Security.Cryptography.RandomNumberGenerator]::GetBytes(32)).ToLower(), 'User')
[Environment]::SetEnvironmentVariable('CIMMERIA_TELEMETRY_HMAC_SECRET', [Convert]::ToHexString([Security.Cryptography.RandomNumberGenerator]::GetBytes(64)).ToLower(), 'User')
```

The server decodes a hex HMAC secret to bytes and needs at least 32. The lab endpoint measures its token as text and needs at least 32 characters.

### 3. Point the client at your server

The client reads its login servers from `<install dir>\SGWGame\Content\UI\Startup\Login\LoginInternal.lua`. With `-PatchLoginInternal`, the helper adds this row after the last existing row inside `LoginMod.loadServerSystems()`. It saves a backup first (`LoginInternal.lua.<yyyymmdd-HHmmss>.bak`) and keeps the file's CRLF line endings:

```lua
LoginMod.servers["Local"] = "http://127.0.0.1:8081"
```

An existing `Local` row that points elsewhere is reported and left alone. Without the flag, add the row yourself in the same place.

Port `8081` is the server's login port (`LOGON_PORT`). The lab reads this file too: `lab_client_start` mints its telemetry token from the login URL of the row named by `server` in `lab-account.json`, or from the only row when there is one. With several rows and no match, the start is refused (see [Troubleshooting](#troubleshooting)).

The Cimmeria desktop launcher rewrites this file before each of its own launches; lab launches don't. If you also play through the launcher on this install, check the file again before a lab session.

### 4. Check `lab-account.json`

The helper writes `<install dir>\Binaries\sessions\lab-account.json` when it's absent. By hand, it is:

```json
{
  "server": "Local",
  "username": "lab",
  "password": "test",
  "character": "Labone"
}
```

- `server` names the `LoginInternal.lua` row the lab mints its telemetry token from. `lab_login` also tries it at server select; your local server lists one shard, `Test`, and when the name isn't in that list `lab_login` logs a warning and uses the preselected row.
- `username` and `password` are the seeded `lab` account. `lab_login` types them with native key presses, which cover letters, digits, space and `-_/.` only. A password with any other character fails the login and every relogin after a crash.
- `character` is the lab character's name, letters only. The lab protects it from deletion and plays it after a crash relaunch. No lab character is seeded: create it once with `lab_create_character`, or `lab_ensure_in_world` with `create` (step 10).

The file is under the game install, not the repo, so it can't be committed by accident.

### 5. Run the server with the lab endpoint

In a terminal opened after step 2, at the repo root:

```powershell
.\cimmeria-server.exe
```

The server reads `CIMMERIA_LAB_MCP_BIND`, `CIMMERIA_LAB_MCP_TOKEN` and `CIMMERIA_TELEMETRY_HMAC_SECRET` from your user environment. Keep `OTEL_EXPORTER_OTLP_ENDPOINT` unset; you have no collector. Look for this line in the log:

```text
lab MCP endpoint listening addr=127.0.0.1:8451
```

`lab MCP endpoint refused to start` means the token is shorter than 32 bytes. No line at all means the bind or the token is missing, usually because the terminal is older than step 2. Keep the endpoint on `127.0.0.1`.

Locally the endpoint uses port 8451, because 8444, the port the colo uses inside its container, is the default of the dev supervisor (`cimmeria-supervisor`, `SUPERVISOR_PORT`) on a dev box.

### 6. Install the lab

The lab runs from installed copies under `%LOCALAPPDATA%\cimmeria-lab\`, not from a target dir. The first install names your client:

```powershell
pwsh tools/lab/install.ps1 -InstallDir 'D:\Games\Stargate Worlds-QA\Working'
pwsh tools/lab/cli/setup.ps1
```

`install.ps1` adds the `i686-pc-windows-msvc` Rust target when it's missing (`rustup target add`), because the 32-bit client needs 32-bit DLLs. It then builds five pieces through the build lane: the supervisor, the `lab.exe` command, the bridge DLL, the `sgw-start32.exe` injector and the client patches DLL. The first build takes a while. `cli/setup.ps1` copies the PowerShell scripts `lab` still runs (`lab start`, `stop`, `restart`, `instances`, `install`, `setup` and `local-setup`) and offers to put `lab.exe` on your `PATH`; open a new terminal afterwards.

After a `git pull`, refresh both with `lab install -From <your checkout>`, then `lab restart`.

### 7. Install the daemon

The daemon is one shared supervisor for every agent session on your machine. It's the default setup.

```powershell
pwsh tools/lab/daemon.ps1 install
```

It copies the supervisor, generates `CIMMERIA_LAB_DAEMON_TOKEN` in your user environment when it's missing, writes `%LOCALAPPDATA%\cimmeria-lab\labd.env`, and registers the per-user scheduled task `CimmeriaLabDaemon`, which starts at logon in your desktop session. Its flags are `-InstallDir` (`CIMMERIA_LAB_INSTALL_DIR`) and `-LabServerUrl` (`CIMMERIA_LAB_MCP_URL`, default `http://127.0.0.1:8451/mcp`); `-Telemetry optional` writes `CIMMERIA_LAB_TELEMETRY=optional`. Each key comes from, first to last: these flags, the `env` block of a stdio `cimmeria-lab` entry in `.mcp.json` if you have one, your process and user environment (so the helper's `CIMMERIA_LAB_INSTALL_DIR` is enough), then the defaults; the patches DLL defaults to the one in `bin\` when it is there. The daemon reads `CIMMERIA_LAB_MCP_TOKEN` and `_URL` from your user environment when it runs, and no token is ever written to `labd.env`. A legacy stdio `cimmeria-lab` entry that holds a literal token has that key skipped, with a warning: keep the token in your user environment. `install -Force` rewrites `labd.env` but keeps the keys you added with `lab env set`.

`labd.env` is the daemon's whole environment. Add the keys it can't guess with the `lab` command:

```powershell
lab env set CIMMERIA_LAB_UAT_SPECS 'C:\src\Cimmeria\docs\guides\uat-specs'
lab env set CIMMERIA_LAB_OPERATOR 'your-handle'
lab restart
```

- `CIMMERIA_LAB_UAT_SPECS` matters: the daemon runs from `%LOCALAPPDATA%\cimmeria-lab`, not from your checkout, so it can't find the specs on its own.
- `CIMMERIA_LAB_PATCHES_DLL` is set for you when `bin\cimmeria_client_patches.dll` exists. Unset, the client runs with the bridge only.
- `CIMMERIA_LAB_OPERATOR` is optional. It's the name each UAT run's manifest records as its operator; the default is `lab agent`.

The [rulebook's `labd.env` table](live-research-lab.md#labdenv-keys) lists every key.

### 8. Register the MCP servers

Two servers make up the lab: `cimmeria-lab` (the daemon) and `lab-server` (your server's endpoint).

#### Claude Code

Copy `.mcp.json.example` to `.mcp.json` (gitignored) and replace each `<CIMMERIA_ROOT>` with your checkout's absolute path, with doubled backslashes. `pwsh setup.ps1 -WithReToolchain` writes it for you when no `.mcp.json` exists. The two lab entries look like this:

```json
"cimmeria-lab": {
  "type": "http",
  "url": "http://127.0.0.1:8779/mcp",
  "headersHelper": "pwsh -NoProfile -File \"C:\\src\\Cimmeria\\tools\\lab\\labd-headers.ps1\"",
  "timeout": 1800000
},
"lab-server": {
  "type": "http",
  "url": "${CIMMERIA_LAB_MCP_URL:-http://127.0.0.1:8451/mcp}",
  "headers": { "Authorization": "Bearer ${CIMMERIA_LAB_MCP_TOKEN:-}" }
}
```

**Keep the `timeout`.** Without it Claude Code gives an HTTP server 60 seconds per call, and a `lab_uat_run` row that runs longer fails with "The operation timed out." while the row is still running.

Claude Code asks before it starts a server from `.mcp.json`. To approve the lab servers once, put this in your own `.claude/settings.local.json` (untracked; merge it with what's there):

```json
{
  "enabledMcpjsonServers": ["cimmeria-lab", "lab-server"],
  "permissions": {
    "allow": [
      "mcp__cimmeria-lab__lab_lease_status",
      "mcp__cimmeria-lab__lab_client_status",
      "mcp__cimmeria-lab__lab_screenshot",
      "mcp__cimmeria-lab__client_ui_state",
      "mcp__cimmeria-lab__client_player_state",
      "mcp__lab-server__server_sessions",
      "mcp__lab-server__server_log_tail"
    ]
  }
}
```

The `allow` list is optional. It names only read-only tools, so your agent still asks before anything that drives the client. Restart Claude Code, then run `/mcp`: both servers should show as connected.

#### Codex

Codex reads MCP servers from TOML. Copy `.codex/config.toml.example` to `.codex/config.toml` in your checkout (gitignored), which Codex loads once you trust the project, or merge its entries into `~/.codex/config.toml`. The lab entries:

```toml
[mcp_servers.cimmeria-lab]
url = "http://127.0.0.1:8779/mcp"
bearer_token_env_var = "CIMMERIA_LAB_DAEMON_TOKEN"
startup_timeout_sec = 30
tool_timeout_sec = 1800

[mcp_servers.lab-server]
url = "http://127.0.0.1:8451/mcp"
bearer_token_env_var = "CIMMERIA_LAB_MCP_TOKEN"
tool_timeout_sec = 300
```

Codex reads both tokens from its own environment, so start it from a terminal opened after steps 2 and 7. Codex doesn't expand `${VAR}` in a `url`, so the `lab-server` URL is written out. The long tool timeout plays the same part as Claude Code's `timeout`. An older Codex may also need `experimental_use_rmcp_client = true` for HTTP servers. The repo tracks a Codex twin of every Claude Code agent in `.codex/agents/`, generated from `.claude/agents/` by `python tools/agent-skills/sync.py`; the one that drives the lab is `lab-driver.toml`.

### 9. Check everything with `lab doctor`

```powershell
lab doctor
```

No line should read `FAIL`. The checks are listed in the rulebook's [`lab doctor` table](live-research-lab.md#commands). Two `WARN`s are expected on a fresh setup: the instance profile is not seeded until the first client launch, and the version check warns when your checkout's `tools/lab` differs from `origin/main` (a branch, or a stale fetch).

### 10. Run your first rows

Ask your agent for a plan-only run first. It drives nothing and needs no lease:

```json
{ "sections": ["first-session"], "rows": ["FS-01", "FS-02"], "plan_only": true }
```

Both rows should come back `SKIPPED` (ready). A `BLOCKED` row names what's missing. Then, once you've said yes, the real run: the same call without `plan_only`. FS-01 starts the client and checks the bridge heartbeat; FS-02 logs in as `lab` and reaches character select. The evidence lands in `%LOCALAPPDATA%\cimmeria-lab\uat-runs\`.

Without an agent, the `lab` command runs the same rows:

```powershell
lab uat first-session -Rows FS-01,FS-02 -PlanOnly
lab uat first-session -Rows FS-01,FS-02
```

Next, get the lab character into the world with `lab_ensure_in_world {create: {alignment, archetype, gender}}`, then read [automated UAT](automated-uat.md) and the [rulebook](live-research-lab.md).

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `lab_client_start` fails with a mint error, or the server answers 500 on `/api/auth/dev-session` | The server has no `CIMMERIA_TELEMETRY_HMAC_SECRET`, or it is under 32 bytes | Set it (step 2) and restart the server. For offline work only, `lab env set CIMMERIA_LAB_TELEMETRY optional` and `lab restart` launch without telemetry uploads |
| `lab_client_start` says `LoginInternal.lua lists N login servers ... and none is named ...` | Several rows in `LoginInternal.lua` and none matches `server` | Add the `Local` row (step 3) so it matches `server`, or set `CIMMERIA_LAB_SERVER_URL=http://127.0.0.1:8081` in `labd.env` |
| The client logs in somewhere other than your server | A leftover row in `LoginInternal.lua`, or the launcher rewrote it | Re-check the file (step 3) |
| `install.ps1` fails at `rustup target add` | No rustup on `PATH`, or no network | Install Rust with rustup, then rerun; `rustup target add i686-pc-windows-msvc` by hand also works |
| `lab_client_start` refuses because an `SGW.exe` runs outside the lab | You started the game yourself, or a client outlived a daemon restart | Close it; `lab doctor` names the pids |
| `lab_login` refuses the password before typing | A character outside letters, digits, space and `-_/.` | Use the seeded `test`, or another password inside that set |
| `lab-server` tools missing, or `401` | The server's token and the one Claude Code or Codex sends differ, or the client started before step 2 | The server and the agent read the same user variable: restart both from a new terminal after changing it, then `lab restart` |
| Rows BLOCKED with "needs the server lab MCP (CIMMERIA_LAB_MCP_URL/_TOKEN)" | `labd.env` lacks `CIMMERIA_LAB_MCP_URL`, or the daemon started before `CIMMERIA_LAB_MCP_TOKEN` was in your user environment | `lab env set CIMMERIA_LAB_MCP_URL http://127.0.0.1:8451/mcp`, then `lab restart` (rerunning `daemon.ps1 install` keeps the existing `labd.env`) |
| `lab_uat_run` says `no spec directory` | The daemon can't find the specs | `lab env set CIMMERIA_LAB_UAT_SPECS <checkout>\docs\guides\uat-specs`, then `lab restart`, or pass `specs_dir` |
| `lab_uat_run` fails with "The operation timed out." after a minute | The `.mcp.json` entry has no `timeout` | Add `"timeout": 1800000` (step 8) |
| `lab doctor`: daemon doesn't answer `/status` | The task isn't running | `pwsh tools/lab/daemon.ps1 status`, then `lab start`; the log is `%LOCALAPPDATA%\cimmeria-lab\labd.log` |

More failure modes are in the rulebook and in [automated UAT § Troubleshooting](automated-uat.md#troubleshooting).

## Related

- [Live Research Lab rulebook](live-research-lab.md): the rules, every tool, the daemon, the `lab` command.
- [Automated UAT](automated-uat.md): `lab_uat_run`, specs, grades and evidence.
- [The lab ADR](../architecture/live-research-lab.md): why the lab is built this way.
- [RE toolchain setup](re-toolchain-setup.md): Ghidra and x64dbg, the lab's static and debugger partners.
- [`lab-uat` skill](../../.claude/skills/lab-uat/SKILL.md): how an agent drives the lab well.
