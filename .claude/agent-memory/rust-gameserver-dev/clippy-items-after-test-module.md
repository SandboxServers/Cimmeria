---
name: clippy-items-after-test-module
description: Workspace clippy -D warnings rejects any item after a #[cfg(test)] mod, and (on 1.98+) rejects chunks_exact with a constant size — both bite wire-format tests
metadata:
  type: project
---

`clippy::items_after_test_module` is enabled (via workspace `-D warnings`), so
a `#[cfg(test)] mod tests { .. }` block **must be the last item in the file**.

**Why:** Many repos tolerate a test module in the middle of a file; this one
fails the blocking clippy job for it. It compiles and tests pass — only clippy
catches it, so it's easy to miss until CI.

**How to apply:** When adding tests to an existing module, append the block at
EOF even if the code under test lives at the top. Free functions that trail the
`impl` block (helpers like `collect_package_files`) must stay above it.

## `chunks_exact(2)` on a UTF-16 wire buffer (clippy 1.98+)

`clippy::chunks_exact_to_as_chunks` fires on any `chunks_exact(<literal>)` and
is deny-by-default under `-D warnings`. It is **newer than the machine default
toolchain**, so a WSTRING decoder written the obvious way passes locally and
fails CI — always run `cargo +1.98.1 clippy` (see
[[stale-branch-clippy-toolchain-drift]]). The fix:

```rust
let (pairs, _) = bytes.as_chunks::<2>();
let s: String = char::decode_utf16(pairs.iter().copied().map(u16::from_le_bytes))
    .map(|r| r.expect("wire text must be valid UTF-16"))
    .collect();
```

`slice::as_chunks` is stable since 1.88, so it is safe on the older default
toolchain too.

## `mod tests;` declarations hide log targets from `target_scan_tests` (2026-09-27, SS-00)

The same "tests last" rule has a second enforcer: `crates/server/src/logging/target_scan_tests.rs`
strips everything from a file's test module onward before scanning for `target: "..."` literals.
A `#[cfg(test)] mod tests;` **declaration** near the top of a `mod.rs` (the tidy-looking place)
makes every `tracing::warn!(target: ...)` below it invisible, so `scan_finds_known_targets` fails
with "scan missed <target>" although the code is plainly there. Put `mod tests;` last too.
