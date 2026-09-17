---
name: Build environment quirks
description: Worktrees need external/ junction-linked before cargo works; cargo's output is block-buffered through the Bash tool, so a hung test looks like a hung build.
metadata:
  type: project
---

## OUTDATED — the rust-lld override is no longer needed

This note used to say `.cargo/config.toml` hardcoded another user's
`rust-lld` path and that every cargo command needed a
`CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS` override. **Confirmed
2026-09-17: that is fixed.** The tracked config now uses a bare
`linker = "rust-lld"` with `linker-flavor=lld-link`, which rustc resolves
from the rustup toolchain's bin directory. Plain `cargo test` links fine on
this host with no env override.

## A fresh worktree cannot build until `external/` is linked

`external/` is gitignored and populated by `setup.ps1` in the main checkout
only, so a new worktree fails in `cimmeria-entity`'s build script with
`C1083: Cannot open source file: '../../external/recast/Detour/Source/...'`.
Fix once per worktree:

```powershell
cmd /c mklink /J "<worktree>\external" "C:\Users\Steve\source\projects\Cimmeria\external"
```

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
