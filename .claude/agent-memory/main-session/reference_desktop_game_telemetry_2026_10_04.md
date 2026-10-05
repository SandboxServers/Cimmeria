---
name: reference_desktop_game_telemetry_2026_10_04
description: "Desktop launcher game telemetry (PR #1241): separate default-off choice, session mint on the login server, DLL injected after patches; proven under Wine on macOS 2026-10-04; accelerator breaks launch supervision; engine builds and tests on Linux"
metadata:
  type: reference
---

Learned 2026-10-04 while adding opt-in game telemetry to `crates/launcher/desktop/` (PR #1241). Contract: `crates/launcher/desktop/docs/launch.md#opt-in-game-telemetry`.

- **The launch helper already takes two DLLs** (`Request.dlls`, max 2), so adding the telemetry DLL after the patch DLL needed no Windows helper rebuild.
- **The player DLL is built by the `launcher runtime probe` workflow** (no `lab-bridge` feature), which logs its SHA-256. `cimmeria-client-patches`, `client-telemetry`, `client-hookgate`, `patch-wire`, `client-launch` and `start32` were unchanged between `d73eea0e` and the PR, so the older staged patch DLL and the new telemetry DLL are from identical sources.
- **It works under Wine.** On macOS (Wine runtime r17, stock Rosetta) the DLL logs `attached`, `build flavour: player`, chains onto the two functions the patch DLL hooks, installs its inline and IAT hooks and uploads over TCP to the login port. Colo operator confirmed 1,315 events of 32 types in the first 15 s.
- **Capture switches are developer-only.** They reach the game only from the launcher's own environment (`open --env CIMMERIA_CLIENT_CAPTURE=unfilter,firehose "<bundle>"` on macOS); the session marker carries no `capture` block.
- **Plan digests.** `launch::Resources` is digested from its JSON. A new optional field must use `serde(default, skip_serializing_if)` or every stored plan stops matching and the launcher reports corrupt state.
- **`rosettax87` breaks launch supervision.** With `ROSETTA_X87_PATH` set, the Wine loader the launcher spawned hands off to a detached `rosettax87` process and exits; `supervisor::run` sees the child exit and records `unknown` about 11 s after start. There is no recovery path for a launch in `reconciliation_required`, so Play stays blocked. Stock Rosetta is tracked correctly through a clean exit.
- **The engine crate builds and tests on Linux** (`cargo test -p cimmeria-launcher-engine --lib`, about 50 s cold), which makes most iteration possible without a Mac. The shell (Tauri) does not; macOS-only modules are behind `cfg(target_os = "macos")`.
- **Shared files.** The engine compiles `crates/launcher/src/telemetry/{endpoint,session}.rs` by path, so those files must not reference `crate::config` or other Windows-launcher-only modules, tests included.
