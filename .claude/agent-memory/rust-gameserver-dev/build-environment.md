---
name: Build environment quirks
description: Cimmeria repo's .cargo/config.toml hardcodes another user's rust-lld path; need an env override to build.
type: project
---

The repo `.cargo/config.toml` hardcodes a `rust-lld` linker path under another user's home directory. To build on this host, prepend:

```
CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS='-C linker=C:\Users\steven.cady\.rustup\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\x86_64-pc-windows-msvc\bin\rust-lld.exe'
```

to every `cargo check`/`cargo test`/`cargo build` invocation.

**Why:** this avoids editing tracked config (which would conflict for the original author).

**How to apply:** wrap every cargo command. Tests that don't need a linker (`cargo check`) sometimes work without it but `cargo test` always needs it.
