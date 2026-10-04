---
name: reference-desktop-engine-test-seams
description: "Desktop launcher engine: it builds and tests on Linux/WSL, and which real paths a test outside a module can reach (2026-10-04)"
metadata:
  type: reference
---

Learned while adding `storage/launcher_summary` to the desktop engine
(`crates/launcher/desktop/engine`) on 2026-10-04, at `ba4b20b6b`:

- The engine crate builds and its whole suite passes on Linux/WSL through the
  lane: `bash tools/build-lane/lane.sh cargo test --locked --manifest-path
  crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine --target-dir
  target/desktop`. Older notes saying engine tests need macOS or native
  Windows are stale. Only the shell crate needs GTK/WebKit and does not
  build here.
- `DesktopState::admit_launch` cannot succeed on Linux: a native install
  needs `cfg!(windows)` and a Wine install needs macOS. A launch test outside
  `storage/launch/` writes `launch-plan-<id>.json` with `atomic::write`,
  begins `OperationKind::Launch` through `operations_mut()` with
  `Sha256(serde_json::to_vec(&plan))` as the digest, then uses the public
  `launch::dispatch`. A replaced helper file gives `Observation::NotStarted`
  on every OS without spawning anything.
- `install_worker::dispatch_with` and `publish` are private. A test that
  needs a real worker with a mock download host must be a child module of
  `install_worker` (the summary tests are in
  `install_worker/summary_tests.rs`). The public `dispatch` uses the real
  catalog URL.
- Install progress is a `watch` channel that keeps only the newest value,
  and a quick download reports `Downloading` once, after the last byte. A
  consumer cannot rely on seeing every stage; a test that needs a stage
  drives its own `ProgressSink::latest()`.
- `admit_install` commits through `&mut self.operations` directly, not
  through `operations_mut()`, so a hook placed only in `operations_mut()`
  does not run on install admission. `FileJournal::commit` is the one place
  every journal write passes.
- `runtime_setup::tests::fixture_with_runtime` gives an installed Wine-backend
  state. `DesktopState::uninstall` on it fails after admission on non-macOS
  (the operation ends in `ReconciliationRequired`); use a native-backend
  fixture to test a successful uninstall.
