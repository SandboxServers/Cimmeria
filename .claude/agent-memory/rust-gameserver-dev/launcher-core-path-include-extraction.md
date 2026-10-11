---
name: launcher-core-path-include-extraction
description: LX-01 traps turning #[path] includes into the cimmeria-launcher-core crate - cfg(test) items the engine's tests used, telemetry dependency closure, test-count accounting
metadata:
  type: project
---

Moving `crates/launcher/src/*` into `cimmeria-launcher-core` (LX-01, 2026-10-10):

- Under `#[path]` the desktop engine's own tests saw the included files' `#[cfg(test)]`
  items (`unpack::test_fixtures`, `manifest::DEV_MANIFEST_PRIVKEY`,
  `ProgressSink::Checkpoint`). A separate crate's `cfg(test)` is off for its
  dependents, so these sit behind `#[cfg(any(test, feature = "test-fixtures"))]`
  and the engine and egui launcher enable the feature as a dev-dependency.
  The error only shows in `clippy --all-targets`/tests, not `cargo check`.
- The spec's telemetry subset (`auth, queue, runner, ...`) does not close:
  every file needs `events`, and `runner` needs `Telemetry` in `mod.rs`, so the
  whole `telemetry/` dir moved. `Telemetry::start_session` and
  `recover_pending_on_startup` now take the queue dir (was `config::exe_dir()`).
- Test accounting: nextest list hides `#[ignore]` tests (`real_sgw_exe`,
  `real_client_rar`); `cargo test -- --list` shows them. The engine's count
  drops by the included modules' tests (477 to 337), which now run in core.
- Core is in both hakari exclude lists, so the workspace-hack did not change.

Related: [[crate-split-extraction-traps]], [[services-split-extraction-traps]].
