# Launcher Consolidation

> Type: how-to. Audience: the Claude Code coordinator, packet workers and maintainers.
> Updated: 2026-10-10. Companions: [work packets](work-packets.md), [acceptance gates and LX-26 results](acceptance.md), [desktop launcher README](../../../crates/launcher/desktop/README.md), [desktop launcher ledger (macOS/Wine)](../playtests/2026-10-03-macos-wine/README.md), [documentation index](../../readme.md).

## Purpose

Cimmeria has three launchers. This campaign leaves one.

| Launcher | Where | State on `main` @ `2c1def5bc` | Fate |
|---|---|---|---|
| Old Tauri prototype | `tools/SGWLauncher/` (`src-tauri` excluded from the workspace) | Dead. Nothing builds it. | Delete (LX-19) |
| egui launcher, `sgw-launcher` | `crates/launcher/src/` | The launcher players use. Released as a bare `sgw-launcher-<tag>.exe`. | Port what it has that the desktop launcher lacks, then delete (LX-18) |
| Desktop launcher (Tauri 2 shell, Effect/TypeScript frontend) | `crates/launcher/desktop/` | Install, Play, login and character select proven once on Windows from a debug build. Released as a preview zip beside the egui exe. | **Keep.** It becomes the only launcher. |

The throwaway prototypes `crates/launcher/prototype-packaging/` and `crates/launcher/prototype-macos/` go too (LX-19).

The desktop launcher is released as a **zip** with a stable download link, and it updates itself from that zip.

## What was found

The egui launcher has these and the desktop launcher does not:

| Gap | egui source | Packet |
|---|---|---|
| The desktop engine compiles 11 egui files through `#[path = "../../../src/..."]` (`manifest`, `client_setup`, `install`, `install_layout`, `install_report`, `patch_dest`, `state`, `unpack`, `install_progress`, `telemetry/endpoint`, `telemetry/session`). The egui tree cannot be deleted while they live there. | `crates/launcher/desktop/engine/src/lib.rs:16-43` | LX-01 |
| Debug log upload to Azure Blob (SAS baked in from `LAUNCHER_LOG_SAS_URL`) | `src/logs.rs`, `src/app/view.rs:399` | LX-05 |
| Server list editor (`LoginInternal.lua` entries). Desktop always writes `default_servers()`. | `src/client_setup/login_servers.rs`, `src/app/view.rs:92` | LX-03 |
| "Load client patches" toggle for a normal install | `src/config.rs:76-98`, `src/app/view.rs:374` | LX-04 |
| Reset client cache / Reset all client state | `src/client_paths.rs`, `src/app/view.rs:448-533` | LX-06 |
| "Changes to your client" list | `src/client_changes.rs`, `src/app/client_changes_panel.rs`, `src/overlay_meta.rs` | LX-07 |
| Launcher-side telemetry pipeline: token refresh, gzipped chunk upload, disk queue, log tailing, end-of-session bundle, process exit code | `src/telemetry/{auth,runner,queue,tail,process_watch}.rs` | LX-08a, LX-08b |
| Telemetry events: `client.launcher.install_result`, client-patches boot verdict, Black Market counts | `src/telemetry/{install_result,patch_log,patch_counts}.rs` | LX-08c |
| A second launcher instance is refused with a message | `src/instance_lock.rs`, `src/main.rs:63-127` | LX-09 |
| Adopting an existing install on Windows (desktop adoption is macOS only) | `src/install.rs:487-531` | LX-10 |
| Self-update that works. Desktop's updater is built but has no feed or key, and its Windows apply expects an installer nothing builds. | `src/self_update/` | LX-13, LX-15 |
| `pack-client-overlay` release tool | `src/bin/pack-client-overlay.rs`, `src/overlay_pack.rs` | LX-14 |

Neither launcher handles these Windows prerequisites today: the WebView2 runtime (the desktop launcher cannot open a window without it), and PhysX System Software ([#1121](https://github.com/SandboxServers/Cimmeria/issues/1121)). LX-11 and LX-12 add them.

The desktop launcher already has, and keeps: Repair, Uninstall, signed game updates with rollback, Wine/macOS support, PhysX setup under Wine, import of egui settings, and anonymous launcher summaries (switched off).

Dropped on purpose: the Atera debug buttons (`AteraLoader.exe` launches). They were developer-only (D-LX4).

Behaviour that differs and stays different: a desktop game update rebuilds the whole install into a stage and swaps it in, while the egui launcher applied only new patches in place.

## Decisions

| ID | Status | Decision | Reason |
|---|---|---|---|
| D-LX1 | **APPROVED** (user, 2026-10-10); maintainer and aurablacklight sign-off **required before LX-18** | Consolidate on the desktop launcher. Retire the egui launcher, `tools/SGWLauncher/` and both prototypes. | One launcher to maintain. The desktop launcher already has Repair, Update and Wine. |
| D-LX2 | **APPROVED** (user) | **Announce only.** No bridge build. The last release stops publishing `sgw-launcher-<tag>.exe`; players are told on Discord, the README and the release notes to download the zip. | Smallest scope. The desktop launcher already imports egui settings (`migration`). |
| D-LX3 | **APPROVED** (user) | **Zip self-update.** The launcher reads a static feed, downloads the new zip, verifies its sha256 and its Minisign signature, swaps its own files and relaunches. No installer. | Matches the zip distribution. Reuses the existing feed, transport and Minisign verification in `engine/src/storage/updater/`. |
| D-LX4 | **APPROVED** (user) | Port debug log upload, the server list editor and the full telemetry pipeline. Drop the Atera debug buttons. | |
| D-LX5 | **APPROVED** (user) | Add a WebView2 runtime check and Windows PhysX System Software setup. | Neither launcher handles either today. |
| D-LX6 | PROPOSED (coordinator) | Port the remaining parity items too: client-patches toggle (LX-04), cache reset (LX-06), "Changes to your client" (LX-07), single instance (LX-09). | They are small, and players rely on cache reset to recover from bad cooked data. |
| D-LX7 | PROPOSED | **Stable download link.** Every launcher release also refreshes a rolling `launcher-current` release (the same pattern as `content-current`) holding `StargateWorlds-Launcher-windows-x64.zip`, its `.sha256`, `.zip.sig` (Minisign) and the updater feed `latest.json`. Docs link to `https://github.com/SandboxServers/Cimmeria/releases/download/launcher-current/StargateWorlds-Launcher-windows-x64.zip`. Dated `launcher-YYYYMMDD-<sha7>` releases stay as history. | `releases/latest/download/...` follows GitHub's single "latest" release, which other release types can take. A rolling tag never moves away. |
| D-LX8 | PROPOSED | **Version.** The compiled launcher version is SemVer `YYYY.M.D` from the release date (for example `2026.10.10`), with a same-day rebuild as `YYYY.M.D-N` refused by the stable channel. The tag keeps its current form. | The updater compares stable SemVer (`docs/updater.md`). |
| D-LX9 | **BlockedDecision** (owner action) | A Minisign updater key pair is generated once. The private key and its password go in Key Vault `cimmeria-kv` and the GitHub secrets `LAUNCHER_UPDATER_MINISIGN_KEY` / `_PASSWORD`; the public key is compiled in from `LAUNCHER_UPDATER_PUBKEY`. | Needs someone with vault and secrets access. LX-15 is dry-run-only until this is done. |
| D-LX10 | **APPROVED** (deferred) | No code signing in this campaign. The player guide documents the SmartScreen and Defender prompts. | No certificate exists. |
| D-LX11 | PROPOSED | Log upload reuses the `LAUNCHER_LOG_SAS_URL` secret and the blob naming `logs/<host>-<utc>-<digest>.zip`. | Same container and retention as today. |
| D-LX12 | **BlockedDecision** | **Windows adoption of an egui install.** Proposed: adopt **in place**. The launcher verifies the folder against the signed release (stock files plus applied patches, read from `launcher-installed.json`), writes its owner marker, and from then on treats the folder as owned for Play, Repair and Update. Uninstall of an adopted folder removes only what the release lists. The macOS path stays a verified copy. | With D-LX2, every current player points the new launcher at an existing multi-gigabyte install. A copy doubles disk use; a reinstall re-downloads the client. |
| D-LX13 | PROPOSED | **Windows PhysX.** Detect PhysX System Software (`PhysXLoader.dll` resolvable and the `AGEIA Technologies` uninstall key). If absent, offer to run the pinned vendor MSI the macOS path already verifies, elevated through `ShellExecuteExW` with `runas`. No registry-only workaround. | Same package and hash as the Wine path. The `enableLocalPhysXCore` trick needs HKLM writes tied to a MAC address and is fragile. |
| D-LX14 | **BlockedDecision** | **macOS players.** Today `docs/guides/macos.md` runs the egui exe under Wine. The desktop launcher's macOS build is not released (signing, notarization and Wine redistribution are open, #1151). Options: (a) keep publishing the egui exe for macOS only until the desktop macOS build ships; (b) document running the desktop Windows zip under Wine (WebView2 under Wine is not proven); (c) accept that macOS players have no launcher until the macOS build ships. **Recommended: (a)**, with LX-18 deleting the egui crate only after the macOS build ships. | Retiring egui without one of these leaves macOS players with nothing. |

## Packets

Status vocabulary: **Ready / BlockedDependency / BlockedDecision / Writing / Review / Integrated / UATPending / Done**.

| Packet | Wave | Title | Agent | Depends on | Status |
|---|---|---|---|---|---|
| LX-01 | 0 | Shared `cimmeria-launcher-core` crate | rust-gameserver-dev | | Ready |
| LX-02 | 0 | Scaffold every new command, preference and view (the contract) | rust-gameserver-dev | LX-01 | BlockedDependency |
| LX-03 | 1 | Server list editor | packet-coder | LX-02 | BlockedDependency |
| LX-04 | 1 | Client-patches toggle | packet-coder | LX-02 | BlockedDependency |
| LX-05 | 1 | Debug log upload | packet-coder | LX-02 | BlockedDependency |
| LX-06 | 1 | Reset client cache and client state | packet-coder | LX-02 | BlockedDependency |
| LX-07 | 1 | "Changes to your client" panel | packet-coder | LX-02 | BlockedDependency |
| LX-08a | 1 | Telemetry transport: token refresh, queue, chunk upload | rust-gameserver-dev | LX-02 | BlockedDependency |
| LX-08b | 2 | Telemetry session: log tailing, exit code, end-of-session bundle | rust-gameserver-dev | LX-08a | BlockedDependency |
| LX-08c | 2 | Telemetry events: install result, patch verdict, BM counts | packet-coder | LX-08a | BlockedDependency |
| LX-09 | 1 | Single instance | packet-coder | LX-02 | BlockedDependency |
| LX-10 | 1 | Windows in-place adoption | rust-gameserver-dev | LX-02, D-LX12 | BlockedDecision |
| LX-11 | 1 | WebView2 runtime check | packet-coder | | Ready |
| LX-12 | 1 | Windows PhysX setup | rust-gameserver-dev | LX-02 | BlockedDependency |
| LX-13 | 1 | Zip self-update apply on Windows | rust-gameserver-dev | LX-02 | BlockedDependency |
| LX-14 | 1 | Move `pack-client-overlay` into `cimmeria-patchset` | packet-coder | | Ready |
| LX-15 | 2 | Release: zip only, `launcher-current`, signed feed | rust-gameserver-dev | LX-13, LX-14; D-LX9 for a live run | BlockedDependency |
| LX-16 | 3 | CI: retire egui jobs, keep client-launch and start32 | packet-coder | LX-18 | BlockedDependency |
| LX-17 | 2 | Window title, version stamp and About | packet-coder | LX-02 | BlockedDependency |
| LX-18 | 3 | Delete the egui launcher | packet-coder | Waves 1-2, D-LX1 sign-off, D-LX14 | BlockedDecision |
| LX-19 | 1 | Delete `tools/SGWLauncher/` and the prototypes | packet-coder | | Ready |
| LX-20 | 3 | `setup.ps1` and `bootstrap/` build the desktop launcher | packet-coder | LX-18 | BlockedDependency |
| LX-21 | 3 | Comment and path references in other crates | packet-coder | LX-18 | BlockedDependency |
| LX-22 | 4 | Player docs and announcement | documentation-writer | LX-15 | BlockedDependency |
| LX-23 | 4 | Developer and operator docs | documentation-writer | LX-18 | BlockedDependency |
| LX-24 | 4 | Agent-memory sweep | documentation-writer | LX-18 | BlockedDependency |
| LX-25 | 0 | Adopt the desktop launcher's open acceptance gates into this ledger | documentation-writer | | Ready |
| LX-26 | 5 | Windows UAT of the released zip | owner (+ lab-uat for Play) | Waves 1-4 | BlockedDependency |
| LX-27 | 5 | Close-out | coordinator | LX-26 | BlockedDependency |

Wave 1 has fourteen packets that run at once after LX-02 merges. LX-11, LX-14, LX-19 and LX-25 need nothing and can start now.

## Out of scope

- Code signing (D-LX10) and the macOS release (#1151); D-LX14 decides what macOS players use in the meantime.
- Moving `crates/launcher/desktop/` up to `crates/launcher/`. Every path in the desktop docs and workflows would change for no player benefit.
- Un-ignoring the timing-sensitive tests in [#1259](https://github.com/SandboxServers/Cimmeria/issues/1259).
- Turning on launcher summaries. Endpoint and consent wording stay with their own ledger.
