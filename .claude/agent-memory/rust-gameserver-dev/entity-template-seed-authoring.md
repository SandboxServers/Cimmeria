---
name: entity-template-seed-authoring
description: Traps when authoring entity_templates / ability_sets seed rows — the one-ability-per-set primary key, the event_set_id animation gate, faction 10 being immutable and load-bearing, level bands hidden in texts.sql monikers, and the omitted-column pattern in the seed INSERT lists.
metadata:
  type: project
---

Learned authoring Harset rebuild packet H11 (33 new `entity_templates` rows plus
two `ability_sets`). Every item below was verified against code or the live DB,
not inferred.

## `ability_set_abilities` holds exactly ONE ability per set

`db/resources/_primary_keys.sql` declares `PRIMARY KEY (ability_set_id)` — the
set id **alone**, not the pair. `db/database.sql` applies `_primary_keys.sql`
(line ~264) *before* the seed files (line ~273), so a second row for the same set
is a duplicate-key error that aborts the whole DB load.

The Rust side is already multi-ready (`array_agg(... ORDER BY ability_id)` in
`cell/spawner/npcs.rs`, and `choose_npc_ability` walks N ids), so the key is the
only blocker. **Don't design a multi-ability NPC set without widening the key
first.**

`choose_npc_ability` sorts ascending and takes the first off-cooldown id, so in a
multi-ability set the *lowest id* is the workhorse and higher ids only fire while
it cools.

## An ability with `event_set_id = NULL` plays no animation

`cell/abilities/use_ability/handle.rs` gates the entire Ability_Begin (1000) /
Ability_End (1001) `onSequence` broadcast on
`ability_def.and_then(|d| d.event_set_id)`. A NULL there means the NPC deals
damage in silence — the "combat animations reach the client but don't play" bug
class.

Check `event_set_id` before putting any ability in an NPC set. In the SGW seed,
most of the interesting staff/melee abilities are NULL: 594 Strike, 540 Staff
Strike, 479 Staff Blast, 1482 Ground Blast, 1768 Double Blast. The auto-attack
block (579 pistol / 584 staff / 712 ribbon device) all have one.

Also watch `max_range`: `ability_ranges` returns the def value verbatim when
non-zero and falls back to `NPC_ATTACK_RANGE` (30) when it is 0. Ability 1482 has
`max_range = 3000`, which makes an NPC that selects it stop closing distance.

## `faction = 10` is immutable at runtime and gates two separate things

`HOSTILE_FACTION = 10` (`cell/combat/mod.rs`). Four gates read it:

- `cell/abilities/use_ability/handle.rs` — a player's single-target ability is
  **rejected** against any target whose faction is not 10. The damage pipeline is
  never entered. So a faction-1 NPC cannot be killed by a player, full stop.
- `cell/abilities/dispatch.rs` and `cell/abilities/cone_aoe/geometry.rs` — non-10
  entities are excluded from AoE/cone target sets.
- `cell/cell_methods/player/interaction/interact.rs` — right-click on an **alive**
  faction-10 NPC is rerouted to auto-attack instead of dialog. (Dead ones still
  reach `handle_interact` so corpses stay lootable.)

Nothing can change it at runtime: `set_aggression` writes only
`entity.aggression`, `Action::ModifyProperty` is declared in
`content-engine/src/actions.rs` with **no executor arm**, and there is no
`set_faction`. Consequence: **an NPC that is talked to in one mission and killed
in another needs two templates**, one per faction, sharing a `name_id` (only
`template_name` is UNIQUE).

Faction 10 alone does *not* make a mob attack on sight — the AI tick admits an
Idle NPC only when `aggression > 0` — so seed hostiles at faction 10 and let the
chain's `set_aggression` start the fight.

## Level bands are hidden in the `texts.sql` moniker names

The original designers encoded rank and level band in the display-name moniker:
`DN_Mb_<Zone>_<Who>_<Rank>_<lo>-<hi>_Fac`, e.g.
`DN_Mb_Harset_Malac_Lt_14-16_Fac`. That is far better evidence than the
`missions.level` column, which is an unset `1` placeholder on whole mission
chains. Grep `texts.sql` for `DN_Mb_` before inferring a mob level.

`level` itself only drives max HP (`200 + 50*level`), kill XP (`10*level`) and
`onLevelUpdate`. Nothing in AI, threat, range or leash reads it.

## Seed INSERT column lists omit newer columns

`entity_templates.sql` and `spawnlist.sql` both ship column lists that predate
`respawn_secs`, `is_stationary`, `wander_radius`, `move_speed` etc. To set one,
extend the column list **on the rows that need it only** — don't rewrite the
whole file. `respawn_secs` resolves as
`COALESCE(spawnlist.respawn_secs, entity_templates.respawn_secs)`, which means a
template default cannot be overridden to "never respawn".

`is_stationary` is a `spawnlist` column; there is no such column on
`entity_templates`.

## A template with neither `components` nor `static_mesh` is permanently invisible

`mercury/aoi/create.rs::append_appearance` picks one branch: `BeingAppearance`
when `body_set` *and* `components` are both non-empty, else
`onStaticMeshNameUpdate` when `static_mesh` is non-empty, else it logs
`aoi.cascade_appearance_missing` and gives up. The AoI tick marks the witness set
before delivery, so it never retries — the entity is invisible for the life of
the space. Props (NULL components) therefore **require** a `static_mesh`.

Similarly, `name_id` NULL means `onNameIdUpdate` is never sent and the NPC
renders with no name.

## `body_components` is not a usable validator for base-body parts

It looks like a registry of legal `component_name` → `body_sets` pairs, but 132
of the 153 shipped templates fail a `body_set = ANY(bc.body_sets)` check against
it. It *is* reliable for weapon components (`WP-*`), which is the useful half
when adding a weapon to a template.

## `weapon_item_id` on `entity_templates` has zero consumers

It appears only under `db/resources/`. The visible weapon comes from the
`components` array (`WP-Jaffa.WP_Staff_Plasma_4A`,
`WP-Goauld.WP_Ribbon_Elec_1A`, `WP-Human.WP_SMG_1A`, `WP-Human.WP_Pistol_1A`).
Several shipped Jaffa templates have a staff ability set and no `WP-*` component,
so they mime the weapon.

## `flags` on a template is `EEntityFlags`, sent straight to the client

`entities/defs/enumerations.xml` `EEntityFlags`. `4` is
`ENTITYFLAG_DoNotDrop` — the value every interactable prop in the seed carries.
`24` on the pet template is `NoPetLeveling | NoPetTargeting`.

Related: [[stat-with-no-consumer-trap]], [[cell-entity-direction-semantics]].
