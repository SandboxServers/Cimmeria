# Signed desktop updater implementation packet

2026-10-04 · branch `launcher/signed-updater` · registered worktree
`.claude/worktrees/launcher-signed-updater` · base `ba4b20b6b`.

Implemented new native updater storage/policy/transport, host dispatch,
Effect workflow and view, with no Apply capability. Production configuration is
unset and reports Disabled. The new reference contract is
`crates/launcher/desktop/docs/updater.md`.

## Integration ownership

New-file implementation and dependency/registration changes are separate commits.
Apply the registration commit as narrow hunks after reconciling other owners;
never replace shared files. Its edits:

- `engine/Cargo.toml`: exact `minisign-verify =0.2.4`, `base64 =0.22.1`,
  `semver =1.0.28`; dev-only `minisign =0.8.0`. Desktop lockfile records these.
- `storage/mod.rs`: `pub mod updater;`; derive Deserialize for StorageError;
  call `state.recover_launcher_update()?` during open after legacy recovery;
  call `self.ensure_updater_idle()?` first in `operations_mut()`.
- `engine/src/lib.rs`: `pub use storage::updater;`.
- `shell/src/host.rs`: `mod updater; pub use updater::UpdaterCommand;`; add
  `updater_config: Option<cimmeria_launcher_engine::updater::Config>` initialized
  to `None`. Do not load endpoint/key from renderer or environment overrides.
- `shell/src/main.rs`: import UpdaterCommand, register `updater_command` returning
  `Result<updater::Snapshot, updater::Error>` and awaiting
  `state.inner().updater_command(request)`.
- Frontend package scripts include updater view tests and `uat:updater`.

Coordinator additionally inserts `ensure_updater_idle()?` at the start of every
mutation admission *before any disk preparation*: install, launch, repair,
uninstall, runtime setup, legacy import/adoption, and directory preference
changes. Exact base-file insertion points are `admit_install_with` in
`storage/install_intent/mod.rs` (its commit bypasses operations_mut),
`admit_launch`, `admit_repair`, `uninstall`, `admit_runtime_setup`,
`import_legacy_with`, and `save_preferences_with` only when the directory changes.
The operations_mut fallback prevents owner races but is too late to
prevent admission helpers writing preparatory files. Updater admission checks
the same operation journal under the same native mutex. Do not wire only a UI
disabled state.

UI: import `mountUpdater` from `updater-view` and mount it once. Supply the native
invoke adapter and existing refresh callback; refresh this view when other native
operations finish so `operation_revision` stays current. Its HTML nodes are:

```html
<section aria-labelledby="updater-heading">
  <h2 id="updater-heading">Launcher updates</h2>
  <p id="updater-status" role="status"></p>
  <pre id="updater-notes"></pre>
  <button id="check-updater">Check for update</button>
  <button id="prepare-updater">Download and verify</button>
  <button id="inspect-updater">Refresh status</button>
</section>
```

Do not add an Install update button. Maintain native min-launcher gating and
surface the compatibility status in launch inspection: a periodic successful
inspect must not erase LauncherTooOld and re-enable Play.

Doc integration: link the updater contract from the desktop README, launcher
guide/design, campaign ledger, and corresponding docs index under the desktop
workflow row of `docs/agents/doc-update-map.md`. This worker does not own those
shared indexes.

## Reproducible checks

From the repository root, compiling calls use the lane:

```sh
bash tools/build-lane/lane.sh cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine --lib
npm ci --ignore-scripts --prefix crates/launcher/desktop/frontend
npm run build --prefix crates/launcher/desktop/frontend
bash tools/build-lane/lane.sh cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-desktop updater
npm test --prefix crates/launcher/desktop/frontend
```

For actual JS/native persistence UAT, set `UPDATER_UAT_BINARY` to the engine
libtest executable printed by the engine test run, then run
`npm run uat:updater --prefix crates/launcher/desktop/frontend`.
The binary path is resolved relative to the frontend working directory.

Verified so far: engine library 334 passed/16 ignored; focused updater 9 tests
included in that pass; frontend check/build and 52 tests passed (including the two
new updater UI guards); native JS UAT passed the actual signed local feed,
download, single dispatch, literal notes, private bytes/offer, persisted Ready
reopen, stale replay and staged-byte tamper cases. Shell updater tests: 2 passed. Engine + shell all-target Clippy with
`-D warnings` passed. Frontend test rerun and native JS UAT passed after the
player-facing wording changes.

Excluded: real production endpoint/key, publication/deployment, release signing,
packaged visuals/IPC, Windows-native behavior, package apply/install, relaunch,
rollback and healthy-start confirmation. No production keys were generated.

## Coordinator integration

The early admission and cleanup guards are now wired, with a regression checking
that blocked calls leave state bytes and the destination unchanged. Summary
consent can still be withdrawn while an update owns the mutation gate. Settings
controls and application composition are wired. Independent review found a stale
updater revision after other operations; the launch observer's existing revision
callback now refreshes updater state without an unconditional refresh cycle.

Integrated checks: engine 335 passed/16 ignored, shell 40 passed/6 ignored,
frontend 53 passed and build passed; strict engine/shell all-target Clippy passed.
The native signed-feed/disk UAT now mounts the actual application composition
and verifies downloading after another native journal operation without manual
recheck. Other view payloads are inert fixtures; packaged visuals and Apply remain
outstanding. A later scoped updater rerun passed 10 tests with its one opt-in
bridge exercised separately by that UAT.
