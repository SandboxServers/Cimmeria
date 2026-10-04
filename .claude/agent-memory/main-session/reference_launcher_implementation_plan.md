# Tauri launcher implementation plan — 2026-10-04

The [implementation plan](../../../docs/analysis/playtests/2026-10-03-macos-wine/launcher-implementation-plan.md)
records the requested Tauri + Effect direction, with Rust authoritative for
install/launch mutation and real Effect workflow coordination. Self-contained
startup checks are deferred until last but remain a release gate. No production
code, deployment, WireGuard connection or live observability probe belongs to
this planning packet.

Inspected contracts: launcher `telemetry/install_result.rs` queues only when
opted in and uploads in the next game telemetry session, leaving failures before
game launch unseen remotely. Server `telemetry/replay.rs` routes ClientNative to
structured `client.native` logs, not native phase spans. Existing game consent
is saved for launch-time snapshots; do not claim immediate DLL revocation.
The plan proposes narrow bounded launcher summaries, independent export consent,
acknowledged queue and validated lifted fields; these are not yet implemented.

2026-10-04 implementation update: `crates/launcher/desktop/engine` now provides
a tested operation contract (nine Mac tests), with an injected journal trait.
It is a separate workspace, requiring explicit manifest checks. No file journal,
Effect integration or mutation worker is connected yet; see the plan ledger.

2026-10-04 foundation update: desktop `storage/` now implements process ownership,
bounded persisted state and uncertain-commit gating. `frontend/` pins Effect
4.0.0; headless UAT uses real Rust `commands.rs` through the `state_bridge`
example and proves preference persistence across restart. Tauri UI, native
app-data selection, migration, game workers and exporter remain unconnected.
Standalone CI is `.github/workflows/launcher-desktop.yml`; root tests omit it.

2026-10-04 shell update: `desktop/shell` now connects native-selected app data
to the approved interface and Effect settings workflow. File-manager reveal
accepts only the saved existing directory, avoiding arbitrary-path IPC and
file-association launch. CI run37181383914 proved engine/JS persistence on
Windows+Mac before the shell; shell/visual/game gates remain separate.
