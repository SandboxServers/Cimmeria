---
name: npcs-cannot-turn-in-place
description: NPC yaw is written only inside npc_movement_tick, which skips path-less NPCs — an attacking NPC's facing is frozen forever and no re-face path exists
metadata:
  type: reference
---

> **Status 2026-09-19 — resolved.** `face_target` was added in #682 (`npc_ai/fight.rs`, called from
> the attack-in-place branch and from `npc_ai/lifecycle`), so the server can now re-face a path-less
> NPC. Keep this file for the mechanism and for the open question it raises — whether a facing-arc
> gate should exist — not as a current defect. The "only production writers of `direction`" list below
> is out of date.

**There is no turn-in-place / yaw-only update anywhere in the server.** Confirmed by exhaustive grep
of `.direction = ` in `crates/services/src/cell/` (2026-09-18). The only production writers of NPC
`direction` are:

- `cell/service/ticks/npc_movement.rs:121`, `:187` — the movement tick
- `cell/service/ticks/npc_respawn/mod.rs:296` — `spawn_dir` at respawn
- `cell/console/entity.rs:241`, `cell/console/placement.rs:272` — GM console only

And `npc_movement_tick` selects its candidate set with `!e.nav_path.is_empty()`
(`npc_movement.rs:41-47`). So the moment an NPC closes to range and attacks —
`cell/service/npc_ai/fight.rs:473` does `npc.nav_path.clear()` — it is **excluded from the tick** and
its yaw is frozen at whatever it held when the path emptied. No amount of target strafing will
re-face it.

This is the mechanism behind "the NPC stops attacking while apparently still holding threat": it
faces correctly at the instant it stops, the player circles, and the NPC cannot follow. There is **no
server-side facing-arc gate** (grep across `cell/abilities/` and `cell/combat/` found none), so if
attacks do stop when facing away, the gate is **client-side** — which makes the frozen yaw fatal
rather than cosmetic.

Scale: in the 2026-09-18 session `attack_in_place` was **123 of 165** logged AI decisions vs `chase`
25. This is the common case, not an edge case.

**How to apply:**
- A turn-in-place costs **nothing new on the wire** — direction bytes are always present on the
  `0x10` UPDATE_AVATAR variant we already send every tick to every witness.
- Fix shape: either let the movement tick process path-less NPCs that hold a combat target (writing
  yaw only, never position), or add a `face_target` helper called from the attack-in-place branch.
- Whether an arc gate *should* exist and how wide is **npc-ai-spawn-advisor's** call; the movement
  side only owns "the server must be able to re-face."
- Compounds with [[npc-broadcast-facing-and-grounding]]: a frozen yaw that was already wrong (the
  `pack_angle` north-snap) stays wrong permanently.
