# Legacy settings preview and import UI

> **Date:** 2026-10-04
> **Branch:** `launcher/migration-ui`
> **Worktree:** `.claude/worktrees/launcher-migration-ui`
> **Base:** `4513fd8d9`
> **Scope:** One import UI phase; migration/parity remains incomplete.

## Delivered

Settings selects executable-adjacent and game-root folders through native dialogs,
then displays a review of folder mapping, identity, config, separate consent and
ordered historical patch claims. Explicit confirmation references a native-held
preview digest and preferences revision. IPC accepts no source paths. Native
source reread, legacy lock and durable transaction stay in `DesktopState`.
Cancellation, changed source/preferences, conflicts and malformed inputs fail
without inventing a new identity. Effect coordinates first-click pending state,
duplicate suppression, uncertainty and read-only status reconciliation. Reopening
shows the persisted imported record, not a synthesized readiness claim.

Import enables no signed ownership, Play, Repair or Uninstall. The next-step copy
says to retain the old launcher for this installation until verified adoption is
available, or use a separate empty directory for a desktop install. Archived
legacy config is not yet consumed by desktop Play. No updater was replaced.

## Verification

- `npm run check`, `npm run build`, `npm test` in the frontend: 47 tests pass.
- Lane native shell tests: 37 pass, 6 intentionally ignored (headless UAT bridges
  and independently gated fixture scenarios); four migration tests pass.
- Lane shell clippy, all targets, `--no-default-features -- -D warnings`: passes.
- `MIGRATION_UAT_BINARY=<native shell test binary> npm run uat:migration`:
  production Effect/view → production native host → temporary app-data, preview
  leaves preferences untouched, explicit single confirmation persists exact
  identity/config/ordered adopted ledger, game consent stays off despite historical
  `enabled`, summary consent stays off, reopen preserves saved state, copy keeps
  ownership and next-step boundaries visible. Host fixture verifies source config
  bytes unchanged and no installed-content receipt.
- Regression proof: changing host confirmation to use the current preferences
  revision instead of the reviewed revision makes
  `changed_preferences_cannot_be_overwritten_by_a_previous_preview` fail. Restore
  the reviewed revision before integration.

Rerun native tests with:

```sh
npm ci --ignore-scripts --prefix crates/launcher/desktop/frontend
npm run build --prefix crates/launcher/desktop/frontend
bash tools/build-lane/lane.sh cargo test --locked \
  --manifest-path crates/launcher/desktop/Cargo.toml \
  -p cimmeria-launcher-desktop --no-default-features
```

The lane log identifies the test executable for `MIGRATION_UAT_BINARY`.
No compiling Cargo invocation bypassed the lane. No game/live launcher data,
telemetry, WireGuard, Windows cross-compilation, publication or release touched.

## Exclusions and next assignment

Visual inspection was attempted against an isolated local HTML fixture. The
computer-use provider reported no available browsers, so no screenshot/layout,
keyboard focus, native chooser or packaged-webview visual proof is claimed.
Native Windows lock interoperability, real-user legacy sources, power loss,
actual game launch, self-contained startup and signing remain unverified here.
The fixture HTTP server was stopped after the unavailable-browser result.

The bounded next adoption assignment is to design and implement a reviewed
legacy-content verification/adoption contract: compare the chosen existing tree
against authenticated release content, define how modifications and unknown
files are preserved, acquire coexistence locks through mutation, bind receipts
to exact native paths/content/identity, and test failure/recovery/reopen before
creating any desktop ownership record. Legacy ledger order and `seed_adopted`
remain historical claims, never authority. If safe adoption cannot be established,
retain explicit unsupported status; do not manufacture an install intent or
receipt just to enable buttons. After adoption, separately wire preserved legacy
launch settings/consent and prove installed/update/launch parity. Updater owner,
signed assets, version mapping and rollback remain a distinct packet.

Coordinator owns shared README, guide/design, campaign ledger and index updates;
this packet adds its migration reference and unique project memory. The engine
migration public re-export and frontend script/test registration are separated
into their own commit for shared-file integration.
