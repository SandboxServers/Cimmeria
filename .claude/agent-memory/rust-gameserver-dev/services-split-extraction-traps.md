---
name: services-split-extraction-traps
description: Traps when extracting a module tree out of cimmeria-services into its own crate (services-crate-split waves) — guards that fire on a crate with no DB tests, allowlist edges that vanish, unreachable_pub, and tests whose helper stays behind
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

Related: [[python-write-mangles-utf8-and-crlf]] (use byte-level scripted edits; the Bash
tool mangles `\\\r` in heredocs, so write scripts with the Write tool),
[[lane-sh-masks-cargo-exit-code]].
