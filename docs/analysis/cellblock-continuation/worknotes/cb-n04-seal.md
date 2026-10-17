# CB-N04 seal: the cluster 1 ambient cast

Status: **sealed** by CB-N04s (npc-ai-spawn-advisor, 2026-10-17). CB-N04m signs
off next; CB-N04a builds from `## Values` and `## SQL` only. Packet:
[w2-packets.md](../w2-packets.md#cb-n04s--seal-the-cluster-1-cast). Candidates:
[route-baseline.md](../route-baseline.md).

## Cluster

Room word: **`cells`** (CB-N04a branch `cellblock/cb-n04-cells`, commit
`feat(cellblock): add cells escape ambience`).

The pocket is the north cell blocks of `Castle_Cellblock.Region2` (point set
2033, a rectangle x -311.19 to -241.99, z -141.13 to -115.19). Two unarmed
Op-CORE prisoners (template 176) stand at x -271 and x -254, z -135, on
navmesh component 3, the same component as the wake room and Prisoner 329.
They are the first living faces a player passes after leaving cell 329, and
their line reacts to the escape.

Why this pocket:

- On the route at first approach: the walk from Prisoner 329 east to the Med
  Station (mission 639 step 2117) crosses Region2.
- Nearest hostile is 26.7 m away (spawn 20, template 15), outside the 18 u
  aggro radius plus 5 m. Every template-24 NID guard is 117 m or more away.
- Nearest protected anchor is 22.5 m (the vial, the Med Station proxy). No
  tutorial prompt, Marsh bark (1176-1178) or CB-N05 chain (11000-11005) uses
  step 2117 on Region2.

Excluded:

- Stasis side (Region1, set 2032): D-CC12 debug hub floor, and both positions
  are outside its polygon (Evidence).
- Wake pod room, Med Station and vial, Preparation, Hallway05, Mess Hall,
  Barracks, Armory side: protected areas, guard clusters or off the route.
- Prisoner 329 cell edge (CB-N05 surface; the cast is 28.8 m away and does
  not speak about 329). Templates 174 (armed), 175 (wanders), 180 (corpse).

## Values

CB-N04a copies this table into its test constants.

| Key | Value |
|---|---|
| `<room>` | `cells` |
| `SPEAKER` | `Op-CORE Soldier` (moniker 7552; template 176 `name_id` 7552) |
| Cast row 1 | tag `Cellblock_Ambient_Prisoner1`, spawn 520, template 176, x -271.0, y 65.45, z -135.0, heading 1.570796 |
| Cast row 2 | tag `Cellblock_Ambient_Prisoner2`, spawn 521, template 176, x -254.0, y 65.45, z -135.0, heading 4.712389 |
| Cast flags | `is_stationary` true on both rows; `set_name` NULL; no `aggression_override` |
| Bark 1 | chain 11010, `scope_id` 639, screen 200160, holder dialog 60160 |
| Bark 1 region key | `Castle_Cellblock.Region2` (lowercase b; point set 2033) |
| Bark 1 happy params | `("mission_639_step_2117_status", "active")` |
| Bark 1 refused: phase not reached | `("mission_639_step_2117_status", "not_active")` (638 phase, or 639 not accepted) |
| Bark 1 refused: phase passed | `("mission_639_step_2117_status", "completed")` (step 2145 onward) |
| `GUARD_CLEARANCE_M` | 117.0 (template 24 only; measured minimum 117.62 m; required floor 31 m) |
| `PROTECTED_CLEARANCE_M` | 22.0 (measured minimum 22.46 m) |

Protected anchors (BigWorld, y up; source is the world-12 spawnlist row unless
stated):

| Anchor | x | y | z | Source |
|---|---|---|---|---|
| `WakePod` | -334.23 | 73.47 | -228.03 | wake position, [route-baseline.md](../route-baseline.md) |
| `AmbernolVial` (Med Station proxy) | -234.04 | 66.52 | -124.70 | spawn 12 |
| `Preparation_RingSwitch` | -193.92 | 56.32 | -152.16 | spawn 17 |
| `HackTheRings_Switch` | -218.08 | 67.04 | -122.72 | spawn 23 |
| `Cellblock_ArmoryRingSwitch` | -54.88 | 26.08 | -163.84 | spawn 79 |
| `Cellblock_UnusedRingSwitch` | -87.04 | 46.24 | -160.32 | spawn 80 |
| `Hallway05_Guard1` | -100.16 | 24.67 | -43.90 | spawn 32 |
| `Hallway05_Guard2` | -101.50 | 24.67 | -51.30 | spawn 33 |
| `MessHall_Guard1` | -96.25 | 34.59 | -91.59 | spawn 29 |
| `MessHall_Guard2` | -95.89 | 34.59 | -98.81 | spawn 28 |
| `Prisoner_329` | -289.81 | 65.47 | -113.14 | spawn 6 |
| `329_CellDoorButton` | -287.29 | 67.26 | -115.04 | spawn 22 |
| `ArmYourself_FrostBody` | -328.30 | 73.47 | -210.27 | spawn 19 |
| `ArmYourself_GuardBody` | -322.51 | 73.47 | -209.83 | spawn 15 |

Heading is cosmetic (stationary). The rows face each other along x if heading
0 is +z and pi/2 is +x; that is not proven here, so CB-U08 checks by eye.

Bark copy (screen 200160, 98 characters, no apostrophes):

> Somebody is loose out there. I heard a cell lock give way. Nobody walks out of this block. Nobody.

## SQL

Exact blocks CB-N04a pastes (the [w2-packets.md templates](../w2-packets.md#seed-templates), filled).
`db/resources/Worlds/Seed/spawnlist.sql`:

```sql
-- CB-N04a (2026-10-17): Cellblock ambient cast, cells. Friendly Op-CORE
-- prisoners (faction 1, no ability set), held in place by is_stationary.
-- Placement, nav probe and guard clearance:
-- docs/analysis/cellblock-continuation/worknotes/cb-n04-seal.md.
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (520, -271.0, 65.45, -135.0, 1.570796, 12, 176, 'Cellblock_Ambient_Prisoner1', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (521, -254.0, 65.45, -135.0, 4.712389, 12, 176, 'Cellblock_Ambient_Prisoner2', NULL, true);
```

`db/resources/Dialogs/Seed/dialogs.sql`:

```sql
-- Cellblock continuation CB-N04a: holder for the cells ambient barks. Never
-- displayed; npc_bark reads its screens' text. Seed-only, no override.
INSERT INTO dialogs (dialog_id, dialog_flags, event_set_id, ui_screen_type, tags, accepts_mission_id, name) VALUES (60160, 0, NULL, 'DUIST_DefaultDialog', NULL, NULL, 'Cellblock - ambient cast bark holder (never displayed)');
```

`db/resources/Dialogs/Seed/dialog_screens.sql` (then set the sequence line to 200160):

```sql
-- Cellblock continuation CB-N04a: cells ambient bark lines (CIMMERIA_AUTHORING).
-- Speaker 0: npc_bark supplies the chat prefix from its own params.
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (60160, 200160, 'Somebody is loose out there. I heard a cell lock give way. Nobody walks out of this block. Nobody.', 0, 0);
```

`db/resources/Content/Seed/castle_cellblock_chains.sql`:

```sql
-- =====================================================================
-- CB-N04a (2026-10-17): Cellblock ambient cast, cells
-- =====================================================================
-- Two friendly Op-CORE prisoners (spawns 520, 521) show the escape is under
-- way. Chain 11010 speaks one line into the crossing player's chat on the
-- first entry to Castle_Cellblock.Region2 while mission 639 step 2117 (find
-- Ambernol in the Med Station) is active. once = true stops the H52
-- step-activation replay speaking twice in a visit; the step gate stops it
-- once the vial step is done. Seal: docs/analysis/cellblock-continuation/
-- worknotes/cb-n04-seal.md. Holder dialog 60160 is never displayed.

-- Chain 11010: Region2 entry during 639/2117 -> Op-CORE prisoner bark
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (11010, 'CB-N04a - cells ambience: enter Region2 during 639 step 2117 -> Op-CORE prisoner bark', 'mission', 639, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (11010, 'enter_region', 'Castle_Cellblock.Region2', 'player', true, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (11010, 'step_status', 639, '2117', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (11010, 'npc_bark', NULL, NULL,
 '{"screen_id": 200160, "speaker": "Op-CORE Soldier", "channel": "say"}', 0, 0);
```

## Evidence

### nav_inspect (exit 0)

Probes: `CastA_Prisoner1 -271.0 65.45 -135.0`, `CastB_Prisoner2 -254.0 65.45
-135.0`, `Prisoner329 -289.808 65.474 -113.139`. Run through
`lane.ps1 cargo run -p cimmeria-navmesh-extractor --bin nav_inspect --
data/spaces/castle_cellblock.nav --probes <file>`; lane log
`cellblock-cb-n04s/20261017-180839-35552.log`.

```text
components  17 (total walkable XZ area 674552.9 m^2)
  3    293          4688.1   [-341.8, -213.1] / [65.4, 73.8] / [-236.2, -102.4]
probes (h-tol 2.00 m, v-tol 3.00 m)
  CastA_Prisoner1      poly=644    component=3    h=0.00 m  dy=-0.03 m  ok
  CastB_Prisoner2      poly=668    component=3    h=0.00 m  dy=-0.03 m  ok
  Prisoner329          poly=990    component=3    h=0.00 m  dy=-0.13 m  ok
```

All three land in component 3 of the rebuilt 17-component mesh (#694), within
0.13 m of the floor. The cast is stationary (skips `spawn_off_mesh`), so this
proves the player's route around the NPCs, not their pathing.

### Region1 containment (D-CC12)

`Castle_Cellblock.Region1` is point set 2032, points (x, z): (-347.17,
-230.14), (-327.67, -240.70), (-318.89, -224.51), (-338.39, -213.94). Ray cast
in x/z: cast 1 (-271.0, -135.0) and cast 2 (-254.0, -135.0) are both outside
(Region1's largest x is -318.89, 47 m west of cast 1). Both lie inside Region2
(y box 65.32 to 75.43; cast y 65.45).

### Distances (horizontal x/z, metres)

3D distance is never smaller, so these are valid floors for the 3D test.

| Hostile (world 12) | Template | To cast 1 | To cast 2 | dy |
|---|---|---|---|---|
| spawn 20 `ArmYourself_NIDGuard` | 15 | 26.69 | 40.36 | +3.09 |
| spawn 10 `ArmYourself_PrisonerRetrievalUnit` | 4 | 52.54 | 36.39 | +1.29 |
| spawn 36 `Barracks_Guard3` | 24 | 134.62 | 117.62 | -40.78 |
| spawn 26 `Barracks_Guard2` | 24 | 140.68 | 123.84 | -40.78 |
| spawn 25 `Barracks_Guard1` | 24 | 153.50 | 136.61 | -40.78 |

Of 14 hostile rows, these are the five nearest. Template-24 minimum 117.62 gives `GUARD_CLEARANCE_M` 117. The test counts
template 24 only.

| Protected anchor | To cast 1 | To cast 2 |
|---|---|---|
| AmbernolVial (12) | 38.37 | 22.46 |
| HackTheRings_Switch (23) | 54.33 | 37.96 |
| Preparation_RingSwitch (17) | 78.97 | 62.48 |
| Prisoner_329 (6) / 329_CellDoorButton (22) | 28.84 / 25.76 | 41.95 / 38.82 |
| FrostBody (19) / GuardBody (15) | 94.60 / 90.84 | 105.76 / 101.46 |
| WakePod | 112.48 | 122.85 |
| Hallway05, Mess Hall guards, ring switches 79/80 | 178.81 to 218.04 | 162.20 to 201.20 |

The minimum is 22.46 (cast 2 to the vial), so `PROTECTED_CLEARANCE_M` is 22.

### Aggro and assist radius

- `crates/cell-world/src/cell/combat/aggression.rs:24` sets
  `DEFAULT_AGGRO_RADIUS` 18.0 u; line 36 sets `AGGRO_VERTICAL_BAND` 4.0 u.
  Templates 15 and 4 have no assist radius; template 24 has `assist_radius` 26
  (NA14, from the NPC memory notes).
- Floors: spawn 20 (template 15) 18 + 5 = 23 m, measured 26.69 (dy 3.09 is
  inside the 4 u band, so the horizontal figure decides); spawn 10 (template
  4) 23 m, measured 36.39; template 24 26 + 5 = 31 m, measured 117.62.
- The cast is faction 1, friendly to everyone in the reaction table, and
  NPC-vs-NPC aggro is witness-gated, so a guard never targets the prisoners.
  The distances hold anyway, so a player who stops to look never pulls a guard.

### Template and chain checks

- Template 176 `Castle_OpCoreSoldier_Unarmed`: class `mob`, faction 1,
  `interaction_type` 0, `event_set_id` 570, level 4, `name_id` 7552, no
  ability set, no weapon. It already spawns in world 8 (`Castle_Pop_*`, spawns
  193 and 199-203), so mesh and body set are known to render.
- No chain, dialog or mission seed names `OpCoreSoldier`, `Cellblock_Ambient`
  or template 176. No chain in `castle_cellblock_chains.sql` aims
  `add_dialog_set`, `set_interaction_type`, `move_entity` or `set_aggression`
  at a tag the cast uses (targets are `ArmYourself_*`, `329_CellDoorButton`,
  `HackTheRings_Switch`, `Preparation_*` and the like).
- Region2 carries chains 1011 and 1012 (gate: 638 `not_active`, so they never
  match in 639/2117) and 1013 (no condition, `once` false, system message
  5040). **1013 shares this crossing in every phase, including 639/2117.**
  CB-N04m rules on it; the message text was not read here.
- Gate: `mission_639_step_2117_status` is the key a `step_status` row reads
  (`crates/content-engine/src/conditions/tests.rs`). Step 2117 text: "Look in
  the Med Station down the hall for some Ambernol." Chains 1018/1019 accept 639
  and complete 638 at the Prisoner 329 dialog; chain 1031 moves 2117 to 2145 on
  Region11 entry.
- Replay: accepting 639 activates 2117, so H52 replays mission-gated
  `enter_region` chains if the server-known position is in Region2. Prisoner
  329 stands at z -113.14, 2 m outside Region2, so the line speaks at that
  replay or at the first Region2 crossing, once. `once` is in memory only: a
  relog still on step 2117 lets the next crossing speak it once more. The gate
  closes when step 2145 begins.

### ID collision check

The packet's literal regex over all 719 local branches returns 16,491 hits,
all legacy rows that reuse the numbers in other ID spaces (for example
`dialog_screen_buttons` ids 520-522 and `dialog_screens` dialog ids 520 and
521); it is too loose to gate on. A table-anchored regex over the same branches
and paths (`INSERT INTO spawnlist` with spawn 520-522, `dialogs` 60160,
`dialog_screens` with screen_id 200160-200162, and the four `content_*`
tables with chain 11010-11012, plus continuation lines) returns **0 hits on
719 branches**. CB-N04a's Step 0 grep of the four seed files is the second gate.

## CB-U08 steps

Lab row for the orchestrator on the integrated branch after W2-INT (ask the
user first, D-CC3). One character, then a second client for AoI.

1. Fresh Castle Cellblock character. Talk to Prisoner 329 and agree to escape
   (638 completes, 639 accepts, step 2117 active). Record whether the 329 cell
   door is open or closed (state unknown from the seed).
2. Walk the first-approach route east towards the Med Station. Stop 6 m west
   of cast 1 with the camera on the pair.
3. Watch: both prisoners visible (`Op-CORE Soldier` plate), standing still,
   facing each other (record if not); no aggro; passage past them not blocked;
   whether the cast stands in an open corridor or behind a door (the navmesh
   cannot tell; record it).
4. Chat shows the 11010 line once, on the first Region2 entry (or at the 329
   dialog if the player is already inside Region2). Record it **with** chain
   1013's system message 5040 at the same crossing, in order.
5. Leave and re-enter Region2 on step 2117: no second line. Relog on 2117 and
   cross Region2: one more line is expected. Take the vial (step 2145): none
   on any later crossing.
6. Second client in AoI sees both NPCs; the line reaches only the crossing
   player. The PRU drone and NID guard behave as before.

SigNoz (service `cell`):

- Spawn: DEBUG `Spawned instance NPC from DB` (`npc_population.rs:139`, fields
  `spawn_id`, `template_id`, `template_name`, `tag`) for spawn 520 and 521 and
  tag like `Cellblock_Ambient_%`; INFO `instance_populated` per space load.
- Spoken: span `content.npc_bark`, INFO `Content: npc bark` with `chain_id`
  11010 and `screen_id` 200160; one per visit.
- Refused: WARN `reason` `screen_not_cached` / `empty_text` /
  `actor_not_player`; expect none.

## Verdict

**sealed.** Pocket `cells`: Region2 north cell blocks, two template-176 rows
(spawns 520 and 521), one bark (chain 11010 on Region2 during 639/2117). Open
for CB-N04m: chain 1013 shares the crossing; the relog replay on step 2117; the
copy against the CB-N05 story. Open for CB-U08: facing and the 329 door state.

## Measurement

- Working tokens: not exposed.
- Wall-clock: start not recorded (the first run died in a crash; this run
  restarted fresh); end 2026-10-17 UTC.
- Lane runs: 1 final `nav_inspect` (exit 0, 2.8 s) plus earlier probe runs.
- Review rounds: 0.
