---
name: Build environment quirks
description: Worktrees need external/ junction-linked before cargo works; the old rust-lld linker override is obsolete; cargo's output is block-buffered through the Bash tool, so a hung test looks like a hung build.
metadata:
  type: project
---

## OUTDATED — the rust-lld override is no longer needed

This note used to say `.cargo/config.toml` hardcoded another user's
`rust-lld` path and that every cargo command needed a
`CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS` override. **Confirmed
2026-09-17: that is fixed.** The tracked `.cargo/config.toml`
`[target.x86_64-pc-windows-msvc]` block now uses a bare `linker = "rust-lld"`
with `-C linker-flavor=lld-link`, which rustc resolves from the rustup
toolchain's own `lib/rustlib/<target>/bin/` directory — portable across
rustup installs. Plain `cargo clippy` / `cargo build` / `cargo test` link
fine on this host with no env override.

**Do not** prepend `CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS=-C linker=...`
any more. An earlier version of this note said to; that advice was for a
superseded config revision that hardcoded `C:\Users\steven.cady\...`.
Setting the env var also *overrides* the config's rustflags, silently
dropping `linker-flavor=lld-link` — so the stale workaround is now actively
worse than doing nothing.

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

## A build can leave an unrelated `Cargo.lock` diff

Running any cargo command may re-resolve a transitive patch version (seen
2026-09-17: `thiserror 2.0.19` → `2.0.20`, two lines). It is nothing to do
with your change. Check `git diff --stat Cargo.lock` before staging and
`git checkout -- Cargo.lock` if the diff isn't yours — `git add <dir>` won't
catch it, but `git add -A` or a careless `git commit -a` would.

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
