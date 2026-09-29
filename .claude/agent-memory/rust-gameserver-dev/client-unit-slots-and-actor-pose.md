---
name: client-unit-slots-and-actor-pose
description: SGW client Lua "units" are GameEntityManager slots (map at +0x130), not entity ids; actor pose at actor+0xDC; worldToPixel only inside PreRender — how the lab world tools read positions/names and project
metadata:
  type: reference
---

Static Ghidra facts (2026-09-29, this client build), used by `crates/lab/src/supervisor/world/`. Not yet checked on a live client when written.

- Lua `unit*` functions take a **slot number**, not an entity id. `FUN_00c67120` looks the number up in a `std::map<int,int>` at `GameEntityManager + 0x130` (head +0x134, size +0x138): slot -> entity id. `Unit.Pet1..4` = 10..13, `Dialog` = 17, `DialogSpeaker` = 18.
- `FUN_00c67bd0` (thiscall mgr, slot, entityId) writes a slot and raises `Event_UI_UnitMappingChanged`; the only Lua handler ignores untracked slots, so slots 7700+ are a private handle for reading any entity with stock `unitName`/`unitLevel`/`unitHostilityToPlayer`/`unitMobId`.
- `unitPosition` = `Entity+0x08` actor + `0xDC` (UE3 `AActor::Location`, Z up, UE3 units); `FRotator` at +0xE8 (65536/turn). `unitOrientation` returns a **fraction of a turn** in [0,1). Fallback actor chain: `[[[[g_pGLevel(0x01EE2684)+0x50]+0x3C]]+0x35C]` (controller or pawn — unverified).
- `view:worldToPixel(Vector3)` returns `(pixel, success)` and exists only inside `Events.PreRender`. A window holds one subscription per event, so chain a stock host (`SCTWin` -> `SCTMod.onPreRender`) instead of adding one.
- Client <-> server coords: `client = (server.z, server.x, server.y) * 100`.

Live-check list: docs/guides/live-research-lab.md § World tools. Related: [[cme-registry-is-a-factory-not-subscribe]].
