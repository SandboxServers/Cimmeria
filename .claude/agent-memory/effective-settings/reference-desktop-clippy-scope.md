---
name: Desktop workspace clippy scope
description: The desktop launcher manifest lints only the engine unless the shell is named
type: reference
---

Observed 2026-10-04: `crates/launcher/desktop/Cargo.toml` sets
`default-members = ["engine"]`. So
`cargo clippy --manifest-path crates/launcher/desktop/Cargo.toml --all-targets`
checks the engine only, and the same holds for `cargo test`. A change to
`shell/` needs `--workspace` or `-p cimmeria-launcher-desktop`.

The shell crate also needs `frontend/dist` before it compiles
(`npm ci --ignore-scripts` then `npm run build` in `frontend/`); both outputs are
ignored by git.

A Play store-lock deadlock in the shell host shipped in a candidate commit
because only the engine had been linted and tested.
