# Amendment 8: LT-25 re-spec (2026-10-21)

The LT-25 coder returned blocked (R9) at the launcher integration tip `b9fc68cc3`, with no edits ([its report](../../../.claude/worktrees/lt-25/.claude/reports/launcher-telemetry/LT-25-coder.md), in the `lt-25` worktree). The amended packet is [work-packets-coverage.md § LT-25](work-packets-coverage.md#lt-25-repl-logic-uat-uatlauncher-telemetry-amendment-8).

## Gaps and rulings

| # | Gap | Ruling |
|---|---|---|
| 1 | Step 5 needs `launch_not_started` (`engine/src/storage/launch/events.rs:36`, `pub(crate)`). The shell crate cannot call it, and its real producers (`worker.rs:158,172,273`, `supervisor.rs:24,28,44`) run only inside the async launch worker. | **Re-spec with a seam.** Add `#[cfg(feature = "test-support")] pub fn launch_not_started_for_test(id, reason)` after `events.rs:45`. It calls the real helper. This follows the engine's existing `test-support` seams (`updater/mod.rs:26`); the shell's dev-dependency already turns the feature on (`shell/Cargo.toml:37`), so release builds don't have it. The call sites stay pinned by `launch/events_tests.rs:16` (LT-17a, LT-18). |
| 2 | Step 4 needs `record_outcome` (`shell/src/telemetry.rs:200`, private). The UAT module is not a descendant of `telemetry`. | **Re-spec with a seam.** Change it to `pub(crate)`, one word. The UAT drives MintFailed, then Dropped, then Sent, which is the sequence of the existing test `telemetry.rs:288`. That yields exactly one `pipeline_recovered`. |
| 3 | Step 1 called `emit_startup` directly. | **Improved.** The first store open already calls it (`host.rs:228`), so the UAT opens the store inside the subscriber with `dispatch(Inspect)`. That is the production path. |
| 4 | Step 3 needed an `expected_revision`, and `launcher_batch` returns `None` unless the build has the DLL resource and the player has opted in (`host/launcher_telemetry.rs:20-28`). | **Specified.** The UAT copies the host fixture from `launcher_telemetry.rs:40-58` with the DLL present, opts in with `GameTelemetryCommand::Set`, and re-reads the revision with `Inspect` just before `SaveClientOptions`. It uses `default_servers()`. |
| 5 | `frontend/scripts/` does not exist. | **Fixed.** The new file is `frontend/launcher-telemetry-native-uat.mjs`, next to `game-telemetry-native-uat.mjs`. Its `package.json` script line goes after line 26 and has no esbuild step. |
| 6 | Branch name. | **Fixed:** `launcher-telemetry/lt-25`, as `ops` created it. |
| 7 | The `.mjs` IP rule had to match LT-40's. | **Split.** The Rust bridge asserts the exact rule (`ip_scrub::contains_ip`, which is public). The `.mjs` ports the token rule with `net.isIP`. |
| 8 | The original said "feeds the records through `launcher_events`, then `launcher_batch`". | **Clarified.** `launcher_batch` already calls `launcher_events` (`launcher_telemetry.rs:29`), so the test calls only `launcher_batch` with `Redaction::from_env()`. |

Dropping steps 4 and 5 was rejected. The existing tests prove that the events are emitted. Only this UAT proves that the `telemetry` and `launch` targets survive the real `ForwardLayer` and `launcher_batch` and pass the allow-list, the field cap and the privacy checks. Each seam costs one line or five.

## Scope change

There are now six files instead of three. The three new ones are engine `events.rs` (+6 lines), shell `telemetry.rs` (one word) and shell `host.rs` (+2 lines, the `#[cfg(test)]` module declaration). Checks gain D2e (engine clippy). The packet stays well under 40k tokens.

## Next step

Dispatch a fresh packet-coder to LT-25 in the unedited `lt-25` worktree. This is a fresh dispatch, not retry 1 of 3: the block was a packet gap.
