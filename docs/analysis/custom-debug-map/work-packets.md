---
title: "Custom Debug Map campaign work packets"
type: analysis
audience: engineers, map authors, reviewers
last_updated: 2026-10-06
---

# Custom Debug Map campaign

**Campaign status: research complete enough to dispatch CM-00; implementation and UAT pending.** This campaign's deliverable is a newly authored, locally playable SGW level built from existing client assets. [Design and evidence](README.md) explain the layout and the editor-output uncertainty. Record packet PRs, hashes, screenshots and UAT here as work lands. The current world 1300 is unaffected until the new world passes UAT.

## Dispatch contract

- Each packet names its own files and test evidence in its PR; never put full client map binaries in git.
- Build and test on the pinned native Windows toolchain through `tools/build-lane/lane.sh` as required by `CLAUDE.md`.
- Read `TESTING.md` before adding runtime tests. Runtime behavior needs a regression test; user-visible behavior or build steps need docs in the same PR.
- Preserve clean stock map trees. Use a disposable QA client working copy for editor saves and a second clean copy for game-load UAT. Record input/output hashes and a file manifest.
- `CM-00` is a hard gate. Static Ghidra evidence proves commands exist; it does not prove the editor's saved level loads in the QA game. Do not mark later packets complete based on editor screenshots.

## Packets

| Packet | Scope, deliverable and acceptance | Depends | Evidence / owner specialty |
|---|---|---|---|
| **CM-00 — editor-to-client gate** | On a disposable install, use **New Level** to create a genuinely new package. Place one BSP floor, wall, stock mesh and light. Save, restart editor, reopen, build/rebuild, then load that package in a clean QA game. Record exact editor commands, file names, package summary/flags, imports, GUIDs, logs and screenshots. Pass = floor renders and collides in game with no missing package. If save emits uncooked assets or game load fails, isolate the missing cook/dependency step in Ghidra and repeat. | None | Ghidra anchors in [feasibility](../debug-area/custom-map-editor-feasibility.md); game archaeology, BigWorld advisor |
| **CM-01 — asset and scale palette** | Inspect stock cooked packages and editor browser for one coherent room kit, terrain/sky/light set, Stargate+DHD rig, door, console, ring rig and two cover obstacles. Record exact package/object paths, material dependencies, dimensions and legal local transform method. Place each in a throwaway new level and verify game load. Pass = reproducible palette manifest, no missing imports. | CM-00 | `crates/upk-objects`, `tools/SceneEditor`, `docs/engine/bigworld-architecture.md`; item/mission specialists only for later interactors |
| **CM-02 — authored geometry** | Construct the [layout](README.md#prototype-layout): central hub, combat room, cover room, fixture room and outdoor yard. Use editor BSP/terrain and palette assets, with collision and safe player routes. Save as a distinct package, not a renamed stock tile. Add one streamed cell only after the single-level version passes. Pass = clean-client walkthrough, collision probes at floor/walls/doorways, cell unload/reload, no missing textures. | CM-01 | Native editor New Level, SaveBigWorldChunks, `MAP REBUILD`; world/streaming review |
| **CM-03 — world wiring and local install** | Reserve unused world id and package name; add `ADDED_WORLDS`, `CookedWorldInfo`, world seed, space bounds, GM-only travel and safe respawn. Build a local installer/transform from the player's stock assets; hash input and output, install beside stock maps, never overwrite originals. Pass = `.gotolocation <name>` loads the new map and a return route works; world id and map path agree across client/server. | CM-02 | [historical world contract](../historical-cellblocks/README.md#world-contract), [DebugArea](../debug-area/README.md#decisions-taken-for-this-campaign), wire and world-data tests; movement and authority review |
| **CM-04 — navigation, cover, occlusion** | Extract new geometry, run NavBuilder and `.occ` generation for the new world, and extract native `SGWSpecCoverNode`/`CoverNodeArray` into world-scoped cover rows. If the editor `BUILDCOVER` output differs, document and bridge it explicitly. Test cover facing, reachability, line of sight, an NPC choosing cover and no traversal through authored walls. Pass = server loads its own `.nav/.occ`, no fallback to a stock map; live NPC cover behavior recorded. | CM-02, CM-03 | [navmesh](../../engine/navmesh-build-pipeline.md), [cover](../../engine/cover-extraction.md); NPC AI, combat, testing review |
| **CM-05 — gate and fixtures** | Place a stock gate/DHD rig with sequence dependencies and server gate/point-set/arrival rows. Add ring, door, console, one safe interactive actor and respawn. Test both directions and failed arrival; keep non-GM entry closed. Pass = open animation, event horizon crossing, safe arrival, no travel loop, working interaction chain. | CM-03, CM-04 | Existing DebugArea ring incident and Harset arrival evidence; movement, mission, item and authority review |
| **CM-06 — map pictures and packaging** | Run native map-thumbnail generator and World Maps UI on the authored map. Inspect generated `<Map>_MapData.upk`; verify overview, tiles, player marker and zoom in game. Build deterministic local source transform/patch recipe, fresh-install and upgrade tests, full file manifest and clean-stock hash check. Pass = clean client displays useful map and installer reproduces identical bytes. | CM-02, CM-03 | Ghidra `0x01035830`, [world-map patch](../../../data/client-patches/README.md#013-ihpet-world-map), patchset tests |
| **CM-07 — full local UAT and close-out** | Two clients if feasible: enter, walk every zone, combat with cover, die/respawn, dial gate, use ring/fixtures, inspect map, exit/reenter, and check memory/streaming and client/server logs. Fix found faults in follow-up packets. Update status docs only at close-out. Pass = evidence table filled with result per capability and known limitations. | CM-04–CM-06 | `docs/guides/unified-uat.md`, `docs/agents/doc-update-map.md`; testing validation |

## CM-00 experiment record

| Check | Result |
|---|---|
| New Level path located in executable | Static pass: `0x00ef6d30` → `0x00ef9970` → `0x00bf7d40` |
| BigWorld save path located | Static pass: `0x00fede90` → `0x00efb650` |
| Map/cover/thumbnail build paths located | Static pass: `0x00ff40b0`, `0x01035830` |
| New package saved by editor | Pending isolated editor run |
| New package loaded by clean game client | Pending isolated game run |
| Collision and stock asset visible in game | Pending |

Do not silently substitute a copied/renamed Harset map for CM-00 or CM-02. Harset is a useful source of meshes, rigs and material dependencies, and its gate placement is a reference; the map geometry in this campaign must be newly authored.
