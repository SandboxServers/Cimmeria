# Resume the macOS/Wine playtest — 2026-10-03

> **Type:** How-to (session handoff)
> **Audience:** The Mac tester and the agent continuing the bring-up
> **Last updated:** 2026-10-03, after saving the launcher workaround
> **Companions:** [Experimental Mac guide](../../../guides/macos.md), [current issues and status #1151](https://github.com/SandboxServers/Cimmeria/issues/1151), [Mac prerequisites #1150](https://github.com/SandboxServers/Cimmeria/issues/1150), [Windows legacy runtimes #1121](https://github.com/SandboxServers/Cimmeria/issues/1121)

## Native launcher implementation

For the later Tauri work, use the [implementation plan](launcher-implementation-plan.md),
[delivery ledger](launcher-implementation-ledger.md), and
[delegation plan with Codex/Claude prompts](launcher-delegation-plan.md).
The Wine playtest notes below describe the earlier launcher workaround.

## Historical Windows egui/WoWSilicon track

**The launcher window blocker is resolved. The game is not installed yet.**
The Windows launcher renders under WoWSilicon with `CX_FWD_COMPAT_GL_CTX=1`.
That setting is now saved in the `cimmeria` profile; a temporary terminal launch
is no longer required. No launcher renderer change is needed for this blocker.

1. Open WoWSilicon, select **Cimmeria**, and press **Play** if the launcher is not
   already running. Confirm the window and manifest appear. The light theme is a
   known cosmetic issue, not the original window-creation failure.
2. Use **Install / Update**. At the last visual check, the seed was not installed
   and all seven manifest patches were missing. No installation was started in
   this session. Allow time and disk space for the download and extraction.
3. Before claiming the game is ready, check prerequisites tracked in
   [#1121](https://github.com/SandboxServers/Cimmeria/issues/1121) and
   [#1150](https://github.com/SandboxServers/Cimmeria/issues/1150): legacy PhysX
   2.x, VC++ 2005 x86, D3DX9/XInput, and the correct D3D9 DLL beside the game's
   `Working/Binaries/SGW.exe`. The DLL staged beside the launcher does not prove
   it is present beside the game.
4. Launch through the launcher, keeping **Load client patches** enabled.
   Telemetry remains the tester's opt-in choice; do not enable it automatically.
   If anything fails, capture the first error, launcher/game logs, and which
   checkpoint failed before changing Wine or the renderer.
5. Ask the human tester to verify the login screen, login to the intended server,
   world entry, and the Black Market window. None of those have passed on this Mac.

## What passed, and what did not

| Check | Evidence and boundary |
|---|---|
| Launcher renders | The tester's screenshot shows the complete launcher UI, successful update status, and schema-1 manifest with seven patches. This was the initial diagnostic launch with the environment flag. |
| Workaround persists | Saved through WoWSilicon **Options → Environment**. A read of its `versions.json` confirmed `settings.environmentVariables` contains exactly `CX_FWD_COMPAT_GL_CTX=1`. |
| Saved profile launches | Closed the prior diagnostic launcher and pressed **Play**. The new launch command included the saved flag; the launcher process stayed alive and WoWSilicon showed **Wine · 1**. No second screenshot was captured, so repeat visual confirmation when resuming. |
| Appearance | The tester reports macOS dark mode but the launcher is light. Wine's Windows-side theme reporting is a plausible explanation, not a verified cause. Tracked in #1151. |
| Installation and gameplay | Not tested: seed installation, patch application, game runtime loading, D3D9 rendering, DLL injection, login, world entry, or Black Market behavior. |

The general Mac guide still predates these findings; use this handoff and the
linked issues for the current blocker status and prerequisite gaps.

## Preserve this setup

The recorded environment is **macOS 26.6.1 (arm64)** with Rosetta 2 and
**WoWSilicon 3.2.2**. Earlier setup checks found approximately 181 GB free; recheck
space before installing rather than treating that number as current.

- Launcher release: `launcher-20260929-0d71e26`, PE32+ x86-64. Its SHA-256 matched
  the release asset. The embedded release signing key verified the live
  `content-current` manifest using Ed25519; schema 1 declared a 4.14 GB seed and
  seven patches. These are dated checks, not guarantees about a later release.
- Wine prefix: `~/WoWSilicon/`. Profile: `cimmeria`, `generic_d3d9`, backend `d9vk`,
  x87 `rosettax87`, executable pointing at `sgw-launcher.exe`.
- WoWSilicon staged a roughly 3.4 MB, 32-bit `d3d9.dll` beside the launcher.
- Wine Mono 11.2.0 was installed. Modern VC++ runtime DLLs were present
  (`vcruntime140.dll`, `vcruntime140_1.dll`, `msvcp140*.dll`, `ucrtbase.dll`).
  This does **not** establish the game's older runtime prerequisites.
- The public login endpoint was TCP-reachable during setup. That does not prove
  authentication or an in-game connection works.

## If the original window error returns

Check the saved environment setting first:

```text
CX_FWD_COMPAT_GL_CTX=1
```

The original error was:

```text
eframe::native::run: Exiting because of error:
  glutin error: extension to create ES context with wgl is not present
```

The source diagnosis found that eframe/glutin first requests desktop OpenGL 3.3
core without the forward-compatible flag. Wine's macOS driver rejects that
request; eframe then attempts OpenGL ES, producing the final missing-extension
error. WoWSilicon's Wine already supports the environment workaround that adds
the missing flag. See the [pinned Wine implementation](https://github.com/WineAndAqua/wine/blob/37540b5d94ac1c86e2599ef55d7f3a15e3237ce8/dlls/winemac.drv/opengl.c#L2134-L2152)
and the investigation in [#1150](https://github.com/SandboxServers/Cimmeria/issues/1150).
The successful launch with the flag confirms a working path on this setup;
it does not establish game-rendering compatibility.

Continue with this setup before trying the deferred alternatives: a Windows
installation copied to the Mac, CrossOver, another Wine, or a launcher
wgpu/software fallback. A copied tree does not carry installed system runtimes,
and launching `SGW.exe` directly skips launcher DLL injection. A wgpu renderer
alone would not guarantee software rendering. D3D9 backend switches do not
address the launcher's OpenGL context request.

There is no native macOS target or cross-compile path in this repo. Any future
launcher code change must build natively on Windows, route every compiling
`cargo` call through `tools/build-lane/lane.sh`, and include tests and the
corresponding docs under [CLAUDE.md](../../../../CLAUDE.md),
[TESTING.md](../../../../TESTING.md), and the
[doc-update map](../../../agents/doc-update-map.md).

## Future launcher research

[Launcher platform options](launcher-platform-options.md) compares a shared native
UI with SwiftUI plus Rust, identifies the real porting boundaries, and proposes
measurement gates. It records the earlier research phase. The subsequent approved Tauri scope is
defined by the implementation plan above.

## Historical Windows-track continuation prompt

```text
Continue the macOS/Wine Stargate Worlds bring-up. Read
 docs/analysis/playtests/2026-10-03-macos-wine/README.md
and GitHub issue #1151 in SandboxServers/Cimmeria first.
The launcher workaround CX_FWD_COMPAT_GL_CTX=1 is saved in the Cimmeria
WoWSilicon profile and the saved-profile launch stayed running. Confirm the
window, then proceed with Install / Update; the game was not installed at the
last check. Preserve client patches and the user's telemetry choice. Verify
legacy runtimes and the game's D3D9 DLL placement, capture the first failure,
and separate automated checks from human login/world/Black Market UAT.
Do not propose a native macOS build or restart the resolved WGL investigation
unless the saved workaround no longer works.
```

## Tauri implementation plan

The [Tauri + Effect implementation plan](launcher-implementation-plan.md) records
the 2026-10-04 direction: reuse Rust installation/launch behavior, implement real
Effect orchestration and focused consent-aware operation summaries, then perform
self-contained startup testing as the final release gate. This packet is a plan,
not production implementation or an observed login result.

Implementation has begun with the [native operation contract](../../../../crates/launcher/desktop/README.md);
see the plan's implementation ledger for tested scope and remaining work.

Implementation evidence: [runtime provisioning](runtime-provisioning.md) records
the pinned Wine inventory, candidate launch environment, distribution gates and
Windows-helper ownership requirements. It does not establish game compatibility.


## Launcher implementation references

- [Requirements and delivery plan](launcher-implementation-plan.md)
- [Dated implementation ledger](launcher-implementation-ledger.md)
- [Migration audit](worknotes/migration-audit.md)
- [Native-window UAT](worknotes/native-window-uat.md)
- [Repair handoff](worknotes/repair-ui.md)
- [Acceptance checklist and current ownership](launcher-acceptance.md)
- [Runtime provisioning evidence](runtime-provisioning.md)

## Current integration evidence

- [Play controls and native lifecycle](worknotes/play-integration.md)
- [Repair review fixes and production-host UAT](worknotes/repair-review-fixes.md)
- [Updater parity research](worknotes/updater-parity-research.md)
