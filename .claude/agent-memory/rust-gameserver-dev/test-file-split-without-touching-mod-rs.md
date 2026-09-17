# Splitting a growing `tests.rs` without touching the shared `mod.rs`

When a packet's regression suite would push a single `tests.rs` past the
500/700-line caps (CLAUDE.md "File organization"), and `mod.rs` is NOT in your
owned-paths list (shared/coordinator-owned), you don't need a `mod.rs` edit at
all:

`#[cfg(test)] mod tests;` in the parent resolves identically to either
`tests.rs` OR `tests/mod.rs` — Rust's module system treats them as the same
declaration target. So:

1. `git mv src/foo/tests.rs src/foo/tests/mod.rs` (via a temp directory name
   first if the target dir name collides with the source file name on the
   same `git mv` — e.g. `git mv tests.rs tests_dir/mod.rs && git mv tests_dir
   tests`).
2. Add `#[cfg(test)] mod newsuite;` inside `tests/mod.rs` (no cfg needed if
   the whole `tests` module is already `#[cfg(test)]` from the parent).
3. Put the new suite in `tests/newsuite.rs`. It reaches the original file's
   private helpers (`setup()`, `decode_feedback()`, etc.) via `use
   super::{setup, decode_feedback};` — plain `fn` (no `pub`) is already
   visible to descendant modules in Rust, so no visibility changes needed on
   the helpers either.

Zero edits to any file outside the two you already own (`tests.rs` and its
new sibling). Used in the Cimmeria legacy-command-parity campaign, P02
(`console/tests.rs` → `console/tests/mod.rs` + `console/tests/p02.rs`) when
adding 19 tests would have taken it from 561 to 953 lines.

See also [[legacy-command-parity-scoping-judgment]].
