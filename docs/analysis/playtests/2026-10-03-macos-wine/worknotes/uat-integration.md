# Adoption UI, effective settings and Wine identity: integration

2026-10-04, branch `launcher/uat-integration`. This note records how three
launcher branches were combined and what the combined tree was checked against.
It is fixture evidence. No game was started, no package was rebuilt, and native
window UAT is still open.

## What was combined

Each branch was squash-merged from its final commit, so the rejected candidate
commits are not carried on their own.

| Branch | Final commit | Base | Worknote |
|---|---|---|---|
| `launcher/wine-computer-use` | `4d17fe1ba` | `90819e967` | [Wine app identity](wine-computer-use.md) |
| `launcher/adoption-ui` | `6c372b849` | `726df367e` | [Adoption UI](adoption-ui.md) |
| `launcher/effective-settings` | `4786bc621` | `726df367e` | [Effective settings](effective-settings.md) |

The Wine branch carries the 30 FPS cap (`DXVK_FRAME_RATE=30`). Its hunks in
`launch/wine.rs` and `launch/wine_tests.rs` match the coordinator's uncommitted
copy byte for byte.

## Conflicts

Git reported none. Every conflict was semantic.

- **Shared fixtures used adoption's old API.** `effective_settings::fixtures`
  built a `PreviewRequest` with required artifacts and read
  `PreviewWorker.result`. Adoption made artifacts optional and the result private.
  The fixtures now pass `Some(..)` and call `wait()`.
- **`publication.rs` keeps both intentions.** Confirmation copies without the
  state lock (adoption), and `published_plan` reads the Published record against
  the admission checkpoint (settings). The file is 574 lines, inside the 500 to
  700 band; it was left whole.
- **The checkpoint outlives publication.** Successful confirmation removes only
  the `adoption-artifacts` download store. `adoption-plan-<work>.json` stays, and
  the settings binding depends on it. A guard now fails if it is deleted.
- **Prerequisite eligibility needs the bundled prerequisite helper.** Adoption's
  Wine host fixture and UAT bridge had no such helper, so they could only print
  the prerequisite target. They now bundle an inert one and assert the target.
- **The shared `HeldDownload` test origin** read from a socket that inherits the
  listener's non-blocking mode on macOS and could panic with `WouldBlock`. The
  accepted socket is now blocking, with a regression test. The game Update and
  Repair cancellation tests use this fixture.

Not changed: `launch::Resources` has no new field, no guest-host crate or bundled
resource exists, and Wine app identity stays opt-in through
`CIMMERIA_WINE_APP_IDENTITY=1`.

## Combined adoption-to-Play checks

`shell/src/host/adoption/play_tests.rs` adopts through the production host and
then exercises the Play side of the same store.

| Test | Backend | What it shows |
|---|---|---|
| `host_published_copy_binds_its_reviewed_settings_and_play_polling_never_waits_on_the_copy` | Portable | Play and installation polling answer while a copy is paused. The published records bind the reviewed patch setting without a restart and after one. A missing checkpoint refuses the settings without failing either view. An older launcher still reports the signed minimum. Source, import record, identity and consent are unchanged. |
| `a_recovered_copy_carries_the_same_settings_binding` | Portable | A copy interrupted after promotion has no binding and breaks neither view. After Recover it binds the reviewed setting. |
| `wine_copy_adopted_through_the_host_with_patches_off_reaches_one_play` | Wine (ignored) | The host-published Wine copy is offered prerequisites at once and after a restart. With recorded prerequisite evidence it admits one Play without a patch artifact; an identical retry adds no attempt. |
| `wine_copy_adopted_through_the_host_with_patches_on_requires_the_verified_artifact` | Wine (ignored) | The same journey with patches reviewed on. A bundle without the patch artifact cannot play the copy and the journal does not move; a bundle with it admits one Play that carries the artifact. |

Guards proven by editing the fix out, watching the test fail, and restoring:

| Mutation | Failing test |
|---|---|
| Delete the checkpoint after publication | `host_published_copy_...` (binding is refused) |
| Hold the state lock through the paused copy | `host_published_copy_...` ("Play polling must not wait for a running copy") |
| Remove `set_nonblocking(false)` | `held_download::tests::a_request_that_arrives_after_the_connection_is_still_served` |

The native-backed adoption UAT (`npm run uat:adoption`) now asserts, after
reopening the store, that the reviewed patch setting is bound and that
prerequisites are offered for the Wine backend only.

## Results

macOS, final tree. Rust commands ran through `tools/build-lane/lane.sh` with
`CARGO_BUILD_JOBS=2`.

| Check | Result |
|---|---|
| `cargo fmt --all --manifest-path crates/launcher/desktop/Cargo.toml -- --check` | clean |
| `cargo clippy --locked … --workspace --all-targets -- -D warnings` | clean |
| `cargo test --locked … --workspace` | 505 passed, 0 failed, 36 ignored: shell 69 (14 ignored), engine 420 (22 ignored), runtime probe 16 |
| Shell suite again after the last test-fixture edits | 69 passed, 0 failed, 14 ignored |
| Shell Wine host tests, `--ignored` | 5 passed: adoption `wine_tests` (2), `play_tests` (2), `launch::adopted_tests` (1) |
| Engine `effective_settings::wine_tests`, `--ignored` | 2 passed |
| Frontend `npm run check`, `npm test`, `npm run build` | clean; 70 passed; bundle built |
| `npm run uat:adoption`, portable backend | passed |
| `npm run uat:adoption`, Wine backend | passed; prerequisites offered after reopen |
| `npm run uat:launch`, `uat:migration` | passed |
| `npm run uat:game-update`, `uat:game-update-apply` (Apply and rollback) | passed |
| `npm run uat:updater`, `uat:updater-apply` | passed |

The workspace run came before the final edits to four shell test files; only the
shell crate changed after it, and its suite was rerun. Engine sources did not
change after the workspace run.

Not rerun, because nothing they exercise changed: the engine RAR and RAR/CAB
Wine fixtures and the real-Wine app identity test. The ignored
`packaged_resource_admits_wine_and_retains_cancellation` test needs a staged
packaged resources directory and belongs with the package build.

The Wine runs used the pinned archive helper recorded in the
[preparation worknote](published-adoption-preparation.md) (SHA-256
`d0c89fad…c6a3`) and an isolated clone of the pinned runtime tree. Each test
copies that tree into its own temporary state and runs the helper headlessly in
its own prefix. No installed launcher state, real prefix or game was touched.

## Limits

- **Fixture, not UAT.** Content is a small inert signed ZIP with a stub
  executable. The published client, its RAR/CAB seed and the production HTTPS
  catalog were not used.
- **Prerequisite success is recorded evidence.** No prerequisite installer ran.
  The Wine journeys move their private runtime aside before Play, so the retained
  Play worker stops at its resource claim and records `NotStarted`.
- **Wine app identity was not rerun under real Wine here.** Its code is unchanged
  from `4d17fe1ba`, where the real-Wine Notepad test passed. That test registers
  the game's bundle identifier and shows a window, so it was left out while the
  coordinator runs live UAT. Whether the two foreground icons seen in the earlier
  negative test are resolved by `LSUIElement` is a runtime check for the
  coordinator.
- **One latent timing dependence, not fixed.** The settings shell journey
  `wine_adopted_copy_reopens_offers_prerequisites_and_admits_one_play` asserts
  that a second Play is `Busy` while the first worker is alive. With no runtime
  that worker exits within milliseconds. It passed here; the new journeys do not
  repeat the assertion.
- **A missing checkpoint is reported as `io`, not `corrupt_state`,** when Play is
  commanded. Play is refused either way.
- **Not built on Windows or Linux.** Windows issues go to the Windows owner.
- **No packaged, visual, focus or Tauri IPC evidence.** The native folder dialog
  was not exercised.
- markdownlint is not installed on this machine; the new markdown was not linted.

## For the coordinator: staging the package

The integrated tree needs these from a packaged build. This packet rebuilt none
of them.

| Bundled resource | Compile-time digest | Needed for |
|---|---|---|
| `windows/cimmeria-archive-worker.exe` | `CIMMERIA_WINDOWS_HELPER_SHA256` | Adoption and Install. Without a verified helper the adoption UI reports it unavailable |
| `windows/cimmeria-prerequisite-worker.exe` | `CIMMERIA_PREREQUISITE_HELPER_SHA256` | Prerequisite setup for any Wine-backed copy, adopted or fresh. Without it no prerequisites are offered, so Play is never reached |
| `windows/cimmeria-launch-worker.exe` | `CIMMERIA_LAUNCH_HELPER_SHA256` | Every Play |
| `windows/cimmeria_client_patches.dll` | `CIMMERIA_CLIENT_PATCHES_SHA256` | Fresh installs, and adopted copies reviewed with patches on. Not needed by a copy adopted with patches off |
| `graphics/d3d9.dll`, optional x87 pair | `CIMMERIA_D3D9_SHA256`, `CIMMERIA_ROSETTA_X87_*` | Mac Play |

The archive helper used in the Wine runs predates the shared archive preflight;
a package should carry a rebuilt one. The Play digests are described in the
[launch contract](../../../../../crates/launcher/desktop/docs/launch.md#desktop-play-control-and-resource-binding).

Remaining gates, in order:

1. Signed Mac rebuild from this branch, with the frontend bundle rebuilt.
2. Native window UAT of adoption in Settings: folder dialog, review, copy,
   cancel, recovery.
3. Real prerequisite preparation for an adopted copy, then a supervised Play of
   the real client, with patches on and off.
4. `CIMMERIA_WINE_APP_IDENTITY=1` Play and the computer-use binding checks in the
   [Wine app identity worknote](wine-computer-use.md).
5. Native Windows build and tests.
