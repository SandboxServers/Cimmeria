---
name: Build environment quirks
description: The old rust-lld linker override is OBSOLETE — .cargo/config.toml was fixed upstream; plain cargo commands work on this host.
type: project
---

**The linker override is no longer needed.** Verified 2026-07-26 on `main` @ 70ec45ef: `.cargo/config.toml`'s `[target.x86_64-pc-windows-msvc]` block now uses the portable bare name `linker = "rust-lld"` plus `linker-flavor=lld-link`, which rustc resolves through the toolchain's own bin directory. There is no hardcoded user-profile path anymore.

Plain `cargo check`, `cargo clippy --all-targets`, and `cargo nextest run -p cimmeria-services` all succeed with no env prefix.

**Historical (do not re-apply):** an earlier config hardcoded a `rust-lld` path under another user's home, and every cargo invocation needed a `CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS='-C linker=...'` prefix. If you see that advice repeated anywhere, it is stale.

**How to apply:** just run cargo normally. If a link failure ever reappears, re-read `.cargo/config.toml` before reaching for an override — that file is the authority, not this note.

See [[local-postgres-port]] for the matching test-DB gotcha.
