# Updater Apply implementation facts

- 2026-10-04: Desktop updater Apply keeps native ownership through installing,
  restart-required and reconciliation-required. Startup acknowledgment uses the
  native current executable and compiled package version; installer spawn is not
  completion. See `crates/launcher/desktop/docs/updater.md`.
- Native shutdown must belong to the retained worker, not the awaited Tauri IPC
  response. A dropped renderer reply must not leave the old process holding its
  state lock while the replacement waits for it.
- Tree fingerprints that authorize rollback/deletion must frame directories with
  child counts and symlinks with target lengths; concatenating recursive entry
  names without directory boundaries permits sibling/descendant ambiguity.
- A derived UUID stage path is not ownership evidence. Mac extraction publishes an
  owner-marked stage by exclusive rename; recovery verifies it before deletion.
  Windows recovery requires a matching recorded installer fingerprint.
- Temporary native Mac bundle/process fixtures prove rename/spawn/recovery logic,
  not signed packaged upgrade, Tauri exit, clean-machine health, Windows installer
  behavior or release provisioning. Production updater configuration remains off.
