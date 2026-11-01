---
name: Build environment quirks
description: Worktrees need external/ junction-linked before cargo works; Dev Drive full recovery; cargo's output is block-buffered through the Bash tool, so a hung test looks like a hung build.
metadata:
  type: reference
---

## A fresh worktree cannot build until `external/` is linked

`external/` is gitignored and populated by `setup.ps1` in the main checkout
only, so a new worktree fails in `cimmeria-entity`'s build script with
`C1083: Cannot open source file: '../../external/recast/Detour/Source/...'`.
Fix once per worktree:

```powershell
cmd /c mklink /J "<worktree>\external" "<main-checkout>\external"
```

Replace `<main-checkout>` with your own main checkout's absolute path (e.g.
`git rev-parse --show-toplevel` run from the main checkout, not the
worktree) — it is not portable across machines/users. See
[[concurrent-claude-sessions]] for the wider worktree-isolation workflow.

## Dev Drive full: move your own target dir to C:

Seen 2026-09-28: every lane build died with `LLVM ERROR: IO failure on output
stream: no space on device` because the shared Dev Drive (`B:\targets`) was at
0 bytes free. `tools/build-lane/lane.sh` uses `$CIMMERIA_TARGET_ROOT/<worktree>`
only when that directory already exists **or** `<worktree>/target` does not.
So: delete *your own* `B:\targets\<worktree>` (never another worktree's) and
`mkdir <worktree>/target`; the next lane call prints `target=<worktree>/target`
and builds on C:. Expect a full rebuild (sccache helps).

## Cargo through the Bash tool looks hung when it isn't

Cargo's progress goes to stderr, which the Bash tool captures block-buffered
— so a long `cargo test` writes **nothing** to the output file until it
exits. A genuinely hung *test* is indistinguishable from a slow build.

Two things that help:

- Run cargo via the **PowerShell** tool instead (`$env:CARGO_TERM_PROGRESS_WHEN
  = "never"`, then `& cargo ... | Select-Object -Last N`) — output streams.
- Diagnose with `Get-CimInstance Win32_Process -Filter "Name='cargo.exe'"` and
  look at `UserModeTime`. A cargo sitting at ~0.2s CPU after minutes is not
  compiling; it is parenting something blocked (usually a test awaiting a
  oneshot/channel reply that will never arrive).

Corollary for tests: always wrap a handler call that awaits a cross-service
reply in `tokio::time::timeout`, so an ordering regression fails fast instead
of wedging the suite. See [[cross-world-transfer-flow]] for the case that
taught this.

## `bash` from the PowerShell tool can be WSL bash

Seen 2026-10-10: `bash tools/build-lane/lane.sh ...` run from the PowerShell
tool resolved to WSL's bash (lane log under `/home/...`, then `no such
command: nextest`). Run lane commands from the Bash tool (Git Bash), or call
Git's `bash.exe` by full path from PowerShell.
