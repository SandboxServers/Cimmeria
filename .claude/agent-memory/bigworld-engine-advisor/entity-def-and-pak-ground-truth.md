---
name: entity-def-and-pak-ground-truth
description: Exhaustively verified facts about entities/defs property flags (only 3 used, no client replication), data/cache PAK provenance (merged build), and SGW.exe watcher linkage — from the 2026-07-25 docs/engine audit
metadata:
  type: project
---

# Verified ground truth: entity defs, PAKs, watcher (2026-07-25 audit)

All facts below were derived by exhaustive grep/zip-inspection/Ghidra during the
`docs/engine/` accuracy audit. Each is re-checkable with the command given.

## SGW uses only THREE property flags — no client replication at all

**Why this matters:** SGW does **not** use BigWorld's automatic client property
replication. Every piece of state the client sees arrives via an explicit
`<ClientMethods>` RPC. There is no "set the property and the engine syncs it" path.

Across all 436 properties in the 36 def files (18 entity + 18 interface):

| Flag | Count |
|---|--:|
| `CELL_PRIVATE` | 310 |
| `BASE` | 64 |
| `CELL_PUBLIC` | 62 |

`OWN_CLIENT`, `OTHER_CLIENTS`, `ALL_CLIENTS`, `BASE_AND_CLIENT`,
`CELL_PUBLIC_AND_OWN` appear **zero times** anywhere in `entities/`.

```bash
grep -rn "OWN_CLIENT\|ALL_CLIENTS\|OTHER_CLIENTS\|BASE_AND_CLIENT\|CELL_PUBLIC_AND_OWN" entities/
```

**How to apply:** if someone asks "which property flag pushes this to the client?",
the answer is none — find or add the ClientMethod. Correct any doc claiming otherwise.

## Other def-file element counts

`<Type>` 436, `<Flags>` 436, `<Exposed/>` 262, `<ArgName>` 1098, `<ServerOnly />` 10,
`<Volatile>` in 7 files, `<LoDLevels>` 22 blocks all empty.
**`<Persistent>`, `<DatabaseLength>`, `<Identifier>` = 2 each** (only
`SGWPlayer.playerName` and `SGWSpaceCreator.areaKey`). **`<DetailLevel>` = 0.**
So persistence is NOT driven by `<Persistent>` — it is explicit server-side DB code.

Property types actually used (17 names): `INT32` 107, `PYTHON` 91, `INT8` 51,
`FLOAT` 36, `CONTROLLER_ID` 29, `UINT8` 22, `WSTRING` 15, `UINT32` 13, `MAILBOX` 10,
`VECTOR3` 7, `StatList` 6, `STRING` 6, then 1 each of `INT16`, `DBID`,
`CharacterInfoList`, `LootItemDefinitionList`, `EscrowRecordList`.
**Absent:** `INT64`, `UINT16`, `UINT64`, `FLOAT32`, `FLOAT64`, `UNICODE_STRING`,
`TUPLE`, inline `ARRAY`/`FIXED_DICT`. SGW always spells 32-bit float `FLOAT`.

## Parent hierarchy trap

Only **three** defs have no `<Parent>`: `SGWEntity`, `SGWBlackMarket`,
`SGWChannelManager`. `Account.def` has `<Parent>GamePawn</Parent>` **inside
`<UnrealProperties>`** — that is the UE3 Actor class, not a BigWorld parent. A naive
`grep '<Parent>'` reports `Account -> GamePawn` and misleads.

Six entities descend directly from `SGWEntity` (easy to miss, outside the combat
branch): `SGWCoverSet`, `SGWEscrow`, `SGWPlayerGroupAuthority`, `SGWPlayerRespawner`,
`SGWSpaceCreator`, `SGWSpawnRegion`, `SGWSpawnSet`.
`SGWPlayerGroupAuthority` and `SGWSpawnSet` implement `GroupAuthority`;
`Account` implements `ClientCache`.

The 8 non-`ServerOnly` entities — the only types the client can be asked to
instantiate: `Account`, `SGWSpawnableEntity`, `SGWBeing`, `SGWPlayer`, `SGWGmPlayer`,
`SGWMob`, `SGWPet`, `SGWDuelMarker`.

## data/cache is a MERGED build, not any single source

21 PAKs, **55,025 entries**, ~22.4 MB on disk, ~34.3 MB uncompressed / ~18.0 MB
compressed. Provenance is readable from ZIP entry timestamps:

| Date | Files | Provenance |
|---|--:|---|
| 2008-12-11 | 17 | QA Build, untouched |
| 2026-03-16 | 3 | QA+Server **merge** re-packed by Cimmeria |
| 2026-02-24 | 1 | Discord stub (`CookedBehaviorEvents.pak`, 120 B, 0 entries) |

The three merged archives keep the **QA MetaData** but carry the **Server entry
count** — proof of merge-into-QA rather than wholesale replacement:
`CookedDataKismetSeqEvent` (meta 7455, 1973 entries), `CookedDataKismetSetEvent`
(meta 7454, 675), `CookedInteractionSet` (meta 6615, 4663).

Category→PAK entry counts sum to exactly 55,025 (verified). Category 21
`pet_command` ships **no PAK at all**; category 22 `behavior_event` is the stub.

## Watcher code IS linked into SGW.exe

Do **not** repeat the claim that the client has no watcher code. Ghidra strings:
`DirectoryWatcher::addChild:...` @ `0x019221c0` and `0x01922218`,
`watcherStringToValue:...` @ `0x01b17410`,
`DataSection::setWatcherValues:...` @ `0x017fcb7c`, `WATCHER_NUB` @ `0x01b194e8`
(referenced from `FUN_01587110`, a `SERVER_COMPONENT`/`WATCHER_NUB`/`UNKNOWN`
component-name lookup). `ENABLE_WATCHERS` was 1 for this build.
**Unresolved:** whether a `WatcherNub` is ever constructed/bound at runtime.

## Cimmeria Rust AoI differs from the C++ docs

`crates/entity/src/world_grid.rs` is a **pull-based radius-query bucket grid**
(`query_radius`, `cell_key`). It has **no hysteresis**, no `visionExceptions_`, no
witness sets, no `WorldGridMember`. Witness bookkeeping lives at
`crates/services/src/mercury/aoi/` and `crates/services/src/cell/`.
AoI radius is **per-entity**, default **100.0**
(`crates/entity/src/cell_entity/construction.rs:30`) — *not* the old
`grid_vision_distance` 3 chunks × 50 m = 150 m.
`crates/entity/src/space.rs` has no navmesh/creator/players/dbEntities.

## Misc verified constants

- Resource fragment `MAX_CHUNK` is **1390** in Cimmeria (was 1000 in C++);
  Mercury `MAX_BODY_LENGTH` 1411, first-fragment header overhead 16 B.
- `entities/spaces.xml` uses **flat attributes**, not nested elements:
  `<Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" />`
- `entities/cell_spaces.xml` carries **names only** (16 non-instanced spaces).
- 5 of 24 spaces have navmesh: agnos, castle_cellblock, harset, harset_storagerm, sgc_w1.
- All 24 `spaces.xml` extent rows in `docs/engine/space-management.md` verified correct.
- `CELL_BASE_*` IDs 0x00–0x11 in `docs/engine/space-management.md` verified correct.
- `BASEMSG_SPACE_DATA` 0x07, `SPACE_VIEWPORT_INFO` 0x08, `SET_SPACE_VIEWPORT` 0x0E,
  `RESOURCE_FRAGMENT` 0x36 — verified.

See also [[bw-reference-tree-absent]].
