---
name: launcher-uat-integration-2026-10-04
description: Semantic traps found when combining the adoption UI, effective settings and Wine app identity launcher branches; where the combined tests and UAT bridges live.
type: reference
---

# Launcher UAT integration (2026-10-04)

Source: combining `launcher/adoption-ui`, `launcher/effective-settings` and
`launcher/wine-computer-use` on `launcher/uat-integration`. Git reported no
conflicts; every problem below was semantic. Handoff:
`docs/analysis/playtests/2026-10-03-macos-wine/worknotes/uat-integration.md`.

- **`adoption-plan-<work>.json` must outlive publication.** The effective
  settings binding compares the Published record with this checkpoint. Deleting
  it makes an adopted copy unplayable. Guard:
  `host::adoption::play_tests::host_published_copy_...`.
- **`effective_settings::fixtures` depends on adoption's public API.** A change
  to `PreviewRequest` or `PreviewWorker` breaks the shared fixtures, not
  production code, so only `--all-targets` builds show it.
- **Prerequisites are offered only when the prerequisite helper is bundled.**
  A host fixture without `prerequisite_helper` reports no prerequisite target for
  any installation. `adoption::fixture::bundle_prerequisite_helper` adds an inert
  one.
- **A copy interrupted after promotion is not installed yet.** The installed
  index is written later, so `effective_launch_binding()` is `Ok(None)`, not an
  error, until Recover finishes.
- **A missing checkpoint surfaces as `JobError::Io`** from a Play command;
  a tampered record surfaces as `CorruptState`.
- **"Second Play is Busy" is timing-dependent in host tests.** With no runtime
  the retained Play worker exits within milliseconds and the next Play is
  admitted. The engine-level admission tests assert it without a worker.
- **`LaunchStatus` fields are private to `host::launch`.** Tests elsewhere read
  the serialized view, which is also what the renderer polls.
- **The adoption host fixture's release has no `min_launcher`.** To exercise the
  signed minimum after a host adoption, sign a release that carries one
  (`play_tests::release`).
- **UAT bridge binaries differ.** `uat:updater` and `uat:updater-apply` need the
  *engine* test binary in `UPDATER_UAT_BINARY`. Adoption, game Update, launch and
  migration bridges are in the *shell* test binary. The wrong one fails with
  `EPIPE`, not a named error.
- **A `wine` test-name filter with `--ignored` also runs
  `packaged_resource_admits_wine_and_retains_cancellation`**, which needs a staged
  packaged resources directory and fails without one.
