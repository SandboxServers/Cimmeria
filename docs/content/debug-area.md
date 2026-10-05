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

This page covers the combat zones from packet DA-04. Its rows are in their own
seed files, `db/resources/Entities/Seed/entity_templates_debug_area_combat.sql`
(templates 1370-1377) and `db/resources/Worlds/Seed/spawnlist_debug_area_combat.sql`
(spawns 13600-13641), both inside DA-04's id block (templates 1370-1399, spawns
13600-13799).

Heights are BigWorld metres. Every DA-04 NPC is a mobile `mob`, so the spawner
grounds it onto `ihpet_crater_light.nav`. On open terrain the navmesh and the
occluder's terrain disagree by up to 2 m, so rows are authored near the mesh
height and grounding does the rest.

> **Placement is checked on the server's data, not in the client.** The rows
> are tested on the real navmesh, occluder and cover markers (below), but
> nobody has walked them in the client yet. That is packet DA-06.

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
