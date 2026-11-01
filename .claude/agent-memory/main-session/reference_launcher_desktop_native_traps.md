---
name: Desktop launcher native-path traps
description: Methods and traps for the desktop launcher's adoption, migration, updater, update and launch engines; a lead, verify in the cited docs and code
metadata:
  type: reference
---

A lead, not evidence. Each trap names where the verified contract lives. Folded from the per-feature launcher memories (2026-10-04 work).

## Run and lint

- `crates/launcher/desktop/Cargo.toml` has `default-members = ["engine"]`, so clippy and tests cover the engine only. A change under `shell/` needs `--workspace` or `-p cimmeria-launcher-desktop`. The shell also needs `frontend/dist` (`npm ci --ignore-scripts`, `npm run build` in `frontend/`).
- The standalone desktop workspace builds into `crates/launcher/desktop/target/`, not the directory the lane prints; `npm run uat:adoption` needs `ADOPTION_UAT_BINARY` pointing at the shell test binary and keeps state under `ADOPTION_UAT_ROOT`.
- Wine tests are `#[ignore]` and need `CIMMERIA_WINE_HELPER`, `CIMMERIA_WINE_HELPER_SHA256`, `CIMMERIA_WINE_RUNTIME_TREE`. Always-run tests use a Native-backed adoption of inert ZIP content and stop at the platform backend gate; macOS Play admission refuses a Native backend.
- A test origin that sets its listener non-blocking passes that mode to accepted sockets on macOS; call `set_nonblocking(false)` after `accept` or reads panic with `WouldBlock`.
- Host methods that reach `mac_wine::repair_recovery::stop_reference` call `Handle::block_on`; run them on `spawn_blocking`.
- Fixture reopen must drop the host and wait for retained store references to drain before reopening the state directory.

## Adoption

- The review must carry the journal revision current when preparation finishes (`Ownership::claim` advances it).
- Confirmation needs an absent destination folder; the host appends the product folder name to the chosen location.
- `Ownership` settles on `Drop` with `try_lock`; workers holding no guard call `Ownership::release()`.
- Lock order: adoption mutex, then store. Release a `Preview` outside the adoption mutex.
- `adoption-plan-<work>.json` must survive publication; deleting it makes adopted copies unplayable (fail closed, no error).
- An interrupted preparation is never resumed by extracting again; cleanup checks the saved directory inode and owner lease, confirms the Wine helper host is gone, and stops only its bound prefix.
- Adoption compares whole-file hashes with a reconstructed signed release, not the imported ledger.

## Migration

- Three distinct legacy sources: executable-adjacent config and identity, and the game-root ledger. Historical telemetry `enabled` is not consent. A legacy ledger can never be promoted to signed ownership.
- Confirm IPC carries only digest, revision and an explicit Boolean; extra fields are rejected. The importer's tests pin the legacy formats.

## Updater, update, launch

- Updater: a Minisign global signature authenticates the trusted-comment version; artifact verification alone does not bind the feed version. Call `ensure_updater_idle()` before other mutation preparations.
- Native shutdown belongs to the retained worker, not the awaited IPC response. After a successful spawn the handoff signal must fire even when final persistence fails; completion comes from startup reconciliation.
- Tree fingerprints that authorize rollback or deletion must frame directories with child counts and symlinks with target lengths. A derived UUID path is not ownership evidence.
- Update recovery must reconcile partial root-receipt and installed-index publication; do not call `installed_content()` during it. Rollback is a new confirmed Update from previous signed evidence with a fresh minimum check; its evidence identity differs from the original even when the manifest digest matches.
- Launch must use the read-only installed identity view; `installed_content()` can migrate and write before a gate rejects. Process start is not login proof; a reopened unfinished Launch stays unknown.
- `SuspendedProcess::terminate` is best effort; resume errors do not prove termination. Mac `VK_DRIVER_FILES` selects the pinned runtime's MoltenVK descriptor after runtime verification.
- Environment and fixture tests prove logic, not rendering, login, signed packaged upgrade or gameplay; those need retained native UAT.

Docs: `crates/launcher/desktop/docs/{migration,updater,launch-mac}.md` and `docs/analysis/playtests/2026-10-03-macos-wine/worknotes/`.
