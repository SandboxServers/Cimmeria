---
name: npc-death-credit-and-respawn-gaps
description: No seeded NPC respawns (respawn_secs NULL table-wide) and DoT/effect-pulse kills never fire the death path at all - both break entity_dead_tag mission chains
metadata:
  type: project
---

Two engine facts that silently break any `entity_dead_tag` content chain.
Verified on e824e8a2.

**1. Nothing in the shipped seed respawns.** `respawn_secs` is resolved as
`COALESCE(spawnlist.respawn_secs, entity_templates.respawn_secs)`
(`cell/spawner/npcs.rs:149`), and **neither** seed file names the column at all
— `db/resources/Worlds/Seed/spawnlist.sql` (167 rows) and
`db/resources/Entities/Seed/entity_templates.sql` both omit it, so every value
is NULL. `normalize_respawn_secs(None) -> None` (`npcs.rs:277`), and
`mark_npc_dead` only stamps `respawn_at` when `respawn_secs.is_some()`
(`cell/combat/state.rs:110`). Result: every mob is one-shot; the corpse stays
forever. Any chain whose premise is "the mob respawns for the next player" is
wrong until the spawn row sets `respawn_secs` (DB `CHECK >= 3`).

Respawn itself, when armed, reuses the SAME entity (`ticks/npc_respawn/mod.rs`)
— the `tag` is never cleared, so the `entity_dead_tag` trigger re-arms
automatically. Threat list, loot, cooldowns and interaction flags are reset
(`npc_respawn/mod.rs:200-233`).

**2. DoT / effect-pulse kills never fire the death path.**
`cell/effects/pulsing/tick.rs::fire_pulse` writes stats through
`calculate_damage` (or raw stat mutation on the invoker-vanished fallback) and
has **no** alive→dead detection. `mark_npc_dead` is only reached from
`cell/abilities/damage_apply/mod.rs:222` and `cell/abilities/death.rs:299`. A
mob whose last point of health comes from a bleed tick therefore sits at 0 HP
with `ai_state` unchanged: no `BSF_DEAD`, no loot, no `respawn_at`, and no
`fire_entity_death` — so the kill-count chain never fires and the mission
sticks.

Kill credit (`cell/abilities/use_ability/kill_credit.rs:92-105`) requires BOTH
the victim's `tag` (from `spawnlist.tag`, set at
`cell/space_manager/spawn.rs:123`) and the killer's `player_id`; a missing
`player_id` warns and skips. The tag match is `event.params["entity_tag"] ==
trigger.entity_tag` (`content-engine/src/triggers/matching.rs:58-69`), so it is
spawn-source agnostic.

Related: [[spawn-timing-instanced-spaces]], [[harset-zone-evidence]]
