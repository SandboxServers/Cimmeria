# Atrea Editor (in-game UnrealEd) — Architecture and Surface

> **Last updated**: 2026-05-25
> **Binary**: SGW.exe (32-bit x86 PE, MSVC; image base `0x00400000`)
> **Reference source**: `Reference/UE3-2004/Development/Src/` (Editor/, UnrealEd/, Launch/)
> **Companion docs**: [editor-source-mapping.md](../editor-source-mapping.md) (function-by-function VA→source map), [atrea-editor-bridge.md](../../architecture/atrea-editor-bridge.md) (MCP bridge spec)

This document is the top-level entry point for the in-game UnrealEd editor that ships inside SGW.exe. It supersedes the apocryphal trio (`docs/technical/atrealoader-exe.md`, `docs/technical/atrealoader-config.md`, `docs/technical/atrearl-loader.md`), which describe only the *DLL injector + sniffer* and miss the editor proper.

## TL;DR

`AtreaEditor.bat` runs `AtreaLoader --enable-group=Editor`. AtreaLoader.exe injects `AtreaRL.dll` into a suspended SGW.exe; AtreaRL applies ~13 byte patches from `AtreaLoader.config.xml` to the running SGW.exe image; those patches flip `GIsEditor=1` (`0x01EAD7AC`), `GIsServer=1`, `GIsGame=0`, swap the global callback VMT to the editor variant, and rewrite the current-package string `L"Launch"` → `L"UnrealEd"`. SGW.exe then runs as UE3's wxWidgets-based UnrealEd — the same editor binary distributed with every UE3 title — with five SGW-specific dialogs (GameMap browser, WorldMap, MapLayer, NewWorldMap, RandomActorSettings) for BigWorld chunked-world authoring. The editor accepts text console commands (`BRUSH ADD`, `ACTOR ADD CLASS=Foo`, `MAP LOAD FILE="..."`, `OBJ SAVEPACKAGE ...`) via a three-tier Exec dispatch chain, all of which can be driven programmatically.

## Activation chain

```text
user double-clicks AtreaEditor.bat
        │
        v
AtreaLoader.exe --enable-group=Editor
        │
        ├── CreateProcessA("SGW.exe ...", CREATE_SUSPENDED)
        │
        ├── VirtualAllocEx + WriteProcessMemory + CreateRemoteThread → LoadLibraryA("AtreaRL.dll")
        │
        └── ResumeThread(SGW.exe main thread)
                │
                v
        AtreaRL.dll DllMain (rev. 36, built Feb 21 2014)
                │
                ├── parse own command-line: --enable-group=Editor was passed via CreateProcess
                ├── read AtreaLoader.config.xml (NOTE: runtime reads the *binary* .config; .xml is source)
                ├── for each <Patch Group="Editor"> with Apply or enabled group: WriteProcessMemory
                │       EditorMode  @ 0x00418AF0 → swap MOV-source bytes (89 35 ↔ 89 1D)
                │       EditorCallbacks  @ 0x004186D2 → swap pointer args
                │       EditorCallbackVMT  @ 0x01D8F52C → swap VMT ptr
                │       EditorCurrentPackage  @ 0x01D8F4A0 → "Launch" → "UnrealEd"
                │       EditorSettings  @ 0x005757BA → setz → setnz
                │       EditorUnknownUi  @ 0x00566919 → CMP imm 0 → 1
                │       DisablePrefabSerialize  @ 0x005CE8E1 → JGE → JMP
                │       (+ EditorSplash, Silent group hides)
                ├── install symbol hooks (FFileManager::MoveFile, UPrefab::Serialize, FArchive::PostLoad, UObject::Serialize)
                └── (optional) start Mercury sniffer, write pcap to sessions/YYYY-MM-DD_HH-MM.pcap
                │
                v
        SGW.exe entry → CRT init → WinMain → AppEntry → ParseCommandLine
                │
                v
        FUN_004185e0 reaches "MOV [GIsEditor=0x01EAD7AC], EBX" at 0x00418AFC
            but with EditorMode patch applied this is now "MOV [...], ESI" → GIsEditor = 1
                │
                v
        EngineInit creates UUnrealEdEngine (not UGameEngine), stored at DAT_01ee1254 (GEngine)
                │
                v
        WxApp::Run → WxUnrealEdApp::OnInit @ 0x00ED4530
                │
                ├── WxLaunchApp::OnInit @ 0x0041D7A0
                │       ├── wxBitmap::LoadFile("PC\EdSplash.bmp") + wxSplashScreen
                │       ├── wxXmlResource::Get()->InitAllHandlers()
                │       └── wxXmlResource::Get()->Load("wxRC/UnrealEd*.xrc")
                │              # Loads: UnrealEd.xrc, UnrealEdPhysics.xrc,
                │              #        UnrealEdRemoteControl.xrc, UnrealEdSGW.xrc,
                │              #        UnrealEdWizards.xrc
                ├── WxEditorFrame ctor @ 0x00FED340  (~21 wxBitmap members init)
                ├── register ~19 dockable panels + sub-editors
                └── (*GEditorFrame->vfunc[0x380])(1)   ← wxFrame::Show(true)
                                                       ← editor is now visible
```

## Patch reference (Editor group)

Full byte-level analysis lives in `AtreaLoader.config.xml`; this is the summary. Image-relative offsets — absolute VA = offset + `0x00400000`.

| Patch | RVA | Containing function | Effect |
|---|---|---|---|
| **EditorMode** | `0x00018AF0` | `FUN_004185e0` (ParseCommandLine/InitGlobals) | Swap MOV-source bytes (`89 35 BC D7 EA 01` ↔ `89 1D AC D7 EA 01` etc.) so `GIsServer=1`, `GIsEditor=1`, `GIsGame=0`. ESI=1, EBX=0 at this point. |
| **EditorCallbacks** | `0x000186D2` | inside `FUN_004185e0` | Swap 4 `PUSH imm32` args to engine-init so it installs editor-flavored `FCallbackEventDeviceEditor`, `FCallbackQueryDeviceEditor`, `FFeedbackContextEditor`, `FOutputDeviceFile`. |
| **EditorCallbackVMT** | `0x0198F52C` | `.data` VMT slot | Rewrite VMT ptr from game (`0x017F8D80`) to editor (`0x017F8DD8`) variant. |
| **EditorCurrentPackage** | `0x0198F4A0` | `.data` UTF-16 string | Replace `L"Launch"` with `L"UnrealEd"`. Drives `UObject::CreatePackage`, `FOutputDeviceFile` log naming. |
| **EditorSettings** | `0x001757BA` | `FUN_00575730` (config-string parser) | `SETZ DL` → `SETNZ DL` — inverts the `wcsicmp(cfg, L"EDITOR")` test so the editor-settings bool at struct offset `+0x28` is set regardless of config. |
| **EditorUnknownUi** | `0x00166919` | `FUN_00566910` | `CMP [ESP+0x4], 0` immediate `00 → 01` — forces editor-UI branch even when called with arg=0. |
| **DisablePrefabSerialize** | `0x001CE8E1` | `FUN_005CE7E0` (prefab serializer) | `JGE 0x005CE9B6` → `NOP; JMP` unconditional — always skips the prefab `Serialize()` loop, preventing partial re-serialization on map open. |
| **EditorSplash** | `0x013FA350` | `.data` UTF-16 string (`Splash` group) | Replace `PC\EdSplash.bmp` with `PC\Splash.bmp` (despite the patch name, this *removes* the editor splash). |
| **EditorChunkLimit** | `0x007FDA41` | editor map-load engine method | `Editor-Disabled` group — `JLE` → `JMP` removes "chunk count ≤ 100" guard. |
| **EditorMyGamesDir** | `0x0008D1E8` | `FUN_0048D080` | `Editor-Disabled` group — NOP the `chdir` into `My Games\FireSky\SGWGame` so packages save next to the binary. |
| **HideEditorBrowserPane** (×2) | `0x00AD56BA`, `0x00B5E789` | `WxUnrealEdApp::vfunc_25` chain | `Silent` group — NOP `BrowserPane::Show(true)` virtual calls. |
| **HideEditorWindow** | `0x00B5F639` | `FUN_00F5F580` | `Silent` group — NOP `WxEditorFrame::Show(true)`. |

After all editor patches apply: `GIsClient=1, GIsServer=1, GIsEditor=1, GIsUCC=0, GIsGame=0`. The engine self-identifies as a single-process editor (client+server+editor, not game).

## Engine-mode globals (UE3 standard, confirmed in this build)

| Flag | VA | Editor value | Notes |
|---|---|---|---|
| `GIsClient` | `0x01EAD7BC` | 1 | UClass/UObject readers, ~50 xrefs |
| `GIsServer` | `0x01EAD7C0` | 1 | UGameEngine readers, gated server path |
| `GIsEditor` | `0x01EAD7AC` | 1 | Atrea patch target. UGameEngine, FFeedbackContext, FOutputDevice readers |
| `GIsUCC` | `0x01EAD7B0` | 0 | UnrealScript compiler mode (off in editor); asserted at `0x0049F714` |
| `GIsGame` | `0x01EB0830` | 0 | FEdObjectPropagator toggles, ACoverLink/FTerrainObject readers |
| `GIsPlayInEditorWorld` | observed in asserts at `0x019677A0` etc.; exact VA TBD | 0 normally, 1 during PIE | PIE-only flag |

## UI surface (wxWidgets, statically linked)

- **wxRC/** — 5 XRC layout files (272 KB total):
  - `UnrealEd.xrc` — main editor dialogs, browsers, mode bars (~50 dialogs + ~6 panels)
  - `UnrealEdPhysics.xrc` — PhAT (Physics Asset Tool) dialogs
  - `UnrealEdRemoteControl.xrc` — single wxPanel `ID_RENDER_PAGE`, the in-game render-tuning HUD ("Render" tab with view-mode/SLOMO/FOV/STAT toggles). **Not a wire protocol** — purely an in-process wx panel.
  - `UnrealEdSGW.xrc` — **SGW-specific** (uses `IDBTN_*`/`IDCBO_*`/`IDLST_*` Hungarian prefixes vs. UE3's stock `ID_*`):
    - `ID_BROWSER_GAMEMAPEDITOR` — Game Map browser
    - `ID_DLG_WORLDMAPS` — World Maps editor
    - `ID_DLG_MAPLAYER` — Map Layer properties
    - `ID_DLG_NEW_WORLDMAP` — New World Map
    - `ID_DLG_RANDOMACTORSETTINGS` — Random Actor Scatter
  - `UnrealEdWizards.xrc` — New Terrain wizard

- **EditorRes/** — 52 toolbar bitmaps. SGW-custom: `SGW_QuadSel_*.bmp` (11 quad-selection tool variants for BigWorld chunk grid editing) and `SaveBigWorldChunks.bmp`.

- **wxRes/** — 708 bitmaps for sub-editors:
  - `ASV_*` AnimSet Viewer · `AnimTree_*` AnimTreeEditor · `CASC_*` Cascade (particles)
  - `CUR_*` CurveEditor · `KIS_*` / `UI_KIS_*` Kismet · `MAT_*` Matinee · `ME_*` Material Editor
  - `PhAT_*` Physics Asset Tool · `RAB_*` Reference Actor Browser
  - `LVT_*` Level Viewport Toolbar · `Btn_*` / `Vbtn_*` main toolbar
  - `TerrainEdit_*` (~55) Terrain brushes · `TerrainProp_*` Terrain layer browser
  - `UI_*` UI Scene Editor · `SCC_*` Source Control · `SCE_*` Sound Cue Editor
  - `Geom_*` Geometry sub-modes · `Prop_*` Property window · `RC*` RemoteControl HUD

## Exec command pipeline

UE3's three-tier text-console dispatcher, fully active in editor mode:

```text
APlayerController::Exec  (UE3 standard, calls into…)
    └── UEditorPlayer::Exec  @ 0x00D3E060
            handles: CloseEditorViewport (only)

UUnrealEdEngine::Exec  @ 0x00EDB0C0    ← engine-level, EDIT* commands
    ├── EDITDEFAULT CLASS=…    → open default-properties dialog
    ├── EDITOBJECT CLASS=… NAME=…
    └── EDITACTOR { TRACE | CLASS=… | NAME=… }

UEditorEngine::ExecBrush  @ 0x00BF5F30   ← engine-level, BRUSH commands (18 sub-commands)
    ├── BRUSH ADD                          → UnEdSrv__BRUSH_ADD @ 0x00BF7D40
    ├── BRUSH SUBTRACT
    ├── BRUSH ADDVOLUME [CLASS=…]          → FUN_00876970
    ├── BRUSH ADDMOVER
    ├── BRUSH FROM INTERSECTION
    ├── BRUSH FROM DEINTERSECTION
    ├── BRUSH IMPORT FILE=… [MERGE=…]
    ├── BRUSH EXPORT FILE=…
    ├── BRUSH LOAD FILE=…
    ├── BRUSH SAVE FILE=…
    ├── BRUSH NEW                          → UEditorEngine__unknown_00807b60
    ├── BRUSH SCALE
    ├── BRUSH MOVETO X=… Y=… Z=…
    ├── BRUSH MOVEREL X=… Y=… Z=…
    ├── BRUSH SET
    ├── BRUSH RESET
    ├── BRUSH MERGEPOLYS
    └── BRUSH SEPARATEPOLYS

UnEdSrv__HandleMapLoad  @ 0x00EF9780
    └── MAP LOAD FILE="…" [PLAYWORLD=1]

UnEdSrv__BRUSH_ADD  @ 0x00BF7D40       ← BRUSH ADD terminal handler

UEditorEngine::vfunc_102 (SpawnActor)  @ 0x00B78B30
    └── ACTOR ADD CLASS=… [SNAP=1]

UUnrealEdEngine::SaveDirtyPackages  @ 0x00FD7800
    └── interactive: wxFileDialog per dirty package, pass 0 (assets) then pass 1 (worlds)

UUnrealEdEngine::PromptAndSavePackage  @ 0x00FD6720
    └── OBJ SAVEPACKAGE PACKAGE="…" FILE="…" [SILENT=TRUE]
```

Sub-handlers documented at v5 standard in Ghidra; see [editor-source-mapping.md](../editor-source-mapping.md) for the full per-source-file map.

## SGW-specific extensions

### SGWPIEScriptManager (deterministic test-replay)

A singleton (`SGWPIEScriptManager` @ `0x00D3D270`) owning two `SGWPIEScripter` slots (0x38 bytes each). Subscribes to **8 CME events** — 7 routed through the main registry, plus `Event_Editor_FrameStart` registered by the manager's own initializer:

| CME event | Handler VA | Action | Source |
|---|---|---|---|
| `Event_Editor_FrameStart` | `0x00D3D060` | Tick both scripters | Manager's own init (not in main registry) |
| `Event_Editor_SetPIEScript1Active` | `0x00D3D250` | Set active_slot=0, reload | Main registry, factory `0x005C0280` |
| `Event_Editor_SetPIEScript2Active` | `0x00D3D260` | Set active_slot=1, reload | Main registry, factory `0x005C02F0` |
| `Event_Editor_PIEScriptLoad` | `0x00D3D070` (via thunk `0x005B7530`) | Load a PIE script from disk | Main registry, factory `0x005C0360` |
| `Event_Editor_PIEScriptStart` | `0x00D3D080` | Call `Start()` on active scripter | Main registry, factory `0x005C03D0` |
| `Event_Editor_PIEScriptTogglePause` | `0x00D3D090` | Toggle `bPaused` | Main registry, factory `0x005C0440` |
| `Event_Editor_PIEScriptStep` | `0x00D3D0B0` | Single-step on active scripter | Main registry, factory `0x005C04B0` |
| `Event_Editor_EndPIE` | `0x00D3D0C0` | Reset/cleanup on PIE exit | Manager's own init (not in main registry) |

**`SGWPIEScripter` layout (0x38 bytes):**

```text
+0x00  void* pSeqBegin       — KismetEntry array begin
+0x04  void* pSeqEnd         — KismetEntry array end (count = (end-begin)/12)
+0x2c  byte  bPaused
+0x2d  byte  bRunning
+0x30  float flAccumulatedTime
+0x34  int   nCurrentStep
```

**KismetEntry (0xC bytes):**

```text
+0x00  UObject* pActor
+0x04  int      nParam
+0x08  float    flTimeThreshold
```

This is a deterministic-replay infrastructure for editor PIE sessions — load a script file, compile to timed `(actor, event, threshold)` triples, replay them against `flAccumulatedTime`. The Cimmeria wireclient (`crates/wireclient/`) is a server-side analog; SGWPIEScript is the client-side replay.

### SGWHomeless (editor dev-tool catch-all)

A **non-polymorphic** C++ singleton — plain struct, no vtable, no inheritance. It exists purely to route editor UI signals through UE3's Exec dispatch chain. The name "homeless" was the developers' term for events that had no dedicated screen-level handler — SGWHomeless bundles them all.

**Singleton accessor** `SGWHomeless__GetSingleton` @ `0x00D40280`: lazy-init pattern matching `SGWPIEScriptManager__GetSingleton`. Init flag at `0x01EF2400` (bit 0); singleton storage at `0x01EF23FC`. `onexit` cleanup registered via `FUN_012375CB(&LAB_017E3480)`.

**Constructor** `SGWHomeless__ctor` @ `0x00D3FFE0`: branches on `g_dwMapCheckDepEnabled`.

- **Zero branch (retail mode):** registers 3 base events + 3 `Event_Option_*` stubs.
- **Non-zero branch (editor mode):** also registers `Event_Editor_BeginPIE`/`Event_Editor_EndPIE` handlers that defer the 22-event registration to PIE-active state.

**22-event editor registration helper** @ `0x00D3EFB0`: sequential CME subscription of 22 editor events, each via a subscription wrapper in `0x00D41DD0`–`0x00D429A0` calling a `MemberCallback` constructor in `0x00D40AD0`–`0x00D41550`. All Pattern A (`NoSubject`). Every handler uses the identical dispatch pattern:

```c
FUN_0041AAB0(local, L"<command>");                              // build FString cmd
(**(code**)(GWorld->ViewportArray[0]->vtable[0x10C]))();        // dispatch to viewport Exec
```

Entry 22 (`Event_Editor_ToggleCombat`) deviates — it calls the Flash/Scaleform external window module via `g_pFlashExternalWindowModule` (`0x01EE1254`) instead.

**Time-of-day handler** `SGWHomeless__OnTimeOfDay` @ `0x00D3FCF0`: subscribed to `Event_NetIn_onTimeofDay`. Reads `Time` (float), `Wind` (float), and `Weather` (bool) fields from the event payload via `FUN_00E3CC20` / `FUN_00D434D0`, writes them to `GLevel+0x384`, `+0x388`, `+0x38c` respectively.

**Subscription totals: 30 CME events** — 8 base (registered in retail) + 22 editor-viewport (registered when `g_dwMapCheckDepEnabled != 0`). Of those, 26 belong to the editor scope (23 `Event_Editor_*` + 3 `Event_Option_*` matching mode/resolution); the remaining 4 are `Event_NetIn_*` (one — onTimeofDay) and `Event_Option_*` server/UI stubs.

**Cimmeria server relevance:** Only `Event_NetIn_onTimeofDay` is server-emittable. The 22 editor events have no server-side meaning. The `g_dwMapCheckDepEnabled` flag is zero in retail mode, so most editor subscriptions never wake in a production client.

**Open sub-questions:**
1. SGWHomeless's struct size is unclear — the constructor adds no fields, suggesting a zero-size or 4-byte handle struct.
2. `Event_Editor_EndPIE` re-registers all 22 subscriptions — intentional reset or bug where subscriptions accumulate?
3. The shared handler for `Event_Option_Resolution/DevWindowedMode/WindowedMode` — distinct events or compiler code-dedup?

### BigWorld chunked sublevels

The streaming sublevel format is `<base_path>\<map_name>-<X_hex><Z_hex>.umap` (format string at `0x019F54D4` and `0x019F558C`). X/Z are 4-digit zero-padded hex grid coordinates. Two loaders:

- `UGameEngine__LoadBigWorldSublevelsFromManifest` @ `0x00ED1B40` — reads pipe-delimited manifest from a `GetTempPath()` file with columns `MapName|BasePath|SubLevelSuffix|xStart|xEnd|zStart|zEnd`. Loads the master `.umap`, then loops `(z, x)` grid coordinates, accumulating streaming levels in `GLevel.StreamingLevels` array at `GLevel+0x260/+0x264/+0x268`.
- `UGameEngine__LoadBigWorldSublevelsWithActor` @ `0x00ED2350` — adds actor-registration context for PIE.

The editor's **SaveBigWorldChunks** toolbar button (command id `0x6774`, registered in `WxMainToolBar::Create` @ `0x01137590`) is handled by `WxMainFrame__OnSaveBigWorldChunks` @ `0x00FEDE90`. Its wxEventTable entry lives at `0x01EF8C94`, populated by `WxMainFrame__InitEventTable` @ `0x017C5C50` (a 9 KB static-init function).

The handler is best described as a **BigWorld streaming-sublevel save with navgen data flush**:

1. SEH frame setup; allocate `abSaveContext[20]` on stack via `FUN_00EFBD70`.
2. Resolve current package name from `GLevel+0x50` into a small-buffer FString.
3. Begin slow-task UI with label `L"Big world navgen export"` (string at `0x01A3F204`) via `GWarn->vtable[0x14]`.
4. Dispatch into `FUN_00EFB650(this, &name)`:
   - Calls `UGameEngine::FindObject(NULL, name, 0)` to resolve the package name to a `ULevel*`.
   - **Path A** (`ULevel->ObjectFlags & 0x20000` set): full navgen pipeline (`FUN_00EFB290`) — loads `TheWorld`, iterates `GWorld->StreamingLevels` array at `+0x260`/`+0x264`, XOR-patches sublevel visibility bits at `+0x60`, calls `FUN_00EFAAA0` + `FUN_00EFAB40` per sublevel to serialize and save.
   - **Path B** (flag clear): iterates `g_pPackageArrayBase` (`0x01EDC69C`, count at `0x01EDC6A0`), and for each dirty package calls `FUN_01172780(ctx, pkg, 0)` — the package serializer.
5. End slow task via `GWarn->vtable[0x18]`; teardown SEH frame.

**Key difference from stock UE3 save:** does NOT call `UUnrealEdEngine__PromptAndSavePackage @ 0x00FD6720`. Saves silently using the package's existing filename; no `wxFileDialog`, no CME notification on completion. The "navgen export" label is intentional — saving BigWorld chunks necessarily flushes their navmesh data, which is the dominant time cost.

**Related BigWorld toolbar commands** in the `0x676E`–`0x6779` event-table cluster (all `WxMainFrame`):

| Cmd | Handler | Inferred name |
|---|---|---|
| `0x676E`–`0x6770` | `LAB_00FE9BE0`–`LAB_00FE9C20` | Unknown BigWorld cmds (3) |
| `0x6771` | `FUN_00FE9CC0` | BigWorld build/export wizard (creates `wxWizard` at `FUN_01132690`) |
| **`0x6774`** | **`WxMainFrame__OnSaveBigWorldChunks`** | **SaveBigWorldChunks (this section)** |
| `0x6775` | `LAB_00FEB960` | Unknown BigWorld cmd |
| `0x6776` | `FUN_00FF6770` | Unknown BigWorld cmd |
| `0x6777` | `FUN_00FF6420` | Unknown BigWorld cmd |
| `0x6779` | `LAB_00FE9D30` | Combo-box selection handler |

IDs `0x6772` and `0x6773` are absent from the event table — toolbar slots cut or never implemented.

## CME editor event catalog

`CMERegistry__RegisterAllEventEmitHandlers @ 0x005C75D0` is a 22 KB monolith registering ~325 `TypedEmitInfo`/`CallbackImpl` factory pairs into the global event-emitter registry. Of those, **37 are editor-relevant**: 29 `Event_Editor_*`, 7 `Event_Option_*` in the main monolith, and 1 `Event_Option_ShowButtonBinds` late-bound elsewhere. Index values for this contiguous block run `0x21`–`0x44` in the BST insertion order.

### Event_Option_ group (8 events)

| Event name | String VA | Factory VA | Handler thunk | Subscriber | Purpose |
|---|---|---|---|---|---|
| `Event_Option_MasterVolume` | `0x01840B00` | `0x005BF560` | `0x005B6730` | `SGWAudioDevice` | Master audio volume changed |
| `Event_Option_DevWindowedMode` | `0x01840B1C` | `0x005BF5D0` | `0x005B67A0` | `SGWHomeless` | Dev-mode windowed toggle |
| `Event_Option_WindowedMode` | `0x01840B3C` | `0x005BF640` | `0x005B6810` | `SGWHomeless` | Full windowed-mode toggle |
| `Event_Option_Resolution` | `0x01840B58` | `0x005BF6B0` | `0x005B6880` | `SGWHomeless` | Screen resolution change |
| `Event_Option_CamOptionChanged` | `0x01840B70` | `0x005BF720` | `0x005B68F0` | `ASGWCamera_Player` | Camera settings changed |
| `Event_Option_MusicVolume` | `0x01840B90` | `0x005BF790` | `0x005B6960` | `SGWAudioDevice` | Music channel volume changed |
| `Event_Option_Rendering` | `0x01840BAC` | `0x005BF800` | `0x005B69D0` | `RenderThreadOptionManager` | Rendering quality settings changed |
| `Event_Option_ShowButtonBinds` | `0x019CB648` | *(late-bound @ `0x00DC7F30`)* | — | `SGWScriptedWindow` | Key-binding overlay; not in main monolith |

### Event_Editor_ group (29 events)

| Event name | Factory VA | Handler thunk | Subscriber | Purpose |
|---|---|---|---|---|
| `Event_Editor_TestSequence` | `0x005BF870` | `0x005B6A40` | `SGWHomeless` | Trigger test sequence from editor |
| `Event_Editor_Close` | `0x005BF8E0` | `0x005B6AB0` | `SGWHomeless` | Close the in-game editor |
| `Event_Editor_SequenceBegin` | `0x005BF950` | `0x005B6B20` | `SGWHomeless` | Sequence playback begin |
| `Event_Editor_SequenceInterrupt` | `0x005BF9C0` | `0x005B6B90` | `SGWHomeless` | Sequence playback interrupt |
| `Event_Editor_SequenceEnd` | `0x005BFA30` | `0x005B6C00` | `SGWHomeless` | Sequence playback end |
| `Event_Editor_TogglePhysicsMode` | `0x005BFAA0` | `0x005B6C70` | `SGWHomeless` | Toggle physics simulation in editor |
| `Event_Editor_ViewWireframe` | `0x005BFB10` | `0x005B6CE0` | `SGWHomeless` | Switch viewport to wireframe |
| `Event_Editor_ViewUnlit` | `0x005BFB80` | `0x005B6D50` | `SGWHomeless` | Switch viewport to unlit |
| `Event_Editor_ViewLit` | `0x005BFBF0` | `0x005B6DC0` | `SGWHomeless` | Switch viewport to lit |
| `Event_Editor_ShowPerformance` | `0x005BFC60` | `0x005B6E30` | `SGWHomeless` | Toggle perf counters overlay |
| `Event_Editor_ShowFPS` | `0x005BFCD0` | `0x005B6EA0` | `SGWHomeless` | Toggle FPS counter |
| `Event_Editor_ScreenShot` | `0x005BFD40` | `0x005B6F10` | `SGWHomeless` | Capture screenshot |
| `Event_Editor_ShadowStats` | `0x005BFDB0` | `0x005B6F80` | `SGWHomeless` | Toggle shadow rendering stats |
| `Event_Editor_CameraDefault` | `0x005BFE20` | `0x005B6FF0` | `SGWHomeless` | Restore default editor camera |
| `Event_Editor_Camera1stPerson` | `0x005BFE90` | `0x005B7060` | `SGWHomeless` | First-person camera mode |
| `Event_Editor_Camera3rdPerson` | `0x005BFF00` | `0x005B70D0` | `SGWHomeless` | Third-person camera mode |
| `Event_Editor_CameraFixed` | `0x005BFF70` | `0x005B7140` | `SGWHomeless` | Fixed-position camera |
| `Event_Editor_CameraFixedTracking` | `0x005BFFE0` | `0x005B71B0` | `SGWHomeless` | Fixed camera with subject tracking |
| `Event_Editor_CameraFree` | `0x005C0050` | `0x005B7220` | `SGWHomeless` | Free-fly camera mode |
| `Event_Editor_Ghost` | `0x005C00C0` | `0x005B7290` | `SGWHomeless` | Ghost/noclip movement |
| `Event_Editor_Walk` | `0x005C0130` | `0x005B7300` | `SGWHomeless` | Normal walk movement |
| `Event_Editor_Use` | `0x005C01A0` | `0x005B7370` | `SGWHomeless` | Interact/use command |
| `Event_Editor_ToggleCombat` | `0x005C0210` | `0x005B73E0` | `SGWHomeless` | Toggle combat mode (dispatches to Flash, not Exec) |
| `Event_Editor_SetPIEScript1Active` | `0x005C0280` | `0x005B7450` → `0x00D3D250` | `SGWPIEScriptManager` | Activate PIE script slot 1 |
| `Event_Editor_SetPIEScript2Active` | `0x005C02F0` | `0x005B74C0` → `0x00D3D260` | `SGWPIEScriptManager` | Activate PIE script slot 2 |
| `Event_Editor_PIEScriptLoad` | `0x005C0360` | `0x005B7530` → `0x00D3D070` | `SGWPIEScriptManager` | Load PIE script from disk |
| `Event_Editor_PIEScriptStart` | `0x005C03D0` | `0x005B75A0` → `0x00D3D080` | `SGWPIEScriptManager` | Start PIE script execution |
| `Event_Editor_PIEScriptTogglePause` | `0x005C0440` | `0x005B7610` → `0x00D3D090` | `SGWPIEScriptManager` | Pause/resume PIE script |
| `Event_Editor_PIEScriptStep` | `0x005C04B0` | `0x005B7680` → `0x00D3D0B0` | `SGWPIEScriptManager` | Single-step PIE script |

The handler thunks at `0x005B6xxx`–`0x005B7xxx` are `__thiscall` thunks; the SGWPIEScriptManager entries dispatch through the thunk to the actual method VA on the manager.

### Events registered outside the main monolith

`Event_Editor_BeginPIE`, `Event_Editor_EndPIE`, and `Event_Editor_FrameStart` are registered by `SGWPIEScriptManager`'s own constructor against UE3 editor-lifecycle emissions — they are NOT in the 325-entry `CMERegistry__RegisterAllEventEmitHandlers` table.

### Subscriber breakdown

| Class | Editor events owned |
|---|---|
| `SGWHomeless` | 26 (23 `Event_Editor_*` + 3 `Event_Option_*` mode/resolution) |
| `SGWPIEScriptManager` | 9 (6 in main monolith + 3 lifecycle outside) |
| `SGWAudioDevice` | 2 (`MasterVolume`, `MusicVolume`) |
| `RenderThreadOptionManager` | 1 (`Rendering`) |
| `ASGWCamera_Player` | 1 (`CamOptionChanged`) |
| `SGWScriptedWindow` | 1 (`ShowButtonBinds`, late-bound) |

## Persistence pipeline

- **Save (dirty packages):** `UUnrealEdEngine__SaveDirtyPackages` @ `0x00FD7800` → 2-pass walk of all `UPackage` objects (pass 0 assets, pass 1 worlds) → `wxFileDialog` per dirty package → `UUnrealEdEngine__PromptAndSavePackage` @ `0x00FD6720` → `UPackage::Save` (`FUN_00EF8840`) → disk.
- **Save (single):** `OBJ SAVEPACKAGE PACKAGE="…" FILE="…" [SILENT=TRUE]` → `UUnrealEdEngine__PromptAndSavePackage` (silent path skips dialog).
- **Load:** `MAP LOAD FILE="…"` → `UnEdSrv__HandleMapLoad` @ `0x00EF9780` → flush current world → `LoadPackage` → `CreateWorld` → set `GWorld` → fire `Event_Editor_PostLoadMap` CME event.
- **File manager:** `FFileManagerWindows` vtable at `0x017F9214` — 20 slots; `vfunc_10` @ `0x004C1E50` is `FindUniqueFilename` (Atrea hook target — generates `<base>0000`, `<base>0001`, … until unused name found).

## Apocryphal docs to retire

The following predate this finding and have been audited against the binary:

| Doc | Verdict | Action |
|---|---|---|
| [docs/technical/atrealoader-exe.md](../../technical/atrealoader-exe.md) | REVISED 2026-05-25 | Filename corrected (`Atera` → `Atrea`); `--fix-aslr` claim rewritten to "one byte at file offset `0x186` flips, only `DYNAMIC_BASE` bit cleared" per mercury-wire-format §S9; Ghidra anchors added with caveat (AtreaLoader.exe not loadable in active Ghidra session — language version skew). |
| [docs/technical/atrealoader-config.md](../../technical/atrealoader-config.md) | REVISED 2026-05-25 | Patch count corrected (`19` → `18`); `AtreaEditor.bat` `-SHOWLOG` flag removed; `ConsoleStdHandle` analysis redone with correct `STD_OUTPUT_HANDLE = -11` arithmetic; XML-vs-binary-config caveat added; `--fix-aslr` prerequisite warning added; Ghidra-anchored Wave 2 patch table integrated. |
| [docs/technical/atrearl-loader.md](../../technical/atrearl-loader.md) | REVISED 2026-05-25 | Symbol-hook count corrected (`10` → `13`); sniffer init replaced (`FUN_1002CE00` → `FUN_10026F30`/`FUN_10021FB0` per mercury-wire-format §S9); "Login redirect" claim removed (lives in `Login.lua`); XML-vs-binary-config distinction added; speculative source filenames removed; all internal addresses flagged speculative pending Ghidra re-verification. |

All three miss the editor entirely. The editor archaeology lives here and in [editor-source-mapping.md](../editor-source-mapping.md).

## Open follow-ups

1. ~~**SGWHomeless constructor**~~ — **Resolved 2026-05-25.** `SGWHomeless__GetSingleton` @ `0x00D40280`, `SGWHomeless__ctor` @ `0x00D3FFE0`. Singleton at `0x01EF23FC`. See §SGWHomeless above.
2. ~~**SaveBigWorldChunks EVT_MENU handler**~~ — **Resolved 2026-05-25.** `WxMainFrame__OnSaveBigWorldChunks` @ `0x00FEDE90`. See §BigWorld chunked sublevels above.
3. **`SaveDirtyPackages` follow-up v5 passes** — `UUnrealEdEngine__PromptAndSavePackage` (30.1% effective) and `UUnrealEdEngine__SavePackageFromContext` (28.2%) have fixable deductions remaining.
4. **MCP bridge implementation** — see [atrea-editor-bridge.md](../../architecture/atrea-editor-bridge.md) Phase A.
5. **`g_pEditorPkgCtx` struct layout** — `+0x58` is FString ptr, `+0x5c` is FString data, but the full struct is undocumented.
6. ~~**CME event registration table**~~ — **Editor scope resolved 2026-05-25.** 37 editor-relevant events catalogued in §CME editor event catalog. The remaining ~288 non-editor entries in the 325-entry monolith are outside the editor scope and left undocumented here.
7. **Handler thunk cluster naming** — the `0x005B6730`–`0x005B7680` range contains one handler thunk per editor event; none renamed yet. A v5 pass would apply `MemberCallback_<EventName>` style names.
8. **Sub-handlers under `UEditorEngine::ExecBrush`** — 8 follow-up sub-handlers identified (BRUSH ADDVOLUME spawner `FUN_00876970`, BRUSH IMPORT `FUN_00C03830`, BRUSH EXPORT `UEditorEngine__unknown_00502490`, `FDebugToolExec__unknown_00EDAFD0` properties dialog, etc.). See Wave 2 ExecBrush report.

## Cross-references

- Per-function VA-to-source map: [editor-source-mapping.md](../editor-source-mapping.md)
- MCP bridge architecture: [atrea-editor-bridge.md](../../architecture/atrea-editor-bridge.md)
- Atrea patch config (source of truth): `binaries/AtreaLoader.config.xml`
- Atrea injector analysis: [docs/technical/atrealoader-exe.md](../../technical/atrealoader-exe.md) (revised 2026-05-25 — launcher-only scope)
- Atrea patch table: [docs/technical/atrealoader-config.md](../../technical/atrealoader-config.md) (revised 2026-05-25 — Ghidra-anchored Editor-group patches)
- Atrea sniffer/patcher: [docs/technical/atrearl-loader.md](../../technical/atrearl-loader.md) (revised 2026-05-25 — AtreaRL.dll runtime hooks)
- Mercury sniffer + ASLR breakage: [docs/drafts/spec/mercury-wire-format.md §S9](../../drafts/spec/mercury-wire-format.md)
- CME event system: [docs/reverse-engineering/findings/cme-event-signal.md](cme-event-signal.md)
