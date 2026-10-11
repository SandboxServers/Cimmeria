---
name: launcher-index
description: Sub-index of launcher memories (desktop Tauri launcher, release feeds, telemetry tailer, adoption, retired egui handoff) moved out of MEMORY.md to keep it small
metadata:
  type: reference
---

# Launcher

- [launcher-native-owner-handle.md](launcher-native-owner-handle.md) — native launch reads installation identity through its locked handle; Windows revert proof must run natively.
- [launcher-state-key-and-egui-wake.md](launcher-state-key-and-egui-wake.md) — sgw_game patches are recorded as `<id>@sgw_game` (core `state.rs`); the egui wake trap retired 2026-10-11.
- [launcher-extracted-mtimes-and-ue3-ini-version.md](launcher-extracted-mtimes-and-ue3-ini-version.md) — extracted files must keep archive DOS times or UE3 flags Default*.ini outdated; zip 1980 placeholder; MakeCAB fixture.
- [desktop-settings-panel-ipc-traps](desktop-settings-panel-ipc-traps.md) — a panel that invokes on mount breaks view.test.ts call pins and sync-throwing fakes; inspect on first snapshot.
- [launcher-self-update-handoff-traps.md](launcher-self-update-handoff-traps.md) — retired egui launcher's handoff (2026-10-11); only "unlock before close on Windows" still applies.
- [launcher-preferences-schema-change-fanout](launcher-preferences-schema-change-fanout.md) — desktop Preferences change: strict TS decode breaks ~10 fixtures; migrate in serde try_from; adoption Plan digest goes through `digest_form`.
- [launcher-windows-adoption-path-traps](launcher-windows-adoption-path-traps.md) — Windows adoption: `<root>\game` invariant, verbatim-root joins, zip `enclosed_name` backslashes, Windows file identity, UAT binary path.
- [desktop-repair-update-terminal-hooks](desktop-repair-update-terminal-hooks.md) — Repair/Update `finish_failure` runs after every commit (dropped handoff); run-end hooks must check the op was open.
- [launcher-release-feed-traps](launcher-release-feed-traps.md) — release zip needs the exe at its root; key/sig base64 forms; Tauri CLI keys work; prove feeds with `examples/release_feed.rs`; macOS job layout and bash 3.2 trap.
- [launcher-telemetry-tailer-traps](launcher-telemetry-tailer-traps.md) — refresh() drops other roots' files; one refresh per tick; NTFS coarse times; shared queue; undo must not bump discards.
- [bootstrap-launcher-install-and-test-server-port](bootstrap-launcher-install-and-test-server-port.md) — -WithLauncher downloads the released zip (LX-20); Docker owns port 8765, so test servers take a free port.
