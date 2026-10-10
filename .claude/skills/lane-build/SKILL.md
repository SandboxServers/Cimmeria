---
name: lane-build
description: Run any compiling cargo command in Cimmeria (check, build, test, nextest, clippy, live-DB tests, the server release build) through the machine-wide build lane, and read its results without rerunning. Use whenever you are about to type `cargo`, need to check or test a crate, run the pre-PR checks, run a live-DB test, build cimmeria-server.exe, debug a lane failure ("exit 28", "os error 112", "status=failed"), or ask what the lane has been doing. Never call cargo directly.
---

# Build through the lane

Several sessions and workers share one workstation. `tools/build-lane/lane.ps1` (twin of
`lane.sh`; both share the slots) is a machine-wide semaphore that also sets up the target
dir, incremental builds and sccache. **Every compiling `cargo` call goes through it.** In one
week of telemetry, 326 direct `cargo` calls bypassed it and competed with every other build.
Background: [docs/architecture/build-system.md](../../../docs/architecture/build-system.md).

## Commands

Call the lane through `pwsh`, not with `&` from inside a PowerShell session, which drops a bare `--`.

```powershell
# Iterate on the crate you changed
pwsh tools/build-lane/lane.ps1 cargo check -p cimmeria-cell
pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-cell
pwsh tools/build-lane/lane.ps1 cargo test -p cimmeria-cell -- --nocapture

# Workspace-wide or timing runs take every slot
pwsh tools/build-lane/lane.ps1 --exclusive cargo clippy --workspace <EXCL> --all-targets -- -D warnings
```

`<EXCL>` is CI's seven exclusions (GUI apps and Windows-only cdylibs):

```text
--exclude cimmeria-app --exclude cimmeria-content-editor --exclude cimmeria-scene-editor --exclude sgw-launcher --exclude cimmeria-client-telemetry --exclude cimmeria-client-patches --exclude cimmeria-lab
```

- `cimmeria-services` is a thin facade over about 20 crates. Check the crate you changed
  (`-p cimmeria-cell`, `-p cimmeria-cell-content`, `-p cimmeria-base-methods`, ...). Build
  the workspace only for final validation.
- The full pre-PR list (fmt, hakari, clippy, build, nextest, doctests, live-DB, lints) is
  in [CLAUDE.md](../../../CLAUDE.md#pre-pr-checklist) and
  [docs/agents/pre-pr-checks.md](../../../docs/agents/pre-pr-checks.md).
- `cargo fmt` does not compile, so it can run outside the lane.

## Read the result; don't rerun

When stdout is not a terminal, the lane prints a summary instead of cargo's output:
`status=`, the exit code, counts, the errors or failing test names, a **failures file** with
every failure in full, and the **log path**. Read the failures file or the log. Rerunning to
"see the output" costs a slot and minutes. Set `LANE_VERBOSE=1` only when you really need
the whole stream.

## Live-DB tests

```powershell
pwsh tools/build-lane/live-db-test.ps1 <nextest filter or test-name substring>
```

From a worktree root, this reloads the worktree's own `sgw_<worktree>` database and runs the
`ci-live-db` profile in one lane slot, the same way CI does. Test rules ([TESTING.md](../../../TESTING.md)):

- Gate with `require_db_or_skip!` and put `live_db` in the fn or module name, or the
  meta-tests fail.
- Get the URL from `test_support::database_url()`, never `DATABASE_URL`.
- Sentinels fit in `i32`; cleanup deletes by exact sentinel, not by range.
- If the shared `:5433` Postgres is down, restart the bundled server (`external/postgresql_server`).

## Server binary and debug builds

```powershell
pwsh tools/build-lane/lane.ps1 cargo build -p cimmeria-server --release
pwsh tools/build-lane/lane.ps1 cargo build -p cimmeria-server --profile dev-debug   # full debug info
```

On a Dev Drive, output goes to `$env:CIMMERIA_TARGET_ROOT\<worktree>\release\`, not
`target\release\`. The lane prints the target dir when it starts. Copy the exe to the repo
root when you need to run it. `dev-debug` builds into its own dir, so it never invalidates
the normal dev build.

## Failures that aren't your code

| Symptom | Meaning | Do |
|---|---|---|
| exit 28, "free space" | Disk guard: under `LANE_MIN_FREE_GB` (default 10) free | `pwsh tools/build-lane/rm-worktree.ps1 --merged`, then `pwsh tools/build-hygiene/sweep.ps1` |
| exit 75 | rm-worktree is retiring this worktree | Stop building there |
| Long wait before start | All slots busy (count in `%LOCALAPPDATA%\cimmeria-build\lane\SLOTS`) | Wait. A dead holder's slot is freed by the next caller, so there is nothing to kill |
| "os error 112" mid-build | Someone built outside the lane and filled the disk | As exit 28 |

## What has the lane been doing?

```powershell
python tools/build-lane/lane_stats.py --recent 20
python tools/build-lane/lane_stats.py --worktree <name> --kind nextest
python tools/build-lane/lane_stats.py --html lane.html     # or --csv lane.csv
```
