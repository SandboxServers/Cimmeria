---
title: Troubleshooting
type: how-to
audience: new contributors, operators
last_updated: 2026-09-26
companion_docs:
  - building.md
  - architecture/build-system.md
  - guides/getting-started.md
  - ../CLAUDE.md
  - ../bootstrap/README.md
  - known-issues.md
  - multiplayer.md
---

# Troubleshooting

Common first-day problems and how to fix them. If a problem isn't here, open an issue with the exact error message, platform, and which step it died on — the maintainers will help and the answer becomes the next entry.

For long-standing bugs in the emulator itself (rather than setup), see [`known-issues.md`](known-issues.md).

---

## Build & toolchain

### `sccache: incremental compilation is prohibited`

**Symptom.** A `cargo` command fails as soon as it starts compiling, with `sccache: incremental compilation is prohibited: Unset CARGO_INCREMENTAL to continue.`

**Root cause.** `CARGO_INCREMENTAL` is set to something other than `0` while sccache is the `RUSTC_WRAPPER`, and sccache refuses to run under it. The build lane (`tools/build-lane/lane.sh`) handles this by dropping sccache for that job. A direct `cargo` call with `RUSTC_WRAPPER` set to sccache in your environment doesn't.

**Fix.** Unset `CARGO_INCREMENTAL`. You don't need it: the dev profile already builds workspace crates incrementally, and sccache only caches third-party crates.

```powershell
Remove-Item Env:CARGO_INCREMENTAL    # PowerShell
unset CARGO_INCREMENTAL              # Git Bash
```

Or run the command through the lane: `bash tools/build-lane/lane.sh cargo <args>`. Why the two settings clash is in [`architecture/build-system.md`](architecture/build-system.md) §3.

---

### A new worktree won't build: missing Recast/Detour headers

**Symptom.** In a worktree, the `cimmeria-entity` build script fails with a missing-header error. The error doesn't mention `external/`.

**Root cause.** `crates/entity/build.rs` compiles Detour from `../../external/recast`, and `external/` is not in git. `git worktree add` gives you a worktree without it.

**Fix.** Create worktrees with the lane's script, which junctions `external/` in from the main checkout (and seeds the target dir when you have a Dev Drive):

```bash
bash tools/build-lane/mk-worktree.sh <branch> <name>    # creates .claude/worktrees/<name>
```

For a worktree you already created by hand, add the junction yourself from PowerShell:

```powershell
New-Item -ItemType Junction -Path <worktree>\external -Target <main checkout>\external
```

When you're done with such a worktree, retire it with `bash tools/build-lane/rm-worktree.sh <name>`, which removes the junction first. A recursive delete can follow the junction and empty the real `external/`. See [`agents/development-workflow.md`](agents/development-workflow.md#retire-it-when-its-pr-merges).

---

### Lane refuses to start: `[lane] refusing to start: N GB free`, or builds fail with "os error 112"

**Symptom.** A lane job exits with code 28 before cargo runs:

```text
[lane] refusing to start: 6 GB free on the drive that holds B:/targets/<worktree>,
[lane] below LANE_MIN_FREE_GB=10. Cargo would fail part-way with "os error 112".
```

Or, from a build that doesn't go through the lane, or from one that filled the disk while it ran, cargo fails part-way with `There is not enough space on the disk. (os error 112)` (`no space left on device` on Linux), often in several sessions at once.

**Root cause.** The drive that holds the target dirs, usually the Dev Drive (`B:\targets\<worktree>`), is nearly full. Every worktree keeps its own target dir of several GB, and worktrees whose PRs had merged were not retired: on 2026-09-26 they filled the 150 GB drive, and on 2026-09-28 it filled twice more and failed every agent's build. Since then the lane checks free space before each job. Below `LANE_MIN_FREE_GB` (default 10) it first prunes the worktree's stale incremental sessions, and if that isn't enough it refuses rather than let cargo die part-way.

**Fix.** Free space, then run the job again. First retire merged worktrees, which frees the most:

```bash
bash tools/build-lane/rm-worktree.sh --dry-run --merged
bash tools/build-lane/rm-worktree.sh --merged
```

It skips worktrees with open PRs, uncommitted changes, recent activity or a running build. It also deletes Dev Drive target dirs whose worktree no longer exists. If that isn't enough, trim the target dirs that remain: stale incremental caches and feature variants no build has used for a day.

```powershell
pwsh tools/build-hygiene/sweep.ps1 -DryRun   # what it would free, per target dir
pwsh tools/build-hygiene/sweep.ps1           # skips dirs a lane job is building in
```

Check free space with `(Get-Volume -DriveLetter B).SizeRemaining / 1GB` in PowerShell. `LANE_MIN_FREE_GB=0` turns the lane's check off, but a build that runs out of disk may leave a target dir that needs `cargo clean -p <crate>`.

---

### A build waits with `[lane] all N slots busy`

**Symptom.** A build started through the lane prints `[lane] all 4 slots busy: [...] ... 60s` once a minute and doesn't start.

**Root cause.** Other builds hold every lane slot. The brackets show each slot's worktree and command, so you can see who is building what.

**Fix.** Wait: the lane starts your build as soon as a slot frees. You don't need to clean up after a crashed build, because a slot whose holder died is freed by the next caller (`[lane] breaking stale slot ...`). Leave other sessions' builds alone. To see which jobs are slow, run `python tools/build-lane/lane_stats.py --recent 20`. Don't raise the slot count in `%LOCALAPPDATA%\cimmeria-build\lane\SLOTS` without measuring first. The job log's lowest-free-RAM figure and `tools/build-metrics/measure-build.ps1` are how you measure; see [`architecture/build-system.md`](architecture/build-system.md) §10.

---

### `cargo check` is fast but `cargo build` takes forever

**Symptom.** `cargo check -p cimmeria-cell` completes in seconds but `cargo build` of the same crate takes minutes.

**Root cause.** This is normal — `cargo check` runs the type-checker without codegen (no machine code emitted). `cargo build` actually compiles to object files and links. The first full build is the slow one; incremental builds are much faster.

**Fix.** Use `cargo check` for iteration. Only `cargo build` (or `cargo run`) when you actually need a runnable binary. Use `cargo nextest run -p <crate>` rather than `cargo test` for the test loop.

---

### Linker error: `link.exe` not found / `cl.exe` not found

**Symptom.** A cargo build fails with a message like `error: linker 'link.exe' not found` or `error[E0463]: can't find crate for ...` mentioning MSVC.

**Root cause.** No MSVC build tools installed. Rust on Windows uses MSVC by default.

**Fix.** Install Visual Studio Build Tools 2022 with the "Desktop development with C++" workload. The `link.exe` and `cl.exe` it provides are what `rustc` invokes. Restart your terminal after install so `PATH` picks them up.

---

### Stale Cargo lock file / target dir

**Symptom.** Errors about "package not found in workspace" or unexpected version conflicts after pulling `main`.

**Fix.**

```powershell
cargo clean -p cimmeria-services    # Or whichever crate is acting up
cargo update                         # Refresh Cargo.lock against latest deps
cargo check -p cimmeria-services
```

`cargo clean` without `-p` nukes the entire target dir — only do that as a last resort, since the next build will be slow. If you only want the disk space back, [`tools/build-hygiene/sweep.ps1`](../tools/build-hygiene/sweep.ps1) removes stale artifacts and keeps the ones your next build needs (don't run it while anything builds).

---

## PostgreSQL & database

### PostgreSQL won't start — port 5433 already in use

**Symptom.** `setup.ps1` aborts at the Database step with `could not bind to port 5433` or `address already in use`.

**Root cause.** Another Postgres instance is already on 5433 — either a previous Cimmeria run, a system-installed Postgres, or pgAdmin.

**Fix.**

```powershell
# Find what's listening:
Get-NetTCPConnection -LocalPort 5433 -State Listen

# If it's a previous Cimmeria run, stop it cleanly:
Import-Module ./bootstrap/CimmeriaBootstrap
Stop-CimmeriaServer

# Or use Docker so it picks a different port mapping:
pwsh setup.ps1 -UseDocker
```

If you have a system Postgres on 5433 that you can't move, you can re-run with `-UseDocker` and it will host PostgreSQL in a container that exposes 5433 on the loopback — but only if 5433 is free. Otherwise change the system Postgres port.

---

### `pgdata` directory has wrong version

**Symptom.** `setup.ps1` aborts with `pgdata version mismatch — use -ResetDatabase`.

**Root cause.** You upgraded the bundled PostgreSQL major version and the existing `pgdata` directory was initialised with the older version. Postgres refuses to start with a wrong-version data directory.

**Fix.** Use the nuclear flag — this **deletes** the data directory and any data in it:

```powershell
pwsh setup.ps1 -ResetDatabase -SkipBuild -NoLaunch
```

You'll lose any custom state. If you cared about it, the seed `test` account and shard record are re-inserted automatically.

---

### `DATABASE_URL` not set / live-DB tests skip silently

**Symptom.** You ran `cargo nextest run` on a crate with live-DB tests and they show as "skipped" rather than failing.

**Root cause.** The `require_db_or_skip!` macro that gates every live-DB test checks `DATABASE_URL`. If unset, the test self-skips so contributors without a local DB don't see false failures.

**Fix.** Start the bundled Postgres, then export the URL:

```powershell
# Bundled local PG:
$env:DATABASE_URL = "postgres://w-testing:w-testing@localhost:5433/sgw"

# Docker PG:
$env:DATABASE_URL = "postgres://w-testing:w-testing@localhost:5433/sgw"

tools/test-live-db.ps1
```

The `ci-live-db` profile serialises tests (`threads-required = "num-test-threads"`) because some guards share sentinel id ranges. Don't try to parallelise it — see [`docs/architecture/integration-test-infra.md`](architecture/integration-test-infra.md).

---

### Database load fails partway with "already exists"

**Symptom.** `setup.ps1 -ForceDatabase` reports "relation already exists" errors during the schema load.

**Root cause.** A previous load was interrupted mid-way — the schema is partially in place but seed data isn't, and the script's idempotent path didn't detect the partial state.

**Fix.**

```powershell
pwsh setup.ps1 -ForceDatabase -SkipBuild -NoLaunch
```

`-ForceDatabase` drops and recreates the `sgw` database before loading, so a partial load is cleaned up. If `-ForceDatabase` alone still fails, escalate to `-ResetDatabase`.

---

## Client connection

### Game client can't connect — "could not reach server"

**Symptom.** The launcher logs in successfully (HTTP auth on port 8081 returns 200) but then the client times out before reaching character select.

**Root cause.** `BASE_EXTERNAL` is the IP the auth server tells the client to use for the BaseApp UDP connection (port 32832). Default is `127.0.0.1`. The client literally connects to whatever IP `BASE_EXTERNAL` resolves to — if it's wrong, the UDP packets go nowhere.

**Fix.** For local play, the default is correct. For LAN play, set `BASE_EXTERNAL` to your **server's LAN IP** before starting the server:

```powershell
$env:BASE_EXTERNAL = "10.0.0.42"
.\cimmeria-server.exe
```

This is **not a bind address** — the server already binds to `0.0.0.0`. `BASE_EXTERNAL` is the IP embedded in the Phase 2 XML response. Setting it to `0.0.0.0` does **not** work (clients can't connect to a wildcard).

Full LAN-setup details in [`multiplayer.md`](multiplayer.md).

---

### Client connects but immediately disconnects after character select

**Symptom.** Login + character select work, but the client drops with "lost connection to server" right when entering the world.

**Root cause.** Several possibilities:

- A wire-format mismatch between the version of the server you built and what the client expects (rare in current `main`, common when bisecting old commits).
- The Cell service failed to start. Check the log for `cell service ready`.
- The shard's `BASE_EXTERNAL` is wrong and the world-entry handshake (which re-uses that address) can't complete.

**Fix.** Check the server log starting at the moment of the disconnect. Look for `mercury_send_error` or `client_handshake_failed` lines — they identify the failing message. If the issue is a wire-format regression you've just introduced, the live-DB and wire-format tests in `crates/services` should be your first stop.

---

### AtreaRL won't launch / "DLL not found"

**Symptom.** AtreaRL (the launcher) crashes or reports a missing DLL.

**Root cause.** AtreaRL is the **original** SGW runtime patcher (`AtreaRL.dll`) loaded by AtreaLoader. We don't ship it; you need a SGW client installation that already has it.

**Fix.** See [`game/sgw/README.md`](../game/sgw/README.md) for client installation. The Cimmeria-authored launcher (`crates/launcher/`, the egui native one) is separate and not the same thing.

---

## Repository & filesystem

### `external/` directory missing after fresh clone

**Symptom.** Files reference `external/postgresql/` or `external/sgw-client/` but the directory doesn't exist.

**Root cause.** `external/` is **not in git**. It's populated by `setup.ps1`. A fresh clone looks broken until you run setup.

**Fix.**

```powershell
pwsh setup.ps1 -SkipBuild -NoLaunch    # Just populates external/ + DB
```

This is a documented repo invariant — see [`CLAUDE.md`](../CLAUDE.md) → "Repo invariants."

---

### `cimmeria-server.exe` is missing after build

**Symptom.** `setup.ps1` reports success but `.\cimmeria-server.exe` isn't at the repo root.

**Root cause.** `setup.ps1` doesn't auto-copy the binary to the root after build — but several runbooks assume it does. The actual binary lives in `target/<profile>/cimmeria-server.exe`, or, for a build that went through the build lane on a Dev Drive, in `$CIMMERIA_TARGET_ROOT\<worktree>\<profile>\` (the lane prints the target dir when it starts).

**Fix.** Either:

```powershell
# Run via cargo:
cargo run -p cimmeria-server

# Or copy manually after build:
Copy-Item .\target\debug\cimmeria-server.exe .
.\cimmeria-server.exe
```

`setup.ps1`'s launch step uses the in-target path directly.

---

## Tests & CI

### Local tests pass but CI fails on `clippy`

**Symptom.** Your PR is failing the `cargo clippy --workspace ... -- -D warnings` job in CI but `cargo clippy` succeeds locally.

**Root cause.** Local `cargo clippy` without `-D warnings` only **shows** warnings; CI treats them as errors. You missed a warning during development. It isn't a version difference: [`rust-toolchain.toml`](../rust-toolchain.toml) pins the Rust version, and CI installs the same one.

**Fix.** Run the exact CI invocation locally:

```powershell
cargo clippy --workspace `
  --exclude cimmeria-app --exclude cimmeria-content-editor `
  --exclude cimmeria-scene-editor --exclude sgw-launcher `
  --exclude cimmeria-client-telemetry --exclude cimmeria-client-patches --exclude cimmeria-lab `
  --all-targets -- -D warnings
```

All seven excludes matter — the four GUI crates (the Tauri admin app, the two
Tauri editors and the egui launcher), the Windows-only client-telemetry and
client-patches cdylibs, and the `cimmeria-lab` supervisor. Dropping `cimmeria-client-telemetry` is the
easy one to miss: it makes a Linux host, such as CI's runners, need
xkbcommon/xcb dev packages. The authoritative list is
[`.github/workflows/test.yml`](../.github/workflows/test.yml).

Fix the warning at the root cause. Don't sprinkle `#[allow(clippy::...)]` per call site — project thresholds for `too_many_arguments` (14) and `type_complexity` (500) are in [`clippy.toml`](../clippy.toml).

---

### Live-DB test passes for me but fails in CI

**Symptom.** CI's live-DB job (`tools/test-live-db.sh`, the `ci-live-db` nextest profile) fails on a test that's green on your machine.

**Root cause.** Common causes:

- Your test uses `cargo test` (parallel by default) and CI uses the `ci-live-db` profile (serialised). Two tests claiming the same sentinel id range collide under parallel execution.
- Your test cleans up by id range (`WHERE id BETWEEN x AND y`) and another test in CI is using a sentinel in that range.
- Your test relies on rows from a previous test surviving — CI starts from a fresh schema each run.

**Fix.** Read [`docs/architecture/integration-test-infra.md`](architecture/integration-test-infra.md) and [`TESTING.md`](../TESTING.md) → Live-DB type. Sentinels must fit in `i32`. Cleanup deletes by **exact** sentinel, not by range. Run locally with the `ci-live-db` profile to repro:

```powershell
$env:DATABASE_URL = "postgres://w-testing:w-testing@localhost:5433/sgw"
tools/test-live-db.ps1
```

---

### `cargo nextest` not found

**Symptom.** `cargo nextest run` reports "no such subcommand."

**Fix.** Install it once: `cargo install cargo-nextest --locked`. CI uses it; local builds need it for the same test runner.

---

## Server runtime

### Server starts but logs are empty / silent

**Symptom.** `cimmeria-server.exe` is running, ports are listening, but `logs/cimmeria-server.log` is empty.

**Root cause.** Default log level might be `WARN`. Most healthy events are `INFO` or `DEBUG`.

**Fix.** Set the log level via env var:

```powershell
$env:RUST_LOG = "info"
.\cimmeria-server.exe

# More verbose:
$env:RUST_LOG = "cimmeria=debug,info"

# Just one module:
$env:RUST_LOG = "cimmeria_cell_combat::cell::combat=trace,warn"
```

The `tracing` filter syntax is in the [`tracing-subscriber` docs](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html).

---

### Python console refuses connection

**Symptom.** You can't connect to the Python console on port 8989 / 8990.

**Root cause.** There is no Python console. The Rust server has no embedded
Python interpreter and no console port — nothing under `crates/` listens on
8989 or 8990, and there is no setting that turns one on. The console belonged
to the deprecated Python/C++ server, which no longer runs.

**Fix.** Use the current equivalents instead:

- **In-game GM commands** run through the client's *native* `/` console. See
  [`docs/architecture/gm-cell-method-gating.md`](architecture/gm-cell-method-gating.md).
- **Remote administration** is the `cimmeria-admin-api` REST/WebSocket surface
  on the admin port (default 8443), documented in
  [`docs/tools/admin-api.md`](tools/admin-api.md).

[`docs/architecture/python-console.md`](architecture/python-console.md) is
retained as a historical reference for the deprecated server only.

> [!WARNING]
> The admin API currently ships with **no authentication** and binds
> `0.0.0.0` — do not expose port 8443 beyond localhost or a trusted LAN.
> Tracked as issue #439.

---

## Documentation & contribution

### "Where does this doc go?"

See the **doc-update map** in [`CLAUDE.md`](../CLAUDE.md) → "Required documentation for every PR." It maps `if you change X → update Y`. Reviewers will check this and send the PR back if a required update is missing.

---

### "Which test type does this need?"

The picker is in [`TESTING.md`](../TESTING.md) — it walks through the eleven test types and identifies which is appropriate for a given bug shape. Picking the wrong one is the #1 cause of PR rework.

Short version:

- Changes to wire format → byte-exact wire-format test.
- Changes to `WHERE` clauses or `rows_affected` → live-DB regression guard.
- Changes to pure logic → unit test.
- Cross-system flow (login → world entry → combat) → smoke or wireclient pcap-replay.

---

## Last resort

If nothing here helps:

1. Check [`known-issues.md`](known-issues.md) for documented bugs that might be the symptom.
2. Search [the GitHub issues](https://github.com/SandboxServers/Cimmeria/issues) — somebody may have hit it already.
3. Open a new issue with: the exact error message, the platform (Windows version), the command that failed, the relevant log lines, and what you tried.

Maintainers respond fastest to issues that show effort — a reproducible failure beats a vague "it doesn't work" every time.
