# LT-16b worknote (done)

## Done (all checks green)
- New `engine/src/storage/client_maintenance/events.rs` (199 lines, 6 tests): `action`, `ClientMaintenanceError::code`, `client_maintenance_finished`, `client_maintenance_failed`, `client_maintenance_removal_failed`, `client_changes_ledger_unreadable`. All use `target: "launcher.repair"`.
- `client_maintenance/mod.rs` (281 lines): `pub(crate) mod events;`; the `profile_unavailable` warn at :98; `client_maintenance_at` logs finished or failed at :115-132; the `tracing::warn!(folder)` removed; `removal_failed` now takes only the `io::Error` and logs its kind; the `tracing::info!(path)` removed.
- `client_changes/mod.rs` (202 lines) :178: `Err(error)` logs `client_changes_ledger_unreadable(error.code())`.
- `client_maintenance/tests.rs`: orchestrator ruling. Two tests now assert the no-path events; each checks that no row contains the profile or Firesky path.

## Deviations
1. `removal_failed` lost its `path` parameter (accepted by the orchestrator). Its callers at :147 and :163 were updated.
2. `client_maintenance/tests.rs` is edited although the packet's file list does not name it (orchestrator ruling, to update two tests that pinned the removed path logs).
3. The successful-reset test calls `client_maintenance_fixture`, not `client_maintenance_in`, because the `client_maintenance_finished` event is emitted by the `client_maintenance_at` wrapper.

## Checks
- D1 `cargo fmt --all` (desktop): status=ok exit=0.
- D2e clippy `-p cimmeria-launcher-engine --all-targets -- -D warnings`: status=ok exit=0.
- D2s clippy `-p cimmeria-launcher-desktop --all-targets -- -D warnings`: status=ok exit=0.
- D3e test engine `client_maintenance`: status=ok exit=0, 22 passed.
- D3e test engine `client_changes`: status=ok exit=0, 10 passed.
- D3s test desktop `client_maintenance`: status=ok exit=0, 1 passed.

## Not committed
- `crates/launcher-core/src/telemetry/launcher_targets.txt` (LF conversion) is left unstaged.
