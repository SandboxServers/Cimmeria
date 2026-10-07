---
title: "Custom Debug Map: purpose-built proving ground"
type: analysis
audience: engineers, map authors, playtesters
last_updated: 2026-10-06
---

# Custom Debug Map

**Status: design and editor-output gate. No playable custom map has been produced or loaded yet.** The requested target is a newly constructed level made from SGW's existing assets, not an alias, tile copy, or redecorated stock world. The [work-packet ledger](work-packets.md) makes the first playable artifact the campaign's entry gate. The existing DebugArea (world 1300) remains the working proving ground until that gate passes.

The first [automation probe](automation-probe.md) found a command-file hook in
`SGW.exe` and verified the editor byte patches against the local QA executable.
The automated shell cannot get past the client's early startup modal. A direct
package experiment produced three renamed scratch scaffolds and successfully
cloned one cover mesh actor from another map; these have not loaded in the
client and are not yet a constructed level.

## Evidence and limits

| Finding | Evidence | Confidence |
|---|---|---|
| The QA executable contains a New Level dialog and creates a fresh editor world after confirmation. | Ghidra `SGW.exe` `0x00ef6d30`, `0x00ef9970`, `0x00bf7d40`; [editor study](../debug-area/custom-map-editor-feasibility.md) | High, static |
| The editor has BigWorld streamed-level save, map rebuild, path definition, cover build and map-thumbnail commands. | Ghidra `0x00fede90`, `0x00efb650`, `0x00ff40b0`, `0x01035830`; [editor study](../debug-area/custom-map-editor-feasibility.md) | High that commands exist; output untested |
| Server world id, client map path and cooked world catalogue can name a new map. | [Historical CellBlocks](../historical-cellblocks/README.md#world-contract) loaded distinct map folders in-client; `added_worlds.rs`, `world_info_overrides.rs`, seed and `spaces.xml` | High for world routing |
| A server navmesh, occluder and cover set can be generated from a client map. | [navmesh pipeline](../../engine/navmesh-build-pipeline.md), [cover extraction](../../engine/cover-extraction.md), `data/spaces/README.md` | High for existing cooked maps; new editor output must be tested |
| Editor save output is accepted by the game client. | No live test yet | **Unknown; gating experiment** |

The existing `tools/SceneEditor` writes JSON, not a cooked `.umap`. The `crates/upk` patcher changes already cooked packages; it is not a whole-map cooker. Renaming a Harset folder would test package aliases, not this request. None of these substitutes closes the editor-output gate.

## Prototype layout

Make a compact original level, approximately 180 × 140 metres, with a single outdoor yard and three rooms. Start with editor BSP and terrain so the geometry is truly authored; dress it with existing SGW meshes, materials, lights and the gate rig. The layout is a test instrument: short sightlines, obvious surfaces and safe routes make failures easy to identify.

```text
                outdoor yard / daylight
        ┌─────────────────────────────────────┐
        │ gate + DHD       terrain slope       │
        │ arrival apron    cover lane          │
        │                     ┌─────────────┐ │
        │                     │ combat room │ │
        │ ┌───────────────┐   ├─────────────┤ │
        │ │ fixture room  │───│ central hub │ │
        │ └───────────────┘   ├─────────────┤ │
        │                     │ cover room  │ │
        │                     └─────────────┘ │
        └─────────────────────────────────────┘
```

| Zone | Construct and place | Proves |
|---|---|---|
| Gate apron | Flat collision floor, SGW gate/DHD rig, arrival point outside the event horizon | Asset imports, gate animation/event, outbound and inbound travel, safe spawn |
| Outdoor yard | Authored terrain/ground, sky, lighting, perimeter, slope and two cover obstacles | Terrain collision, outdoor lighting, streaming, long range combat |
| Central hub | BSP floor/walls/door openings, lamps, signs and return point | New-room construction, interior/exterior transition, respawn |
| Cover room | Two different obstacle heights and deliberately placed cover markers on each side | Cover extraction, node facing, NPC use and occlusion |
| Combat room | Open duel lane and a few destructible/interactive fixtures if supported by stock assets | Spawn, threat, abilities, death and cleanup |
| Fixture room | Existing consoles, doors, ring rig and vendor/mission interactors as later packets | Actor imports, interaction and content chains |

Build one contiguous playable floor first. Do not add a second streamed cell until the single-level save/reopen/game-load check succeeds. After that, split indoor and outdoor geometry across two cells and verify both stream. Use a distinct package and world id, with no stock map mutation. Keep all generated client bytes in a local scratch tree; distribution needs a source transform or patch recipe that obeys the [no CME bytes rule](../../../data/client-patches/README.md#the-rule-no-cme-bytes).

## Acceptance

The proof is complete only when a clean local QA client loads the distinct map, shows all three constructed rooms and outdoor yard, collides with their floors/walls, animates the gate, shows map art, permits travel and respawn, and demonstrates an NPC taking authored cover. The server must load the new world's own `.nav`/`.occ` and `cover_sets`/`cover_nodes`. Screenshots, client log, package manifest and recorded in-client checks belong in the packet ledger. An editor viewport screenshot alone is not acceptance.
