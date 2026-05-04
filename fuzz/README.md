# cimmeria-fuzz

Fuzz targets for the Mercury wire-level decode path — the
most corruption-exposed surface in the codebase.

## Targets

- **`parse_incoming`** — drives `cimmeria_mercury::packet::parse_incoming`
  with arbitrary bytes. Must never panic on any input.
- **`add_fragment`** — chains `parse_incoming` → `FragmentAssembler::process_parsed`.
  Drives the fragment header parser + reassembly state machine with
  arbitrary input. Must never panic regardless of how malicious the
  fragment headers are (impossible total counts, conflicting first_seq
  values across fragments, repeats, etc.).

## Why this crate is excluded from the workspace

`libfuzzer-sys` requires nightly Rust (the `-Z sanitizer` flag).
Including the crate as a regular workspace member would force the
whole workspace onto a nightly toolchain even when only stable
crates are being built. Excluded here so `cargo build --workspace`
on stable continues to work.

## Running

```bash
# One-time setup
cargo install cargo-fuzz

# From the repository root, change into this directory:
cd fuzz

# Run a single target indefinitely (Ctrl+C to stop)
cargo +nightly fuzz run parse_incoming

# Time-bound run (e.g. 60 seconds — useful for CI)
cargo +nightly fuzz run parse_incoming -- -max_total_time=60
cargo +nightly fuzz run add_fragment    -- -max_total_time=60

# Reproduce a crash from a saved input
cargo +nightly fuzz run parse_incoming -- artifacts/parse_incoming/crash-<hash>
```

## CI integration

Not currently wired into the per-PR workflow because nightly Rust
is not part of the default toolchain. Add a separate scheduled
workflow (e.g. nightly cron) that runs each target for a few
minutes and uploads any crash artefacts when it finds them.

## Stable-Rust complement

A stable-Rust property test in
[`crates/mercury/src/packet/parse_proptest.rs`](../crates/mercury/src/packet/parse_proptest.rs)
covers the same "never panics on arbitrary input" contract using
proptest, which runs in every per-PR CI invocation. It's not as
thorough as fuzzing — proptest has a shrinker but no coverage-
guided exploration — but it catches the obvious panic sources
without the nightly dependency.
