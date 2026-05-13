---
name: reference_entity_defs
description: Entity def canonical source locations and their relationship to the running server
metadata:
  type: reference
---

# Entity Definitions — Canonical Source

Entity definitions are the BigWorld engine's contract between client and server for every distributed object.

## Primary sources

- `game/sgw/Common/res/entities/entities.xml` — master list of all entity type names
- `game/sgw/Common/res/entities/defs/*.def` — per-type property/method definitions

**Entity types registered:** Account, SGWBeing, SGWBlackMarket, SGWChannelManager, SGWCoverSet, SGWDuelMarker, SGWEntity, SGWEscrow, SGWGmPlayer, SGWMob, SGWPet, SGWPlayer, SGWPlayerGroupAuthority, SGWPlayerRespawner, SGWSpaceCreator, SGWSpawnRegion, SGWSpawnSet, SGWSpawnableEntity

## Repo mirror

`entities/defs/` in the repo root contains identical copies of the .def files. These are the working versions used for Cimmeria server development.

`entities/defs/editor/` contains editor-only metadata not needed for server operation.

## Key .def files for server work

- `SGWPlayer.def` — player entity (most methods, all client-visible properties)
- `SGWBeing.def` — base class for all mobile entities
- `SGWMob.def` — NPC/enemy entities
- `Account.def` — login/character-select entity

## How to apply

When investigating what properties/methods the client expects on a given entity, read the corresponding .def file from `entities/defs/`. The .def is authoritative — the Cimmeria Rust server must implement exactly these exposed properties and methods.
