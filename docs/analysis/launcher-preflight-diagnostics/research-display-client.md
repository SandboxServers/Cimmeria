---
title: "Research: client display knobs (SGW.exe)"
type: analysis
audience: launcher-preflight-diagnostics architect
last_updated: 2026-10-24
---

# Research: which display knobs SGW.exe honours

This research supports owner decision D2 in [OWNER-DECISIONS.md](OWNER-DECISIONS.md). It covers the client side only; Wine and macOS knobs are in `research-display-wine.md`.

The evidence is static. It comes from file reads of the stock install and the live Wine-prefix profile (read-only, not edited), `SGW.exe` string scans, and capstone disassembly of `SGW.exe`. No Ghidra project was available on this machine, and nothing was confirmed live. Addresses are virtual addresses (image base `0x00400000`; for `.rdata`, file offset + `0x401800` = VA).

## Answer

Two layers control display mode, and **the second one wins once the game is running**:

1. **The UE3 startup frame.** At `UGameEngine::Init` the client reads the ini keys `[WinDrv.WindowsClient] StartupFullscreen / StartupResolutionX / StartupResolutionY`, plus the command-line switches `WINDOWED`, `ResX=` and `ResY=`.
2. **SGW's CME SystemOptions.** The options `video.windowedMode` (bool) and `video.resolution` (an enum index into the D3D mode list) are stored in the user's `SavedSystemOptions.xml`. One shared handler (`0x00d3faa0`) applies them by calling `FViewport::Resize(X, Y, fullscreen = !windowedMode)`.

`[SystemSettings] Fullscreen/ResX/ResY` does not exist in this build (no section, no strings). The client has **no borderless option** and **no vsync option**.

**Recommended mechanism:** before every launch, the launcher writes `SavedSystemOptions.xml` (`windowedMode`, and `resolution` for fullscreen) **and** the user `SGWEngine.ini` `[WinDrv.WindowsClient]` keys (startup mode and windowed size). Command-line args (`-windowed ResX=W ResY=H`) are optional belt-and-braces; the current macOS helper can't pass them yet. Confidence: medium-high that SavedSystemOptions is the authority, medium that args alone are enough.

**Side finding: the likely D2 cause.** The D2 diagnosis checked only the ini (`StartupFullscreen=False`). The same profile's `SavedSystemOptions.xml` has `windowedMode` **false**, which is the stock default. The options handler then resizes to fullscreen at the enumerated mode. That matches the observed takeover (the display switched to 1280x800) despite the ini.

## 1. Ini keys

### The chain and the user copy

| File | `[WinDrv.WindowsClient]` values |
|---|---|
| `Working/Engine/Config/BaseEngine.ini:186-192` | `StartupFullscreen=True`, `StartupResolutionX=640`, `StartupResolutionY=480` |
| `Working/Engine/Config/GameplayEngine.ini:351-354` | `StartupFullscreen=False`, `StartupResolutionX=1024` (trailing tab in the file), `StartupResolutionY=768` |
| `Working/SGWGame/Config/DefaultEngine.ini` | none (only `[FullScreenMovie]` at line 7) |
| User `Documents/My Games/Firesky/SGWGame/Config/SGWEngine.ini:214-220` (Wine prefix) | `StartupFullscreen=False`, `EditorPreviewFullscreen=False`, `StartupResolutionX=1024`, `StartupResolutionY=768` |

The user file is a full generated copy. Its `[IniVersion]` (line 743) is `0=1246377216 1=1246377216 2=1246377232`, which matches the mtimes of BaseEngine.ini, GameplayEngine.ini and DefaultEngine.ini. The precedence rule is already documented in [docs/client/limits/engine.md](../../client/limits/engine.md#ini-chain-and-lab-override-targets):

- While `[IniVersion]` matches the chain timestamps, the user `SGWEngine.ini` is used as the complete config, and any key edited there wins.
- If the stamps don't match, the client prompts "Your ini is outdated" (`FUN_00492470`) and regenerates the file from the chain. That would drop launcher edits, and a headless client would block on the modal prompt.

What this means for the launcher:

- Editing keys in the user file is safe.
- Never touch the chain files or the `[IniVersion]` section. A client repair or update that rewrites a chain file changes its mtime and triggers regeneration. That is one more reason the launcher must re-apply its keys before **every** launch.

### Where the binary reads them

UTF-16 strings: `StartupFullscreen` at `0x01849868`, `StartupResolutionX` at `0x0184989c`, `StartupResolutionY` at `0x018498d4`.

`UClient` property registration (`0x005e7c80`-`0x005e7f95`, class `Engine.Client`, `.\Src\UnClient.cpp`, class size `0x68`):

| Field | Offset | Notes |
|---|---|---|
| `InitialButtonRepeatDelay` | `+0x48` | |
| `ButtonRepeatDelay` | `+0x4c` | |
| `StartupFullscreen` | `+0x50` | `UBoolProperty`, category "Display" |
| `StartupResolutionX` | `+0x54` | |
| `StartupResolutionY` | `+0x58` | |
| (native, not config) | `+0x5c`, `+0x60`, `+0x64` | Used by `CreateViewportFrame`, see section 2. Writer not traced. |

### Authority and rewrite behaviour

- **`[SystemSettings]`** is a later UE3 feature and doesn't exist in this build. There's no section in any chain file or the user file, and no `SystemSettings` key strings beside the display keys. `[WinDrv.WindowsClient]` is the only ini layer.
- **Vsync:** there is no key anywhere in the chain, the user file or the binary (`UseVSync` is absent). The launcher already works around this with DXVK's frame limiter (`crates/launcher/desktop/engine/src/storage/launch/wine.rs:35-70`, comment "the client has no vsync setting", `FRAME_RATE_LIMIT = "30"`). The client-telemetry D3D9 seam records the actual `PresentationInterval` (`crates/client-telemetry/src/hooks/seams/d3d9.rs:46`).
- **Rewrite on exit:** the client does **not** rewrite `SGWEngine.ini` on exit. The user file's mtime is still Oct 4 after the 2026-10-24 12:06-12:10 run. It **does** rewrite `SavedSystemOptions.xml` on exit, and also on that run's crash: the file was rewritten at 12:10:30, alongside a crash dump from 12:10. Last writer wins.

## 2. Command-line arguments

All three live in one UTF-16 block in `.rdata`: `ResX=` at `0x018f12ec`, `ResY=` at `0x018f12f8` and `WINDOWED` at `0x018f1304`.

- UE3 `ParseParam` is case-insensitive and matches with or without the leading dash, so `-windowed` works.
- There is no `FULLSCREEN` string in that block. `-fullscreen` is not a switch in this build; fullscreen is simply "ini true and no `WINDOWED`".

Consumer, inside `UGameEngine::Init` viewport creation (`0x008f5d3a`-`0x008f5e0c`). `appCmdLine` is `0x00483290`, `Parse(INT)` is `0x004848b0` and `ParseParam` is `0x00484390`:

```text
Client = GEngine->Client                       ; [GEngine+0x2cc]; Client->Init(this) via vtable+0x10c at 0x008f5bf7
Parse(appCmdLine(), L"ResX=", &Client->StartupResolutionX)   ; 0x008f5d44 (+0x54)
Parse(appCmdLine(), L"ResY=", &Client->StartupResolutionY)   ; 0x008f5d61 (+0x58)
Fullscreen = Client[+0x5c] != 0 && !ParseParam(appCmdLine(), L"WINDOWED")   ; 0x008f5d7a-0x008f5d9d
Frame = Client->vtable[0x118](ViewportClient, Localize("Product"),
                              Client[+0x60], Client[+0x64], Fullscreen, 0) ; CreateViewportFrame
GameViewport->vtable[0x114](Frame)
```

There is a caveat. `ResX=`/`ResY=` write the config fields at `+0x54`/`+0x58`, but `CreateViewportFrame` takes its size from `+0x60`/`+0x64` and its fullscreen flag from `+0x5c`. Whatever fills `+0x5c..+0x64` (probably `Client->Init`, copying from the Startup fields) was not traced. So:

- `WINDOWED` certainly forces the **initial** frame to windowed.
- Whether `ResX=`/`ResY=` size the initial frame is unproven.
- They do size every later windowed resize, because the options handler reads `+0x54`/`+0x58`. See section 4.

Other startup switches found, which may be useful to the launcher:

| Switch | String VA | xref | Effect |
|---|---|---|---|
| `nomovie` | `0x017f7ee0` | `0x00416491` | Skips startup movies (`[FullScreenMovie] StartupMovies` = FireSkyLogo, SGWLogo, esrb, WormHole, blankmovie; `SGWEngine.ini:730`) |
| `NOSPLASH` | `0x017fa2f4` | `0x0041d7bf` | Skips the splash screen |
| `NUKEINIS` / `UPDATEINISAUTO` | `0x01802664` / `0x01802678` | | Ini-regeneration control (engine.md) |
| `SETRES` | `0x01846508` | `0x005d4cda` | A console exec command, not a command-line switch |

These are absent from the binary: dash-prefixed `-windowed` and `-fullscreen` literals (expected, see above), `borderless` and `nomoviestartup`.

**Lab precedent.** `crates/lab/src/supervisor/process.rs:54-79` launches every lab client with `-windowed ResX=1280 ResY=720` (`DEFAULT_WINDOW`). Its comment records why: a profile that never saved its options opened at the desktop resolution (5120x1440, 2026-10-10), because stock `SystemOptions.xml` defaults `windowedMode` to false. The lab profiles **also** seed `SavedSystemOptions.xml` with `windowedMode` true ([live-research-lab.md:773](../../guides/live-research-lab.md)). So the lab evidence doesn't isolate whether `-windowed` alone beats a saved `windowedMode=false`.

## 3. What the launcher does today, and where this would live

**Nothing writes display settings today.** A search of `crates/launcher`, `crates/launcher-core` and `crates/client-launch` finds no write of `StartupFullscreen`, `SGWEngine.ini`, `ResX`, `WINDOWED` or `windowedMode`.

Relevant seams:

- **`crates/launcher-core/src/client_setup/mod.rs`, `prepare(install_dir, servers)`**: runs after Install/Update and before every launch. It is idempotent (stock case, login servers, ASLR off). It only touches `install_dir` today; display settings are per-user under Documents, so it needs the profile path as well.
- **`crates/launcher-core/src/client_changes.rs`**: the "changes to your client" list (`ChangeGroup` Setup/Patch/Launch/Server; `ChangeStatus::EveryLaunch`, and so on). Its Setup rows already include "Login servers" and "ASLR off in SGW.exe" (EveryLaunch). A "Display mode: windowed 1280x720" row with `EveryLaunch` fits this model: the launcher owns the keys and re-applies them on every launch, so ini rewrites and repair can't lose them.
- **Profile path**: `crates/launcher-core/src/client_paths.rs`. It defines `FIRESKY_SUBPATH = "My Games/Firesky"` and gets Documents from `SHGetFolderPathW(CSIDL_PERSONAL)`. Under Wine, the client's Documents is inside the prefix (`<prefix>/drive_c/users/<user>/Documents/My Games/Firesky/SGWGame/`). `crates/launcher/desktop/engine/src/storage/client_maintenance/mod.rs` resolves this on Windows only, so macOS needs the prefix-based variant.
- **Passing args on macOS needs a schema change.** The helper wire `Request {schema_version, operation_id, exe, directory, dlls}` in `crates/launcher/desktop/runtime-probe/src/game_launch/mod.rs:8-14` is `#[serde(deny_unknown_fields)]` and has **no args field**. The engine builds it in `storage/launch/wine.rs:35-70`, and `worker.rs:418` passes `arguments: vec![]`. Adding SGW.exe args means a `schema_version` bump in both the helper and the engine. The Windows `client-launch` start32 request already carries args (the lab uses it).

## 4. In-client display settings UI and persistence

**Definitions.** The stock file `Working/SGWGame/Content/XML/SystemOptions.xml` (755 lines) defines these options:

| Option | Lines | Group | Type | Default | `changeEvent` | Notes |
|---|---|---|---|---|---|---|
| `resolution` | 76-87 | `video` | enum | (runtime) | `Event_Option_Resolution` | Text "Fullscreen Video Resolution", tooltip "Change the video resolution while running in full screen mode"; "resolution choices are added at runtime"; `waitsForApply='true'` |
| `windowedMode` | 99-110 | `video` | bool | `false` | `Event_Option_WindowedMode` | Text "Window Mode"; `waitsForApply='true'` |
| `devWindowedMode` | 555-566 | `internal` | bool | `true` | `Event_Option_DevWindowedMode` | Internal group, hidden from players |

There is no borderless, vsync or windowed-size option. The windowed size comes only from the ini or args (`+0x54`/`+0x58`).

**UI.** `Working/SGWGame/Content/UI/Common/Options/Options.lua` (722 lines) builds generic option lists through `getOptionGroupCount`/`getOptionInfo`. The flow:

1. Edits call `proposeOptionBool/Enum/Float` (lines 97-114).
2. OK (176-184) and Apply (187-195) call `acceptAllChanges()`. Cancel/Close (161-173, 198) calls `revertAllChanges()`.
3. Accepting fires each option's `changeEvent`. `onOptionsChangeApplied` is at line 586.

**Handler.** The three events share one handler. The thunks `0x00d3fcc0`, `0x00d3fcd0` and `0x00d3fce0` are each `call 0x00d3faa0; ret 8`, registered at `0x00d40057`-`0x00d4013f`. This answers open question 3 in [atrea-editor.md](../../reverse-engineering/findings/atrea-editor.md) (factories `0x005BF5D0`/`0x005BF640`/`0x005BF6B0`, thunks `0x005B67A0`/`0x005B6810`/`0x005B6880`): they reach the same function. `0x00d3faa0` has no direct callers; it runs only on events. Recovered logic:

```text
vp  = dynamic_cast<FViewport>(GEngine(0x01ee1254)->GamePlayers[0]->ViewportClient(+0x64)->Viewport(+0x44))
win = getOption("video", "windowedMode")          ; string 0x019bc2f0, xref 0x00d3fb16
X = Client->StartupResolutionX (+0x54); Y = Client->StartupResolutionY (+0x58)
if (!win.bool) {
    idx   = getOption("video", "resolution").int  ; string 0x019bc318, xref 0x00d3fb99
    modes = EnumerateD3DModes()                    ; 0x00ec3f20, 16-byte entries
    X, Y  = modes[idx]
}
vp->Resize(X, Y, Fullscreen = !win.bool)           ; vtable[1], call at 0x00d3fc79
```

So:

- In **windowed** mode, the size is the ini `StartupResolutionX/Y` (as possibly overridden by `ResX=`/`ResY=`).
- In **fullscreen** mode, it is the D3D mode at index `resolution`. Index 0 is the first enumerated mode. That is not the desktop resolution, and its value depends on the adapter, so the launcher can't map a resolution to an index without enumerating modes the same way.
- Fullscreen here is D3D9 exclusive fullscreen; the D3D reset path is `0x00ec4c90` ([multi-client-lab.md](../../reverse-engineering/findings/multi-client-lab.md)). What the lab described as "borderless at desktop res" is this fullscreen path.

**Persistence.** The options are written to `Documents/My Games/Firesky/SGWGame/SavedSystemOptions.xml` (string `0x0183fd30`, xref `0x0057449b`; ASCII `SystemOptions.xml` at `0x017f9690`). The file is CRLF. The live user file's relevant entries, in its own format:

```xml
<options valueInt="0" version="1" optionName="resolution" groupName="video"></options>
<options valueBool="false" version="1" optionName="windowedMode" groupName="video"></options>
<options valueBool="true" version="1" optionName="devWindowedMode" groupName="internal"></options>
```

The file also holds `ShaderModel` 4 and others. `WindowStates.xml` (string `0x0183f438`) stores UI window positions, not the display mode.

**Applied at startup?** Static proof that the options manager fires `changeEvent` for loaded values is missing; the load site near `0x0057449b` was not traced. The empirical evidence says it does:

1. The stock lab profile opened fullscreen at desktop resolution with ini `StartupFullscreen=False`.
2. The D2 profile has ini False and saved `windowedMode=false`, and took over the display.

## Recommendation for the launcher

1. **Owned keys, applied on every launch.** This is a `client_changes` row with `EveryLaunch` status.
   - `SavedSystemOptions.xml`: set `video.windowedMode` (true for windowed). Keep `resolution` as the user's or a known-safe index for fullscreen. Create the file with the stock format if it's missing. Preserve every other entry and CRLF.
   - `SGWEngine.ini` `[WinDrv.WindowsClient]`: set `StartupFullscreen` to match, and set `StartupResolutionX/Y` to the windowed size. Leave `[IniVersion]` and the chain files alone.
2. **Optional args:** add `-windowed ResX=W ResY=H` for windowed mode. This needs the macOS helper `Request` schema bump. It protects the first frame before options apply.
3. **Reading in-game changes back.** The client rewrites `SavedSystemOptions.xml` on exit, so an in-game toggle is overwritten by the launcher's value at the next launch. Either read the file back after exit and update launcher Settings, or say in Settings that the launcher controls display mode.
4. **Not client knobs:**
   - **Borderless:** none in the client. A borderless look needs windowed mode at desktop size plus the Wine or OS layer (see `research-display-wine.md`), or a client patch.
   - **Vsync:** none in the client. Keep the DXVK limiter or DXVK's own vsync config.
5. **Arbitrary fullscreen resolutions** need the mode index. Prefer offering windowed (any size) and fullscreen (the user's in-game choice) rather than a launcher-side fullscreen resolution picker.

## Open questions

| Question | Evidence that would resolve it |
|---|---|
| Does the options manager fire `Event_Option_WindowedMode` at load, before or after the first frame? | Trace the loader around `0x0057449b`, or a lab run with a breakpoint at `0x00d3faa0` during startup. |
| What writes `Client+0x5c/+0x60/+0x64` (the `CreateViewportFrame` inputs)? | Decompile `UWindowsClient::Init` (`Client` vtable `+0x10c`). |
| Does `-windowed` alone keep a profile with saved `windowedMode=false` windowed? | A lab run with that profile and no options seeding. |
| How does `resolution` index 0 order its modes (by size, by refresh rate)? | Decompile `0x00ec3f20`. |
