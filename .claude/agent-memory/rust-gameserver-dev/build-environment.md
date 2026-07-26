---
name: Build environment quirks
description: Windows-native cargo builds need NO linker override; the old hardcoded rust-lld path note is obsolete. Worktrees still need an external/ junction.
metadata:
  type: feedback
---

**No linker override is needed on this host.** `cargo check` / `cargo test` /
`cargo clippy` / `cargo nextest` all work with a bare invocation from the repo
root or a worktree.

**Why this entry exists:** an earlier version of this memory claimed the
repo's `.cargo/config.toml` hardcoded another user's `rust-lld` path, and that
every cargo command had to be wrapped in
`CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS='-C linker=...'`. **That is no
longer true and following it wastes time.** The tracked config now reads:

```toml
[target.x86_64-pc-windows-msvc]
linker = "rust-lld"
rustflags = ["-C", "linker-flavor=lld-link"]
```

Bare `rust-lld` resolves via the rustup toolchain's own bin directory, so it
is portable across rustup-managed Windows installs. Re-verified 2026-07-26 by
running `cargo check -p cimmeria-services` and a full `cargo nextest run` with
no env wrapper at all.

**How to apply:** just run cargo directly. If a link *does* fail with a
missing-linker error, read `.cargo/config.toml` before reaching for an env
override — and correct this note with what you find.

**Worktree gotcha (still current):** a fresh worktree has no `external/`
(gitignored, populated by `setup.ps1`). Junction-link it from the main
checkout before building, or the build dies with a `DetourNavMesh.h` error
that never mentions `external/`:

```
cmd /c mklink /J "<worktree>\external" "C:\Users\Steve\source\projects\Cimmeria\external"
```

See [[concurrent-claude-sessions]].
