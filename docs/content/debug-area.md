---
title: "Debug Area"
type: reference
audience: engineers, testers
last_updated: 2026-10-04
---

# Debug Area

The Debug Area is server world **1300 `DebugArea`**, a GM test map on the shipped
client map Ihpet_Crater_Light (decisions D-DA1 to D-DA9 in the
[campaign plan](../analysis/debug-area/README.md)). It is shared and always
loaded, and only GMs can get there: `.gotolocation DebugArea` (or the native
`/gmgotolocation`) lands on respawner 130 at the south compound. Each zone
tests one group of server systems. Every spawn carries a `DebugArea_*` tag;
the stasis-room hub's tests count `DebugHub_*`, so the two never mix (D-DA6).

This page covers the zones from packets DA-03 and DA-04, in zone order; DA-05
completes it with the station index and the UAT mapping. Each packet's rows are
in their own seed files:

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

Heading 0 faces +Z and -1.5708 faces -X.

Heights are BigWorld metres. Every DA-04 NPC is a mobile `mob`, so the spawner
grounds it onto `ihpet_crater_light.nav`. On open terrain the navmesh and the
occluder's terrain disagree by up to 2 m, so rows are authored near the mesh
height and grounding does the rest.

> **Placement is checked on the server's data, not in the client.** The rows
> are tested on the real navmesh, occluder and cover markers (below), but
> nobody has walked them in the client yet. That is packet DA-06.

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
| Hostile pen | Three faction-10 templates as seeded elsewhere: 24 NID Guard (SMG set 3), 35 Ba'al's Jaffa (592 fallback), 78 Straegis Fighter (592 fallback). Hostile on sight. | Yes | On approach |

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

Zone Z6 is the flat sunken pit north of the compound, centre
(250.0, -32.4, -725.0). Its floor is clear of geometry for 40 m or more around
the centre. The crater floor stands 25 to 40 m above it to the north, east and
south. The gentle way in is a ramp from the south-west, around x 190-200,
z -755 to -790. Two fights run there (D-DA8). Both are NPC-vs-NPC combat from
[#1009](../gameplay/npc-ai.md#npc-vs-npc-combat-1009).

**Fight 1, Praxis against NID (joinable).** Faction 3 (Praxis) against faction
10 (Straegis), the only seeded mutually hostile pair. Players react as faction
3, so the Praxis squad is friendly to a player and cannot be damaged by one.
The NID squad is hostile and damageable. To join on the Praxis side, walk down
the ramp and shoot a NID guard. A NID guard also takes a player who comes within
30 m on the pit floor. Kill credit goes to the killing blow: finish a NID
guard the Praxis wore down and you get the full XP and loot; wear one down and
let a Praxis finish it and you get nothing. That is the #1009 rule, not a bug.

| Spawn | Template | Tag | Name shown | Faction | Position (x, y, z) |
|---:|---:|---|---|---:|---|
| 13600 | 1370 | `DebugArea_Arena_Praxis1` | Op-CORE Soldier | 3 | (238, -32.4, -721) |
| 13601 | 1371 | `DebugArea_Arena_Praxis2` | Praxis Jaffa Guard | 3 | (238, -32.4, -725) |
| 13602 | 1370 | `DebugArea_Arena_Praxis3` | Op-CORE Soldier | 3 | (238, -32.4, -729) |
| 13603 | 1372 | `DebugArea_Arena_NID1` | NID Guard | 10 | (262, -32.4, -721) |
| 13604 | 1372 | `DebugArea_Arena_NID2` | NID Guard | 10 | (262, -32.4, -725) |
| 13605 | 1372 | `DebugArea_Arena_NID3` | NID Guard | 10 | (262, -32.4, -729) |

**Fight 2, Lucia Green against Lucia Yellow (spectators only).** Faction 27
against faction 29, one of the player-safe hostile pairs, on the pit's west
half. It stands 44 m or more from every NID guard, outside their 30 m aggro
radius, so a tester can walk right up to it, or loot-check a corpse, without
being pulled into fight 1. DA-08's ring pad at (210, -33.28, -725) sits
between the two pairs, 10-12 m from the nearest. Each side is hostile
to the other and friendly to faction 3 both ways. Neither is hostile to faction
3 or 10, so fight 1 and fight 2 never mix. No player can damage either side
(only faction 10 is damageable). The fight shows that an NPC-only kill pays
nobody: both templates carry loot table 2, yet the corpse rolls no loot, shows
no loot cursor, pays no XP and moves no mission
([combat-system.md](../gameplay/combat-system.md#npc-vs-npc-kills-pay-nobody-1009)).
The server logs `loot.drop event=skipped reason=npc_only_kill loot_table_id=2`.

| Spawn | Template | Tag | Name shown | Faction | Position (x, y, z) |
|---:|---:|---|---|---:|---|
| 13610 | 1373 | `DebugArea_Arena_Green1` | Green Sniper | 27 | (214, -32.6, -716) |
| 13611 | 1373 | `DebugArea_Arena_Green2` | Green Sniper | 27 | (218, -32.35, -716) |
| 13612 | 1374 | `DebugArea_Arena_Yellow1` | Yellow Faction | 29 | (206, -32.3, -734) |
| 13613 | 1374 | `DebugArea_Arena_Yellow2` | Yellow Faction | 29 | (210, -31.95, -736) |

**Why it fights, and why it repeats.**

- The squads stand 24 m apart (fight 2: 20 to 22 m). The templates set
  `aggro_radius` 30 (fight 2: 26), so each side's Idle scan reaches the other.
  The default radius, 18 m, would not.
- The scan runs only while a player has the NPC in their AoI (150 m). From the
  services plaza the pit is about 200 m away. Walk to the compound's north side
  or the rim to start the fights. A fight already under way finishes without a
  witness.
- The other way round: any player within 150 m keeps the arena cycling. That
  includes testers at DA-02's dummy range (about 148 m) and DA-03's gallery
  and assist slope (about 130 m). Every round writes `npc_ai.aggro
  cause=proximity` and `loot.drop reason=npc_only_kill` rows, so filter
  SigNoz queries on the `DebugArea_Arena_` tag prefix (or exclude it) when
  you read another station's AI or DPS telemetry.
- Every arena row respawns 30 s after death (template `respawn_secs`). The
  winners walk home, heal to full on arrival and go Idle. The losers come back
  and the scan starts the next round, after the 5 s post-reset window (NA12).
- The arena squads carry no loot (fight 1) and no `use_cover` (there is no
  cover marker in the pit). The fight-1 Jaffa fires the Jaffa staff set (4)
  and carries the staff, the others the SMG set (3); the Yellow Faction pair
  keeps the pistol set (1) of its look's template, 219.

**Spectators are not pulled.** The aggro scan has a 4 m vertical band. A
player watching from the crater floor is 25 to 40 m above the pit and never
becomes a candidate. A player on the low south-west ramp is inside the band
but 40 m or more from the nearest NID guard. The Praxis and Lucia rows never
target a player at all.

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
  never looks for NPC targets, and the assist gate refuses NEUTRAL NPCs, so
  shooting one never pulls its neighbours 4 m away;
- is still **damageable** (the damage gate is faction 10) and **fights back**
  when shot, then leashes home at the default 50 u;
- **respawns after 20 s**, so the line refills;
- has its own tag, `DebugArea_Gallery_<template_id>` (for example
  `DebugArea_Gallery_24` is the NID Guard).

The Twilla Tree (80) and the Straegis Beacon (77) are `is_stationary`: a plant
and a beacon that walked would be wrong.

**Placed: 99 of the 101 hostile templates.** 24 carry an ability set (sets 2,
3, 4 and 5: the drones, the NID and SGC humans, the named Jaffa and the
Goa'uld); the other 75 fall back to 592 Pistol Shot, so a creature fires a
pistol until it gets its own kit. 62 have no display name (no `name_id`, or an
empty moniker such as 4's): their nameplate is blank, which is why each one
has a tag.

**Excluded**, with the reason (the live-DB guard
`live_db_debug_area_npcs` names them in `GALLERY_EXCLUSIONS`):

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

## Reach between stations

The plan keeps hostile zones apart. DA-03's guard
(`no_station_reaches_another`) checks, for every pair of NPCs in different
stations where one is hostile, that a player inside one's aggro radius is
outside the other's and that neither is inside the other's assist radius,
counting patrol routes and wander discs at their nearest point. A second
guard keeps every DA-03 hostile more than twice its aggro radius plus 25 u
from the other packets' zones (Z1, Z2, Z3, Z6, Z8, Z9).

## Tests

- `crates/cell/src/cell/service/tests/npc_ai/debug_area_combat/` runs on the real
  `ihpet_crater_light.nav` and `.occ`, the world-1300 cover seed, and every
  world-1300 row of the seed files. It needs no database.
  - `arena.rs`: every arena row engages only the opposing squad when watched,
    and nothing while nobody watches. A spectator on the rim or the far slope
    is never pulled. A player in the pit is fought by the NID squad only.
  - `arena.rs` also pins fight 2 at 40 m or more from every NID guard, and its
    spectator spots include both sides of fight 2.
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

- The pit "floor" is a water collision plane (`WaterCollisionPrefab_Square`,
  y -33.28; the terrain is 10-25 m below it), found by DA-08. The navmesh lies
  on that plane, so the arena squads stand and walk on what renders as water.
  Check in the client that they are not drawn swimming or sunk.
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
| 13210 | 24 | `DebugArea_Yard_Hostile_1` | NID Guard | 10 Straegis | (436.0, -1.61, -800.0) |
| 13211 | 35 | `DebugArea_Yard_Hostile_2` | Ba'al's Jaffa | 10 Straegis | (440.0, -1.11, -796.0) |
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
