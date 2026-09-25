---
name: faction-derived-aggro-na13
description: NA13 (2026-09-25) made faction 10 hostile ON SIGHT via the reaction table; players react as wire faction 3 not server 0; chain-armed spawns need spawnlist.aggression_override=3; aggression wire is onAggressionOverrideUpdate, not onEntityProperty(6)
metadata:
  type: project
---

NA13 (branch `npcai/na13-faction-aggro`, 2026-09-25) replaced `CellEntity::aggression: i32`
with `aggro: AggroProfile { override_level: Option<MobAggression>, radius_override }`.
Effective aggression = override, else `FACTION_REACTION_TABLE[3][npc.faction]`
(`cell/combat/faction_reaction.rs`, pinned to `entities/defs/enumerations.xml` by a test).

**Why it matters / traps:**
- Faction 10 now **aggroes on sight** (18 u horizontal, |dy| <= 4, navmesh LoS with Unknown
  failing closed where a mesh exists). Every earlier note saying "faction 10 alone does not
  aggro" or "aggression is always 0" is stale. SGC_W1 Ba'al Jaffa, Castle guards/PRUs,
  Romney/Muelbach/Bravo officers all became proximity-hostile.
- Players react as faction **3** (`mercury::aoi::PLAYER_FACTION`, what the client is told);
  their server `CellEntity::faction` stays 0. Row 0 of the table is NEUTRAL toward 10, so
  using the server faction silently disables all aggro.
- A mob a chain must start needs `spawnlist.aggression_override = 3` (NEUTRAL) or
  `spawn_entity {"aggression": 3}`; only spawns 20 and 10 carry it. Content level 0 = NEUTRAL
  (pre-NA13 "passive"), `None` = faction-derived. Surrender sets `Some(Neutral)`, never `None`.
- The `same_faction` gate compares SERVER factions, so a HOSTILE override on a faction-0 NPC
  still never aggroes.
- Client aggression display is `onAggressionOverrideUpdate` (SGWMob, derived index 27,
  unverified) handled at `0x00d31bd0`; not `onEntityProperty` type 6. No wire shipped.
- GM `.aggro off` is keyed by character id (`SpaceManager::gm_aggro_off`), proximity only.

**How to apply:** when a zone "pulls a whole room" or a scripted mob engages early, check
override vs faction first; see [[leash-reset-na12]] and [[faction-10-gates-everything]].
