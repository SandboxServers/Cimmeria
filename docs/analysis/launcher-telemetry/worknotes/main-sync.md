# main-sync: origin/main into launcher-telemetry/launcher

- Worktree: `.claude/worktrees/lt-app-integration`
- Before: `29de38489`; merged `origin/main` at `e7487da3e` (#1976, #1977, #1978)
- Merge commit: `482df7c60` (`git merge --no-ff origin/main`)

## Conflict resolutions (all unions)

| File | Resolution |
|---|---|
| `engine/src/storage/game_telemetry/launcher_event.rs` | Main's hunk was empty at the conflict point; kept the branch's LT-40 test `launcher_events_scrub_ip_literals_from_message_and_fields`, plus every non-conflicting main hunk. |
| `engine/src/test_capture.rs` (add/add) | Branch file (LT-40 `contains_ip` import, `Captured::assert_no_ip`, the assertion loop in `capture`, and its 3 self-tests) with main's `tracing::callsite::rebuild_interest_cache()` call inside the `with_default` closure. |
| `shell/src/test_capture.rs` (add/add) | Branch file. Main's version was a strict subset: no `rebuild_interest_cache` in main's shell copy, so none was added. |
| `shell/src/telemetry.rs` | Imports: kept the branch's `health::{Recovery, SharedHealth, TelemetryHealth}` and `SendOutcome` (main's import lines are a subset). Tests: kept the branch's `two_failures_then_a_success_emit_exactly_one_pipeline_recovered`; main had nothing at that point. |
| `frontend/package.json` | The `test` script lists the union: main's `adoption-reasons.test.ts` plus the branch's `ui-event.test.ts` and `telemetry-status-view.test.ts`. |

No semantic conflicts.

## Working-tree note

The engine test `wire_column_matches_the_shared_target_file` failed at first because the working copy of `crates/launcher-core/src/telemetry/launcher_targets.txt` was still CRLF from before main's `.gitattributes` `eol=lf` rule. I re-checked it out from the index (index LF, now working tree LF). Nothing was staged for it.

## Checks (lane, foreground, `--manifest-path crates/launcher/desktop/Cargo.toml`)

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | ok |
| `cargo clippy --locked -p cimmeria-launcher-engine --all-targets -- -D warnings` | ok |
| `cargo clippy --locked -p cimmeria-launcher-desktop --all-targets -- -D warnings` | ok (frontend dist built first) |
| `cargo test --locked -p cimmeria-launcher-engine` | 720 passed, 0 failed, 3 ignored |
| `cargo test --locked -p cimmeria-launcher-desktop` | 141 passed, 0 failed, 6 ignored |
| frontend `npm ci --ignore-scripts`, `npm run build`, `npm test` | build ok; 189 passed, 0 failed |

dist and node_modules were not committed. Nothing was pushed.