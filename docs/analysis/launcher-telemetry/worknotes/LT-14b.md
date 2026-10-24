# LT-14b worknote

Status: done. Commit is the LT-14b commit on `launcher-telemetry/lt-14-install`.

Done:
- `engine/src/operation_numbers.rs` (new): process-local operation numbers and elapsed_ms, inline tests.
- `engine/src/error_codes.rs` (new): `code()` for ContractError, StorageError, IntentError, inline tests.
- `engine/src/lib.rs`: `mod error_codes;` and `pub mod operation_numbers;` after `pub use operations::*;`.
- `engine/src/storage/install_worker/events.rs` (new): install_started, install_finished, PhaseLog, install_reconcile_unknown, install_seed_forget_failed, install_stage_failed, mark_uncertain_failed, inline tests.
- `engine/src/storage/install_worker/mod.rs`: call lines per spec, `pub mod events;`. Net +16 lines.
- `engine/src/storage/install_worker/telemetry_tests.rs`: `a_failed_download_logs_the_install_lifecycle`.
- `shell/src/host/install/mod.rs`: mark_uncertain call routed to events.

Left: nothing.

Last check: `pwsh tools/build-lane/lane.ps1 cargo test ... -p cimmeria-launcher-desktop install` status=ok exit=0 (19 passed).

Environment note: the desktop shell build needs `crates/launcher/desktop/frontend/dist` (tauri generate_context). It was absent, so `npm ci` and `npm run build` were run in the frontend folder (both gitignored).