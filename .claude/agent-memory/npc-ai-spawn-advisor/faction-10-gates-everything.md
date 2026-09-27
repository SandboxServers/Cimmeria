---
name: faction-10-gates-everything
description: entity_templates.faction==10 (HOSTILE_FACTION) is the single switch for "player can damage this NPC" AND "right-click attacks instead of talks"; there is no runtime way to change it.
metadata:
  type: project
---

> **Stale since NA13 (2026-09-25):** the `aggression` field and "faction 10 alone never aggroes" claims below are superseded; see [[faction-derived-aggro-na13]].

# `faction = 10` is a template-only, load-bearing switch (measured 2026-09-17)

`HOSTILE_FACTION = 10` (`cell/combat/mod.rs:21`, mirrors python
`Atrea.enums.FACTION_Aggressive`). Four independent gates read it:

| Gate | File | Effect when `faction != 10` |
|---|---|---|
| Player single-target ability (#444) | `abilities/use_ability/handle.rs:223-237` | `useAbility` **rejected**, damage pipeline never entered |
| Player AoE | `abilities/dispatch.rs:124` | NPC excluded from target set |
| Player cone AoE | `abilities/cone_aoe/geometry.rs:87` | NPC excluded |
| Right-click `interact` | `cell_methods/player/interaction/interact.rs:44-46` | routed to **dialog**, not auto-attack |

NPC attackers are deliberately NOT gated (npc_ai calls the same entry
point), so a faction-1 NPC **can shoot the player and the player cannot
shoot back**.

## There is no runtime faction change

- `set_aggression` writes `entity.aggression` only
  (`content/executor/world/mod.rs:80-93`). It does **not** touch faction.
- `Action::ModifyProperty` exists in `content-engine/src/actions.rs` but has
  **no executor arm** in `crates/services` — it is a no-op.
- No `SetFaction` action exists.

**Consequence:** an NPC that must be talked to *and* later killed cannot be
one template. Either author two templates and swap them
(`destroy_tagged_entity` + `spawn_entity`), or add a `set_faction` primitive.
`aggression` stays 0 at spawn for a faction-10 mob, so faction 10 alone does
NOT make a mob attack on sight (`npc_ai/dispatch.rs:59` admits Idle only when
`aggression > 0`); it only makes it *attackable*. That makes "seed faction 10,
flip aggression from the chain" the correct pattern for ambush mobs.

`npc_ai_idle_auto_aggro` picks targets by `p.faction != npc_faction`
(`npc_ai/fight.rs:44`) — factions merely have to differ, there is no
hostility table.

Related: [[level-is-hp-and-xp]], [[harset-zone-evidence]]
