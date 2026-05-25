---
name: sgwhomeless-full-recovery
description: Complete v5 RE recovery of SGWHomeless: singleton addresses, 30 CME subscriptions, handler VAs, architectural findings, Ghidra renames applied 2026-05-25
metadata:
  type: project
---

> [!NOTE] PROMOTION TARGET: docs/reverse-engineering/findings/atrea-editor.md §SGWHomeless
>
> Full v5 documentation complete 2026-05-25. Supersedes and CORRECTS the singleton address in [[cme-anomalies-resolved]].

## Corrected singleton address

Prior memory `cme-anomalies-resolved.md` claimed singleton at `DAT_01ef2380`. **WRONG.**
- Correct singleton storage: `g_pSGWHomeless` @ `0x01ef23fc` (type: `void *`)
- Init flag: `g_dwSGWHomelessInitFlag` @ `0x01ef2400` (type: `uint`, bit 0 = initialized)
- `DAT_01ef2380` is `SGWPIEScriptManager`'s singleton (different class, adjacent address)

## Key function addresses (all renamed in Ghidra 2026-05-25)

| Ghidra name | Address | Description |
|---|---|---|
| `SGWHomeless__GetSingleton` | `0x00d40280` | Lazy singleton accessor; returns `&g_pSGWHomeless` |
| `SGWHomeless__ctor` | `0x00d3ffe0` | Constructor; registers CME subscriptions based on `g_dwMapCheckDepEnabled` |
| `SGWHomeless__RegisterEditorEventSubscriptions` | `0x00d3efb0` | Registers 22 editor-viewport CME subscriptions; called on BeginPIE or direct |
| `SGWHomeless__RegisterEditorModeAndSubscriptions` | `0x00d3f220` | Registers "editor" mode string + 22-event set; called on EndPIE |
| `SGWHomeless__OnEditorTestSequenceOrOptionChange` | `0x00d3f690` | Entity-name lookup + actor iterator; handles TestSequence + 3 Option events |
| `SGWHomeless__OnSlashCmdTestSequence` | `0x00d3f4a0` | GameBeing name lookup + mount/spawn |
| `SGWHomeless__OnNetTimeOfDay` | `0x00d3fcf0` | Writes ToD/Wind/Weather to GLevel +0x384/+0x388/+0x38c |
| `SGWHomeless__OnOptionResolution` | `0x00d3fcc0` | Tail-call stub → `SGWHomeless__OnEditorTestSequenceOrOptionChange` |
| `SGWHomeless__OnOptionDevWindowedMode` | `0x00d3fcd0` | Tail-call stub → same |
| `SGWHomeless__OnOptionWindowedMode` | `0x00d3fce0` | Tail-call stub → same |
| `SGWHomeless__OnEditorToggleCombat_FlashDispatch` | `0x00d3ec30` | Flash/Scaleform external window call (not a UE3 Exec command) |

## Globals renamed

| Address | Name | Type |
|---|---|---|
| `0x01ef23fc` | `g_pSGWHomeless` | `void *` |
| `0x01ef2400` | `g_dwSGWHomelessInitFlag` | `uint` |
| `0x01ee1254` | `g_pFlashExternalWindowModule` | `void *` |

## Architecture summary

- SGWHomeless is NOT polymorphic: no vtable, no COL. RTTI string `.?AVSGWHomeless@@` at `0x01DE97DC` (type_info only).
- All 30 CME subscriptions use Pattern A (NoSubject MemberCallback).
- MemberCallback ctor cluster: `0x00d40ad0`–`0x00d41550` (30 ctors, 0x80 spacing approx).
  - Editor subset: `0x00d40ad0`–`0x00d41550` (22 ctors)
  - Base/extra subset: `0x00d406d0`–`0x00d40a50` (8 ctors)
- Subscription wrapper cluster: `0x00d415d0`–`0x00d429a0` (30 wrappers, 0x80-0x90 spacing).

## Base subscriptions (8, from SGWHomeless__ctor)

| CME Event | Condition | Handler | Purpose |
|---|---|---|---|
| `Event_Editor_TestSequence` | always | `0x00d3f690` | Entity name+action lookup |
| `Event_SlashCmd_TestSequence` | `g_dwMapCheckDepEnabled == 0` | `0x00d3f4a0` | GameBeing spawn |
| `Event_NetIn_onTimeofDay` | `g_dwMapCheckDepEnabled == 0` | `0x00d3fcf0` | GLevel ToD/Wind/Weather sync |
| `Event_Editor_BeginPIE` | `g_dwMapCheckDepEnabled != 0` | `0x00d3efb0` | Deferred 22-event reg |
| `Event_Editor_EndPIE` | `g_dwMapCheckDepEnabled != 0` | `0x00d3f220` | Re-reg + mode string |
| `Event_Option_Resolution` | always | `0x00d3fcc0` | → `0x00d3f690` |
| `Event_Option_DevWindowedMode` | always | `0x00d3fcd0` | → `0x00d3f690` |
| `Event_Option_WindowedMode` | always | `0x00d3fce0` | → `0x00d3f690` |

## Editor subscriptions (22, from RegisterEditorEventSubscriptions)

All dispatch via `GWorld->ViewportArray[0]->vtable[0x10c]` (UE3 Exec), except entry 22.

| # | CME Event | Handler VA | UE3 Exec command |
|---|---|---|---|
| 1 | `Event_Editor_Close` | `0x00d3e060` | `"CloseEditorViewport"` |
| 2 | `Event_Editor_SequenceBegin` | `0x00d3e0f0` | `"TestSequence Begin"` |
| 3 | `Event_Editor_SequenceInterrupt` | `0x00d3e180` | `"TestSequence Interrupt"` |
| 4 | `Event_Editor_SequenceEnd` | `0x00d3e210` | `"TestSequence End"` |
| 5 | `Event_Editor_TogglePhysicsMode` | `0x00d3e2a0` | `"TogglePhysicsMode"` |
| 6 | `Event_Editor_ViewWireframe` | `0x00d3e330` | `"viewmode wireframe"` |
| 7 | `Event_Editor_ViewUnlit` | `0x00d3e3c0` | `"viewmode unlit"` |
| 8 | `Event_Editor_ViewLit` | `0x00d3e450` | `"viewmode lit"` |
| 9 | `Event_Editor_ShowFPS` | `0x00d3e4e0` | `"STAT FPS"` |
| 10 | `Event_Editor_ShowPerformance` | `0x00d3e570` | `"SHOW PERFORMANCECOLORATION"` |
| 11 | `Event_Editor_ScreenShot` | `0x00d3e600` | `"shot"` |
| 12 | `Event_Editor_ShadowStats` | `0x00d3e690` | `"dumpdynamicshadowstats"` |
| 13 | `Event_Editor_CameraDefault` | `0x00d3e720` | `"Camera Default"` |
| 14 | `Event_Editor_Camera1stPerson` | `0x00d3e7b0` | `"Camera FirstPerson"` |
| 15 | `Event_Editor_Camera3rdPerson` | `0x00d3e840` | `"Camera ThirdPerson"` |
| 16 | `Event_Editor_CameraFixed` | `0x00d3e8d0` | `"Camera Fixed"` |
| 17 | `Event_Editor_CameraFixedTracking` | `0x00d3e960` | `"Camera FixedTracking"` |
| 18 | `Event_Editor_CameraFree` | `0x00d3e9f0` | `"Camera FreeCam"` |
| 19 | `Event_Editor_Ghost` | `0x00d3ea80` | `"Ghost"` |
| 20 | `Event_Editor_Walk` | `0x00d3eb10` | `"Walk"` |
| 21 | `Event_Editor_Use` | `0x00d3eba0` | `"Use"` (wchar_t at `0x019bc634`) |
| 22 | `Event_Editor_ToggleCombat` | `0x00d3ec30` | Flash external window call (not Exec) |

## Completeness scores (v5 workflow, 2026-05-25)

| Function | Effective score | Notes |
|---|---|---|
| `SGWHomeless__GetSingleton` | 82.6% | Unfixable: SEH vars, scorer lag on set_global |
| `SGWHomeless__ctor` | 75.8% | Unfixable: SSA-split register vars, 1 scorer-lag global |
| `SGWHomeless__RegisterEditorEventSubscriptions` | 68.7% | Unfixable: 9 SEH-frame vars (15pts structural), LAB_016fe1ae SEH stub |

## GLevel weather layout (from SGWHomeless__OnNetTimeOfDay)

| GLevel offset | Size | Type | Field |
|---|---|---|---|
| +0x384 | 4 | float | Time-of-day value |
| +0x388 | 4 | float | Wind speed |
| +0x38c | 1 | bool | Weather flag |

## Open questions

- What is the connection between `Event_Option_Resolution/DevWindowedMode/WindowedMode` and the entity-name lookup in `SGWHomeless__OnEditorTestSequenceOrOptionChange`? The shared handler reads "name" and "eventAction" params — do these option events carry actor-name payloads?
- Class object size is unknown (constructor has no explicit field initializers; fields are opaque). Likely 0 bytes (empty struct used only for subscription bookkeeping) or very small.
