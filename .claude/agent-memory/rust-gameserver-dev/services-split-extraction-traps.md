---
name: services-split-extraction-traps
description: Traps when extracting a module tree out of cimmeria-services into its own crate (services-crate-split waves) — guards that fire on a crate with no DB tests, allowlist edges that vanish, unreachable_pub, tests whose helper stays behind, and the shim for a file that stays behind
metadata:
  type: project
---

Learned extracting `cimmeria-resources` (wave W1b, 2026-09-26). Plan:
`docs/architecture/services-crate-split.md`; its §4 "Deviations" list records each wave.

- **A `cimmeria-test-support` dev-dependency forces a live-DB list entry**, even with zero
  `require_db_or_skip!` tests. `live_db_wrapper_lists_every_test_support_crate` keys on the
  dev-dependency, not on DB use. Add the crate to BOTH `tools/test-live-db.sh` and `.ps1`
  in the same order (a second guard compares them).
- **Allowlist edges into a moved module disappear.** `tools/layering/check.py` treats a
  `pub use cimmeria_x::…` re-export as external and stops, so any allowlisted edge whose
  target moved out becomes stale and fails the guard. Delete the line (`--prune`).
  `crate-map.toml` rows that now match nothing are NOT flagged.
- **`#![warn(unreachable_pub)]` + clippy `-D warnings`** flags `pub` items inside private
  modules (`mod patches_castle; pub const …`). Narrow to `pub(super)`; that is the only
  code change a pure move needs beyond widening what the compiler asks for.
- **A test whose helper stays in the monolith must be split out of the moved file.** Grep
  moved test files for `crate::mercury`, `crate::cell`, other not-yet-moved modules
  before `git mv`. It goes to a `*_tests.rs` file in services (`#[cfg(test)] mod x;` is
  invisible to the layering guard), and any private item it names must become `pub`.
- **Intra-doc links to modules left behind break silently** (`[`super::cooked_data::…`]`);
  CI never runs `cargo doc`. Turn them into plain code spans with the full crate path.
- **`module_path!()` changes** — rename `FILE_LAYERS` rows (stale-target guard catches
  it), add `cimmeria_<crate>=debug` to `OTEL_FILTER` if the modules relied on
  `cimmeria_services=debug`, and add the crate dir to `IN_PROCESS_CRATES`.
- **Drop now-unused deps from services** (`zip` left with resources); nothing warns.
- Baseline test counts: `nextest list --message-format oneline` before any `git mv`,
  then diff the moved names by suffix (the crate prefix and `::bin`/lib id change).
  `lane.sh --exclusive` queues behind every running single-slot job, so start the
  baseline first and do only non-Rust edits while it waits.

Learned extracting `cimmeria-cell-cover` (W2a), where one file (`stance.rs`) stayed:

- **Partial extraction = keep the old `mod.rs` as a shim**: `pub use
  cimmeria_x::cell::cover::*;` + `mod stance;` + its `pub use`. Zero call-site edits.
  The file left behind imports shared helpers through `super::` (the glob), so any
  `pub(crate)`/`pub(super)` helper it used must become `pub` and be re-exported.
- **check.py resolves every item used through such a shim to the shim module
  itself** (a glob from an external crate resolves to nothing). Map the shim in
  `crate-map.toml` to the crate that will own the leftover file (world here); that
  also retires the "mod re-exports leftover" allowlist edge. Check `--edges <shim>`
  first: every importer must sit at or above that crate.
- **No existing guard notices a missing `cimmeria_<crate>=debug` OTEL_FILTER row for
  a crate with no `FILE_LAYERS` row** (the file-parity guard walks file rows only).
  Add a `<crate>_events_keep_their_index` parity test and prove it fails with the row
  removed.
- A dev-dependency on the same crate with `features = ["test-support"]` beside the
  normal dependency is how a `#[cfg(any(test, feature = "test-support"))]` hook
  reaches the higher crate's tests; `pub use …::*` forwards it at the old path.

Related: [[python-write-mangles-utf8-and-crlf]] (use byte-level scripted edits; the Bash
tool mangles `\\\r` in heredocs, so write scripts with the Write tool),
[[lane-sh-masks-cargo-exit-code]].
