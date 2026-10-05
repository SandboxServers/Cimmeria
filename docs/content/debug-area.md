---
title: "Debug Area"
type: reference
audience: engineers, testers
last_updated: 2026-10-05
---

# Debug Area

The Debug Area is server world **1300 `DebugArea`**, a GM test map on the shipped
client map Ihpet_Crater_Light (decisions D-DA1 to D-DA9 in the
[campaign plan](../analysis/debug-area/README.md)). It is shared and always
loaded, and only GMs can get there: `.gotolocation DebugArea` (or the native
`/gmgotolocation`) lands on respawner 130's point at the south compound, which is
the stargate's arrival pin (see [Stargate](#stargate)).
`.gotolocation DebugArea <x> <y> <z>` moves you to those coordinates; it
always moves you, never your selection (DA-F4). Each zone
tests one group of server systems. Every spawn carries a `DebugArea_*` tag;
the stasis-room hub's tests count `DebugHub_*`, so the two never mix (D-DA6).

**GM only, enforced by the server.** An account below GameMaster (access
level 2) never enters world 1300. At login and on every cross-world
transfer (stargate, ring, content teleport, a respawner in another world,
a GM `.summon` or `.gotolocation`) the base sends such a player to their
faction's character-creation start instead (Praxis: the Castle_CellBlock
stasis room; SGU: SGC_W1). Once that world has loaded they read "The Debug
Area is for GMs only. You have been returned to your faction's starting
point.", and the base logs a WARN `gm_only_world_refused` naming the
player, the account and both worlds. The rule lives in
`crates/base-session/src/base/world_entry/gm_only_worlds.rs`.

This page covers the zones from packets DA-02, DA-03 and DA-04, the
stargate (DA-07), the ring transports that link them (DA-08) and the
System Lords' summit (DA-09).
[Which station tests what](#which-station-tests-what) (DA-05) maps each
restored system to its station and UAT step. Each packet's rows are
in their own seed files:

- DA-02 (Z2 services plaza, Z3 training dummies): the `*_debug_area_plaza.sql`
  files (templates 1300-1302 and 1310-1314, spawns 13001-13022 and
  13100-13104, buy list 1300, chains 13000-13008), inside DA-02's block
  (templates 1300-1329, spawns 13000-13199). See
  [Services plaza and dummies](#services-plaza-and-dummies).
- DA-03 (Z4 faction yard, Z5 AI behaviour slope, Z7 enemy gallery):
  `db/resources/Entities/Seed/entity_templates_debug_area_npcs.sql` (templates
  1330-1333 and 1340-1343), `db/resources/Events/Seed/point_sets_debug_area_npcs.sql`
  (patrol set 13200) and `db/resources/Worlds/Seed/spawnlist_debug_area_npcs.sql`
  (spawns 13200-13245 and 13300-13398), inside DA-03's block (templates
  1330-1369, spawns 13200-13599).
- DA-04 (Z6 arena, Z8 cover course, Z9 death and respawn test):
  `db/resources/Entities/Seed/entity_templates_debug_area_combat.sql` (templates
  1370-1377) and `db/resources/Worlds/Seed/spawnlist_debug_area_combat.sql`
  (spawns 13600-13641), inside DA-04's block (templates 1370-1399, spawns
  13600-13799).
- DA-09 (the System Lords' summit): `entity_templates_debug_area_lords.sql`
  (templates 1400-1406), `spawnlist_debug_area_lords.sql` (spawns
  13850-13856) and `db/resources/Dialogs/Seed/ambient_chatter_lords.sql`
  (chatter group 1), inside DA-09's block (templates 1400-1409, spawns
  13850-13869). See [System Lords' summit](#system-lords-summit).

Heading 0 faces +Z and -1.5708 faces -X.

Heights are BigWorld metres. Every DA-04 NPC is a mobile `mob`, so the spawner
grounds it onto `ihpet_crater_light.nav`. On open terrain the navmesh and the
occluder's terrain disagree by up to 2 m, so rows are authored near the mesh
height and grounding does the rest.

> **Placement is checked on the server's data, not in the client.** The rows
> are tested on the real navmesh, occluder and cover markers (below), but
> nobody has walked them in the client yet. That is packet DA-06.

## Which station tests what

DA-05's map from each restored system to the station that tests it. The step ids are the [unified UAT guide's Debug Area section](../guides/unified-uat.md#debug-area); the lab runs the same rows from [debug-area.toml](../guides/uat-specs/debug-area.toml). Systems tied to their own map (the Castle CellBlock tutorial, Castle, Harset, the historical CellBlocks and the Cellblock-to-Castle ring) stay tested in their own zones.

| System (unified UAT section) | Station | Steps |
|---|---|---|
| Travel and GM-only access | Z1 arrival, respawner 130 | DA-U1, DA-U2 |
| Ability trees, ability mechanics | Z2 ability granter and reset NPC, trainer; Z3 dummies; `.effects`, `.cooldowns` | DA-U3 to DA-U5, DA-U19 to DA-U21 |
| Vendors, special ammo, consumables | Z2 vendor and munitions vendor | DA-U6 to DA-U8 |
| Dialog UI | Z2 Airman Lance (dialog 5738) | DA-U9 |
| Minigames | Z2 Livewire terminal | DA-U10 |
| Loot | Z2 crate, any Z7 or Z9 kill | DA-U11, DA-U31, DA-U37 |
| Bank and vault, organizations | Z2 Bankers and registrars | DA-U12, DA-U13, DA-U15 |
| Mail | Z2 Sgt. Harriman | DA-U14 |
| Black market | Z2 Machra | DA-U16 |
| Crafting | Z2 supplies vendor and the four stations | DA-U17 |
| Pets | Z2 Goa'uld Advanced Skills | DA-U18 |
| NPC AI: factions, aggro radius, assist | Z4 faction yard, Z5 assist trio | DA-U22 to DA-U24, DA-U28 |
| NPC AI: patrol, wander, leash | Z5 AI behaviour slope | DA-U25 to DA-U27 |
| Combat against every hostile template | Z7 enemy gallery | DA-U29 to DA-U31 |
| NPC-vs-NPC (#1009) | Z6 arena | DA-U32 to DA-U34 |
| Cover and line of sight | Z8 cover course | DA-U35, DA-U36 |
| Death and respawn | Z9 death yard, respawners 130 and 131 | DA-U37, DA-U38 |
| Starter kit and seeded characters | A new character at Z3; the seed | DA-U39 to DA-U44 |
| Gate travel (outbound) | The Debug Area stargate (DA-07, #1232) | DA-U45 |
| Ring transport | The eight ring stations (DA-08, #1234) | DA-U46 |
| NPC appearance, ambient chatter | The System Lords' summit (DA-09) | DA-U47 |
| GM console parity | Anywhere in world 1300 | [GM console command parity](../guides/unified-uat.md#gm-console-command-parity) |

What only a live client can settle is the ordered [DA-06 checklist](../analysis/debug-area/README.md#da-06-live-client-checks).

## Heights in the DA-03 zones

On open terrain the navmesh and the client's terrain (the occluder) disagree
by up to 3.3 m. Every DA-03 spawn takes its y from the occluder's terrain top,
at a spot where the navmesh is within 0.6 m of it (1.0 m at the wanderer). The
faction yard rows sit at z -806 to -794, south of the plan's centre
(416, -786), because north of z -790 the two surfaces drift 2 to 4 m apart.
The test `service::tests::npc_ai::debug_area` in `cimmeria-cell` checks every
row against both surfaces.

## Faction yard

Zone Z4, on a gentle slope rising east. Three rows face west: friendly at
x 396, neutral at x 412, and a hostile pen at x 436 to 440.

| Row | What it shows | Can be shot | Fights |
|---|---|---|---|
| Friendly | Two factions the reaction table makes friendly to players: 1 (World Object, what every seeded friendly NPC uses) and 9 (Friendly_Ambient). | No | Never |
| Neutral | Faction 7 (Neutral_Ambient): neutral to players. | No | Never |
| Neutral, pinned | A faction-10 Jaffa with `aggression_override` 3 (NEUTRAL) on the spawn: passive, but damageable. | Yes | Only when shot |
| Hostile pen | Three faction-10 templates as seeded elsewhere: 35 Ba'al's Jaffa (592 fallback) at the front, 24 NID Guard (SMG set 3) at the back, 78 Straegis Fighter (592 fallback). Hostile on sight. | Yes | On approach |

Players can damage only faction 10 (`player_may_attack_pve`), so the neutral
row answers the plan's question "damageable or not" with **not**: a neutral
faction-7 NPC cannot be attacked at all. The pinned Jaffa is the damageable
neutral, the same mechanism as the gallery. None of the friendly or neutral
factions has an enemy in the reaction table, so no row ever starts an
NPC-vs-NPC fight.

Faction 3 (Praxis, what players react as) is left out of the friendly row on
purpose. It is mutually hostile with faction 10, so a faction-3 friendly would
fight any pen NPC that chased a player within 18 u of it. NPC-vs-NPC is the
arena's job (DA-04, D-DA8).

How to test:

- **Friendly and neutral never aggro.** Walk the friendly and neutral rows:
  nothing engages. The pen is 24 u or more from the neutral row.
- **Proximity radius.** Walk east from the neutral row. The pen engages at the
  default 18 u radius, about x 418 to 422.
- **Assist inside the pen.** Shoot one pen NPC from more than 18 u away: the
  other two join (default assist radius 10 u; the NID Guard's template radius
  is 26 u). The pinned Jaffa never joins: NEUTRAL NPCs are refused by the
  assist gate.
- **The converse.** Shoot a pinned Jaffa: only that Jaffa fights. The NID
  Guard, the one pen NPC with a long assist radius, stands at the back of the
  pen, 28 u from both pinned Jaffa, outside its 26 u radius whatever the
  height difference.
- **Line of sight and the vertical band** are not separately staged here: the
  yard is open ground and the pen is 6 to 7 m above the friendly row. The
  Castle Cellblock and Castle cover them (`aggro_castle`, `assist_barracks`).
- **Respawn.** Damageable rows respawn after 30 s.

## AI behaviour slope

Zone Z5, west of the yard: a slope rising north from y 12 to y 22 at x 26 to
28, the open ground east of it, and a flat dry basin at x 116. Every NPC is
faction 10 and level 1, falls back to 592 Pistol Shot, and respawns after 30 s.

| Station | Template | Behaviour | Tuning |
|---|---|---|---|
| Patrol | 1340 Jaffa Foot Soldier | Walks point set 13200 `DebugArea.Patrol.Slope` from A (28, 12.39, -760) to B (26, 21.83, -644) and back, 4 s at each end. | Default aggro 18 u |
| Wander | 1341 Lucian Scavenger | Wanders within 8 u of (74, -0.25, -746), 3 to 8 s at each stop. | Default aggro 18 u |
| Leash | 1342 Svarog Jaffa | Kite it east, downhill: past 15 u (plus the 5 u band) from its spawn it gives up, walks home and evades. | `leash_distance` 15, `aggro_radius` 12 |
| Assist trio | 1343 Beleth Jaffa | A at (116, -696); B 6 u north of A; C 14 u south of A. | `aggro_radius` 6, `assist_radius` 10 |

How to test:

- **Patrol.** Watch the patroller cross the slope. Approach it and it breaks
  off to fight; after the fight it leashes home and resumes the route.
- **Wander.** The wanderer stops and moves on inside its disc. It is hostile,
  so it engages a player inside 18 u.
- **Leash.** Pull the leash Jaffa (within 12 u) and run east past x 85. It
  turns back, walks to (60, 5.08, -680) and ignores shots on the way (NA12
  evade). It does not re-aggro for 5 s after the reset.
- **Assist.** Stand 10 u or more west of the trio (their aggro radius is 6 u,
  so the approach pulls nothing) and shoot A: B joins, C stays. Shoot C: nobody
  joins (C is 14 u from A and 20 u from B). Assist never chains.

Approach the leash station from the south or east: the patrol route is 33 u
west of it.

## NPC-vs-NPC arena

Zone Z6 is the **east shelf**, a flat terrain terrace at y -11.12 east of the
sunken pit, about x 318-395, z -680 to -749. Two fights run there (D-DA8). Both
are NPC-vs-NPC combat from
[#1009](../gameplay/npc-ai.md#npc-vs-npc-combat-1009).

**Why not the pit (DA-F2).** DA-04 put the arena on the pit floor at
(250, -32.4, -725). That floor is a water collision plane
(`WaterCollisionPrefab_Square`, y -33.28) over a lakebed 10 to 25 m lower. The
navmesh lies on the water, so the squads stood on it, but a player sinks
through: the DA-06 live-client run found the player at y -52.03 and the squads
drawn 19 m overhead. DA-F2 moved both fights to the shelf, 22 m above the
water.

- **Ground.** The shelf is terrain: the occluder's layer is Terrain, and the
  navmesh agrees with it within 0.01 m. None of the map's nine fluid volumes
  reaches it.
- **Walls.** The shelf is the floor of a ruin, and its low walls block sight at
  eye height almost everywhere. Fight 1 stands in the one strip where a
  3-against-3 line 24 m apart has clear sight: between the long east-west wall
  at z -736 and the south edge wall at z -749. A grid search on the occluder
  found it. The rows are 3 m apart: with the third pair at z -746, the third
  NID guard saw past the long wall's west end and pulled a tester standing in
  the fight-2 room (#1244 review).
- **Getting there.** The south edge is a 3 to 14 m drop. The way on is from
  the east, through the gap at x 390-400, z -738 to -744. DA-08's faction-yard
  ring pad (394, -10.59, -738) stands in that gap.
- **Watching.** The terrain north of the shelf is 17 to 20 m above it. The
  `arena.rs` test checks it as a spectator spot that nothing pulls.

**Fight 1, Praxis against NID (joinable).** Faction 3 (Praxis) against faction
10 (Straegis), the only seeded mutually hostile pair. Players react as faction
3, so the Praxis squad is friendly to a player and cannot be damaged by one.
The NID squad is hostile and damageable.

- **The lines.** The NID guards hold the west line (x 354) and the Praxis the
  east line (x 378), so the guards stay 40 m from the faction-yard pad.
- **Joining.** Walk in from the east past the Praxis line. Inside 30 m of the
  guards they take you, and you fight on the Praxis side.
- **Kill credit.** Credit goes to the killing blow. Finish a NID guard the
  Praxis wore down and you get the full XP and loot; wear one down and let a
  Praxis finish it and you get nothing. That is the #1009 rule, not a bug.

| Spawn | Template | Tag | Name shown | Faction | Position (x, y, z) |
|---:|---:|---|---|---:|---|
| 13600 | 1370 | `DebugArea_Arena_Praxis1` | Op-CORE Soldier | 3 | (378, -11.12, -738) |
| 13601 | 1371 | `DebugArea_Arena_Praxis2` | Praxis Jaffa Guard | 3 | (378, -11.12, -741) |
| 13602 | 1370 | `DebugArea_Arena_Praxis3` | Op-CORE Soldier | 3 | (378, -11.12, -744) |
| 13603 | 1372 | `DebugArea_Arena_NID1` | NID Guard | 10 | (354, -11.12, -738) |
| 13604 | 1372 | `DebugArea_Arena_NID2` | NID Guard | 10 | (354, -11.12, -741) |
| 13605 | 1372 | `DebugArea_Arena_NID3` | NID Guard | 10 | (354, -11.12, -744) |

**Fight 2, Lucia Green against Lucia Yellow (spectators only).** Faction 27
against faction 29, one of the player-safe hostile pairs, in the open room
north of the long wall.

- **Distance from fight 1.** Fight 2 stands 39 m or more from every NID guard.
  A tester can walk right up to it, or loot-check a corpse, and stay outside
  the guards' 30 m aggro radius, so fight 1 never pulls them in.
- **Factions.** Each side is hostile to the other and friendly to faction 3
  both ways. Neither is hostile to faction 3 or 10, so fight 1 and fight 2
  never mix. No player can damage either side (only faction 10 is
  damageable).
- **No pay.** The fight shows that an NPC-only kill pays nobody. Both
  templates carry loot table 2, yet the corpse rolls no loot, shows no loot
  cursor, pays no XP and moves no mission
  ([combat-system.md](../gameplay/combat-system.md#npc-vs-npc-kills-pay-nobody-1009)).
  The server logs `loot.drop event=skipped reason=npc_only_kill loot_table_id=2`.

| Spawn | Template | Tag | Name shown | Faction | Position (x, y, z) |
|---:|---:|---|---|---:|---|
| 13610 | 1373 | `DebugArea_Arena_Green1` | Green Sniper | 27 | (364, -11.12, -700) |
| 13611 | 1373 | `DebugArea_Arena_Green2` | Green Sniper | 27 | (364, -11.12, -696) |
| 13612 | 1374 | `DebugArea_Arena_Yellow1` | Yellow Faction | 29 | (343, -11.12, -700) |
| 13613 | 1374 | `DebugArea_Arena_Yellow2` | Yellow Faction | 29 | (343, -11.12, -696) |

**Why it fights, and why it repeats.**

- The squads stand 24 m apart (fight 2: 21 m). The NID guards set
  `aggro_radius` 30, the Praxis 28 and fight 2 26, so each side's Idle scan
  reaches the other. The default radius, 18 m, would not.
- The Praxis radius is 28, not 30. The Idle scan considers NPCs within twice
  the radius, and the Praxis line is 59 m from the faction yard's damageable
  faction-10 rows. At 28 the scan's reach is 56 m, which keeps the yard out
  (the `debug_area::reach` guard).
- The scan runs only while a player has the NPC in their AoI (150 m). From the
  services plaza the shelf is about 230 m away. Walk to the faction yard or the
  terrain north of the shelf to start the fights. A fight already under way
  finishes without a witness.
- The other way round: any player within 150 m keeps the arena cycling. That
  includes testers at the faction yard (about 60 m) and, just, the Gallery
  east ring pad (about 149 m); the dummy range, the AI slope and the gallery
  rows are out of range. Every round writes `npc_ai.aggro
  cause=proximity` and `loot.drop reason=npc_only_kill` rows, so filter
  SigNoz queries on the `DebugArea_Arena_` tag prefix (or exclude it) when
  you read another station's AI or DPS telemetry.
- Every arena row respawns 30 s after death (template `respawn_secs`). A
  winner whose threat list empties with the kill resets: it walks home (a
  step, since it fights from its spawn), heals to full and goes Idle. The
  losers come back and the scan starts the next round, after the 5 s
  post-reset window (NA12).
- These post-kill resets are not a leash loop. A leash triggered by
  `target_dead` or `target_gone` is not counted toward `npc_ai.leash
  event=loop` (DA-F2). Before that, every arena fighter that won three rounds
  inside 60 s wrote the loop WARN, which the first 30 minutes of
  v2026-10-05.1 on the colo showed for six arena rows.
- With no player in AoI, the fights' dropped health, state and attack
  updates write nothing. A no-witness WARN is written only when a player is
  involved (see
  [npc-ai.md](../gameplay/npc-ai.md#npc-vs-npc-combat-1009)).
- The arena squads carry no loot (fight 1) and no `use_cover` (there is no
  cover marker on the shelf). The fight-1 Jaffa fires the Jaffa staff set (4)
  and carries the staff, the others the SMG set (3); the Yellow Faction pair
  keeps the pistol set (1) of its look's template, 219.

**Spectators are not pulled.** The aggro scan has a 4 m vertical band. A
player watching from the terrain north of the shelf is 17 to 20 m above it and
never becomes a candidate. A player on the slope below the south edge is 7 m
or more below it. A player on the shelf beside fight 2 is 49 m or more from the
nearest NID guard. The Praxis and Lucia rows never target a player at all.

## Enemy gallery

Zone Z7, the terrace at y 23.06: two rows, z -592 (facing south) and z -612
(facing north), at x 55 to 171 and x 305 to 465. It holds every hostile
(faction 10) template once, 4 m apart (8 m for the large bodies: BattleWalker,
Carnosaur, Horden, Rhinolion, Straegis Titan, Twilla Tree), grouped by family:

| Family | Count | Where |
|---|---:|---|
| Machines and drones | 4 | z -592, x 55-69 |
| Straegis | 3 | z -592, x 79-89 |
| Creatures | 7 | z -592, x 99-137 |
| Humans: NID and SGC | 16 | z -592 x 147-171, then z -612 x 55-95 |
| Humans: Lucian | 2 | z -612, x 103-107 |
| Goa'uld | 1 | z -612, x 115 |
| Jaffa elites (named) | 5 | z -612, x 123-139 |
| Jaffa, male | 27 | z -612 x 147-171, then z -592 x 305-409 |
| Jaffa, male, Unas armour | 6 | z -592, x 417-437 |
| Jaffa, female | 22 | z -592 x 445-465, then z -612 x 305-365 |
| Jaffa, female, Unas armour | 6 | z -612, x 373-393 |

Every gallery spawn (D-DA9):

- is **passive**: `aggression_override` 3 (NEUTRAL), sent to the client as
  `onAggressionOverrideUpdate`. Walking the line pulls nothing, a gallery NPC
  never looks for NPC targets, and the assist gate refuses NEUTRAL NPCs, so a
  single-target shot never pulls its neighbours 4 m away. An area ability
  (ground AoE, cone) damages every gallery NPC it reaches, and each one it
  hits fights back;
- is still **damageable** (the damage gate is faction 10) and **fights back**
  when shot, then leashes home at the default 50 u;
- **respawns after 20 s**, so the line refills;
- has its own tag, `DebugArea_Gallery_<template_id>` (for example
  `DebugArea_Gallery_24` is the NID Guard).

The override only narrows what a gallery NPC (or a pinned Jaffa) seeks. It is
still a valid *target* for an NPC whose faction is hostile to 10 (factions 2,
3, 11 and others), so such an NPC must never be placed within its scan radius
of the gallery; the reach guards below check that.

The Twilla Tree (80) and the Straegis Beacon (77) are `is_stationary`: a plant
and a beacon that walked would be wrong.

**Placed: 99 of the 101 hostile templates.** 24 carry an ability set (sets 1
to 5: the drones, the NID and SGC humans, the named Jaffa and the Goa'uld); the other 75 fall back to 592 Pistol Shot, so a creature fires a
pistol until it gets its own kit. 62 have no display name (no `name_id`, or an
empty moniker such as 4's): their nameplate is blank, which is why each one
has a tag.

**Excluded**, with the reason (the live-DB guard
`live_db_debug_area_npcs` names them in `GALLERY_EXCLUSIONS`). "Hostile"
is every faction row 3 of the client's reaction table marks HOSTILE to players
(10, 11, 13, 17, 21, 22, 24-26, 28, 30, 32, 34 and 41), derived from
`enumerations.xml` at test time; only faction 10 is seeded on a mob today.

| Template | Name | Why |
|---:|---|---|
| 140 | NPC Child 1 | A child, not an enemy; faction 10 is a seed artefact. |
| 141 | NPC Child 2 | A child, not an enemy; faction 10 is a seed artefact. |

The Debug Area's own station templates (1300-1399) are placed at their
stations, not in the gallery. A hostile template added later makes
`live_db_debug_area_npcs::debug_area_npcs_live_db_gallery_covers_every_hostile_template`
fail until it is placed here or excluded with a reason.

No gallery NPC is in reach of another station: the nearest hostile, the
patroller's B end, is 43 u from the gallery's west end, against an 18 u aggro
radius and a 10 u assist radius.

## Cover course

Zone Z8 is the west wing of the south compound, entered through the doorway
at (204, 7.0, -926). It is open to the sky and split by ruined walls: a long
wall on x 151 separates the hall (x 151-203) from a west room, which has a
doorway at z -961 to -963. The riflemen are hostile, ranged NID guards
(template 1375: faction 10, SMG set 3, `use_cover` true, `aggro_radius` 25,
level 5, loot table 2, 30 s respawn). They use NA22/NA23 cover and the
ranged step-back (D-NA15) described in [npc-ai.md](../gameplay/npc-ai.md#cover-system)
and [cover-system.md](../architecture/cover-system.md).

| Spawn | Tag | Position (x, y, z) | Starts |
|---:|---|---|---|
| 13620 | `DebugArea_Cover_Rifleman1` | (166, 6.7, -930) | In the open in the hall. Seeks a slot when shot at. |
| 13621 | `DebugArea_Cover_Rifleman2` | (142, 6.7, -958) | In the open in the west room, facing its doorway. Seeks a slot. |
| 13622 | `DebugArea_Cover_Rifleman3` | (181.33, 6.58, -964.62) | Holding Mid/Better marker 130000029/1 on the hall's south wall, 0.8 m behind it, facing north into the hall. |

The world-1300 cover markers come from DA-01's extraction of the map
(6,324 nodes in 706 sets, set ids `1300 * 100000 + n`). Same-floor markers
near each station:

| Around | within 10 m | within 20 m | within 30 m |
|---|---:|---:|---:|
| Rifleman 1 (166, -930) | 25 in 4 sets | 67 in 14 sets | 147 in 24 sets |
| Rifleman 2 (142, -958) | 14 in 4 sets | 76 in 15 sets | 130 in 26 sets |
| Rifleman 3 (181.3, -964.6) | 22 in 4 sets | 51 in 9 sets | 112 in 14 sets |
| Doorway (204, -926) | 13 in 2 sets | 72 in 6 sets | 135 in 12 sets |

What a tester can check:

- **Spawned in cover (NA22).** Rifleman 3 holds its marker from spawn
  (`cover.hold event=spawn_reserved`). Walk down the hall within 25 m of it and
  it engages you from the slot, firing past its prop from the peek point
  (NA23), with Cover Stance. Its marker is 37 m or more from the other two
  riflemen, beyond the 30 m a seeking NPC searches, so neither takes it while
  rifleman 3 is dead: it respawns holding it every time.
- **Seeking cover.** Shoot rifleman 1 from the hall (for example from
  (186, 6.7, -935)) or rifleman 2 from its doorway (156, 6.7, -962). It walks
  to a free marker facing you, stops with zero velocity and takes Cover
  Stance. The three riflemen are too far apart to assist each other, so each
  is a separate fight.
- **Flanking.** Walk round a rifleman holding cover until you are more than
  20 degrees past side-on. It gives the slot up and does not take the same
  slot again for 6 s.
- **Step-back.** Close to within 2 m of a rifleman that is not in cover. It
  steps back about 5 m, at most once every 3 s.
- **Line of sight through the walls.** The occluder blocks rifleman 2's view
  through the x 151 wall: a player 24 m away in the hall at (165, 6.7, -950)
  is not acquired, but a player in the west room's doorway is.
- **Player cover.** A player standing within 1.5 m of a marker counts as in
  cover against a rifleman's shot from in front of it (NA32 damage reduction;
  players hold no slot and get no stance). Players use the same LoS rule as
  NPCs: a shot through the x 151 wall is refused with "You do not have Line of
  Sight to your target" (NA31).

## Death and respawn test

Zone Z9 is open, rising terrain around respawner **131** 'Debug Area Respawn
Test' (438, 10.4, -916). Respawner **130** 'Debug Area Arrival'
(251, 8.0, -962) is the other world-1300 respawner.

| Spawn | Template | Tag | Name shown | Position (x, y, z) | What it is |
|---:|---:|---|---|---|---|
| 13630 | 1376 | `DebugArea_Death_Operative1` | NID Operative | (466, 13.33, -911) | Lethal squad |
| 13631 | 1376 | `DebugArea_Death_Operative2` | NID Operative | (466, 13.91, -915) | Lethal squad |
| 13632 | 1376 | `DebugArea_Death_Operative3` | NID Operative | (466, 14.25, -919) | Lethal squad |
| 13633 | 1376 | `DebugArea_Death_Operative4` | NID Operative | (469, 12.86, -913) | Lethal squad |
| 13640 | 1377 | `DebugArea_Respawn_Fast` | Cellblock Guard | (430, 9.86, -910) | Respawns 10 s after death |
| 13641 | 1377 | `DebugArea_Respawn_Slow` | Cellblock Guard | (430, 10.03, -922) | Respawns 30 s after death |

**The lethal squad.** Four level-31 NID Operatives (1,750 HP each) with the
SMG set (559, 200 Focus / 20 Health a shot, every AI tick). They stand 28 to
31 m east of respawner 131, over a rise that blocks the line of sight from it.
Their `aggro_radius` is 10, so you have to walk up to them. The closest one
acquires you and the other three join through same-room assist. A fresh
character (a Commando starts with 760 Health and 1,570 Focus) dies in five AI
ticks, about 10 s, measured with the seeded abilities; the live-DB smoke
allows seven. They respawn 60 s after
death and carry loot table 2.

**The respawn-timer targets.** Two level-1 guards (250 HP) seeded NEUTRAL
(`aggression_override` 3). They do not aggro but fight back when shot, and
they are faction 10, so a player can kill them. Both use template 1377,
whose `respawn_secs` is 30. Spawn 13640 overrides it with its own 10, which
shows `spawnlist.respawn_secs` taking precedence over the template's. They
carry loot table 2, so their corpses exercise loot.

### Procedure

1. `.gotolocation DebugArea`, then go east to respawner 131.
2. **NPC respawn timer.** Kill the Fast guard. Note the time of death: it
   stands up again at its spawn 10 s later, the Slow one 30 s after its
   death. Loot the corpse before it comes back. The server logs the respawn
   in the NPC respawn tick.
3. **Player death.** First make sure you can be found and killed: `.aggro on`
   (a GM with `.aggro off` is never acquired) and god mode off. Then walk east
   over the rise to the NID Operatives. They kill
   a low-level character in seconds. The Defeat Window opens with the
   world's respawners. That should list 130 and 131; check the list in the
   client.
4. **Respawn point.** Pick 131 and you come back beside it. Pick 130 and you
   come back at the arrival. Let the timer run out (auto-respawn) and you come
   back at the respawner nearest your death, which is 131
   (`player.respawn reason=respawner_id_unset`, `distance_m`).
5. **State reset.** After the respawn, Health and Focus are full, the dead and
   movement-lock flags are cleared, cooldowns are reset, and you are out of
   combat. The squad has dropped you from its threat lists and walked home.
   The server applies no death penalty: no XP loss and no durability loss.
6. **No re-death.** Your death drops you from every threat list at once and
   the squad walks home evading; a walking-home NPC does not look for targets.
   Home again, it looks only 10 m around itself, and respawner 131 is 28 to
   31 m away behind a rise. So even when the squad chased you onto 131 and you
   click Release straight away, you are not shot again (pinned by a test).

No fall or environmental damage exists on the server, so Z9 has no fall-death
test.

## Ring transports

DA-08. Eight ring stations let a tester travel between the Debug Area's zones
the way rings work elsewhere in the game: the rings rise around the pad, flash
and drop, and the travellers arrive on the destination pad as its rings drop
there. Every station reaches every other one.

### Stations

Positions are BigWorld metres. "Pad" is the arrival point the server seeds
(the rig's base platform plus 0.537 m, as in Castle CellBlock). The console is
the ring switch beside the pad that you right-click.

| Station | Pad (x, y, z) | Serves | Console tag | Region |
|---|---|---|---|---|
| Compound | (224.0, 7.44, -938.0) | Z1 arrival (36 m), Z2 services plaza (32 m), Z8 cover course entry (23 m); Z3 dummies are 74 m north in the same compound | `DebugArea_Ring_Compound` | 35 |
| Faction yard | (394.0, -10.59, -738.0) | Z4 faction yard, 52 m south | `DebugArea_Ring_FactionYard` | 36 |
| AI slope | (81.0, 0.59, -782.0) | Z5 wanderer (36 m) and patrol (57 m) | `DebugArea_Ring_AiSlope` | 37 |
| Pit overlook | (176.0, -6.65, -702.0) | The pit's west ledge, 26 m above the pit (a water plane, K27). No longer an arena-viewing spot: since DA-F2 moved the arena to the east shelf it is 167 to 205 m from the squads, outside the 150 m AoI, so a GM who rings here neither sees nor starts the fights | `DebugArea_Ring_ArenaRim` | 38 |
| Arena shelf | (331.0, -10.58, -693.0) | Z6 arena, the east shelf, beside the fights: fight 2 (Lucia) is 12 to 33 m away, the fight 1 NID guards 50 m and its Praxis 65 m. The station to use for the arena | `DebugArea_Ring_ArenaPit` | 39 |
| Gallery west | (127.0, 23.60, -559.0) | Z7 enemy gallery west half, 33 m north of the rows | `DebugArea_Ring_GalleryWest` | 40 |
| Gallery east | (436.0, 23.63, -566.0) | Z7 enemy gallery east half, 26 m north of the rows | `DebugArea_Ring_GalleryEast` | 41 |
| Death yard | (437.0, 11.84, -937.0) | Z9 death and respawn test and respawner B, 21 m north | `DebugArea_Ring_DeathYard` | 42 |

No station is within 25 m of a hostile that can aggro (DA-03 and DA-04
checked their seeded rows against these points). Region 38 keeps its place as a valid travel point (the Pit overlook); region 39 kept its id, tag
and sequences (`DebugArea_Ring_ArenaPit`, `..._Seq_3`) when DA-F1 moved it
off the pit: the pit "floor" is the water collision plane at y -33.28, which
the navmesh treats as ground, so the first pad (210, -33.28, -725) rendered
on the water surface and players who stepped off sank to y -52. The new pad
is the largest clear disc on the east shelf (radius 6.5 m of flat Terrain at
y -11.12, nothing solid up to 8 m above it, found by DA-F2 on the real
occluder and navmesh data); the console is 3 m east and 1.3 m south of it.

### Using a ring

1. Right-click the console beside a pad. The destination list opens on the
   world map, with the other seven stations as transporter icons.
2. Pick a destination, then step onto the pad within 60 s.
3. The rings play for about 4 s, you are moved, the destination pad's rings
   play, and you can move again about 5.5 s after arriving.

Anyone standing on the pad when it fires travels too. Players who can see
the traveller see the source rings animate as well. A player watching only
the destination pad may not see its rings drop: see the known limitation in
[ring-transport-system.md](../gameplay/ring-transport-system.md#kismet-sequences-ue3-visual-effects).

### What needs the client patch

The rigs exist only in client patch `011-debug-area-rings-fix`
([data/client-patches](../../data/client-patches/README.md)), which needs
`007-castle-armory-ring` applied first (or `010-debug-area-rings`, which it
repairs). Without it the consoles and the trip still work, but there is no
ring hardware on the pads and no animation.

**010 is retired: it hung the client.** 010 was published for about half an
hour on 2026-10-05 and pulled. Any client that loaded the Ihpet Crater map
with it (world 1300, or world 73) took an access violation at `SGW.exe+0xbc6a0`
and froze. The cause was one name flag, not the rig: the cloned
`LightingChannels` struct names a property `Dynamic`; in Castle's name table
that entry loads on the client, in Ihpet's it is editor-only, and the client
reads an editor-only name as `None`, which ends the property list early.
Everything after it in the object is read three bytes off, and the next tag
name is garbage. The cloner now gives a name its own client-loadable entry
when the target's entry is narrower. `011` ships the rebuilt chunk, and it
upgrades a 010 install in place from 010's own output: see the
[patch README](../../data/client-patches/README.md#011-debug-area-rings-fix),
which also has the byte-level evidence.
World 73 (the live Ihpet Crater) uses the same map file, so it shows the eight
ring platforms too. They do nothing there: world 73 has no pads, consoles or
chains. The platforms and rings block players on the client
(`bBlockActors`) but not on world 73's navmesh, which is fine for scenery.
None is within 50 m of the DHD or the gate region. The patched chunk is
the first Ihpet chunk to reference `GLB-Global` (a 9 MB package), which
loads on demand for anyone within 500 m of it.

What only the live client can show (DA-06 ran it on 2026-10-05; outcomes in the [ledger](../analysis/debug-area/README.md#da-06-results)):

- The cloned base platform has no lightmap (`LMT_None`) and no light
  environment, in a map with baked outdoor light. 007's interior copy
  rendered lit; outdoors it may look black or flat. DA-06: the Compound and
  Death yard bases render lit (check 28).
- The ring sound is the FMOD event `prp_gen/rings/transport`. Its waveform
  is in the stock `audio/genprp/prp_gen.fsb`, which patch 006 does not
  copy (006 copies `prp_gen.fev` and `prp_gen_gate.fsb` into `Audio/UI`).
  Whether this map's copy of the event resolves is unproven. There is no
  evidence yet that 007's Armory ring sound plays either: its phase 1
  in-client test is still pending, and SigNoz has no client log naming
  `rings/transport` or sequence 10187 (searched 2026-10-04, 30 days).
  DA-06: the client's `audio.event` `transport` starts and stops with
  result 0; nobody has listened yet (check 30).
- The arena station (region 39, the Arena shelf) moved off the water in DA-F1:
  its rig stands on flat terrain at (331, -11.12, -693), and nothing on the
  shelf is a fluid volume, so the chunk's `SeqEvent_Touch` splash chain does
  not apply there. Check in the client that the rig's rings clear the ruin
  walls just behind its rear pillars (da06 saw them close, not touching;
  the v2026-10-05.2 re-check saw a gap, but never caught the rising discs).

### Seed and code

| What | Where |
|---|---|
| Pads, consoles | `db/resources/Worlds/Seed/debug_area_rings.sql` |
| Sequences, event sets, trigger volumes | `db/resources/Events/Seed/debug_area_ring_events.sql` |
| Console chains (`interact_tag` -> `trigger_transporter`) | `db/resources/Content/Seed/debug_area_ring_chains.sql` |
| Sequence ids the client resolves | `crates/resources/src/base/sequence_overrides.rs` (`DEBUG_AREA_RING_RIGS`) |
| Ring FSM | [ring-transport-system.md](../gameplay/ring-transport-system.md#debug-area-world-1300--8-regions-fully-connected-cimmeria-da-08) |

## Reach between stations

The plan keeps hostile zones apart. The guards in
`service::tests::npc_ai::debug_area::reach` read every world-1300 row of every
seed file, so another packet's NPCs take part as soon as they are seeded:

- `no_station_reaches_another`: for every pair of NPCs in different stations
  (at least one of them DA-03's) where one is hostile to players, a player
  inside one's aggro radius is outside the other's, and neither is inside the
  other's assist radius. Patrol routes and wander discs count at their nearest
  point. The pinned Jaffa and the pen are separate stations.
- `no_da03_hostile_reaches_a_respawner`: a player arriving at a world-1300
  respawner is outside every DA-03 hostile's aggro radius with 25 u to spare.
- `no_npc_fighter_reaches_a_da03_target`: no NPC that looks for NPC targets
  (DA-04's faction 3, 27 and 29 squads, for example) has a DA-03 NPC it would
  attack within twice its aggro radius (the scan's consider radius), and the
  reverse.

## Tests

- `crates/cell/src/cell/service/tests/npc_ai/debug_area_combat/` runs on the real
  `ihpet_crater_light.nav` and `.occ`, the world-1300 cover seed, and every
  world-1300 row of the seed files. It needs no database.
  - `arena.rs`: every arena row engages only the opposing squad when watched,
    and nothing while nobody watches. A player on the shelf between the
    squads is fought by the NID squad only.
  - `arena_pull_map.rs` (DA-F2): a lone player stepped over a 2 m navmesh
    grid around the shelf, at four heights, is engaged by a NID guard only
    inside the fight-1 strip, never in the fight-2 room, on the terrain above,
    on the slope below or on the east approach.
  - `arena.rs` also pins fight 2 at 39 m or more from every NID guard, and its
    spectator spots include both sides of fight 2.
  - `arena.rs` (DA-F2) checks that every arena row stands on occluder terrain
    within 0.5 m, so a row moved back onto the pit's water plane fails, and
    that the navmesh routes from the Z1 arrival and the east approach onto the
    shelf.
  - `cover.rs`: rifleman 3 spawns holding its marker and the others in the
    open, its marker is beyond the other riflemen's cover search, and it
    engages a tester in the hall from the slot. Rifleman 2 sees its doorway
    but not through the x 151 wall. Riflemen 1 and 2, with rifleman 3 dead,
    walk to a slot facing the tester, take Cover Stance and never take rifleman
    3's marker.
  - `death_respawn.rs`: every DA-04 spawn stands on the navmesh. A player on
    respawner 130 or 131 is never pulled by any world-1300 NPC, from its spawn,
    its patrol points or its wander circle. A tester killed on 131 by the
    chasing squad and respawned at once is not taken again over 40 s of AI
    ticks. The lethal squad engages together, and the respawn targets wait.
  - `live_db.rs` checks the same rows through the loaders: zones, the faction
    pairs against the reaction table, the cover in reach, and the respawn
    timers, and each template's aggro radius. It also runs two fights with the
    seeded abilities: the squad kills a fresh character within seven AI ticks,
    and the spectator pair's kill rolls no loot.
- `crates/cell/src/cell/service/tests/npc_ai/debug_area/` (DA-03) runs on the
  same files and reads every world-1300 row of every seed file, DA-04's
  included. It needs no database.
  - `placement.rs`: every DA-03 spawn stands on the navmesh (within 0.6 m; the
    wanderer 1.0 m) and on the occluder terrain; the patrol legs, the wander
    disc and the leash kite route are walkable; the assist trio sees itself.
  - `isolation.rs`: walking the gallery pulls nothing and shooting a gallery NPC
    rallies none; the pen engages only a player who walks up; the friendly and
    neutral rows never fight; the assist trio rallies exactly one neighbour;
    shooting a pinned Jaffa wakes only that Jaffa.
  - `reach.rs`: the reach guards described under
    [Reach between stations](#reach-between-stations).
  - `crates/cell-catalog/src/cell/spawner/tests/live_db_debug_area_npcs.rs`
    checks the DA-03 rows through the loaders: zones, the stations' behaviour
    columns, gallery coverage of every template hostile to players, and the
    point-set sequences.

## Live-client risks

DA-06 checked these on 2026-10-05; each outcome is in the [ledger](../analysis/debug-area/README.md#da-06-results), and what is still open is in its [follow-ups](../analysis/debug-area/README.md#open-follow-ups).

- The pit "floor" is a water collision plane (`WaterCollisionPrefab_Square`,
  y -33.28; the terrain is 10-25 m below it), found by DA-08. The navmesh lies
  on that plane, but a player sinks to the lakebed (DA-06: y -52). DA-F2 moved
  the arena squads off it to the east shelf, and DA-F1 moved the arena ring
  pad (region 39) there too. Only the Pit overlook pad (region 38) and the
  pit's west ledge look down on the water now.
- The east shelf is a ruin floor, and its walls are checked only in the
  occluder. Check in the client that fight 1's two lines see each other and
  that the walk on from the east gap is open.
- NPC heights on the Z9 terrain: the rows are grounded onto the navmesh, which
  is up to 2 m off the rendered terrain there.
- Whether the client draws NPC-sourced hits on a non-player target
  (`CP19` in the [unified UAT guide](../guides/unified-uat.md#castle-population)).
- Whether the client crouches an NPC standing at a cover marker; there is no
  server-to-client pose message.
- The Lucia Green and Yellow monikers and the female Yellow body have not been
  seen in this map.
- DA-03: the patroller walks navmesh heights, which float up to about 2.6 m
  above the terrain on the slope between z -690 and -650.
- DA-03: 75 gallery NPCs fall back to 592 Pistol Shot and play the pistol
  animation on creature and staff bodies; 62 have blank nameplates.
- DA-03: how the client colours a passive (NEUTRAL-override) faction-10 NPC has
  not been seen on this map. The largest gallery bodies may overlap at 8 m.

## DA-03 spawn tables

### Faction yard and AI behaviour slope

| Spawn | Template | Tag | Name shown | Faction | Position (x, y, z) |
|---:|---:|---|---|---|---|
| 13200 | 1330 | `DebugArea_Yard_Friendly_1` | Airman | 1 World Object | (396.0, -7.97, -806.0) |
| 13201 | 1330 | `DebugArea_Yard_Friendly_2` | Airman | 1 World Object | (396.0, -8.27, -802.0) |
| 13202 | 1331 | `DebugArea_Yard_Friendly_3` | Op-CORE Soldier | 9 Friendly_Ambient | (396.0, -8.16, -798.0) |
| 13203 | 1331 | `DebugArea_Yard_Friendly_4` | Op-CORE Soldier | 9 Friendly_Ambient | (396.0, -7.92, -794.0) |
| 13204 | 1332 | `DebugArea_Yard_Neutral_1` | Sewer Falls Resident | 7 Neutral_Ambient | (412.0, -5.16, -806.0) |
| 13205 | 1332 | `DebugArea_Yard_Neutral_2` | Sewer Falls Resident | 7 Neutral_Ambient | (412.0, -5.44, -802.0) |
| 13206 | 1333 | `DebugArea_Yard_NeutralPinned_1` | Jaffa Non Combatant | 10 Straegis | (412.0, -5.66, -798.0) |
| 13207 | 1333 | `DebugArea_Yard_NeutralPinned_2` | Jaffa Non Combatant | 10 Straegis | (412.0, -6.00, -794.0) |
| 13210 | 35 | `DebugArea_Yard_Hostile_1` | Ba'al's Jaffa | 10 Straegis | (436.0, -1.61, -800.0) |
| 13211 | 24 | `DebugArea_Yard_Hostile_2` | NID Guard | 10 Straegis | (440.0, -1.11, -796.0) |
| 13212 | 78 | `DebugArea_Yard_Hostile_3` | Straegis Fighter | 10 Straegis | (436.0, -1.86, -792.0) |
| 13240 | 1340 | `DebugArea_Slope_Patrol` | Jaffa Foot Soldier | 10 Straegis | (28.0, 12.39, -760.0) |
| 13241 | 1341 | `DebugArea_Slope_Wander` | Lucian Scavenger | 10 Straegis | (74.0, -0.25, -746.0) |
| 13242 | 1342 | `DebugArea_Slope_Leash` | Svarog Jaffa | 10 Straegis | (60.0, 5.08, -680.0) |
| 13243 | 1343 | `DebugArea_Slope_Assist_A` | Beleth Jaffa | 10 Straegis | (116.0, -7.19, -696.0) |
| 13244 | 1343 | `DebugArea_Slope_Assist_B` | Beleth Jaffa | 10 Straegis | (116.0, -7.19, -690.0) |
| 13245 | 1343 | `DebugArea_Slope_Assist_C` | Beleth Jaffa | 10 Straegis | (116.0, -7.19, -710.0) |

### Enemy gallery

#### Machines and drones

| Spawn | Template | Template name | Name shown | Level | Ability set | Position (x, z) |
|---:|---:|---|---|---:|---|---|
| 13300 | 4 | Prisoner retrieval unit | (none) | 1 | 2 (PRU) | (55, -592) |
| 13301 | 145 | Prisoner retrieval unit - Castle | Prisoner Retrieval Unit | 1 | 2 (PRU) | (59, -592) |
| 13302 | 81 | Ancient Drone | Agnos Drone | 1 | 592 fallback | (63, -592) |
| 13303 | 70 | BattleWalker | (none) | 1 | 592 fallback | (69, -592) |

#### Straegis

| Spawn | Template | Template name | Name shown | Level | Ability set | Position (x, z) |
|---:|---:|---|---|---:|---|---|
| 13304 | 77 | Straegis Beacon | Straegis Beacon | 1 | 592 fallback | (79, -592) |
| 13305 | 78 | Straegis Fighter | Straegis Fighter | 1 | 592 fallback | (83, -592) |
| 13306 | 79 | Straegis Titan | Straegis Titan | 1 | 592 fallback | (89, -592) |

#### Creatures

| Spawn | Template | Template name | Name shown | Level | Ability set | Position (x, z) |
|---:|---:|---|---|---:|---|---|
| 13307 | 74 | AMBRat | Rat | 1 | 592 fallback | (99, -592) |
| 13308 | 76 | ScavDog | (none) | 1 | 592 fallback | (103, -592) |
| 13309 | 73 | Lenny | Lenny | 1 | 592 fallback | (107, -592) |
| 13310 | 72 | Horden | (none) | 1 | 592 fallback | (113, -592) |
| 13311 | 75 | Rhinolion | (none) | 1 | 592 fallback | (121, -592) |
| 13312 | 71 | Carnosaur | Carnosaur | 1 | 592 fallback | (129, -592) |
| 13313 | 80 | Twilla Tree | Twilla Vines | 1 | 592 fallback | (137, -592) |

#### Humans: NID and SGC

| Spawn | Template | Template name | Name shown | Level | Ability set | Position (x, z) |
|---:|---:|---|---|---:|---|---|
| 13314 | 15 | Cellblock Guard | Cellblock Guard | 1 | 1 (NID pistol) | (147, -592) |
| 13315 | 24 | NID Guard | NID Guard | 1 | 3 (NID SMG) | (151, -592) |
| 13316 | 146 | NID Guard - Castle outside | NID Guard | 1 | 3 (NID SMG) | (155, -592) |
| 13317 | 148 | NID Guard - Castle inside | NID Guard | 1 | 3 (NID SMG) | (159, -592) |
| 13318 | 169 | Castle_Romney | NID Interrogator Romney | 1 | 3 (NID SMG) | (163, -592) |
| 13319 | 170 | Castle_Muelbach | Warden Muelbach | 1 | 3 (NID SMG) | (171, -592) |
| 13320 | 171 | Castle_BravoOfficer | NID Officer | 1 | 3 (NID SMG) | (55, -612) |
| 13321 | 181 | NID Guard - Castle inside L2 | NID Guard | 2 | 3 (NID SMG) | (59, -612) |
| 13322 | 184 | NID Guard - Castle outside L2 | Exterior NID Guard | 2 | 3 (NID SMG) | (63, -612) |
| 13323 | 182 | NID Guard - Castle inside L3 | NID Guard | 3 | 3 (NID SMG) | (67, -612) |
| 13324 | 185 | NID Guard - Castle outside L3 | Exterior NID Guard | 3 | 3 (NID SMG) | (71, -612) |
| 13325 | 183 | NID Guard - Castle inside L4 | NID Guard | 4 | 3 (NID SMG) | (75, -612) |
| 13326 | 186 | NID Guard - Castle outside L4 | Exterior NID Guard | 4 | 3 (NID SMG) | (79, -612) |
| 13327 | 223 | Dawson (hostile) | Dawson | 11 | 3 (NID SMG) | (83, -612) |
| 13328 | 218 | NID Operative | NID Operative | 31 | 3 (NID SMG) | (91, -612) |
| 13329 | 222 | Lance Corporal Grogan (hostile) | Lance Corporal Grogan | 36 | 3 (NID SMG) | (95, -612) |

#### Humans: Lucian

| Spawn | Template | Template name | Name shown | Level | Ability set | Position (x, z) |
|---:|---:|---|---|---:|---|---|
| 13330 | 151 | Lucian - Blue Faction Scientist | Blue Faction Scientist | 1 | 592 fallback | (103, -612) |
| 13331 | 152 | Lucian - Slum Dweller | Lucian Slum Dweller | 1 | 592 fallback | (107, -612) |

#### Goa'uld

| Spawn | Template | Template name | Name shown | Level | Ability set | Position (x, z) |
|---:|---:|---|---|---:|---|---|
| 13332 | 211 | Ashrak Assassin | Ashrak Assassin | 43 | 5 (Goa'uld) | (115, -612) |

#### Jaffa elites

| Spawn | Template | Template name | Name shown | Level | Ability set | Position (x, z) |
|---:|---:|---|---|---:|---|---|
| 13333 | 200 | Mala'c | Mala'c | 15 | 4 (Jaffa staff) | (123, -612) |
| 13334 | 203 | Ra's Jaffa Infiltrator | Ra's Jaffa | 24 | 4 (Jaffa staff) | (127, -612) |
| 13335 | 202 | Bra'hin | Bra'hin | 25 | 4 (Jaffa staff) | (131, -612) |
| 13336 | 208 | Free Jaffa Attacker | Free Jaffa Warrior | 38 | 4 (Jaffa staff) | (135, -612) |
| 13337 | 221 | Petbe (hostile) | Petbe | 42 | 4 (Jaffa staff) | (139, -612) |

#### Jaffa, male

| Spawn | Template | Template name | Name shown | Level | Ability set | Position (x, z) |
|---:|---:|---|---|---:|---|---|
| 13338 | 34 | SGC Jaffa | Jaffa | 1 | 592 fallback | (147, -612) |
| 13339 | 35 | SGC Ba'al Jaffa | Ba'al's Jaffa | 1 | 592 fallback | (151, -612) |
| 13340 | 82 | Bull Jaffa | (none) | 1 | 592 fallback | (155, -612) |
| 13341 | 83 | Asian Jaffa | (none) | 1 | 592 fallback | (159, -612) |
| 13342 | 84 | Cat Jaffa | (none) | 1 | 592 fallback | (163, -612) |
| 13343 | 85 | Cobra Jaffa | (none) | 1 | 592 fallback | (167, -612) |
| 13344 | 86 | Croc Jaffa | (none) | 1 | 592 fallback | (171, -612) |
| 13345 | 87 | Demon Jaffa | (none) | 1 | 592 fallback | (305, -592) |
| 13346 | 88 | Dragon Jaffa | (none) | 1 | 592 fallback | (309, -592) |
| 13347 | 89 | Eagle Jaffa | (none) | 1 | 592 fallback | (313, -592) |
| 13348 | 90 | Falcon Jaffa | (none) | 1 | 592 fallback | (317, -592) |
| 13349 | 91 | Horse Jaffa | (none) | 1 | 592 fallback | (321, -592) |
| 13350 | 92 | Hyena Jaffa | (none) | 1 | 592 fallback | (329, -592) |
| 13351 | 93 | Jackal Jaffa | (none) | 1 | 592 fallback | (333, -592) |
| 13352 | 94 | Mayan Jaffa | (none) | 1 | 592 fallback | (357, -592) |
| 13353 | 95 | Morrigan Jaffa | (none) | 1 | 592 fallback | (361, -592) |
| 13354 | 96 | Naga Jaffa | (none) | 1 | 592 fallback | (365, -592) |
| 13355 | 97 | Praxis Jaffa | (none) | 1 | 592 fallback | (369, -592) |
| 13356 | 98 | Praxis Jaffa 2 | (none) | 1 | 592 fallback | (373, -592) |
| 13357 | 99 | Ra Jaffa | (none) | 1 | 592 fallback | (381, -592) |
| 13358 | 100 | Standard Jaffa | (none) | 1 | 592 fallback | (385, -592) |
| 13359 | 101 | Savarog Jaffa | (none) | 1 | 592 fallback | (389, -592) |
| 13360 | 102 | Tiki Jaffa | (none) | 1 | 592 fallback | (393, -592) |
| 13361 | 111 | Viking Jaffa | (none) | 1 | 592 fallback | (397, -592) |
| 13362 | 142 | Ra Jaffa 2 | (none) | 1 | 592 fallback | (401, -592) |
| 13363 | 143 | Ra's Officer | Ra's Officer | 1 | 592 fallback | (405, -592) |
| 13364 | 144 | Ra's Jaffa | Ra's Jaffa | 1 | 592 fallback | (409, -592) |

#### Jaffa, male, Unas armour

| Spawn | Template | Template name | Name shown | Level | Ability set | Position (x, z) |
|---:|---:|---|---|---:|---|---|
| 13365 | 105 | Unas_1 | (none) | 1 | 592 fallback | (417, -592) |
| 13366 | 106 | Unas_2 | (none) | 1 | 592 fallback | (421, -592) |
| 13367 | 107 | Unas_3 | (none) | 1 | 592 fallback | (425, -592) |
| 13368 | 108 | Unas_4 | (none) | 1 | 592 fallback | (429, -592) |
| 13369 | 109 | Unas_5 | (none) | 1 | 592 fallback | (433, -592) |
| 13370 | 110 | Unas_6 | (none) | 1 | 592 fallback | (437, -592) |

#### Jaffa, female

| Spawn | Template | Template name | Name shown | Level | Ability set | Position (x, z) |
|---:|---:|---|---|---:|---|---|
| 13371 | 112 | Asian Jaffa Female | (none) | 1 | 592 fallback | (445, -592) |
| 13372 | 113 | Bull Jaffa Female | (none) | 1 | 592 fallback | (449, -592) |
| 13373 | 114 | Cat Jaffa Female | (none) | 1 | 592 fallback | (453, -592) |
| 13374 | 115 | Cobra Jaffa Female | (none) | 1 | 592 fallback | (457, -592) |
| 13375 | 116 | Croc Jaffa Female | (none) | 1 | 592 fallback | (461, -592) |
| 13376 | 117 | Demon Jaffa Female | (none) | 1 | 592 fallback | (465, -592) |
| 13377 | 118 | Dragon Jaffa Female | (none) | 1 | 592 fallback | (305, -612) |
| 13378 | 119 | Eagle Jaffa Female | (none) | 1 | 592 fallback | (309, -612) |
| 13379 | 120 | Falcon Jaffa Female | (none) | 1 | 592 fallback | (313, -612) |
| 13380 | 121 | Horse Jaffa Female | (none) | 1 | 592 fallback | (317, -612) |
| 13381 | 122 | Hyena Jaffa Female | (none) | 1 | 592 fallback | (321, -612) |
| 13382 | 123 | Jackal Jaffa Female | (none) | 1 | 592 fallback | (325, -612) |
| 13383 | 124 | Mayan Jaffa Female | (none) | 1 | 592 fallback | (329, -612) |
| 13384 | 125 | Morrigan Jaffa Female | (none) | 1 | 592 fallback | (333, -612) |
| 13385 | 126 | Naga Jaffa Female | (none) | 1 | 592 fallback | (337, -612) |
| 13386 | 127 | Praxis Jaffa 2 Female | (none) | 1 | 592 fallback | (341, -612) |
| 13387 | 128 | Praxis Jaffa 1 Female | (none) | 1 | 592 fallback | (345, -612) |
| 13388 | 129 | Standard Jaffa Female | (none) | 1 | 592 fallback | (349, -612) |
| 13389 | 130 | Svarog Jaffa Female | (none) | 1 | 592 fallback | (353, -612) |
| 13390 | 131 | Tiki Jaffa Female | (none) | 1 | 592 fallback | (357, -612) |
| 13391 | 138 | Viking Jaffa Female | (none) | 1 | 592 fallback | (361, -612) |
| 13392 | 139 | Clothed Jaffa Female 1 | (none) | 1 | 592 fallback | (365, -612) |

#### Jaffa, female, Unas armour

| Spawn | Template | Template name | Name shown | Level | Ability set | Position (x, z) |
|---:|---:|---|---|---:|---|---|
| 13393 | 132 | Unas 1 Female | (none) | 1 | 592 fallback | (373, -612) |
| 13394 | 133 | Unas 2 Female | (none) | 1 | 592 fallback | (377, -612) |
| 13395 | 134 | Unas 3 Female | (none) | 1 | 592 fallback | (381, -612) |
| 13396 | 135 | Unas 4 Female | (none) | 1 | 592 fallback | (385, -612) |
| 13397 | 136 | Unas 5 Female | (none) | 1 | 592 fallback | (389, -612) |
| 13398 | 137 | Unas 6 Female | (none) | 1 | 592 fallback | (393, -612) |

## Services plaza and dummies

Packet DA-02. Zone Z2, the services plaza, holds a copy of every hub service,
an ability granter, an ability reset NPC and a munitions vendor. Zone Z3, the
dummies range just north of it, holds five training dummies that never fight
back (D-DA7).

Right-clicking an NPC you can see from more than 5 m away prints "You are
too far away from <name>. Move closer to interact." in chat (DA-F5); a Banker
and an organization registrar print their own lines. The server logs it as
`event = "interaction.out_of_range"` with the distance. Two limits: the line
goes out only for an NPC in your view (in your AoI witness set or AoI radius),
so a crafted `interact` on a far NPC id cannot read its name; and at most one
line per 1.5 s, so clicking while you walk up does not fill chat. A dropped
line logs `interaction.out_of_range_unseen` or
`interaction.out_of_range_throttled` at DEBUG.

### Where it is

The plaza is centred on (252.0, 7.0, -923.0), in the courtyard of the crater
floor's south compound, about 39 units north of the Z1 arrival point. Its
centre is clear for 12 units. Twenty-one NPCs stand on a ring of radius 10.5
round it, about 2.9 units apart, each facing the centre. The ring has two
30-degree gaps: one to the south, which is where you walk in from the
arrival, and one to the north, towards the dummies.

- The east arc (x above 252), south to north: ability granter, ability reset,
  trainer, pet trainer, vendor, munitions vendor, loot crate, Livewire
  terminal, dialog NPC, mail clerk, Black Market auctioneer.
- The west arc, south to north: Storage Officer, Team Banker, Command Banker,
  Team registrar, Command registrar, crafting supplies vendor, then the four
  crafting stations. Every station is within 5 units of the next, so one spot
  reaches them all.

The dummies stand on one line at z -872, from x 240 to x 264, 6 units apart,
facing south towards the plaza.

| Spawn | Template | Tag | Name shown | Position (x, y, z) | Heading |
|---:|---:|---|---|---|---:|
| 13020 | 1300 | `DebugArea_AbilityGranter` | Train Testing Abilities | (254.72, 7.10, -933.14) | -0.2618 |
| 13021 | 1301 | `DebugArea_AbilityReset` | Jay Test Abilities | (257.25, 6.99, -932.09) | -0.5236 |
| 13002 | 301 | `DebugArea_Trainer` | Archetype Skills Trainer | (259.42, 6.99, -930.42) | -0.7854 |
| 13007 | 360 | `DebugArea_PetTrainer` | Goa'uld Advanced Skills | (261.09, 6.99, -928.25) | -1.0472 |
| 13001 | 300 | `DebugArea_Vendor` | Basic Equipment Quartermaster | (262.14, 6.99, -925.72) | -1.3090 |
| 13006 | 1302 | `DebugArea_MunitionsVendor` | Consumables | (262.50, 6.99, -923.00) | -1.5708 |
| 13005 | 304 | `DebugArea_LootCrate` | Crate | (262.14, 6.99, -920.28) | -1.8326 |
| 13004 | 303 | `DebugArea_LivewireTerminal` | Terminal | (261.09, 6.99, -917.75) | -2.0944 |
| 13003 | 302 | `DebugArea_DialogNpc` | Airman Lance | (259.42, 6.99, -915.58) | -2.3562 |
| 13011 | 390 | `DebugArea_MailClerk` | Sgt. Harriman | (257.25, 6.99, -913.91) | -2.6180 |
| 13012 | 305 | `DebugArea_Auctioneer` | Machra | (254.72, 6.99, -912.86) | -2.8798 |
| 13008 | 370 | `DebugArea_Banker` | Storage Officer | (249.28, 7.06, -933.14) | 0.2618 |
| 13009 | 371 | `DebugArea_TeamBanker` | Storage Officer | (246.49, 6.99, -931.94) | 0.5527 |
| 13010 | 372 | `DebugArea_CommandBanker` | Storage Officer | (244.16, 6.99, -929.98) | 0.8436 |
| 13013 | 330 | `DebugArea_TeamRegistrar` | Organization Registrar | (242.48, 6.99, -927.44) | 1.1345 |
| 13014 | 331 | `DebugArea_CommandRegistrar` | Organization Registrar | (241.61, 6.99, -924.52) | 1.4254 |
| 13019 | 314 | `DebugArea_CraftSupplies` | Common Materials Components | (241.61, 6.99, -921.48) | 1.7162 |
| 13015 | 310 | `DebugArea_Station_BioMedical` | BioMedical Crafting Station | (242.48, 6.99, -918.56) | 2.0071 |
| 13016 | 311 | `DebugArea_Station_Electronics` | Electronics Crafting Station | (244.16, 6.99, -916.02) | 2.2980 |
| 13017 | 312 | `DebugArea_Station_PowerSystems` | Power Systems Crafting Station | (246.49, 6.99, -914.06) | 2.5889 |
| 13018 | 313 | `DebugArea_Station_Materials` | Materials Crafting Station | (249.28, 6.99, -912.86) | 2.8798 |
| 13100 | 1310 | `DebugArea_Dummy_L1` | Jaffa (level 1) | (240.00, 7.08, -872.00) | 3.1416 |
| 13101 | 1311 | `DebugArea_Dummy_L10` | Jaffa (level 10) | (246.00, 6.59, -872.00) | 3.1416 |
| 13102 | 1312 | `DebugArea_Dummy_L25` | Jaffa (level 25) | (252.00, 6.59, -872.00) | 3.1416 |
| 13103 | 1313 | `DebugArea_Dummy_L50` | Jaffa (level 50) | (258.00, 6.70, -872.00) | 3.1416 |
| 13104 | 1314 | `DebugArea_Dummy_Friendly` | Injured SGC Guard | (234.00, 6.82, -872.00) | 3.1416 |

Every spawn is `is_stationary`: none walks, and none is checked for being off
the navmesh. The tags start `DebugArea_`, never `DebugHub_` (D-DA6), because
the hub's tests count that prefix.

Heights are the navmesh floor under each point, read with `nav_inspect`
against `data/spaces/ihpet_crater_light.nav`. The whole plaza is one open
walkable region at 6.99 to 7.10, and the campaign survey found the plaza
interior agrees with the occluder terrain to about 1 unit.

> **Placement is unchecked in the client.** No client has loaded world 1300
> yet. Packet DA-06 checks in the lab that each NPC stands on the floor and
> that the ring is clear of the courtyard's walls and props.

The names are monikers the client PAK ships; a new `texts.sql` id cannot
render. No shipped moniker names an ability granter, so the granter and the
reset NPC use the original developers' own test trainers, "Train Testing
Abilities" (21666) and "Jay Test Abilities" (22555). The granter wears
General Hammond's look and the reset NPC Sam Carter's, so neither is mistaken
for the trainer beside them.

### Services copied from the hub

Fifteen plaza NPCs spawn the hub's own templates, so each behaves exactly as
its hub twin; the [hub page](debug-hub.md#what-each-npc-tests) describes what
each one tests. The differences:

- **Dialog NPC (template 302).** The hub's dialogs 60100 and 60101 are
  quarantined (a client that received them crashed on map load), so the hub's
  dialog NPC shows nothing. The plaza's copy shows dialog 5738, which the
  client ships: two screens (Next pages through them) and one Generic button.
  No mission, dialog set or other chain uses 5738. Its text is a mission line
  out of context; the dialog UI is what is being tested. The button sends
  `dialogButtonChoice(5738, 198)`, and chain 13003 says "Dialog round trip
  complete" in chat as Airman Lance. Closing with X sends nothing.
- **Gate Mail Clerk (template 390).** The hub clerk mails from a quarantined
  dialog, so it cannot mail at all. The plaza's clerk mails on the click
  itself (chain 13007): the same mail, 5 Health Slappack TC1 and 50
  naquadah, and the same limit of one per character every 10 minutes. The
  limit is kept separately from the hub clerk's.
- **Black Market auctioneer (template 305).** Chain 13008 opens the Black
  Market for the plaza's own tag; the hub's chain 5030 binds the hub's tag.
  Both NPCs are the same seeded auctioneer.
- **Livewire terminal and loot crate (templates 303 and 304).** Chains 13004
  and 13005 (Livewire and its victory line) and 13006 (loot table 3), the
  hub's chains for the plaza's tags.
- **Vendor, trainer, pet trainer, Bankers, registrars, crafting stations and
  supplies.** These need no chain; they answer through the built-in
  interaction paths, as in the hub.

### Ability granter (template 1300)

Right-click the granter. If you are a GM, you get every ability of your
archetype's tree that you do not know, all three branches and the capstones,
and every running cooldown is cleared. The result is the same as typing
`/gmgiveallabilities`.

1. A chat line says how many abilities are on their way and how many
   cooldowns were cleared, for example "Ability granter: granting 41
   abilities of your Commando tree (all branches and capstones); 2
   cooldown(s) cleared."
2. The base saves them to the character (`sgw_player.abilities`, not a
   trainer purchase: no training points move, and a respec keeps them).
3. The Abilities window refreshes (`onKnownAbilitiesUpdate`), and a second
   line confirms: "Ability granter: granted 41 abilities from your tree
   (saved; no points spent)". The base logs the grant with
   `source = npc_granter`, so it is never mistaken for a typed
   `/gmgiveallabilities` (`source = command`).

A second click on the same NPC within a second is ignored: each click
is a locked database write, and the first one already answered.

If you already know the whole tree, the cooldowns are still cleared and a
line says so. The action bar is not filled: the server cannot write it.
Drag the abilities from the Abilities window.

A player who is not a GM gets "Only a GM can use this. Your abilities and
cooldowns are unchanged." and nothing else happens. The check is the
account's access level, the same one the native GM commands use.

How it works: the click fires chain 13000, whose content action
`gm_ability_bulk` (change `grant_all`) does the GM check, clears the
cooldowns and sends the base the same message `/gmgiveallabilities` sends.
See [content-engine-vocabulary.md](content-engine-vocabulary.md). The
template carries `INT_Trainer` (128) for the trainer cursor and no trainer
list, so the trainer window never claims the click. Each click writes one
`event = "ability_granter"` row on target `content` with the player and
every granted ability id and name.

### Ability reset (template 1301)

Right-click the reset NPC to go back to your archetype's character-creation
starter abilities, with the tree points you spent refunded and every cooldown
cleared, as `/gmresetabilities` does. It removes quest and GM grants too: it
is the clean slate a repeatable ability UAT starts from. GM only, like the
granter. Chain 13001, action `gm_ability_bulk` with change `reset`.

### Munitions vendor (template 1302)

Right-click to open a store that sells (buy list 1300):

| Rows | Items |
|---|---|
| 13001-13005 | 100 rounds of each bullet special ammo: Armor Piercing, Hollow Point, Incendiary, EMP, Explosive (items 9000-9004) |
| 13006-13015 | 100 of each dart special ammo: Poison, Disease, Tranquilizer, EMP, Radioactive, Stim, Coagulant, Nanite, Antidote, Adrenaline (9005-9014) |
| 13016 | 5 Health Slappack TC1 (2893) |
| 13017-13019 | One SI 3 9mm Pistol (55) for 300 naquadah, SGHC 6 SMG (21) for 1000 and CO2 Pistol Dartgun (3584) for 1 |

Everything else costs 1 naquadah. The pistol and the SMG cost what sell
list 2 pays for them, so nothing here can be bought and sold back at a
profit; `live_db_vendor_arbitrage` holds every buy list in the seed to that
rule. Sell, repair and recharge use list 2, as the hub vendor does. A purchase
lands in the main bag. The special ammo is drawn by a reload once
`ammo.finite_special` is on (#1026). Deployables are abilities, not items:
get them from the ability granter. The name is the client's "Consumables"
vendor moniker (27264).

### Training dummies (templates 1310-1314)

Five dummies that never fight back, however much you shoot them (D-DA7).
Right-click a hostile one to attack it, or target it and use any ability.

| Template | Level | Faction | Health | Use |
|---:|---:|---|---|---|
| 1310 | 1 | 10 (hostile) | 1,000,000 | Damage against a level-1 target |
| 1311 | 10 | 10 (hostile) | 1,000,000 | Damage against a level-10 target |
| 1312 | 25 | 10 (hostile) | 1,000,000 | Damage against a level-25 target |
| 1313 | 50 | 10 (hostile) | 1,000,000 | Damage against a level-50 target |
| 1314 | 10 | 9 (Friendly_Ambient) | 500,000 of 1,000,000 | Heals and buffs |

- **They never attack, chase or leash.** The templates carry
  `training_dummy = true`. At spawn that puts on the same "never fights back"
  mark the GM `.dummy` command places, which keeps the NPC out of the AI tick
  entirely. A plain hostile NPC set to NEUTRAL still fires back once shot,
  which is why D-DA7 asked for the mark.
- **They do not die in a test run.** 1,000,000 Health. If one is killed
  anyway, it respawns 30 seconds later, and the kill pays no XP.
- **Combat ends on its own.** Ten seconds after the last hit on a dummy,
  everyone who hit it leaves combat with it (regeneration and the
  out-of-combat holster come back). A dummy has no leash to do that.
- **The friendly dummy takes heals.** It starts at half Health, so a heal has
  room to land and shows its size; nothing but a heal changes its Health.
  Target it and cast a heal: the heal lands on it instead of falling back to
  you. Players cannot attack it, and no NPC targets it.
- **The friendly dummy stands at the west end of the line**, 6 m past the
  level-1 dummy, on open floor. It first stood at the east end (264), where a
  wall footprint between x 259.4 and 261 kept a healer more than 5 m away,
  out of Health Heal's range (DA-F7). The cell-world test
  `the_friendly_dummy_is_reachable_and_visible_from_the_dummy_line` walks the
  line to it on the navmesh and checks a healer 4 m out can see it.
- **Heal feedback.** Health Heal (1646) and Recuperation (1218) show the
  client's Medkit icon, the art the client gives its health-restore items
  (Health Slappack, Plasma Burn Treatment Kit); the shipped entries had
  `IconMissing`, and the shipped imagesets hold no health-heal ability icon
  (`Heal_Focus_Heal` is Heal Focus's, already on the bar). A heal cast out of
  range reads "Your target is out of range" instead of the raw
  `CONDITION_FEEDBACK_OutsideWeaponRange` token. Both are cooked-data patches
  the server pushes (`crates/resources/src/base/attribute_patches/`, DA-F6),
  so a client resyncs abilities and error strings once on its next login.
- They carry no ability set and no loot table, and the names are the client's
  "Jaffa" (8168) and "Injured SGC Guard" (7882).
- A GM can place one anywhere with `.spawn 1310` (or 1311 to 1314); it is a
  training dummy too. The `.dummy` command still places a temporary one in
  front of you ([commands.md](../commands.md#dev-console--commands)).

### Where it lives

| What | Where |
|---|---|
| Templates 1300-1302, 1310-1314 | `db/resources/Entities/Seed/entity_templates_debug_area_plaza.sql` |
| Spawns 13001-13022, 13100-13104 | `db/resources/Worlds/Seed/spawnlist_debug_area_plaza.sql` |
| Heal icons (1646, 1218) and the out-of-range text (error 42) | `crates/resources/src/base/attribute_patches/`, `db/resources/Abilities/Seed/abilities.sql`, `db/resources/Texts/Seed/error_texts.sql` |
| The too-far `interact` line | `crates/cell-interactions/src/cell/interactions/dispatch/range_feedback.rs` |
| Buy list 1300 (rows 13001-13019) | `db/resources/Items/Seed/item_lists_debug_area_plaza.sql` |
| Chains 13000-13008 | `db/resources/Content/Seed/debug_area_plaza_chains.sql` |
| The `training_dummy` column | `db/resources/Entities/Tables/entity_templates.sql` |
| The training-dummy mark, its Health and its quiet sweep | `crates/cell-world/src/cell/space_manager/training_dummy.rs`, `crates/cell-combat/src/cell/combat/threat/training_dummy_release.rs` |
| The friendly dummy as a heal target | `classify` in `crates/cell-combat/src/cell/abilities/use_ability/support_shot.rs` |
| The `gm_ability_bulk` action | `crates/cell-content/src/cell/content/executor/ability_granter.rs` |
| The tree plan and the cooldown clear it shares with the GM commands | `plan_tree_grant` in `crates/cell-world/src/cell/space_manager/tree_grant.rs`, `reset_all_cooldowns` in `crates/cell-combat/src/cell/abilities/cooldown_reset.rs` |

### Tests

| Guard | What it pins |
|---|---|
| `cell-catalog` `spawner/tests/live_db_debug_area_plaza.rs` | Every DA-02 spawn is in world 1300, stationary, `DebugArea_*`-tagged, on a seeded template with a shipped name; the plaza NPCs on the ring and the plaza floor, the dummies on their line, all at least 2.5 apart; templates 1310-1314 and only they are training dummies, loaded as such by both loaders, hostile at levels 1, 10, 25 and 50 plus one friendly; each chain-driven NPC has one `interact_tag` chain running its action; buy list 1300 sells every special-ammo item |
| `cell-content` `executor/tests/ability_granter.rs` | A GM gets exactly the tree abilities they lack (capstone included, no repeats), cleared cooldowns with the client's clear timers and a first-click line; a non-GM gets the refusal line and nothing else; a GM who knows the tree gets a line and no write; the reset forwards a reset |
| `cell` `service/tests/npc_ai/training_dummy.rs` | An NPC spawned from a `training_dummy` record gets the mark and the dummy Health and never attacks its threat target, where the same record without the flag shoots; the friendly one starts at half Health |
| `cell-combat` `death/npc_only_kill_tests.rs` | A player's kill of a training dummy pays no XP, where the same mob unmarked pays |
| `cell-combat` `combat/threat/training_dummy_release.rs` | A dummy's attackers stay in combat while hits land and leave it 10 s after the last; an ordinary NPC is left to its leash |
| `cell-combat` `use_ability/tests/beneficial_training_dummy.rs` | A heal aimed at a friendly training dummy lands on it; an unmarked neutral NPC still falls back to the caster, and a hostile dummy is never healed |
| `cell-world` `space_manager/tests/debug_area.rs` (`the_friendly_dummy_is_reachable_and_visible_from_the_dummy_line`) | The walk from the L1 dummy to the friendly dummy stays on navmesh polygons, and a healer 4 m out has line of sight past the occluder (DA-F7) |
| `resources` `attribute_patches/tests.rs` | 1646 and 1218 are served with the Medkit icon and error 42 with readable text, nothing else in those entries changes, both categories resync, and the seed rows match (DA-F6) |
| `cell-methods` `player/interaction/mod.rs` (`out_of_range_interact_on_a_plain_npc_tells_the_player_to_move_closer`, `out_of_range_interact_on_an_npc_out_of_view_sends_nothing`, `repeated_out_of_range_clicks_send_one_line_per_interval`), `cell-interactions` `dispatch/tests/mod.rs` | A too-far click on a visible NPC sends exactly one line naming it; an NPC out of view or a missing target sends nothing; a second click inside 1.5 s sends nothing (DA-F5) |
| `base-session` `world_entry/gm_only_worlds.rs`, `base-world-entry` `gate_travel/tests/gm_only_world.rs`, `base-methods` `world_entry_db.rs` (`live_db_a_non_gm_saved_in_the_debug_area_logs_in_at_the_faction_start`) | A non-GM is refused world 1300 on a cross-world transfer and at login and arrives at the Praxis or SGU start with a line owed; a GM goes in |
| `cell-catalog` `spawner/tests/live_db_vendor_arbitrage.rs` | No buy list in the seed sells an item for less than any sell list pays for it |
| `content-engine` `loader/tests/action_conversion.rs`, `interact_tag_linter` | `gm_ability_bulk` converts for `grant_all` and `reset` and drops anything else; the plaza's interact chains are allowlisted (template-default cursor bits) |

## Stargate

The map's own stargate, 28 m behind the arrival point, is the Debug
Area gate (stargate 29, packet DA-07). It is **outbound only**.

**What it does.** Right-click the DHD beside the gate. If your account is a
GM, the dialling window lists every gate on a world this server can load
but one (Men'fa (SGU), below), by name, from the first open:
The Castle, Harset, Tollana, Omega Site, Beta Site E1, Men'fa (Praxis),
both Ihpet Craters, Lucia, Agnos, SGC, SGC W1 and Dakara E1. Dial one, wait for
the gate to open, walk into the event horizon. The extra addresses last
until you leave the Debug Area and are never saved to your character; a
gate you actually travel to stays learned, as after any gate trip.

The addresses are granted when you arrive in the Debug Area, not when the
DHD opens (DA-F3). The client looks each new address up in its cooked
stargate table after it arrives and never redraws an open DHD, so addresses
granted as the window opened listed as "Unknown" until it was reopened.
Arriving gives the client the walk to the DHD to look them up. The DHD open
still checks again and grants anything missing.

**What it does not do.**

- Nobody can dial the Debug Area, from anywhere, and its address never
  enters anyone's address book. The way back is `.gotolocation DebugArea`.
- A non-GM never gets the extra addresses. Since DA-02 a non-GM cannot be in
  the Debug Area at all (they are sent to their faction start), and even a
  GM demoted while inside gets no more on the next DHD open.
- The 14 gates on worlds this server has no map for (Hebridan, Pen-Lai,
  Asgard High Council and the rest) are not offered. The server log names
  them on every arrival and DHD open.
- `/gmdhd 29` is refused like any other dial into the Debug Area.

- Men'fa (SGU) (Menfa_Light) is not offered either: its gate row is about
  192 m below the map's playable surface. It comes back once its arrival is
  pinned from an in-client look.

**Known arrival problem.** SGC W1's gate row is 3.4 m from the navmesh and
1.3 m above the floor beside it. You land on the gate and the client drops
you onto the floor. Report anything worse.

**Landing spot.** `.gotolocation DebugArea` without coordinates lands on the
gate's arrival point, the Z1 arrival (251.0, 8.0, -962.0), in front of the
gate and facing away from it.

**Checking it.**

| Step | Expect | Fail |
|---|---|---|
| As a GM, arrive in the Debug Area and right-click its DHD | The dialling window opens and lists the 13 gates above by name on the first open | Nothing opens; the list is empty, holds only your own addresses, or shows "Unknown" rows |
| Dial Harset | The gate opens about 4 s later; walking in loads Harset | Refusal line "Failed to dial: ..."; the gate never opens |
| From any other world's DHD, look for "Debug Area" | Not listed | Listed, or dialable |

A non-GM cannot reach the hub DHD in the client any more (DA-02's GM-only
entry), so the non-GM branch is checked by the unit test
`a_non_gm_at_the_hub_gets_no_addresses_and_cannot_dial`.

Server log fields to search when it misbehaves: `reason = "gm_dial_hub_grant"`
(one per GM arrival in the Debug Area and per DHD open: who, what was
granted, what was left out; `trigger` is `world_entry` or `dhd_open`),
`reason = "dial_hub_not_gm"`, `reason = "dial_hub_is_outbound_only"` (a dial
into the Debug Area was refused). Mechanism and design:
[gate-travel.md § Debug Area dial-out](../gameplay/gate-travel.md#debug-area-dial-out).

## System Lords' summit

Packet DA-09. Six System Lords stand in a circle on the east side of the
south compound's courtyard and squabble in say chat. Ra's Jaffa stands
behind his lord and says "Indeed." It tests NPC appearance (the lords are
dressed from the client's own Goa'uld costume packages) and
[ambient chatter](ambient-chatter.md), the NPC-to-NPC say-chat system this
packet adds.

**Getting there.** `.gotolocation DebugArea 282 7.2 -953`, which lands you
4.5 m behind Ba'al, looking across the circle at Ra. On foot it is 30 m east
of the services plaza and 36 m north-east of the Z1 arrival.

**Seating.** The centre is (282.0, 6.9, -944.0), on the courtyard's paving
(occluder geometry at 6.9, the navmesh 0.2-0.3 m above it), with nothing at
head height for 7.5 m round. The nearest other NPC is 26 m away. Each lord
stands on a 4.5 m ring facing the centre:

| Spawn | Template | Tag | Name | Position (x, z) | Dressed in |
|---:|---:|---|---|---|---|
| 13850 | 1400 | `DebugArea_Lords_Ra` | Ra | (282.0, -939.5), north | His NPC kit: cape, dress, crowned helmet, torso and armour |
| 13851 | 1402 | `DebugArea_Lords_Anat` | Anat | (285.9, -941.75) | Her full NPC kit |
| 13852 | 1405 | `DebugArea_Lords_Nerus` | Nerus | (285.9, -946.25) | His NPC robes (a human body, as shipped) |
| 13853 | 1401 | `DebugArea_Lords_Baal` | Ba'al | (282.0, -948.5), south, facing Ra | The dark, gold-trimmed Yellow Trader robes with his own head |
| 13854 | 1403 | `DebugArea_Lords_Athena` | Athena | (278.1, -946.25) | Anat's gown and bracer, a circlet, her own white hair and head |
| 13855 | 1404 | `DebugArea_Lords_Morrigan` | Morrigan | (278.1, -941.75) | Anat's breastplate, pauldron and boots, war paint, a ribbon device and her own head |
| 13856 | 1406 | `DebugArea_Lords_RaJaffa` | Ra's Jaffa | (282.0, -936.5), behind Ra | Ra's Jaffa armour and a staff |

Every lord is faction 1 (friendly), level 50 (the Jaffa 30), with no
ability set and no loot. Players cannot attack them, and they never fight.
The shipped templates (41-45, 53) leave Ba'al, Athena and Morrigan as heads
on a bare base body. The client ships no female Goa'uld armour beyond
Anat's NPC kit, so Athena and Morrigan each wear a different part of it:
no two lords wear the same outfit, though the three women share Anat's
red-and-bronze palette. Every component name is a `BodyComponent` export of
the client's packages (`query-index scan <package> BodyComponent`).

**Two things the lab check found** (2026-10-05, a local server on this
branch, [ledger](../analysis/debug-area/README.md#da-09-system-lords-summit)):

- Adding `NPC_Goauld.NPC_Ra_Head_00` or `NPC_RaG_FingerNail_00` to Ra's kit
  makes the client draw its placeholder cube (a purple box with a face)
  instead of Ra. Template 41's kit already carries his crowned head.
- The summit first stood on the palace terrace north of the enemy gallery
  (218, 30.9, -532). The terrain there draws white with magenta streaks in
  this client, and the terrace east of it is unfinished grey void, so the
  summit moved to the paved courtyard. Outdoor terrain all over this map
  draws white in this client; paving and buildings draw properly.

**The chatter.** Ambient chatter group 1 holds 17 exchanges of petty
squabbles over thrones, lunch, the sun's paperwork, the sarcophagus rota,
reply-all and, mostly, the shol'va Teal'c. One exchange starts when you
come within 18 m of a lord. Its lines follow 3 to 9 s apart, sized to read
the line before, then the summit is quiet for 30 s before the next. From
the landing spot and anywhere in the circle you hear every lord. Nobody at
the services plaza or the arrival hears them. Each line shows in the chat
window as `[Lord] says <line>`.

| Step | Expect | Fail |
|---|---|---|
| `.gotolocation DebugArea 282 7.2 -953` | Seven NPCs stand in a circle on the courtyard paving, named Ra, Anat, Nerus, Ba'al, Athena, Morrigan and Ra's Jaffa, each in a different outfit | An NPC missing, floating, sunk or a purple cube; two dressed alike |
| Wait by the circle | Within a second or two a scene starts in chat, each line from the lord who says it | No line in 5 s; lines from "?" or an empty name |
| Wait for the scene to end | 30 s of quiet, then the next scene | Lines run on without a pause; scenes repeat at once |
| Walk 30 m away | The lines stop | Lines still arrive far from the circle |
| Right-click a lord | Nothing happens: they are scenery | An attack starts or an error shows |

Seed: `entity_templates_debug_area_lords.sql` (templates 1400-1406),
`spawnlist_debug_area_lords.sql` (spawns 13850-13856) and
`db/resources/Dialogs/Seed/ambient_chatter_lords.sql` (group 1). Guards:
`service::tests::npc_ai::debug_area::lords` in `cimmeria-cell` (the lords
stand on the mesh and the paving, a listener is out of every hostile's
reach, every line's speaker is seated and in earshot of the centre and of
the landing spot, and neither the plaza nor a respawner hears the summit)
and `live_db_ambient_chatter` in `cimmeria-cell-catalog`.
