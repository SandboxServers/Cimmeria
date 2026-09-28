# Castle Population Ledger

> Type: reference and how-to. Audience: content authors, UAT testers and the Castle coordinator.
> Updated: 2026-09-28 (packet CP01, seed only). Companions: [Castle rebuild ledger](../castle-rebuild/README.md) (missions 701-708 and the story actors this pass works around), [unified UAT guide, Castle population](../../guides/unified-uat.md#castle-population).

## Purpose

World 8 ("Castle") had about 40 spawn rows before this pass, most of them the story actors of missions 701-708 and a belt of level-1 NID guards on the outdoor field. The interior, from the Armory to the Throne Room, was nearly empty. This packet adds 24 friendly and 36 hostile spawns, 13 templates and 8 patrol loops so the zone reads as a prison in the middle of a breakout. It is seed only: no engine code changed.

## Story frame

Taken from the shipped dialogs; the placements do not contradict them.

- The Castle is an NID prison on a frozen planet. Op-CORE prisoners are breaking out from the inside.
- Sgt. Gerschon holds the Armory: "lots of people heading for the gate and the NID guys are not happy" (dialog 2573).
- Capt. Copplemann's fire team was wiped out by NID reinforcements: "They didn't make it" (2574).
- Col. Marsh and Moh'katan's Praxis Jaffa came from outside and are pinned at the Stargate, unable to dial (2584).
- The rest of the Op-CORE prisoners "are following behind" the player (2584).
- Cut content, dialogs 4982-4985: soldiers out of the East Wing stasis pods are "collapsing with seizures"; the Level-5 infirmary holds the ambernol, and Ogilvie is "in charge of" the supply. Objective text 19893 says "Speak to Ogilvie to learn where the Ambernol is stored."

## Id blocks

| Table | Ids | Notes |
|---|---|---|
| `entity_templates` | 174-186 | 174-180 friendly, 181-186 hostile. 187-199 stay free. |
| `spawnlist` | 189-212 (friendly), 247-282 (hostile) | Both blocks were unused gaps below the Harset block (300-399). |
| `point_sets` | 2086-2093 | Type `Patrol`, shape `Path`, never loaded as client regions. |
| `point_set_points` | 2413-2430 | Waypoints of 2086-2093. |

## Decisions

| ID | Decision | Why |
|---|---|---|
| D-CP01 | Every row is a static `spawnlist` row. No phasing, no chain spawns. | Castle actors are static rows (D-CA06); the population must be the same for every player. |
| D-CP02 | New NID guard templates at levels 2, 3 and 4, in an inside variant (clone of 148) and an outside variant (clone of 146). Templates 145, 146 and 148 are not changed. | The Castle missions are levels 3-4 and every existing guard is level 1. Editing 146/148 would re-level every pre-existing Castle guard at once. |
| D-CP03 | Guards reuse the shipped guard names: 7417 'NID Guard' inside, 7703 'Exterior NID Guard' outside. | Both monikers end in `_St_1-5`, their level band, so levels 2-4 fit the strings. `name_id` must be a shipped moniker; new ids cannot render. |
| D-CP04 | Radii on the new guards: `aggro_radius` 15 inside and 20 outside, `assist_radius` 12. Each group is authored with its members within 12 u of a neighbour and at least 12 u from any other group. | Shooting one guard rallies its own post and nothing else; assist never chains. Interior posts sit in corridors where the default 18 u reaches through a doorway into the next room. The one deliberate exception is the Interrogation Block pair, which stands 5-6 u from Romney as his escort. |
| D-CP05 | Friendlies are faction 1 with no ability set. | Faction 1 cannot be damaged and reads FRIENDLY to players. NPCs do not fight each other yet, so a friendly never needs a weapon ability. |
| D-CP06 | Friendlies stand at least 5 u outside every hostile's aggro radius. `Castle_Standoff_*` rows face hostile ground from behind cover. The caged prisoners are the one exemption. | Inside range, a friendly would stand idle next to a guard shooting the player. When NPC-vs-NPC combat exists, search for the `Castle_Standoff_` tag prefix and move those rows into range deliberately. |
| D-CP07 | No hostile's aggro radius, plus 3 u for where a player stands, reaches a respawner, the ring pad or a mission actor the player talks to or uses. Patrols are checked along the whole loop. | A player who respawns or stops to talk must not be shot, and with a 120 s respawn a mistake here recurs every two minutes. |
| D-CP08 | Hostiles respawn after 120 s; friendlies have no timer. | 120 s is the zone-wide Castle value (CA05). Faction-1 friendlies can never die. |
| D-CP09 | New guards carry no loot table, like 146 and 148. | Not asked for. Loot table 2 ('Cellblock NID guard default': naquadah and a Health Slappack) is the obvious candidate if the owner wants one; see the open questions. |
| D-CP10 | Ogilvie is named with moniker 7342 (`DN_NPC_MG_Ogilvie_Hebridan_PraxisContact` = 'Ogilvie'). | The Castle's own moniker 8895 (`DN_npc_int_Ogilvie_Castle`) ships an empty string and would render no name. |

## Templates

| Id | Template name | Shown as | Level | Class | Notes |
|---|---|---|---|---|---|
| 174 | Castle_OpCoreSoldier | Op-CORE Soldier (7552) | 4 | mob | Prisoner garb, SMG, head 02 |
| 175 | Castle_OpCoreSoldier_Wander | Op-CORE Soldier (7552) | 4 | mob | 174 with a 3 u wander and a 6-15 s dwell; Armory only |
| 176 | Castle_OpCoreSoldier_Unarmed | Op-CORE Soldier (7552) | 4 | mob | Stasis-sick and caged prisoners, no weapon |
| 177 | Castle_Medic | Castle Medic (6964) | 4 | mob | Female body so the triage group is not one body four times |
| 178 | Castle_SgtStanton | Sgt. Stanton (7036) | 50 | mob | Armed; level 50 is the seed's talk-only sentinel |
| 179 | Castle_Ogilvie | Ogilvie (7342) | 50 | mob | Unarmed |
| 180 | Castle_OpCoreSoldierCorpse | Op-CORE Soldier (7552) | - | being | Mesh `CA-Props.CA-PrisonerCorpse00`, like template 14 |
| 181 / 182 / 183 | NID Guard - Castle inside L2 / L3 / L4 | NID Guard (7417) | 2 / 3 / 4 | mob | Clone of 148; aggro 15, assist 12 |
| 184 / 185 / 186 | NID Guard - Castle outside L2 / L3 / L4 | Exterior NID Guard (7703) | 2 / 3 / 4 | mob | Clone of 146; aggro 20, assist 12 |

Heads avoid 01 (every NID guard), 05 (Zuritska) and 09 (Gerschon), so no ambient marine shares a face with a named actor.

### What `level` drives

Measured on `main` at `0c1ee689`. A mob's level is read in three places and nowhere else:

| Effect | Formula | Source | L2 / L3 / L4 |
|---|---|---|---|
| Max HP | 200 + 50 x level | `crates/cell-world/src/cell/space_manager/spawn.rs` (`spawn_npc_from_record_into`) | 300 / 350 / 400 |
| Kill XP | 10 x level | `crates/cell-combat/src/cell/abilities/loot_drop.rs` (`kill_xp`) | 20 / 30 / 40 |
| Client level display | `onLevelUpdate` on the AoI create packet | `mercury/aoi/create.rs`, for class `being` and `mob` only | shown on the target frame |

Damage, hit chance, aggro, leash and threat do not read `level`. A level-4 guard hits exactly as hard as a level-1 one; the spread changes how long a guard takes to kill and what it pays. Level is template-only (`spawnlist` has no level column).

## Placement

Every coordinate was probed against `data/spaces/castle.nav` with `NavMesh::is_point_valid` and `start_poly_snap` (the checks `crates/entity/tests/castle_navmesh.rs` makes), and `y` is the navmesh floor height from `get_height_near`, not a guess. The component column comes from `nav_inspect --probe`. Component 250 is the connected mesh from the Armory to the outdoor field, 116 is Checkpoint Bravo, the bunker and Checkpoint Alpha, and 468 is the isolated Symbiote Chamber pocket. Nothing is placed on component 501, the sealed mirror wing of the Interrogation Block.

Headings follow the seed convention: 0 faces +Z, pi/2 (1.570796) faces +X.

### Friendlies (spawns 189-212)

| Spawn | Tag | Template (shown as, level) | Position (x, y, z) | castle.nav | Notes |
|---|---|---|---|---|---|
| 189 | `Castle_Pop_Armory_Soldier1` | 174 (Op-CORE Soldier, 4) | 446, 70.34, 1000.5 | valid, comp 250 |  |
| 190 | `Castle_Pop_Armory_Soldier2` | 174 (Op-CORE Soldier, 4) | 458.5, 70.18, 995.5 | valid, comp 250 |  |
| 191 | `Castle_Pop_Armory_Soldier3` | 175 (Op-CORE Soldier, 4) | 437, 70.18, 1003 | valid, comp 250 | wanders 3 u |
| 192 | `Castle_Pop_Armory_Soldier4` | 175 (Op-CORE Soldier, 4) | 451, 70.18, 988 | valid, comp 250 | wanders 3 u |
| 193 | `Castle_Pop_Armory_Wounded1` | 176 (Op-CORE Soldier, 4) | 432.5, 70.18, 986 | valid, comp 250 |  |
| 194 | `Castle_Pop_Hall_MarineCorpse1` | 180 (Op-CORE Soldier (corpse), -) | 367.5, 70.38, 947.5 | valid, comp 250 |  |
| 195 | `Castle_Pop_Hall_MarineCorpse2` | 180 (Op-CORE Soldier (corpse), -) | 367.5, 70.38, 958 | valid, comp 250 |  |
| 196 | `Castle_Pop_Hall_MarineCorpse3` | 180 (Op-CORE Soldier (corpse), -) | 349.5, 70.38, 962 | valid, comp 250 |  |
| 197 | `Castle_Ogilvie` | 179 (Ogilvie, 50) | 362, 70.18, 886 | valid, comp 250 |  |
| 198 | `Castle_Pop_Infirmary_Medic1` | 177 (Castle Medic, 4) | 352, 70.18, 885 | valid, comp 250 |  |
| 199 | `Castle_Pop_Infirmary_Sick1` | 176 (Op-CORE Soldier, 4) | 349, 70.48, 883.5 | valid, comp 250 |  |
| 200 | `Castle_Pop_Prisoner1` | 176 (Op-CORE Soldier, 4) | 234, 66.98, 1048 | valid, comp 250 |  |
| 201 | `Castle_Pop_Prisoner2` | 176 (Op-CORE Soldier, 4) | 284, 66.98, 1048 | valid, comp 250 |  |
| 202 | `Castle_Pop_Prisoner3` | 176 (Op-CORE Soldier, 4) | 290, 66.98, 1050 | valid, comp 250 |  |
| 203 | `Castle_Pop_Prisoner4` | 176 (Op-CORE Soldier, 4) | 236, 66.98, 1024 | valid, comp 250 |  |
| 204 | `Castle_Standoff_SgtStanton` | 178 (Sgt. Stanton, 50) | 500, 28.18, 652 | valid, comp 250 |  |
| 205 | `Castle_Standoff_Courtyard_Soldier1` | 174 (Op-CORE Soldier, 4) | 507.5, 28.18, 640 | valid, comp 250 |  |
| 206 | `Castle_Standoff_Courtyard_Soldier2` | 174 (Op-CORE Soldier, 4) | 507.5, 28.18, 644.5 | valid, comp 250 |  |
| 207 | `Castle_Standoff_Courtyard_Soldier3` | 174 (Op-CORE Soldier, 4) | 507.5, 28.18, 661.5 | valid, comp 250 |  |
| 208 | `Castle_Standoff_Courtyard_Soldier4` | 174 (Op-CORE Soldier, 4) | 507, 28.18, 666 | valid, comp 250 |  |
| 209 | `Castle_Standoff_Alpha_Jaffa1` | 160 (Praxis Jaffa Guard, 50) | 807, 55.06, 527 | valid, comp 116 |  |
| 210 | `Castle_Standoff_Alpha_Jaffa2` | 160 (Praxis Jaffa Guard, 50) | 812, 55.02, 523.5 | valid, comp 116 |  |
| 211 | `Castle_Standoff_Alpha_Jaffa3` | 160 (Praxis Jaffa Guard, 50) | 803, 56.36, 533 | valid, comp 116 |  |
| 212 | `Castle_Pop_Alpha_WoundedJaffa` | 160 (Praxis Jaffa Guard, 50) | 795, 56.26, 520 | valid, comp 116 |  |

### Hostiles (spawns 247-282)

| Spawn | Tag | Template (shown as, level) | Position (x, y, z) | castle.nav | Notes |
|---|---|---|---|---|---|
| 247 | `Castle_Pop_HallPost_1` | 183 (NID Guard, 4) | 351, 70.38, 1000.5 | valid, comp 250 |  |
| 248 | `Castle_Pop_HallPost_2` | 182 (NID Guard, 3) | 356, 70.38, 1006 | valid, comp 250 |  |
| 249 | `Castle_Pop_HallPost_3` | 181 (NID Guard, 2) | 350, 70.38, 993 | valid, comp 250 |  |
| 250 | `Castle_Pop_HallPatrolN_1` | 182 (NID Guard, 3) | 279, 70.18, 952 | valid, comp 250 | patrol set 2086 |
| 251 | `Castle_Pop_HallPatrolN_2` | 181 (NID Guard, 2) | 282, 70.18, 952 | valid, comp 250 | patrol set 2087 |
| 252 | `Castle_Pop_HallPatrolW_1` | 182 (NID Guard, 3) | 262, 70.18, 929 | valid, comp 250 | patrol set 2088 |
| 253 | `Castle_Pop_HallPatrolW_2` | 181 (NID Guard, 2) | 262, 70.18, 932 | valid, comp 250 | patrol set 2089 |
| 254 | `Castle_Pop_HallPRU_1` | 145 (Prisoner Retrieval Unit, 1) | 256, 70.25, 996 | valid, comp 250 | patrol set 2090 |
| 255 | `Castle_Pop_IntBlock_1` | 182 (NID Guard, 3) | 248, 67.18, 1032.5 | valid, comp 250 |  |
| 256 | `Castle_Pop_IntBlock_2` | 181 (NID Guard, 2) | 249, 67.18, 1039.5 | valid, comp 250 |  |
| 257 | `Castle_Pop_Symbiote_1` | 182 (NID Guard, 3) | 386, 55.38, 940 | valid, comp 468 | stationary |
| 258 | `Castle_Pop_Symbiote_2` | 181 (NID Guard, 2) | 391, 55.38, 925 | valid, comp 468 | stationary |
| 259 | `Castle_Pop_Symbiote_3` | 145 (Prisoner Retrieval Unit, 1) | 383, 55.38, 932 | valid, comp 468 | stationary |
| 260 | `Castle_Pop_CommsDoor_1` | 182 (NID Guard, 3) | 266, 55.38, 885 | valid, comp 250 |  |
| 261 | `Castle_Pop_CommsDoor_2` | 181 (NID Guard, 2) | 277, 55.38, 887 | valid, comp 250 |  |
| 262 | `Castle_Pop_TransA_1` | 182 (NID Guard, 3) | 370, 48.32, 826 | valid, comp 250 |  |
| 263 | `Castle_Pop_TransA_2` | 181 (NID Guard, 2) | 378, 48.22, 828 | valid, comp 250 |  |
| 264 | `Castle_Pop_TransB_1` | 183 (NID Guard, 4) | 360, 48.38, 768 | valid, comp 250 |  |
| 265 | `Castle_Pop_TransB_2` | 182 (NID Guard, 3) | 366, 48.38, 772 | valid, comp 250 |  |
| 266 | `Castle_Pop_TransB_3` | 181 (NID Guard, 2) | 357, 48.38, 758 | valid, comp 250 |  |
| 267 | `Castle_Pop_TransC_1` | 182 (NID Guard, 3) | 410, 43.38, 770 | valid, comp 250 |  |
| 268 | `Castle_Pop_TransC_2` | 182 (NID Guard, 3) | 420, 43.97, 770 | valid, comp 250 |  |
| 269 | `Castle_Pop_TransPatrol_1` | 181 (NID Guard, 2) | 352, 48.38, 714 | valid, comp 250 | patrol set 2091 |
| 270 | `Castle_Pop_TransPatrol_2` | 181 (NID Guard, 2) | 352, 48.38, 717 | valid, comp 250 | patrol set 2092 |
| 271 | `Castle_Pop_Throne_1` | 183 (NID Guard, 4) | 366, 38.38, 643 | valid, comp 250 |  |
| 272 | `Castle_Pop_Throne_2` | 183 (NID Guard, 4) | 366, 38.37, 663 | valid, comp 250 |  |
| 273 | `Castle_Pop_Throne_3` | 182 (NID Guard, 3) | 376, 38.33, 643 | valid, comp 250 |  |
| 274 | `Castle_Pop_Throne_4` | 182 (NID Guard, 3) | 376, 38.37, 663 | valid, comp 250 |  |
| 275 | `Castle_Pop_FieldW_1` | 186 (Exterior NID Guard, 4) | 611, 18.38, 703 | valid, comp 250 |  |
| 276 | `Castle_Pop_FieldW_2` | 185 (Exterior NID Guard, 3) | 619, 18.28, 695 | valid, comp 250 |  |
| 277 | `Castle_Pop_FieldW_3` | 184 (Exterior NID Guard, 2) | 628, 18.41, 690 | valid, comp 250 |  |
| 278 | `Castle_Pop_FieldW_4` | 145 (Prisoner Retrieval Unit, 1) | 600, 20.02, 690 | valid, comp 250 | patrol set 2093 |
| 279 | `Castle_Pop_BunkerApproach_1` | 185 (Exterior NID Guard, 3) | 985, 47.67, 427 | valid, comp 116 |  |
| 280 | `Castle_Pop_BunkerApproach_2` | 184 (Exterior NID Guard, 2) | 994, 47.58, 425 | valid, comp 116 |  |
| 281 | `Castle_Pop_BunkerTunnel_1` | 183 (NID Guard, 4) | 1052, 48.18, 432 | valid, comp 116 |  |
| 282 | `Castle_Pop_BunkerTunnel_2` | 182 (NID Guard, 3) | 1058, 48.18, 426 | valid, comp 116 |  |

### Patrol loops

| Set | Name | Walked by | Waypoints |
|---|---|---|---|
| 2086 / 2087 | `Castle.Patrol.HallNorth_A` / `_B` | 250 / 251 | x 279 / 282, z 952 to 976 (the corridor to the Interrogation Block), 4 s dwell |
| 2088 / 2089 | `Castle.Patrol.HallWest_A` / `_B` | 252 / 253 | z 929 / 932, x 262 to 300 (the west hall toward the Comms level), 4 s dwell |
| 2090 | `Castle.Patrol.InterrogationAntechamberPRU` | 254 | z 996, x 256 to 298 |
| 2091 / 2092 | `Castle.Patrol.ThroneApproach_A` / `_B` | 269 / 270 | z 714 / 717, x 352 to 420 (the room above the Throne Room entrance), 4 s dwell |
| 2093 | `Castle.Patrol.FieldWestPRU` | 278 | Four-point loop round the rock at (621, 19.5, 702) |

A two-point set is walked as a back-and-forth line. Guard pairs use parallel sets 3 u apart so the two guards walk side by side instead of stacking on one line. Every leg routes all the way on `castle.nav`.

### Per-zone notes

| # | Zone | What is there | Clearances that shaped it |
|---|---|---|---|
| 1 | Armory / ring-pad arrival | Four armed marines at the lockers around Gerschon (two wander 3 u), one stasis-sick marine | 8 u or more off the ring pad (466.4, 991.5, radius 3.5); no hostile |
| 2 | 701 hallway | Three-guard post (L4, L3, L2) in the junction room at the top of Copplemann's corridor; two patrol pairs; a PRU patrol in the Interrogation antechamber; three marine corpses along the corridor walls near Copplemann | The post is 40 u or more from Copplemann, so the 701 talk and Livewire are never pulled; the corpses sit off the corridor centreline her escort walks |
| 2b | Level-5 infirmary (Op-Core Triage) | Ogilvie, a Castle Medic and a stasis-sick marine inside point set 2051 `Castle.Infirmary` | 35 u or more from `Castle_PRU1`, the nearest hostile |
| 3 | Interrogation Block | Two guards 5-6 u from Romney as his escort; four caged prisoners | 19 u or more from Zuritska's cell door, so freeing Zuritska does not pull them |
| 4 | Symbiote Chamber | Two guards and a PRU, `is_stationary` | Isolated component 468: nothing can path in or out |
| 5 | Comms approach | Two guards in the antechamber the 704 escort walks through | 27 u or more from Zuritska's comms spot and the terminal |
| 6 | Transition corridors (Comms level to Throne Room) | Posts of 2, 3 and 2 guards; a patrol pair | Zero spawns before; posts are 41 u or more apart |
| 7 | Throne Room | Four guards (two L4, two L3) on the main floor between the pillars | 22 u or more from the Throne respawner and 36 u from the Access Panel |
| 8 | Front Courtyard | Sgt. Stanton and four marines behind the east barricade, facing the field | 57 u or more from the nearest field hostile; the NE corner is avoided |
| 9 | Outdoor field, west | Three exterior guards (L4, L3, L2) round the rock, a PRU loop | 55 u or more from the pre-existing field guards, patrol loop included |
| 10 | Muelbach's bunker | Two exterior guards on the approach, two interior guards in the room past her | 17 u or more from Muelbach |
| 11 | Checkpoint Alpha | Three Praxis Jaffa (template 160) facing the north-east ramp the Bravo road climbs, one more at the back of the room | Far from every hostile (the field is 25-30 u lower) |

"Wounded" (Armory, Checkpoint Alpha) is narrative only: there is no pose column and no per-spawn health, so the wounded marine is the unarmed template standing near the others, and the wounded Jaffa is a Jaffa at the back of the room.

## Excluded, and TODO

- **TODO (owner, in-client): the elevated cover cluster near (636, 34, 296).** `nav_inspect` puts walkable floor 10-21 m below that y, so the cover sits on a structure (a wall walk or battlement) nobody has identified. A guard perch there needs an in-client `.location` check first.
- **TODO (owner, in-client): the Front Courtyard NE corner (508, 36, 672).** It failed the nav height check by 7.8 m. The courtyard marines stay at z 666 or below.
- **Future: NPC-vs-NPC combat.** The `Castle_Standoff_*` rows and the caged prisoners are placed for a world where NPCs do not fight. When that lands, move the standoff rows into range on purpose and update the friendly-clearance guard.
- **CA15** (the optional infirmary and symbiote branches) can bind a dialog to `Castle_Ogilvie`; this pass leaves him ambient.

## Tests

| Test | Kind | What it pins |
|---|---|---|
| `every_castle_spawn_is_on_the_mesh` (`crates/entity/tests/castle_navmesh.rs`) | No-DB, existing | Picks up all 60 new rows automatically |
| `every_castle_patrol_leg_is_routable` (same file) | No-DB, new | Every World 8 `Patrol` waypoint is on the mesh and every leg, including the one that closes the loop, routes all the way |
| `castle_population_live_db_rows_are_world8_blocks_with_their_templates` (`crates/cell-catalog/src/cell/spawner/tests/live_db_castle_population.rs`) | Live-DB, new | 24 + 36 rows in World 8 on the assigned templates, unique tags, 120 s hostile respawn, patrols start on their spawn, all six guard templates placed |
| `castle_population_live_db_friendlies_are_safe_and_named` | Live-DB, new | Faction 1, no hostile override, `name_id` resolves to a non-empty `resources.texts` string |
| `castle_population_live_db_guard_templates_are_levels_2_to_4_and_old_ones_unchanged` | Live-DB, new | 181-186 levels, radii, SMG kit and cover; 145/146/148 level, faction, name, radii, respawn and loot unchanged |
| `castle_population_live_db_hostile_aggro_clears_respawners_ring_pad_and_actors` | Live-DB, new | D-CP07 for every World 8 hostile, patrol loops included |
| `castle_population_live_db_friendlies_stand_outside_every_hostile_aggro_radius` | Live-DB, new | D-CP06 |

### Mutation proof (2026-09-28)

Each mutation was applied to the seed, the tests were rerun, and the seed was restored.

| Mutation | Failing test and message |
|---|---|
| Delete hostile spawn 282 | `..._rows_are_world8_blocks_...`: "hostile block 247-282 must hold 36 rows" |
| Ogilvie's `name_id` set to 8895 | `..._friendlies_are_safe_and_named`: "name_id 8895 resolves to Some((\"\",))" |
| Template 146 re-levelled 1 to 2 in place | `..._guard_templates_are_levels_2_to_4_...`: "template 146 must stay level 1" |
| `Castle_Pop_Throne_1` moved to (355, 38.38, 650) | `..._hostile_aggro_clears_...`: "comes within 10.0 u of respawner 'Throne Checkpoint Respawn'" |
| `Castle_Standoff_Courtyard_Soldier1` moved to (560, 24, 612) | `..._friendlies_stand_outside_...`: "is 7.5 u from hostile Castle_PRU4" |
| Spawn 275 lifted 5 u to y 23.38 | `every_castle_spawn_is_on_the_mesh` names spawn 275 |
| Waypoint 2424 moved into the Symbiote Chamber (386, 55.38, 940) | `every_castle_patrol_leg_is_routable`: both legs of set 2091 report `partial` |

Moving waypoint 2424 to the Throne Room floor did **not** fail the patrol test: that floor is on the same connected mesh, so the leg routes. The proof uses an isolated component instead.

## Open questions for the owner

1. Should the new guards drop loot (table 2: naquadah 5-50 at 80 %, one Health Slappack)? The zone now has 36 more hostiles and no new heals.
2. Should the outdoor guards keep the 'Exterior NID Guard' name, or show 'NID Guard' like the pre-existing field guards (template 146)?
3. The two TODO spots above need an in-client look before anything is placed there.

## UAT checklist

Run on a build with this packet. A GM character can reach each zone with `.gotolocation Castle <x> <y> <z>` using the positions above. Use `.bug <note>` at any wrong spot. Canonical ids: CP1-CP16.

| # | Do | Expect | Notes |
|---|---|---|---|
| CP1 | Arrive in the Armory by the ring | Four armed marines at the weapon lockers and one unarmed marine, all named "Op-CORE Soldier"; two of the armed ones take a few steps now and then. Nobody stands on the ring pad. | |
| CP2 | Right-click a marine | No attack starts; nothing else happens. | Ambient NPCs carry no dialog |
| CP3 | Talk to Gerschon and accept 701 | No hostile interrupts the conversation. | |
| CP4 | Walk west out of the Armory hall | A three-guard post in the junction room at the top of Copplemann's corridor. Their levels read 4, 3 and 2. Shooting one pulls that post, not the patrols further west. | Pre-existing guard 89 and PRU5 are also on this stretch |
| CP5 | Reach Copplemann | Three dead Op-CORE soldiers in the corridor around her. Talking to her and playing the Livewire is never interrupted, including after the post respawns (120 s). | Corpse invisible: see K5 |
| CP6 | Go to the south end of the corridor (Op-Core Triage) | Ogilvie, a female Castle Medic and an unarmed marine. Die and respawn at the Op-Core Triage checkpoint: nothing shoots you on arrival. | |
| CP7 | Walk the corridor north to the Interrogation Block, and the west hall | Two pairs of guards walking side by side and pausing at each end; a drone patrolling the Interrogation antechamber. | |
| CP8 | Enter the Interrogation Block | Two guards stand beside Romney; four prisoners behind cell doors. Freeing Zuritska at her cell does not pull Romney's guards. | |
| CP9 | Visit the Symbiote Chamber (398.8, 55, 935) | Two guards and a drone hold their positions and fire. | Stationary by design |
| CP10 | Escort Zuritska to the Communications room | Two guards in the antechamber before the room. In the room, at the terminal and while talking to Zuritska, nothing aggroes. | |
| CP11 | Walk from the Comms level to the Throne Room | Guard posts in the cross-shaped room, at the corridor cover and in the east side room; a patrol pair in the big room before the Throne Room. | |
| CP12 | Enter the Throne Room; use the Access Panel; die and respawn at the Throne checkpoint | Four guards on the floor between the pillars. The Access Panel and the respawn point are out of their reach. | |
| CP13 | Go to the Front Courtyard | Sgt. Stanton and four marines behind the east barricade, facing the field. Nobody fights. | |
| CP14 | Cross the west of the outdoor field | Three "Exterior NID Guard" by the rock (levels 4, 3, 2) and a drone circling it. | |
| CP15 | Go to Muelbach's bunker | Two exterior guards on the approach; two guards in the room past Muelbach. | |
| CP16 | Reach Checkpoint Alpha; die and respawn there | Three Praxis Jaffa facing the ramp you came up, one more at the back. Nothing shoots you on respawn. | |

Things only a human can check: every placement looks sensible (nobody inside a wall, a locker or a table), the medic's female body and the soldiers' faces render, headings face where the notes say, and the level numbers show on the target frame.
