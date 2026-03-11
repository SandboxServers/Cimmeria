# Editor Ghidra Map — SGW.exe Function ↔ Reference Source

> **Last updated**: 2026-03-08
> **Reference source**: `Reference/UE3-2004/Development/Src/` (Editor/, UnrealEd/, Launch/)
> **Binary**: SGW.exe (32-bit x86 PE, MSVC)

This document maps functions identified in SGW.exe (via Ghidra) to their corresponding UE3 reference source files. Functions were identified by string cross-references, decompilation comparison, and debug symbol analysis.

**Naming convention**: `(label)` suffix means the address was inside an existing larger function in Ghidra and was annotated as a label rather than a standalone function rename.

---

## Launch/Src/Launch.cpp + LaunchEngineLoop.cpp
| VA | Ghidra Name | Reference Function | Notes |
|----|-------------|-------------------|-------|
| 0x00416010 | GuardedMain | WinMain/__try block | Calls FEngineLoop::Init, wxEntry() |
| 0x004185E0 | FEngineLoop__Init | FEngineLoop::Init() | GIs* flags, engine class load, GEngine->Init() |
| 0x00417FE0 | EngineInitLoop | FEngineLoop::Init() (engine create section) | UGameEngine vs UUnrealEdEngine, Login_Map check |
| 0x00416EC0 | FEngineLoop__Tick (label) | FEngineLoop::Tick() | PIE guard: GIsEditor && GEditor+0x52c |

## Editor/Src/UnEditor.cpp
| VA | Ghidra Name | Reference Function | Notes |
|----|-------------|-------------------|-------|
| 0x00B22E60 | UEditorEngine__Init | UEditorEngine::Init() | Sets GEditor, loads edit pkgs, creates client |
| 0x00B232E0 | UEditorEngine__Tick (label) | UEditorEngine::Tick() | Large, many asserts |
| 0x00B22AE0 | UEditorEngine__PreviewAudioComponent (label) | — | SGW-specific, audio preview |
| 0x00B26010 | UEditorEngine__Cleanse (label) | UEditorEngine::Cleanse() | GC, trans reset, flush |
| 0x00B21E50 | UEditorEngine__ClearComponents (label) | UEditorEngine::ClearComponents() | |
| 0x00B21F80 | UEditorEngine__UpdateComponents (label) | UEditorEngine::UpdateComponents() | |
| 0x00B20CC0 | UEditorEngine__Serialize (label) | UEditorEngine::Serialize() | |
| 0x00B214E0 | UEditorEngine__Destroy (label) | UEditorEngine::Destroy() | |
| 0x00B21970 | UEditorEngine__InitEditor (label) | UEditorEngine::InitEditor() | UEngine::Init, topics, TempModel |
| 0x00B20EC0 | UEditorEngine__PlayMap_Enter | PIE Enter | GIsPlayInEditorWorld |
| 0x00B20F10 | UEditorEngine__EndPlayMap_Exit | PIE Exit | Resets GWorld |

## Editor/Src/PlayLevel.cpp
| VA | Ghidra Name | Reference Function | Notes |
|----|-------------|-------------------|-------|
| 0x00BE37C0 | UEditorEngine__GetMapPath (label) | — | Gets MapPath from config |
| 0x00BE43D0 | UEditorEngine__EndPlayMap (label) | UEditorEngine::EndPlayMap() | Event_Editor_EndPIE |
| 0x00BE4770 | UEditorEngine__CreateEditorPlayer (label) | — | Creates UEditorPlayer |
| 0x00D3E060 | UEditorPlayer__Exec (label) | UEditorPlayer::Exec() | CloseEditorViewport handler |

## UnrealEd/Src/UnrealEdEngine.cpp
| VA | Ghidra Name | Reference Function | Notes |
|----|-------------|-------------------|-------|
| 0x00ECF5B0 | UUnrealEdEngine__UUnrealEdEngine | Constructor | Size 0x5D8, vtable 0x019F4E6C |
| 0x00FD6510 | UUnrealEdEngine__Init | UUnrealEdEngine::Init() vtable[68] | Calls parent, loads extra pkgs |
| 0x00EDB0C0 | UUnrealEdEngine__Exec (label) | UUnrealEdEngine::Exec() | EDITDEFAULT/EDITOBJECT/EDITACTOR |
| 0x00EDAFD0 | UUnrealEdEngine__ShowClassProperties (label) | ShowClassProperties() | |
| 0x00FD6DE0 | UUnrealEdEngine__UpdateStatusBar (label) | — | Status bar pos/rot/scale |
| 0x00FD7800 | UUnrealEdEngine__SaveDirtyPackages (label) | SaveDirtyPackages() | Interactive, file dialog |
| 0x00FD87E0 | UUnrealEdEngine__GetUnrealEdOptionsClass (label) | — | |
| 0x00FD8710 | UUnrealEdEngine__GetUnrealEdKeyBindingsClass (label) | — | |
| 0x00ECDF60 | UUnrealEdEngine__StaticClass (label) | StaticClass() | |

## UnrealEd/Src/New/UnrealEdApp.cpp
| VA | Ghidra Name | Reference Function | Notes |
|----|-------------|-------------------|-------|
| 0x00ED4530 | WxUnrealEdApp__OnInit | WxUnrealEdApp::OnInit() | 894 lines (ref=161) |

## UnrealEd/Src/New/FCallbackDeviceEditor.cpp
| VA | Ghidra Name | Reference Function | Notes |
|----|-------------|-------------------|-------|
| 0x00ECF7E0 | FCallbackDeviceEditor__Send_PIE (label) | Send(ECallbackType) | PIE enter/exit dispatch |
| 0x00ECF910 | FCallbackDeviceEditor__Send_Query (label) | Send(ECallbackType,INT) | Resource query |
| 0x00ECF960 | FCallbackDeviceEditor__Send_Route (label) | Send(ECallbackType) | String routing |

## UnrealEd/Src/New/EditorFrame.cpp
| VA | Ghidra Name | Reference Function | Notes |
|----|-------------|-------------------|-------|
| 0x00ED04E0 | WxEditorFrame__Create (label) | — | Factory, wxCreateDynamicObject |
| 0x00FED580 | WxEditorFrame__SerializeOptions (label) | — | Load FramePos/Size from config |
| 0x00FF3560 | WxEditorFrame__ViewportConfig (label) | — | EditorFrame.cpp source refs |
| 0x0100BC20 | WxEditorFrame__AutosaveUI (label) | — | 842 lines, autosave bitmaps |
| 0x0100E6D0 | WxEditorFrame__OnTimer_Autosave (label) | WEditorFrame::OnTimer() | MAYBEAUTOSAVE equivalent |
| 0x00FF91C0 | WxMainMenu__ctor (label) | — | 2719 lines! |

## Editor/Src/UnEdSrv.cpp (editor commands)
| VA | Ghidra Name | Notes |
|----|-------------|-------|
| 0x00EF9780 | UnEdSrv__MAP_LOAD (label) | MAP LOAD FILE handler |
| 0x00BF7D40 | UnEdSrv__BRUSH_ADD (label) | BRUSH ADD command |
| 0x00BF5F30 | UEditorEngine__SafeExec (label) | Large dispatcher (Brush Add/Subtract/AddVolume) |

## Editor/Src/UnEdCsg.cpp (BSP CSG)
| VA | Ghidra Name | Notes |
|----|-------------|-------|
| 0x00C06BC0 | bspRebuildGeometry (label) | RebuildingGeometry string |
| 0x00C0CA30 | bspGeometryOptimization (label) | |
| 0x00BF7470 | bspRebuildOptimizing (label) | RebuildBSPOptimizingGeometry |
| 0x00BF3450 | bspRebuildingGeometry (label) | |
| 0x00C05CC0-0x00C09740 | UnEdCsg__helper/operation_1-4 (labels) | CSG sub-functions |

## Editor/Src/UnEdExp.cpp (T3D export)
| VA | Ghidra Name | Notes |
|----|-------------|-------|
| 0x00BDB720 | UnEdExp__ExportMap_1 (label) | "Begin Map" header |
| 0x00BE2200 | UnEdExp__ExportMap_2 (label) | Second variant |
| 0x00BD9300 | UnEdExp__ExportActor (label) | "Begin Actor" |

## Editor/Src/UnPrefab.cpp
| VA | Ghidra Name | Notes |
|----|-------------|-------|
| 0x005CDA60 | UPrefab__func_1 (label) | |
| 0x005CD140 | UPrefab__Validate (label) | |
| 0x005CE7E0 | UPrefab__Serialize_large (label) | |
| 0x005CFCF0 | UPrefab__ValidateBrush (label) | NULL model check |
| 0x00C00DE0 | UnEdPrefab__PREFAB_cmd (label) | PREFAB/SELECTACTORSINPREFABS |
| 0x00FE21A0 | UnEdPrefab__Create (label) | |
| 0x00FE2260 | UnEdPrefab__AddPrefab (label) | |
| 0x00FE27D0 | UnEdPrefab__ToNormalActors (label) | |
| 0x00FE46A0 | UnEdPrefab__UpdateResetPrefab (label) | |

## Editor/Src/UnEditor.cpp (MODE command)
| VA | Ghidra Name | Notes |
|----|-------------|-------|
| 0x00FE6820 | UEditorEngine__Exec_MODE (label) | MODE MAPEXT/GRID/SPEED/etc |

## Package Save Functions
| VA | Ghidra Name | Notes |
|----|-------------|-------|
| 0x00F2B8E0 | SavePackage_ParticleSystem (label) | OBJ SAVEPACKAGE |
| 0x00FAABC0 | SavePackage_handler_2 (label) | |
| 0x00FD6720 | SavePackage_Silent (label) | SILENT=TRUE |
| 0x01044D80 | SavePackage_Interactive_1 (label) | With file dialog |
| 0x0103F140 | SavePackage_Interactive_2 (label) | |
| 0x00BB1030 | SaveDirtyPackages_Commandlet (label) | OnlySaveDirtyPackages |
| 0x00FE6180 | TopicHandler__BROWSECLASS (label) | |

## Build System
| VA | Ghidra Name | Notes |
|----|-------------|-------|
| 0x01137590 | BuildGeometryAndLighting (label) | Very large function |
| 0x00FEA070 | MenuBrushCSG_AddSubtract (label) | |
| 0x01000FC0 | MenuBrushAdd_Flags (label) | |
| 0x00FF4D60 | MenuBrushAddVolume (label) | |

## Core Assertion Handler
| VA | Ghidra Name | Notes |
|----|-------------|-------|
| 0x00486000 | appFailAssert | cdecl(expr, file, line) |

## Global Data
| VA | Ghidra Name | Notes |
|----|-------------|-------|
| 0x01EF2E74 | GApp | WxUnrealEdApp* (EditorFrame at +0x58) |
| 0x01EE2684 | GWorld | UWorld* |
| 0x01EE1254 | GEngine | UEngine* |
| 0x01EF134C | GEditor | UEditorEngine* |
| 0x01EAD7AC | GIsEditor | UBOOL |
| 0x01EAD7B0 | GIsUCC | UBOOL |
| 0x01EAD7BC | GIsClient | UBOOL |
| 0x01EAD7C0 | GIsServer | UBOOL |
| 0x01EB0830 | GIsGame | UBOOL |
| 0x01EAD7EC | GBatchModeFlag | UBOOL |
| 0x01ECADE0 | FName__GNames | TArray<FNameEntry*>* |
| 0x01EDC69C | UObject__GObjObjects | TArray<UObject*>* |
| 0x01EF244C | g_EntityManager | EntityManager* |
| 0x019F4E6C | UUnrealEdEngine__vtable | vtable pointer |
