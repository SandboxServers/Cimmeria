---
name: desktop-engine-tests-need-cargo-test
description: cargo nextest cannot list the desktop launcher engine's tests (harness=false process tests print PASS lines); run cargo test through the lane instead
metadata:
  type: reference
---

`cargo nextest run --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine`
fails with "creating test list failed": the `harness = false` integration
tests (`helper_supervisor_process` and friends) print `PASS ...` lines that do
not end in `: test`. Use `cargo test` (through the lane, with
`--target-dir target/desktop --locked`); `--lib -- <filter>` runs one unit test.

Related: [[launcher-core-path-include-extraction]].
