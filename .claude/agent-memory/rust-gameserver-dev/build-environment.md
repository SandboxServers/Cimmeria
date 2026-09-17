---
name: Build environment quirks
description: Cimmeria native-Windows cargo builds work with no env override; the old hardcoded rust-lld path in .cargo/config.toml was fixed upstream. Worktrees need external/ junctioned in.
type: project
---

## Linker: no override needed any more (re-verified 2026-09-17)

The tracked `.cargo/config.toml` `[target.x86_64-pc-windows-msvc]` block now uses a **bare
`rust-lld`** name plus `-C linker-flavor=lld-link`, which rustc resolves via the toolchain's own
`lib/rustlib/<target>/bin/` directory. It is portable across rustup installs.

**Do not** prepend `CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS=-C linker=...` any more. An
earlier version of this note said to; that advice was for a superseded config revision that
hardcoded `C:\Users\steven.cady\...`. Plain `cargo clippy` / `cargo build` / `cargo test` now work
unmodified on this host (confirmed by a full-workspace clippy run that linked proc-macro dylibs).

**Why it matters:** setting the env var also *overrides* the config's rustflags, silently dropping
`linker-flavor=lld-link` — so the stale workaround is now actively worse than doing nothing.

## Worktrees need `external/` junctioned in

`external/` is gitignored and populated by `setup.ps1`, so a fresh git worktree has none. Several
crates (notably `cimmeria-entity`, which builds recast/detour C++ via `cc`) fail without it. Fix:

```powershell
New-Item -ItemType Junction -Path "<worktree>\external" -Target "C:\Users\Steve\source\projects\Cimmeria\external"
```

See [[concurrent-claude-sessions]] for the wider worktree-isolation workflow.
