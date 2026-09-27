---
name: npc-aggression-broadcast-na33
description: SGWMob flat ClientMethod indices for onAggressionOverrideUpdate/Cleared (27/28) and the legacy setAggression client-dead wire-format bug, verified 2026-09-25
metadata:
  type: project
---

> [!NOTE] PROMOTION TARGET: docs/reverse-engineering/findings/npc-aggression-broadcast.md (already written and merged, PR pending as of this memory)

NA33 (2026-09-25): confirmed SGWMob's own `ClientMethods` (`onAggressionOverrideUpdate` INT8, `onAggressionOverrideCleared` none-args) sit at flat indices **27** and **28** — SGWMob shares indices 0-26 with SGWPlayer (identical `SGWSpawnableEntity → SGWBeing` ancestor prefix), and `Lootable` (SGWMob's only `<Implements>`) contributes zero client methods, so SGWMob's own two begin immediately at 27. Ghidra: paired-handler registration `0x00d31cd0` wires `MemberCallback<GameMob, Event_NetIn_onAggressionOverrideUpdate>` and the `...Cleared` sibling; the Update handler `0x00d31bd0` reads `aAggressionLevel` INT8 and stores it at `GameMob + 0x16c`.

**Load-bearing gotcha for future NPC/mob wire work:** past index 26, SGWPlayer and SGWMob's flattened index spaces **numerically collide but mean different things** — SGWPlayer's `Communicator` interface also occupies 27-33. Never reuse a `mercury::method_idx::*` constant across entity types without checking which `class_id` it was derived for.

Also resolved a real 2009 shipped bug: python `SGWMob.setAggression` (the runtime-change path) broadcast `onEntityProperty(GENERICPROPERTY_MobAggression)`, for which NA13 had already found no client consumer — dead wire traffic. Only `createOnClient` (the one-time spawn/reconnect snapshot) used the real `onAggressionOverrideUpdate` ClientMethod, and only when an override was already set. Cimmeria uses the real ClientMethod on every runtime change now (finishing `createOnClient`'s intent, no client patch needed) — this is the same "which wire call did legacy *actually* use vs. which one worked" trap as the `GENERICPROPERTY_*` property-vs-ClientMethod split documented elsewhere in this campaign.

`UIAggressionLevel` (RTTI `0x01de972c`) is a Lua-scriptable enum type registration (`0x00ab1a5e`, alongside `UIArchetype`/`TargetType`/`UIStatType`), not a CEGUI widget — no consuming Lua script was located (no client Lua source in this repo tree), so the exact on-screen effect (nameplate/reticle color, interaction verb) is inferred, not observed.
