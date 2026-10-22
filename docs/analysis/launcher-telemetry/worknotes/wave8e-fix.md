# wave8e-fix

Worktree lt-light-integration, branch launcher-telemetry/light-adoption. Final tip `8581a0833` (base `ebf8a46f3`). Not pushed, not rebased. Six commits:

- 29a5f3f3a N7
- 9162bf0d1 N6
- a62d4ac0f N5
- f611cde1f N3, N4
- c631a0413 N1
- 8581a0833 N2 and the light.rs comment

## Per note

- **N7.** The tests were already in sibling files, so I split production code instead.
  - `publication.rs` went from 699 to 562 lines. `inspect`, `Interrupted`, `interrupted`, `recover` and `abandon` moved to the new `publication_recovery.rs` and are re-exported with `pub use`, so the `crate::adoption::*` paths are unchanged. `finish` became `pub(super)`.
  - `in_place.rs` went from 692 to 627 lines. `abandonable` and `abandon` moved to the new `in_place_abandon.rs` and are re-exported. `release_files` and `move_files` became `pub(super)`.
  - Engine test count was 831 before and 831 after the split.
- **N6.** `cooked_tier` now uses the local constants `STOCK_DIR` and `TIER_PARENT` with `.len()`, plus `debug_assert!(is_char_boundary)`. The new test passes sources shorter than the prefix (stock dir without the slash, one character short, non-ASCII, `"W"`) and expects `None` without a panic.
  - **Not met: the test does not fail when the literals are put back.** The `bundled_cooked_alias` guard already returns `None` for all of those sources, so the old literals behave the same on them. The test is a no-panic guard only.
- **N5.** `Err(Error::Cancelled) => {}` was added. The failure WARN now carries `event = "adoption_light_replay_failed"`. The `telemetry.md` row is added.
  - Two new tests. Revert proof: with the old arm, the cancel test and the failure test both fail.
- **N3.** `incremental_failed_row_carries_the_closed_reason` in `events.rs`. It fails when the event string is renamed.
- **N4.** `an_unreadable_record_launches_untagged_and_logs_one_closed_warning`. It writes garbage into `installed-content.json`. It fails when the event string is renamed.
- **N1.** I added `LaunchStatus::assemble(.., game: Option<&Path>, ..)`, which `launch_command` now calls. The new test `the_status_reports_the_admitted_copys_incomplete_patch_counts` runs on Windows and asserts 2 and 3 in the serialized JSON. It fails when the counts are hard-coded to (0, 0).
  - **Deviation from the note:** the test calls `assemble` with a ledger in a temp folder. It does not build an adopted install through `launch_command`, because that fixture is macOS/Wine only.
- **N2.** The `telemetry.md` row is added after `patch_incomplete`, CRLF kept. The test is `a_failed_retry_logs_one_warning_with_the_patch_label_and_closed_code`. It asserts WARN, target `launcher.install`, event, `patch:` prefix and `reason` equal to `unexpected_status`. It fails when the event is renamed.
  - launcher-core has no tracing-subscriber dev-dependency, so I wrote a small `Subscriber` in the test file instead of changing Cargo.toml or the lockfile.
- **light.rs comment.** Reworded (it is in `storage/adoption/light.rs`, not `adoption/light.rs`): the original stays only in the both-tiers case, and a placed tier renames the stock directory.

## Checks (all via the lane, `--locked`, fmt first, clippy `-D warnings`)

- Clippy is clean on cimmeria-launcher-core, cimmeria-launcher-engine and cimmeria-launcher-desktop.
- Engine `cargo test`: 836 passed, 0 failed, 3 ignored (+5 new).
- Desktop shell: 147 passed, 0 failed, 7 ignored.
- Core: 375 passed, 0 failed, 2 ignored.
- `regen.py` rewrote the generated test-count blocks, which were stale because of the new tests, in `README.md`, `docs/readme.md`, `doc-update-map.md` and `testing/inventory/README.md`. `regen.py --check --skip-crate-graph` now reports current. These are included in the N2 commit.
- I did not stage dist, node_modules or `launcher_targets.txt`.


# wave8e-fix

Worktree lt-light-integration. Fixed the flaky `a_failed_retry_logs_one_warning_with_the_patch_label_and_closed_code` (crates/launcher-core/src/install_tolerant_tests.rs). Not pushed.

- Cause (matches the wave8e-verify hypothesis): callsite interest is cached process-wide. A concurrent libtest thread hitting the `warn!` with no dispatcher can cache it as "never" before this test's scoped dispatcher counts.
- Fix, the repo standard (as in engine commands.rs): module-level `static SERIAL: Mutex<()>` locked for the test, a held `_second = tracing::Dispatch::new(...)`, and `tracing::callsite::rebuild_interest_cache()` right after `set_default`, before `run`.
- Deviation: core has no tracing-subscriber dev-dep and no shared helper, so the second Dispatch wraps the existing local `Rows` subscriber instead of `registry()`. No lockfile change.
- Deviation: the test is now a sync `#[test]` with an explicit current-thread runtime and `block_on`, so the std Mutex guard is not held across an `.await` (clippy await_holding_lock).
- Other capture tests in core: none. The only `set_default`, `with_default` or `Subscriber` use in launcher-core is this test, so nothing else needed the SERIAL.
- fmt: clean. clippy `--all-targets -D warnings` on cimmeria-launcher-core: clean.
- Stability: full cimmeria-launcher-core suite run 5 times through the lane, foreground. All 5 gave 375 passed, 0 failed, 2 ignored.
- Revert proof: replaced the `warn!` in install_tolerant.rs with a no-op, ran the test. It failed (1 failed). File restored from backup, status shows only the test file modified.

