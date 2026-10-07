---
title: "Custom Debug Area map: SGW editor feasibility"
type: analysis
audience: engineers, map authors
last_updated: 2026-10-06
---

# Custom Debug Area map: SGW editor feasibility

This is static analysis of the local QA `SGW.exe` in Ghidra, checked against
the local QA files and Cimmeria's existing map tools. It does not claim that
a newly authored map has loaded in the game client. The earlier
[map-selection study](map-selection-study.md) compares the four stock maps;
this note examines a purpose-built alternative.

## What the executable confirms

| Operation | Ghidra evidence in QA `SGW.exe` | Meaning and limit |
|---|---|---|
| Start a new level | `FUN_00ef6d30` creates `WxDlgNewLevel` from `ID_DLG_NEW_LEVEL` and finds its `IDRB_ADDITIVE` control. `FUN_00ef9970` displays it and, after OK, calls `UEditorEngine__unknown_00bf7d40`. That function logs `starting new map` and creates/switches the editor level. | There is a real blank-level path, distinct from loading a copied map. Its complete saved/cooked output is not yet tested. |
| Load a level | `UnEdSrv__HandleMapLoad` at `0x00ef9780` calls the package loader, switches the world, and broadcasts `Event_Editor_PostLoadMap` for a non-partial map. | A saved map can be reopened in the editor; this does not prove game-client compatibility. |
| Save BigWorld chunks | Toolbar command `0x6774`, `WxMainFrame__OnSaveBigWorldChunks` at `0x00fede90`, calls `FUN_00efb650`. Its flagged-level branch `FUN_00efb290` finds `TheWorld`, iterates `GWorld->StreamingLevels` in batches of 50, updates streaming, and calls `FUN_00efaaa0` / `FUN_00efab40` for each level. | The editor has a real multi-chunk save path. Test its output filenames, master-level references and client load; do not equate this with a validated cooker. |
| Rebuild and cover | The editor build-menu handler at `0x00ff40b0` issues `MAP REBUILD`, `MAP REBUILD ALLVISIBLE`, `DEFINEPATHS`, `BUILDCOVER` and `BUILDCOMBATZONES` on distinct menu paths. | Native editor commands exist. Cimmeria still needs its own `.nav`, `.occ` and world-scoped cover seed for server AI. |
| Build map pictures | `MapThumbnailGenerator` constructor at `0x01034a60` reads `Minimap.ThumbnailDimension` and `WorldMapTexSize`. Its `FUN_01035830` path says `Building map thumbnails`, walks map cells, and calls `FUN_01034be0` for map overview generation. Build-menu case `0x4f33` calls it through `FUN_00ef9a60`. | The original editor contains a map-picture generator. The exact output package, tile quality and save sequence need a lab run. |
| Edit world-map collection | `FUN_0115c3a0` loads `ID_DLG_NEW_WORLDMAP`; `FUN_0115c0a0` loads `ID_DLG_WORLDMAPS`. `FUN_0115c4a0` returns a name entered in the new-world-map dialog. | These dialogs manage map artwork/layers. **New World Map is not New Level** and does not by itself build playable geometry. |

The local QA install has `AtreaEditor.bat`, `AtreaLoader.exe`, `AtreaRL.dll`
and `UnrealEdSGW.xrc`. [Atrea editor archaeology](../../reverse-engineering/findings/atrea-editor.md)
describes the editor-mode patch group and SGW's BigWorld UI. The proposed
[editor bridge](../../architecture/atrea-editor-bridge.md) is still an ADR,
not a working automation connection. The separate `tools/SceneEditor` can
inspect and edit extracted actor data but its `save_zone` writes JSON, not a
playable `.umap` (`tools/SceneEditor/src/commands/scene.rs`).

**Correction to prior archaeology:** the handler at `0x00fe9cc0`, previously
called a possible BigWorld build/export wizard, calls `FUN_01132690`, which
constructs a wxWizard explicitly titled `RandomActorSettings`. It is an
actor-scatter dialog. No BigWorld map-creation claim should rest on it.

## What we can build around that editor

1. **Client map:** copy a small map folder as a disposable authoring base,
   then use the original editor to strip/replace geometry and place existing
   SGW meshes, materials, lights, ring rigs and authored markers. A truly
   blank map is the second proof, after the copy-based path works. Save to a
   new package name and verify all streamed level names and dependencies.
2. **Server world:** create a unique world id and `CookedWorldInfo` map path,
   space/arrival/respawn rows, GM route and ring regions. The historical
   CellBlock worlds prove a new map folder and server identity can load in the
   QA client ([contract](../historical-cellblocks/README.md#world-contract)).
3. **AI:** run the existing geometry extraction and NavBuilder pipeline for
   a new `.nav`, then build a matching `.occ`. Run `cover_extract` on placed
   `SGWSpecCoverNode`/`CoverNodeArray` markers, or author equivalent
   world-scoped cover rows. Check cover facing and NPC use in-client
   ([navmesh](../../engine/navmesh-build-pipeline.md),
   [cover](../../engine/cover-extraction.md)).
4. **Map UI:** try the native thumbnail generator and World Maps dialog on
   the prototype, then inspect its `<Map>_MapData.upk`. `MapData` carries the
   world-map collection, layer data, tile textures and default overview
   texture. The proven `world_map_rebake` patch repairs an existing texture;
   it is not a complete new-MapData authoring tool
   ([patch 013](../../../data/client-patches/README.md#013-ihpet-world-map)).

## First decisive experiment

Work on a separate QA-client copy. In Atrea Editor, create one small level
or clone a small map into a new folder, add one floor, wall, light and ring,
save it, and record the files written. Reopen in a clean editor process.
Then give the QA game client a new `CookedWorldInfo` that points to it and
enter through a temporary GM-only world. Check the rendered floor, collision,
streaming levels, ring Matinee, map window and crash log. Only after this
passes should the facility design and content migration be treated as a
buildable project.

The chief unknown is the **editor-to-game package boundary**: whether its
saved output contains every cooked flag, dependency, streaming reference and
asset/light record the QA game loader needs. Static decompilation establishes
the editor functions; only an isolated editor-save and game-load test settles
that boundary. There was no live editor or game-client run in this pass.
