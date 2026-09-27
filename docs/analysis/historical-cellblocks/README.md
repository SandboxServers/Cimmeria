# Historical CellBlocks: Seven Map States as Archaeology Worlds

**Date:** 2026-09-26
**Status:** Server wiring built (branch `feat/historical-cellblocks`). The client patch is installed in the local QA client with the streaming-filename fix described below. The in-client UAT has not run.
**Trigger:** an external handoff (`CLAUDE_HANDOFF_HISTORICAL_CELLBLOCKS_7VERSIONS_ENGLISH.md`, the server pack `SGW_Historical_CellBlocks_SERVER_DEV_7VERSIONS_ENGLISH.zip` and the client pack `SGW_Historical_CellBlocks_CLIENT_7VERSIONS_FINAL.zip`). It asks for every recoverable historical state of the Castle CellBlock map to be reachable in game, beside the stock world, so the states can be compared before anything is restored into world 12.

## Summary

Seven historical CellBlock maps each load as their own world, 1201–1207. A GM reaches one with `.gotolocation CellBlock43 -334.231 73.472 -228.026` and returns with `.gotolocation Castle_CellBlock -334.231 73.472 -228.026`. Stock `Castle_CellBlock` (world 12) is unchanged on both sides.

The worlds are empty on purpose. No missions, spawns, cover, respawners, ring routes or scripts are seeded for them. They exist for walking and looking.

The server side is server-authoritative: seed rows, `spaces.xml` entries, the base's world-id and client-map tables, and seven new `CookedWorldInfo` entries pushed to clients over the existing cooked-data handshake. No on-disk PAK changes. The client side is seven new map folders beside the stock one, and three of them needed a fix before they could stream (see [Client patch](#client-patch)).

Evidence labels: `CONFIRMED` (read from the packages, the PAKs or the code), `INFERENCE`, `UNRESOLVED`.

## World contract

| Build | World (typed) | World id | Client map (package) | Source |
|---|---|---:|---|---|
| 43485 | `CellBlock43` | 1201 | `C43485_CellBlock` | complete build |
| 55124 | `CellBlock55` | 1202 | `C55124_CellBlock` | complete build |
| 57050 | `CellBlock57` | 1203 | `C57050_CellBlock` | complete build |
| 58674 | `CellBlock58` | 1204 | `C58674_CellBlock` | complete build |
| 60130 | `CellBlock60` | 1205 | `C60130_CellBlock` | 58674 + CME VPatch 58674→60130 |
| 62429 | `CellBlock62` | 1206 | `C62429_CellBlock` | 60130 + CME VPatch 60130→62429 |
| 63682 | `CellBlock63` | 1207 | `C63682_CellBlock` | 62429 + CME VPatch 62429→63682 |
| current QA | `Castle_CellBlock` | 12 | `Castle_CellBlock` | the client's own, untouched |

The typed world name deliberately differs from the package name. The package alias is 16 characters, the same length as `Castle_CellBlock`, so the handoff author could rename every package reference in place without moving any table offsets.

The handoff reports that each VPatch output matched the destination MD5 stored in the original patch data. The older delta chain (45032→46694→47977→49486) is excluded: its 45032 source mapset is missing, and the first delta's source MD5 does not match 43485, so applying it would fabricate a state rather than recover one. None of this was re-verified here; the patch data is not in the handoff.

## Server wiring

| Piece | Where | What it does |
|---|---|---|
| The table | `crates/wire/src/mercury/world_data/historical_cellblocks.rs` | `HISTORICAL_CELLBLOCKS`, the one Rust copy of the contract above |
| World id and client map | `world_id_for_name`, `client_map_for_world` in `crates/wire/src/mercury/world_data/mod.rs` | `onClientMapLoad` sends `areaName = CellBlock43`, `mapPath = C43485_CellBlock`, `WorldID = 1201`; `setupWorldParameters` sends 1201 |
| Seed | `db/resources/Worlds/Seed/worlds.sql` | Seven rows: `flags = 1`, `has_script = false`, `navmesh_mode = 'advisory'`, and the stock CellBlock row's movement values copied verbatim |
| Spaces | `entities/spaces.xml` | Seven `Instanced="true"` entries with the stock CellBlock bounds; none in `entities/cell_spaces.xml` |
| Client world table | `crates/resources/src/base/world_info_overrides.rs` | Seven `COOKED_WORLD_INFO` entries pushed per key on category 12; see [World info overrides](../../architecture/mission-pak-overrides.md#world-info-overrides-category-12) |
| Fail-closed fallback | `resolve_space_id_fallback` in `crates/base-session/src/base/world_entry/space_registry.rs` | Returns `None` for these worlds. Gate travel ends the session and login refuses the entry, instead of falling back to the stock CellBlock space |

The fallback only matters when the cell drops its `CreateEntity` reply, which today happens only when the create fails. The table's unknown-world default is the stock CellBlock space. Without the refusal, a player bound for `CellBlock43` would be handed a world entry for that space while the cell held their entity nowhere.

Each arrival gets a fresh instance, as for the stock CellBlock. No `.nav` or `.occ` loads for these worlds: those files are keyed by world name (`data/spaces/cellblock43.nav`), and the stock `castle_cellblock.nav` would not match the older geometry anyway. With no mesh, movement is not contained, which is harmless in a world with no NPCs.

`sgw_player.world_id` is filled from `resources.worlds` by name when a gate-travel arrival persists, so the seed rows are also what records a player's world as 1201 rather than keeping the origin's id.

### Reconciled against the handoff

- **Paths.** The handoff predates the services crate split (#825). `world_data` is now in `cimmeria-wire`, the resource cache in `cimmeria-resources`, and the space registry in `cimmeria-base-session`.
- **XML shape.** The handoff's generator emitted a self-closing element with no namespaces, in a different attribute order. The shipped `CookedWorldInfo.pak` entries have five SOAP namespace declarations, attributes in the order `Flags MinPerDay MinToRealMin ClientMap World WorldID`, and an explicit end tag. The generator reproduces that shape byte for byte, and a test pins it against real shipped entries. `CONFIRMED`
- **Duplicated tables.** The handoff added seven literal match arms to each of the two lookup functions. Both read the one table instead, and the world-info entries are built from it too.

## Client patch

The seven folders go under `Working\SGWGame\CookedPC\Maps\` beside `Castle_CellBlock`. The client's package cache scans `CookedPC` recursively, which is how the stock folder resolves. No ini change and no PAK change is needed on the client. The handoff's installer (`Install-Historical-CellBlocks.ps1 -ClientRoot <…\Stargate Worlds-QA>`) hashes the stock folder before and after and refuses to finish if it changed.

### Streaming filename fix (43485, 55124, 57050)

As shipped, these three builds would load their persistent level and none of the 64 streaming sublevels. `CONFIRMED`

- The files are named `C43485_CellBlock_00000000.umap`, with an underscore.
- Each persistent map references its sublevels as `C43485_CellBlock-00000000`, with a dash. UE3 resolves a streaming level by package name, which is the file name.
- Each sublevel's own name table carries the dash form too, and stock `Castle_CellBlock` and builds 58674 onward name their files that way.

The corrected package renames those 192 files to the dash form. File contents are unchanged (the multiset of SHA-256 hashes is identical), and the manifest's `OutputFile` column is rewritten to match. After the rename, in all seven folders, every alias-prefixed name in the persistent map resolves to a file. The only file the persistent map does not name is `<alias>_MapData.upk`, which loads by naming convention, as in stock.

### Checks run on the payload

| Check | Result |
|---|---|
| Output files against the manifest's SHA-256 (262 rows, the four complete builds) | all match |
| Persistent maps of the three VPatch builds against `NEW_BUILD_VALIDATION.json` | all match, file counts and byte totals too |
| Any `castle_cellblock` name left in any package (any case) | none |
| Alias names colliding with an existing client package | none |
| Package imports missing from the client | none (the three not under `CookedPC` are `Engine`, `EngineMaterials` and `EditorMeshes`, which live under `Working\Engine\Content` and `SGWGame\Content\FRScript`) |
| Package GUID shared with a stock package | none |
| Stock `Castle_CellBlock` tree SHA-256 before and after install | identical |

## What the UAT must settle

These are properties of the recovered packages that no server change can fix. Each could stop a map from loading or rendering correctly.

- **Package version.** The historical packages are Epic 486 with licensee version 6 (7 for 63682). The client's own are licensee 8. Older licensee versions load only as far as SGW's own serializers kept back-compat branches. `UNRESOLVED`
- **Uncooked 43485.** Build 43485's packages carry flags `0x20001`, without `PKG_Cooked`. The later builds carry `0xa0009`. `UNRESOLVED` whether the cooked client accepts them.
- **No MapData in 43485 and 55124.** Those two builds have no `_MapData.upk`; it first appears in 57050. `UNRESOLVED` whether the client needs one.
- **Shared package GUIDs.** Unchanged files carried across the VPatch lineage keep their GUIDs, so 55124 and 57050 share 58 GUIDs, and 58674, 60130 and 62429 share 63. This matters only if two historical worlds are resident at once. `INFERENCE`: harmless for one world at a time.
- **Old Kismet.** The historical maps carry their own Kismet. A sequence that fires on level load could reference content or server state that does not match current Cimmeria. `UNRESOLVED`

The UAT for each world records load success, streaming, geometry and prop differences, collision, native cover-node behaviour, unexpected current content, and package or namespace errors. It ends back in `Castle_CellBlock`, which must behave as before.

| World | Loads | Streams | Geometry / props | Collision | Cover nodes | Errors |
|---|---|---|---|---|---|---|
| `CellBlock43` | | | | | | |
| `CellBlock55` | | | | | | |
| `CellBlock57` | | | | | | |
| `CellBlock58` | | | | | | |
| `CellBlock60` | | | | | | |
| `CellBlock62` | | | | | | |
| `CellBlock63` | | | | | | |

### Mixing servers during the UAT

A client that logs in to a server with this change takes the bumped category-12 version. Logging the same client in to a server without it (the colo before this deploys) empties the client's whole world table. See [A bumped category must keep its override list everywhere](../../architecture/mission-pak-overrides.md#a-bumped-category-must-keep-its-override-list-everywhere) for the cause and the repair.

## Out of scope

- Restoring anything, including the 43485 cover nodes, into stock world 12. That waits for the comparison.
- Missions, spawns, cover, respawners and ring routes in the historical worlds.
- The 45032→49486 delta chain, until its source mapset turns up.
