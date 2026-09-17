---
name: worktree-build-gotchas
description: Two environment traps when building/testing cimmeria-services from a .claude/worktrees/ worktree — missing external/ and false-failing log-capture tests under parallel cargo test
metadata:
  type: project
---

# Worktree build/test gotchas (cimmeria-services)

Two traps that make a *correct* change look broken when working from a
`.claude/worktrees/agent-*` worktree rather than the main checkout.

## 1. `external/` is absent in worktrees — build scripts fail before your code compiles

`external/` is gitignored and populated by `setup.ps1`, which only ever ran in
the main checkout. A worktree therefore has no `external/`, and
`cimmeria-entity`'s build script (cc-rs compiling Recast/Detour via relative
paths like `../../external/recast/Detour/Source/*.cpp`) dies with
`C1083: Cannot open source file`. This happens *before* any of your crates
compile, so it masks whatever you were actually trying to check.

**Why:** the repo invariant in CLAUDE.md ("`external/` is not in git") is
stated for fresh clones; the worktree case is the same problem but easy to
misread as a real build break in your own change.

**How to apply:** create a directory junction once per worktree, then build
normally. It is gitignored, so it never shows up in `git status`:

```powershell
New-Item -ItemType Junction `
  -Path "<worktree>\external" `
  -Target "C:\Users\Steve\source\projects\Cimmeria\external"
```

Do NOT re-run `setup.ps1` for this — the user prefers to drive builds/setup
themselves (see the user's `feedback_build` memory), and a junction is
sufficient and non-destructive.

## 2. Three `cell/` log-capture tests false-fail under parallel `cargo test`

Running `cargo test -p cimmeria-services --lib` (the whole suite) reliably
fails these three:

- `cell::dispatch::router::tests::unhandled_cell_method_warns_with_method_index_and_args_len`
- `cell::service::tests::npc_ai::ability_range::npc_ai_fight_warns_when_handle_use_ability_returns_false`
- `cell::service::tests::npc_ai::state_machine::stationary_no_los_or_range_emits_structured_decision_log`

They pass individually and with `-- --test-threads=1`. Cause: `LogCapture`
(see `test_support`) installs a global `tracing` subscriber, so concurrently
running tests capture each other's events.

**Why:** CI does not hit this — it runs `cargo nextest`, which is
process-per-test. So these are *not* a regression you introduced, and they are
not a CI blocker.

**How to apply:** if you see exactly these three failing after an unrelated
change, re-run them serially to confirm before chasing them. Use
`cargo nextest run` locally if you want a result that matches CI. Only treat a
log-capture failure as real if it still fails at `--test-threads=1`.

Related: [[MEMORY]]
