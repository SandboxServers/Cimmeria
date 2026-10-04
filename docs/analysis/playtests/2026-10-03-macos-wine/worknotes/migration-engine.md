# Bounded legacy migration engine

> 2026-10-04; branch `launcher/legacy-migration`; base `0b10d869c869793ab506dbf9215ddb91714a244b`.

Implemented `engine/src/storage/migration/`: bounded source parsing, exact-byte
archive, typed identity/config/ordered historical ledger, actual legacy process
lock, preview digest confirmation, conflicts/idempotence, atomic intent followed
by recoverable preference transition, and startup completion. Source JSON remains
untouched. Game consent reads explicit `opted_in` only; launcher summaries remain
independent. Historical claims do not create content receipts or deletion rights.

## Integration contract

Cherry-pick the engine/docs/tests commit followed by the separate registration
commit. Registration adds `pub mod migration` and calls
`state.recover_legacy_import()?` before returning `DesktopState::open`. No Cargo
changes required. Shared shell/frontend files are untouched.

After Repair releases shared files, expose native-selected source directories via
`LegacySource { launcher_directory, game_directory }`. Under the command mutex,
call `preview_legacy_import(&source)`, display both native folders, the original
configured install path and imported values, obtain visible confirmation, then
call `import_legacy(&source, &preview.confirmation, preferences.revision)`.
Refresh preferences and `legacy_import()` after success. Keep `MigrationError`
codes visible; uncertainty requires reopen, `source_changed` requires a fresh
preview, `busy` requires the legacy launcher to exit. Do not send paths from an
untrusted web caller directly to these native APIs.

Read `crates/launcher/desktop/docs/migration.md` for bounds, record recovery and
coexistence scope. The separate registration commit also links this document from the desktop README
and documentation index; retain those links when integrating shared docs. Do not call packet
8 complete: updater ownership, version mapping, installed update/launch parity,
and user-facing import integration remain separate work.

## Validation

Native Mac engine all-targets suite: 305 passed, 15 ignored; all-targets Clippy
with `-D warnings`, formatting and diff whitespace checks passed. The consent guard was
revert-verified: aliasing old `enabled` to `opted_in` made the schema fixture fail,
then the correct behavior was restored. Focused migration suite
covers 11 tests, including a separate Python process holding the actual Unix
legacy `flock`. Checks run through the build lane. Re-run:

```bash
bash tools/build-lane/lane.sh cargo test --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine --all-targets
bash tools/build-lane/lane.sh cargo clippy --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine --all-targets -- -D warnings
cargo fmt --manifest-path crates/launcher/desktop/Cargo.toml --all -- --check
```

Not covered: Windows-native lock interoperability/power-loss durability, frontend
REPL/visual UAT (no frontend changed), actual legacy install or live game launch.
No telemetry, network service, updater, WireGuard, merge or publication was run.
