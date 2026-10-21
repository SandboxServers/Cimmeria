# LT-16a worknote (retry 2 of 3)

## Done (checks green)
- New `engine/src/storage/repair/events.rs` (5 tests), `pub mod events;` in `repair/mod.rs`.
- Call sites in `repair/preparation/mod.rs`, `repair/commit/mod.rs`, `uninstall/mod.rs`, shell `host/repair/mod.rs`.
- Passing test in `repair/preparation/tests.rs`.
- DEBUG format argument removed from `confirmed_removal_logs_uninstall_started_and_finished`.
- Race fix (test-only, `uninstall/tests.rs`): a module-level `static SERIAL: Mutex<()>` and `fn serial()` guard. Every test in the module that removes an installation takes `let _serial = serial();` as its first line (11 tests). The capture test keeps the `rebuild_interest_cache()` line inside its closure.
- Cause: a tracing callsite first hit with no subscriber caches interest "never". A parallel uninstall test hitting `uninstall_finished` first dropped the capture test's event. Production events are unchanged.

## Checks (final)
- `storage::uninstall::` filter, run 3 times in parallel: status=ok exit=0, 10 passed each time.
- `cargo fmt` engine and desktop: status=ok exit=0.
- `cargo clippy --all-targets -- -D warnings` engine: status=ok exit=0. Desktop: status=ok exit=0.
- `cargo test -p cimmeria-launcher-engine repair`: status=ok exit=0, 42 passed.
- `cargo test -p cimmeria-launcher-desktop repair`: status=ok exit=0, 5 passed, 1 ignored (pre-existing `#[ignore]` for the JSON-lines UAT fixture).

## Known residual risk
- Tests outside `uninstall/tests.rs` (adoption, install_worker, update journey, and others) also call `uninstall` and emit `uninstall_finished` without a lock. The filter in the packet is deterministic now. The full engine suite may still race the capture test in principle. A shared lock would have to live in `test_capture.rs` (not a packet file).

## Deviations
- Commit subject is `LT-16a: repair and uninstall events` (from the brief), not the packet's `repair and uninstall lifecycle events`.
- Desktop `repair` run once failed `host::repair::integration_tests::retained_host_handoff_...` (`install_status failed: Io`) while a second cargo run was active in parallel. Rerun alone: passed.

## Next step
- Commit the packet files only (`git add` the 8 modified files, the new events.rs and this worknote). Do not commit `crates/launcher-core/src/telemetry/launcher_targets.txt`.
