# Native game Update facts

- 2026-10-04: `storage/update` reconstructs a distinct signed target through retained preparation, preserves the permanent `InstallIntent`, and separately publishes root receipt and installed index. Recovery must reconcile their partial combinations; calling `installed_content()` during partial publication is not recovery.
- A derived staged-content digest detects post-preparation file changes. It does not compare old user modifications against publisher content. Native confirmation binds old/new signed identities; presentation must disclose backup-only preservation and no user-data merge.
- Rollback is a new confirmed Update reconstruction from previous signed evidence with a fresh minimum check. Never restore executable bytes from an old modified backup.
- Uninstall must recognize completed Repair and Update work/backup directories against saved plan, checkpoint, permanent owner and role evidence. Name-only allowlisting is insufficient.
- Evidence and remaining platform/UI gates: `docs/analysis/playtests/2026-10-03-macos-wine/worknotes/game-update-native.md` and `crates/launcher/desktop/docs/migration.md`.
