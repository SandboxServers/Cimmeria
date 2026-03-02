# The Crazy Wall -- Content Association Map

Cross-reference intersection document mapping every connection between every content
type in the Stargate Worlds server emulator. This is the "pin board with red string"
view of the data -- where systems touch, where links break, and where content floats
in isolation.

**Last updated:** 2026-03-01

**Companion documents:**
- [content-inventory.md](content-inventory.md) -- Raw counts and distributions
- [zone-audit.md](zone-audit.md) -- Per-zone completeness scorecards
- [archetype-content-map.md](archetype-content-map.md) -- Per-archetype content availability

---

## Table of Contents

1. [Confidence Framework](#confidence-framework)
2. [How to Read This Document](#how-to-read-this-document)
3. [Section 1: Entity Template Connections](#section-1-entity-template-connections)
4. [Section 2: Mission Connections by Zone](#section-2-mission-connections-by-zone)
5. [Section 3: Broken Foreign Keys and Dead References](#section-3-broken-foreign-keys-and-dead-references)
6. [Section 4: Orphaned Content Census](#section-4-orphaned-content-census)
7. [Section 5: The Reconstruction Web](#section-5-the-reconstruction-web)
8. [Section 6: Content Connectivity Summary](#section-6-content-connectivity-summary)
9. [Section 7: The String Map](#section-7-the-string-map)

---

## Confidence Framework

Every claim in this document carries a confidence tag:

| Tag | Meaning |
|-----|---------|
| **CONFIRMED** | Verified in DB + code + tested at runtime |
| **HIGH** | Present in DB or code, cross-referenced against at least one other source |
| **MEDIUM** | Inferred from cross-referencing multiple sources (DB schema, file names, entity defs) |
| **LOW** | Based on client file names, directory structure, or structural inference only |
| **ASSUMPTION** | Based on MMO industry norms or inferred developer intent with no direct evidence |

---

## How to Read This Document

This document traces the **foreign key graph** of the Stargate Worlds content database
and Python scripting layer. Every connection listed here represents an actual FK
relationship (DB column pointing to another table's primary key) or a runtime reference
(Python script loading a definition by ID).

The critical distinction throughout:

- **DB-driven connections** exist as column values in `resources.sql` INSERT statements.
  These are structural -- they exist whether or not the server code reads them.
- **Script-driven connections** exist as hardcoded IDs in Python source files. These are
  behavioral -- they only matter when the script executes.

Most game systems were intended to use DB-driven connections but were only wired through
scripts for the ~2% of content that received implementation. The remaining ~98% has
DB rows with empty or default FK values.

---

## Section 1: Entity Template Connections

153 entity templates exist. Each template has FK columns pointing to abilities,
loot, weapons, dialogs, vendors, trainers, speakers, patrols, and events. This
section maps every non-default FK for every template.

### 1.1 Templates with Rich Connections

These are the templates where multiple FK columns have non-default values.
"Rich" is relative -- in this codebase, having 3+ connections is exceptional.

| Template ID | Name | Class | Dialogs (interaction_set) | Abilities (ability_set) | Loot (loot_table) | Vendor (buy/sell) | Trainer | Weapon | Speaker |
|---:|---|---|---|---|---|---|---|---|---|
| 4 | Prisoner retrieval unit | mob | -- | Set 2 (ability 221) | -- | -- | -- | -- | -- |
| 15 | Cellblock Guard | mob | -- | Set 1 (ability 579) | Table 2 (NID guard) | -- | -- | Item 55 (pistol) | -- |
| 17 | Prisoner 329 | being | -- | -- | -- | -- | -- | -- | Speaker 253 (Lethander) |
| 24 | NID Guard | mob | -- | Set 3 (ability 559) | Table 2 (NID guard) | -- | -- | Item 21 (SMG) | -- |
| 25 | Interaction Debug NPC | being | -- | -- | -- | Buy list 1, Sell/Repair/Recharge list 2 | List 1 (debug) | -- | -- |
| 48 | CaptCoppleman | being | -- | -- | -- | -- | -- | Item 21 (SMG) | -- |
| 148 | NID Guard - Castle inside | mob | -- | -- | -- | -- | -- | Item 21 (SMG) | -- |
| 149 | Sgt. Gerschon | being | -- | -- | -- | -- | -- | Item 21 (SMG) | -- |
| 166 | Sandbox Greeting NPC | being | interaction_set 100000 | -- | -- | -- | -- | -- | -- |
| 167 | Sandbox Ba'al | being | interaction_set 100001 | -- | -- | -- | -- | -- | -- |

**Confidence: HIGH** -- all values from `resources.entity_templates` INSERT statements.

### 1.2 FK Usage Across All 153 Templates

| FK Field | Templates Using It | Percentage | Target Table |
|---|---:|---:|---|
| `event_set_id` | 129 | 84.3% | `event_sets` |
| `weapon_item_id` | 5 | 3.3% | `items` |
| `ability_set_id` | 3 | 2.0% | `ability_sets` |
| `static_interaction_sets` | 3 | 2.0% | (see Section 3 -- BROKEN) |
| `loot_table_id` | 2 | 1.3% | `loot_tables` |
| `interaction_set_id` | 2 | 1.3% | `dialog_sets` |
| `buy_list_id` | 1 | 0.7% | `vendor_lists` |
| `sell_list_id` | 1 | 0.7% | `vendor_lists` |
| `repair_list_id` | 1 | 0.7% | `vendor_lists` |
| `recharge_list_id` | 1 | 0.7% | `vendor_lists` |
| `trainer_list_id` | 1 | 0.7% | `trainer_ability_lists` |
| `speaker_id` | 1 | 0.7% | `speakers` |
| `patrol_path_id` | 0 | 0.0% | (table does not exist) |

Event sets are the only well-connected FK (84.3%). Every other relationship is at
or near zero. This means 133 hostile mob templates have no abilities, no loot, and
no weapons assigned through the database.

**Confidence: HIGH**

### 1.3 Static Interaction Sets (All Broken)

Three templates reference `static_interaction_sets` -- but every referenced ID is
invalid:

| Template ID | Name | static_interaction_sets | Status |
|---:|---|---|---|
| 1 | DHD | {6891} | BROKEN -- ID 6891 does not exist in any table |
| 25 | Interaction Debug NPC | {6891, 7505, 3851} | BROKEN -- all three IDs are orphaned |
| 162 | DHD_Frost | {6891} | BROKEN -- same nonexistent ID as template 1 |

The IDs 6891, 7505, and 3851 do not correspond to any `dialog_set_id`,
`interaction_set_map_id`, or other known identifier in the database. These are
dead references -- likely pointing to content that was deleted or never created.

**Confidence: HIGH** -- verified by searching all INSERT statements for these IDs.

### 1.4 Dialog Flow Through Entity Templates

The intended DB-driven dialog path:

```
entity_templates.interaction_set_id
    --> dialog_sets.dialog_set_id
        --> dialog_set_maps (via dialog_set_id)
            --> dialogs (via dialog_id)
                --> dialog_screens (via dialog_id)
                    --> dialog_screen_buttons (via dialog_screen_id)
```

Only 2 templates use `interaction_set_id`: templates 166 and 167 (sandbox test
NPCs). The remaining 151 templates have `interaction_set_id = NULL` or 0.

The actual game dialog system bypasses this entire chain. Python scripts dynamically
add dialog sets at runtime using `addDialog(template_id, interaction_set_map, mission_id)`.
The DB FK is a vestigial design -- the scripts are the real wiring.

**Confidence: HIGH**

### 1.5 Entity Interactions Table

The `entity_interactions` table was intended to gate NPC interactions behind mission
state. It contains exactly 1 row:

| Template ID | Name | Interaction Set Map | Condition |
|---:|---|---:|---|
| 43 | Anat | 3127 | `missions_not_accepted = {742}` |

This is the only DB-driven mission-gated NPC interaction in the entire game.
Template 43 (Anat, a Goa'uld NPC in Harset) shows interaction set map 3127 only
when the player has NOT accepted mission 742 ("Giving the Walls Ears"). Once
mission 742 is accepted, this interaction disappears.

All other mission-gated NPC interactions are handled entirely through Python
script `addDialog()` / `removeDialog()` calls.

**Confidence: HIGH**

### 1.6 Event Set Distribution

129 of 153 templates reference an `event_set_id`. The distribution:

| Event Set ID | Templates Using It | Notes |
|---:|---:|---|
| 570 | ~80+ | Most common -- used by generic mob templates |
| Other IDs | 1-5 each | Unique or low-frequency sets |

675 total event sets exist in the database. Only ~50-60 unique event set IDs are
referenced by entity templates, leaving ~615 event sets (91%) that are not
directly referenced by any template FK. Some of these may be referenced by
stargates (12 stargates have event sets) or by Python scripts, but the majority
appear orphaned.

**Confidence: MEDIUM** -- exact per-ID breakdown requires a full cross-reference
query; the pattern is clear from sampling.

---

## Section 2: Mission Connections by Zone

This section traces every scripted mission's connections to NPCs, dialogs, items,
regions, minigames, and sequences. Only the 16 scripted missions are mapped --
the remaining 1,024 have no script and therefore no traceable runtime connections.

### 2.1 Castle_CellBlock Missions (622-687) -- Fully Mapped

The Castle Cellblock is the most complete zone. 13 mission scripts drive a linear
tutorial chain from "wake up in a cell" to "escape the facility."

**Source scripts:**
- `python/cell/spaces/Castle_CellBlock.py` (511 lines -- space controller)
- `python/cell/missions/Castle_CellBlock/ArmYourself.py`
- `python/cell/missions/Castle_CellBlock/Prisoner_329.py`
- `python/cell/missions/Castle_CellBlock/FindAmbernol.py`
- `python/cell/missions/Castle_CellBlock/HackTheRings.py`
- `python/cell/missions/Castle_CellBlock/Preparation.py`
- `python/cell/missions/Castle_CellBlock/EscapeTheCellblock.py`
- `python/cell/missions/Castle_CellBlock/MessHall.py`
- `python/cell/missions/Castle_CellBlock/Hallway01Controller.py` through `Hallway05Controller.py`
- `python/cell/missions/Castle_CellBlock/Aftermath.py`

| Mission | Name (inferred) | NPCs (interact tags) | Dialogs | Items (grant) | Items (remove) | Regions | Minigame | Sequences |
|---:|---|---|---|---|---|---|---|---|
| 622 | Arm Yourself | ArmYourself_FrostBody | dialog.open 3995 | 55 (pistol), 3730 (Frost's letter) | -- | -- | -- | 10000 |
| 638 | Speak to Prisoner 329 | 329_CellDoorButton, Template 17 (Prisoner 329) | choice 5020/5021 (Jaffa), 2299/2300 (Human) | -- | -- | Region2 | -- | -- |
| 639 | Find Ambernol | ArmYourself_AmbernolVial, ArmYourself_PrisonerRetrievalUnit | 2297 | 19 (ambernol vial) | 19 (consumed on use) | Region11 | -- | 10001 |
| 640 | Hack the Rings | HackTheRings_Switch | -- | -- | -- | teleport region 2 | Livewire | -- |
| 641 | Preparation | Preparation_SMG1A, Col Marsh (pet), Preparation_Terminal | 5023 (Jaffa) / 3999 (Human), 3998 | 21 (SMG) | -- | -- | Livewire | -- |
| 680 | Escape the Cellblock | Preparation_RingSwitch | -- | -- | -- | Region9 | -- | -- |
| 681 | Mess Hall | MessHall_Guard1, MessHall_Guard2 | -- | -- | -- | Region9 | -- | -- |
| 682 | Hallway 01 | Hallway01 guards | -- | -- | -- | Region3 (exit) | -- | -- |
| 683 | Hallway 02 | Hallway02 guards | -- | -- | -- | Region4 (entry) | -- | -- |
| 684 | Hallway 03 | Hallway03 guards | -- | -- | -- | Region5 (entry) | -- | -- |
| 685 | Hallway 04 | Hallway04 guards | -- | -- | -- | -- | -- | -- |
| 686 | Hallway 05 | Hallway05 guards | -- | -- | -- | Region6 (entry) | -- | -- |
| 687 | Aftermath | Cellblock_WoodenCrate | 3942 (Human) / 3943 (Jaffa) | Human: 3347, 3359, 3372, 3387, 3401, 3325; Jaffa: 3482, 2797 | -- | -- | -- | -- |

**Confidence: HIGH** -- all connections traced directly from Python source files.

**Dialog sets added by scripts (not DB FKs):**

| Script Context | Dialog Set Map ID | Target Template | Mission Gate |
|---|---:|---:|---|
| Castle_CellBlock.py init | 5229 | 14 (entity tag) | 622 |
| Castle_CellBlock.py Region2 entry, Jaffa path | 5866 | 17 (Prisoner 329) | -- |
| Castle_CellBlock.py Region2 entry, Human path | 2794 | 17 (Prisoner 329) | -- |
| Prisoner_329.py, Jaffa path | 5865 | 17 (Prisoner 329) | -- |
| Prisoner_329.py, Human path | 2793 | 17 (Prisoner 329) | -- |

**Confidence: HIGH** -- verified from `Castle_CellBlock.py` lines 225-231, 229,
and `Prisoner_329.py`.

### 2.2 Castle Missions (701-702)

Castle is the persistent overworld containing Castle_CellBlock as an instance.
Mission scripts are minimal -- the space script (`Castle.py`) handles ring
transport only.

| Mission | NPCs | Dialogs | Items | Regions |
|---:|---|---|---|---|
| 701 | Template 48 (CaptCoppleman), Castle_SgtGerschon, Castle_Coppleman | 2573-2576, dialog set 3062 | -- | ThroneRoom, CastleFrontCourtyard, Infirmary |
| 702 | -- | -- | -- | -- |

Mission 701 connections are **inferred from DB mission_step text and NPC template
names** rather than from scripts. No Python script exists for missions 701-702.
Mission 702 has zero identifiable connections.

**Confidence: MEDIUM** -- based on DB mission labels and zone NPC names, not
verified in script.

### 2.3 SGC_W1 Missions (1559, 1561, 1562)

SGC Wing 1 is the SGU faction starting zone (counterpart to Castle_CellBlock for
Praxis). It has a space script and one mission script covering three chained missions.

**Source scripts:**
- `python/cell/spaces/SGC_W1.py` (480 lines -- space controller)
- `python/cell/missions/SGC_W1/SecurityOffice.py` (234 lines -- missions 1561+1562)

| Mission | NPCs (interact tags) | Dialogs | Items | Sequences | Other |
|---:|---|---|---|---|---|
| 1559 | SGCW1_GenHammond, SGC_W1_Tealc, SGC_W1_ElevatorButton1, SGCW1_AirmanWalking, SGC_W1_FirearmBody | 5354, 5355, 5356, 5357, 5358 | 55 (pistol) | 10005, 2319, 10002, 10013, 10012 | Waypoint movement for AirmanWalking |
| 1561 | SGCW1_AirmanBody, SGC_W1_NaqBomb, SGC_W1_ElevatorButton2 | 5359, 5362, 5363, 5365 | 5168 (radio) | -- | Livewire minigame, entity.dead tag SGC_W1_JaffaBomb |
| 1562 | SGC_W1_ElevatorButton2 | 5366 | -- | 10009 | Teleport (moveTo with position) |

**Mission 1559 flow:**
1. Player loads into SGC_W1 --> auto-accept mission 1559
2. Interact with SGCW1_GenHammond --> dialog 5354 --> advance to step 4613
3. Dialog choice 5354 --> play sequence 10005, disable Hammond interaction, enable Tealc
4. Interact with SGC_W1_Tealc --> dialog 5355 --> advance to step 4614
5. Dialog choice 5355 --> play sequences 2319+10002, disable Tealc, enable ElevatorButton1
6. Interact with ElevatorButton1 --> complete objective 5353 --> dialog 5357
7. Dialog choice 5357 --> teleport player, complete objective 5358, play sequence 10013
8. Interact with FirearmBody --> complete mission 1559, give item 55 (pistol), dialog 5358

**Mission 1561 flow:**
1. Entity dead tag SGC_W1_JaffaBomb fires --> dialog 5359
2. Dialog choice 5359 --> accept mission 1561
3. Interact SGCW1_AirmanBody --> dialog 5362, advance to step 4621, give item 5168 (radio)
4. Item use 5168 --> dialog 5363, advance to step 4622
5. Interact SGC_W1_NaqBomb --> Livewire minigame --> advance to step 4623
6. Item use 5168 again --> complete mission 1561, dialog 5365

**Mission 1562 flow:**
1. Dialog choice 5365 --> accept mission 1562, enable ElevatorButton2
2. Interact SGC_W1_ElevatorButton2 --> advance step 4625, teleport, dialog 5366, sequence 10009

**Confidence: HIGH** -- all connections from `SGC_W1.py` and `SecurityOffice.py`.

### 2.4 Harset Mission (742)

One mission script exists for the Harset zone, covering mission 742 ("Giving the
Walls Ears").

**Source script:** `python/cell/missions/Harset/GivingTheWallsEars.py` (262 lines)

| Mission | NPCs (interact tags/templates) | Dialogs | Items (grant) | Items (remove) | Dialog Sets Added/Removed |
|---:|---|---|---|---|---|
| 742 | Template 163 (Petbe), Template 164, Template 43 (Anat), Template 53 (Nerus), FirstBug, SecondBug, ThirdBug | 2636, 2637, 2638, 2639, 2640 | 2819 (disguise), 2820 (bugs x3), 2864 (map) | 2820 (consume bug on use), 2864 (remove on completion) | Add 3129 to template 163, remove 3129 from 163, add 1000000 to 164, add 3130 to 43, remove 3130 from 43, add 3131 to 53 |

**Mission 742 flow:**
1. Mission accepted (via entity_interactions on template 43) --> add dialog set 3129
   to template 163 (Petbe), give 3x item 2820 (bugs)
2. Dialog choice 2638 --> give item 2819 (disguise), remove dialog set 3129 from
   template 163, advance to step 2503
3. Item use 2819 --> advance to step 2504, dialog 2637, add dialog set 1000000 to
   template 164
4. Dialog set open 1000000 --> check entity tag (FirstBug/SecondBug/ThirdBug),
   complete objectives 2913/2914/2915 (one per bug), remove item 2820 per bug
5. All 3 bugs placed (counter == 3) --> advance to step 2505, add dialog set 3130
   to template 43 (Anat)
6. Dialog choice 2639 --> remove dialog set 3130 from template 43, give item 2864
   (map), advance to step 2506, add dialog set 3131 to template 53 (Nerus)
7. Dialog choice 2640 --> remove item 2864, complete mission 742

This is the most complex single mission in the codebase. It demonstrates the full
dialog set add/remove pattern, multi-objective tracking with a counter, item
grant/consume cycles, and multi-NPC interaction chains.

**Confidence: HIGH** -- verified from `GivingTheWallsEars.py`.

### 2.5 Unscripted Missions (1,024 of 1,040)

The remaining 1,024 missions have no Python scripts. Their DB rows contain:
- Mission name text (via `text_id` FK to localization)
- Step/objective/task structure (all `task_type = 1`)
- Mission labels indicating intended zone placement
- Zero reward XP, zero reward currency, zero-to-eight reward items

Without scripts, these missions cannot:
- Be offered to the player (no `accept()` call)
- Track progress (no `advance()` / `completeObjective()` calls)
- Grant rewards (no `complete()` call)
- Gate NPC dialogs (no `addDialog()` / `removeDialog()` calls)

The step text in these missions frequently references NPC names, zone names, and
item names -- but these are display strings, not FK references. There are no
entity template IDs, item design IDs, or dialog IDs embedded in the mission
step data itself.

**Confidence: HIGH**

---

## Section 3: Broken Foreign Keys and Dead References

Every broken reference found across the content database, ordered by severity.

### 3.1 Critical Breaks (System-Level)

| # | Reference | Source | Target | Status | Confidence |
|---:|---|---|---|---|---|
| 1 | `static_interaction_sets {6891}` | Templates 1, 162 (DHD, DHD_Frost) | Unknown table | ORPHANED -- ID does not exist in any table | HIGH |
| 2 | `static_interaction_sets {6891, 7505, 3851}` | Template 25 (Debug NPC) | Unknown table | ORPHANED -- all three IDs are dead | HIGH |
| 3 | `patrol_path_id` | 0 of 153 templates | `paths` table | TABLE MISSING -- the paths table does not exist in the DB dump; the FK column exists in schema but has no target | HIGH |
| 4 | `stargate_addresses` table | 28 stargates reference addresses | `stargate_addresses` | TABLE EMPTY -- 0 rows; gates cannot function as player-initiated travel | HIGH |
| 5 | `spawn_sets`, `spawn_points` tables | Referenced in engine code | (expected tables) | TABLES MISSING -- NPC placement uses `spawnlist` (167 entries) and Python scripts instead | HIGH |

### 3.2 Data Integrity Breaks

| # | Reference | Source | Target | Status | Confidence |
|---:|---|---|---|---|---|
| 6 | `accepts_mission_id` | Only 1 of 5,411 dialogs has it set | Dialog 2636 -> Mission 742 | FUNCTIONALLY UNUSED -- mission-dialog link is 99.98% script-driven | HIGH |
| 7 | `missions_completed[]` conditions | All dialog_set_maps | `missions` table | ALL EMPTY -- every `missions_completed` array is `{}` across all 4,670 maps | HIGH |
| 8 | `missions_not_accepted[]` conditions | All dialog_set_maps | `missions` table | ALL EMPTY -- every `missions_not_accepted` array is `{}` across all 4,670 maps | HIGH |
| 9 | `prerequisite_abilities[]` | All archetype_ability_tree rows | `abilities` table | ALL EMPTY -- every prerequisite array is `{}` across all 169 entries | HIGH |
| 10 | `level` field | All archetype_ability_tree rows | (integer constraint) | ALL VALUE 1 -- no level-gated ability unlocks exist | HIGH |

### 3.3 Empty Classification Tables

| # | Table | Expected Purpose | Row Count | Confidence |
|---:|---|---|---:|---|
| 11 | `item_categories` | Item taxonomy / filtering | 0 | HIGH |
| 12 | `effect_categories` | Effect taxonomy / filtering | 0 | HIGH |
| 13 | `stargate_addresses` | Gate dialing addresses | 0 | HIGH |

These tables have schema definitions but zero data rows. The systems they were
designed to support were never populated.

### 3.4 Deprecated but Persisted

| # | Reference | Details | Confidence |
|---:|---|---|---|
| 14 | Loot table 1 ("Cpl. Frost body - DEPRECATED") | Marked DEPRECATED in its name field but not removed from the database. Contains 2 entries (items 3730 and 55) but is not referenced by any active entity template. | HIGH |

**Confidence for all items in this section: HIGH** -- verified by cross-referencing
INSERT statements across all tables in `resources.sql`.

---

## Section 4: Orphaned Content Census

Content is "orphaned" when it exists in the database but is not reachable through
any FK relationship, script reference, or runtime loading path. This section
quantifies the orphan rate for each content type.

### 4.1 Orphaned Dialogs

| Metric | Count |
|---|---:|
| Total dialog sets | 1,178 |
| Referenced by entity template `interaction_set_id` | 2 (templates 166, 167) |
| Referenced by entity template `static_interaction_sets` | 3 (all BROKEN -- see Section 3) |
| Referenced by entity_interactions table | 1 (template 43, set map 3127) |
| Referenced by Python `addDialog()` calls | ~10 (sets 2793, 2794, 3062, 3129, 3130, 3131, 5229, 5865, 5866, 1000000) |
| **Total reachable dialog sets** | **~13** |
| **Orphaned dialog sets** | **~1,165 (98.9%)** |

Of 1,178 dialog sets containing 4,670 dialog_set_maps pointing to 5,411 dialogs
with 13,466 screens and 4,350 buttons -- approximately 98.9% of the top-level sets
have no path from any NPC, script, or interaction to the player.

These orphaned dialogs are fully authored content. They have speaker names, dialog
text (via `text_id`), screen layouts, and button choices. The content exists -- it
was simply never wired to an NPC that could present it.

**Confidence: MEDIUM** -- the 10 script-referenced sets were found by grepping
Python source for `addDialog` and `interaction_set_map` calls. Unimplemented
scripts that would have referenced additional sets are possible but not present
in the codebase.

### 4.2 Orphaned Items

| Metric | Count |
|---|---:|
| Total items | 6,059 |
| Referenced by blueprint products | 494 |
| Referenced by blueprint components | 346 |
| Referenced by mission scripts (grants) | ~20 (items 19, 21, 55, 2797, 2819, 2820, 2864, 3325, 3347, 3359, 3372, 3387, 3401, 3482, 3730, 5168, plus a few others) |
| Referenced by mission reward entries | 8 |
| Referenced by vendor lists | 6 |
| Referenced by loot tables | 2 (item 3730, item 55 in deprecated table) |
| Referenced by entity template `weapon_item_id` | 2 unique (item 21, item 55) |
| **Total unique referenced items** | **~595** |
| **Orphaned items** | **5,464 (90.2%)** |

Item 55 (pistol) and item 21 (SMG) are the most cross-referenced items in the game.
Item 55 appears in: loot table 1 (deprecated), entity template weapon (template 15),
mission script grants (missions 622, 1559). Item 21 appears in: entity template
weapon (templates 24, 48, 148, 149), mission script grants (mission 641).

**Confidence: HIGH**

### 4.3 Orphaned Abilities

| Metric | Count |
|---|---:|
| Total abilities | 1,886 |
| In Soldier ability tree | 84 |
| In Commando ability tree | 85 |
| Shared between trees (overlap) | 16 |
| **Unique abilities in trees** | **153** |
| In ability sets (NPC combat) | 3 (abilities 221, 559, 579) |
| As starting abilities | 3 (abilities 592, 594, 597 -- overlap with trees) |
| Referenced by effect scripts | varies (effect scripts reference ability IDs) |
| **Estimated reachable abilities** | **~172** |
| **Orphaned abilities** | **~1,714 (90.9%)** |

The 1,714 orphaned abilities include abilities intended for the 6 unimplemented
archetypes, abilities for NPC combat that were never assigned to ability sets,
and abilities that may exist in client data but have no server-side reference.

**Confidence: MEDIUM** -- ability-to-effect linkage is complex; some abilities
may be reachable through effect chains not fully traced.

### 4.4 Orphaned Entity Templates

| Metric | Count |
|---|---:|
| Total entity templates | 153 |
| Placed by spawnlist entries | ~50 (167 spawnlist entries across ~50 unique templates) |
| Referenced by mission script tags | ~25 additional (tags like ArmYourself_FrostBody, SGCW1_GenHammond, etc.) |
| **Estimated spawnable templates** | **~75-100** |
| **Orphaned templates** | **~53-78 (35-51%)** |

Entity templates without spawn entries or script references exist as definitions
only. They describe NPCs that were designed but never placed in a world.

**Confidence: MEDIUM** -- spawnlist entries are exact (167 rows), but script
tag references require searching all Python files.

### 4.5 Orphaned Missions

| Metric | Count |
|---|---:|
| Total missions | 1,040 |
| With Python scripts | 16 |
| With `accepts_mission_id` in dialogs | 1 (mission 742, dialog 2636) |
| **Total reachable missions** | **16** |
| **Orphaned missions** | **1,024 (98.5%)** |

The 16 scripted missions are the only missions a player can accept, progress, and
complete. The remaining 1,024 missions have DB definitions with step/objective/task
structures but no script to drive them.

Even among the 16 scripted missions, reward distribution is minimal. Only missions
622 (pistol + letter), 639 (ambernol), 641 (SMG), 687 (armor sets), 742 (disguise +
bugs + map), 1559 (pistol), and 1561 (radio) grant items. Zero missions grant XP.
Zero missions grant currency.

**Confidence: HIGH**

### 4.6 Orphaned Effects

| Metric | Count |
|---|---:|
| Total effects | 3,216 |
| With Python scripts | 4 (IDs 264, 658, 659, 3091) |
| With NVP parameters | 7 (17 total NVP entries) |
| Referenced by abilities with effect_ids | ~1,703 |
| **Estimated truly orphaned** (no ability reference, no script, no NVP) | **~1,513 (47.0%)** |

The effect orphan rate is lower than other content types because abilities
frequently reference effect IDs. However, most of those referenced effects are
empty shells -- no script, no parameters, no target collection method. The link
exists but the destination is hollow.

**Confidence: HIGH**

### 4.7 Orphaned Speakers

| Metric | Count |
|---|---:|
| Total speakers | 602 |
| With non-empty names | 134 (22.3%) |
| Referenced by entity template `speaker_id` | 1 (template 17, speaker 253 "Lethander") |
| Referenced by dialog screens (as speaker for dialog text) | ~134 (named speakers) |
| Empty/placeholder speakers | 468 (77.7%) |

Named speakers like "Vala Mal Doran," "Daniel Jackson," "Jack O'Neill," "Teal'c,"
"Ba'al," and "General Hammond" appear in dialog screens as the NPC identity
delivering lines. However, only 1 entity template has a direct `speaker_id` FK.
The other 133 named speakers are used by the dialog system's display layer but
are not linked to entity templates through the database.

**Confidence: MEDIUM** -- speaker-to-dialog-screen linkage is HIGH, but
speaker-to-entity-template linkage is definitively LOW.

### 4.8 Orphaned Worlds

| Metric | Count |
|---|---:|
| Total worlds (DB) | 91 |
| In `spaces.xml` (loadable) | 24 |
| With Python space scripts | 11 |
| **Worlds not in spaces.xml** | **67 (73.6%)** |

67 worlds exist as database definitions only. They have world IDs, names, and
some have stargate entries, but they cannot be loaded by the server because they
have no entry in `spaces.xml`. These include 26 MissionTest worlds (empty test
maps), 5 pocket instances, and ~36 planned game worlds that never received
server-side configuration.

**Confidence: HIGH**

---

## Section 5: The Reconstruction Web

This section presents complete relationship graphs for the zones that have
sufficient data to trace. Each diagram shows every verified connection between
content types for that zone.

### 5.1 Castle_CellBlock -- Complete Reconstruction

The most complete zone in the game. Every connection shown below has been verified
in Python source and/or database INSERT statements.

```
[Space Script: Castle_CellBlock.py (511 lines)]
  |
  +-- Entity Spawns (from spawnlist, 28 entries):
  |   +-- Template 15 (Cellblock Guard) --> Ability Set 1 (ability 579)
  |   |                                 --> Loot Table 2 (NID guard default)
  |   |                                 --> Weapon: Item 55 (pistol)
  |   +-- Template 24 (NID Guard) --> Ability Set 3 (ability 559)
  |   |                           --> Loot Table 2
  |   |                           --> Weapon: Item 21 (SMG)
  |   +-- Template 4 (Prisoner retrieval unit) --> Ability Set 2 (ability 221)
  |   +-- Template 17 (Prisoner 329) --> Speaker 253 (Lethander)
  |
  +-- Mission Chain (linear, 13 missions):
  |   [622] "Arm Yourself!"
  |   |  Trigger: dialog.open::3995
  |   |  Script: ArmYourself.py
  |   |  Grants: Item 55 (pistol) to slot 3, Item 3730 (Frost's letter) to slot 0
  |   |  Tags: ArmYourself_FrostBody (disables interaction on complete)
  |   |  Sequences: 10000
  |   |  Dialog sets: adds 5229 to entity 14
  |   v
  |   [638] "Speak to Prisoner 329"
  |   |  Trigger: Region2 entry (Castle_Cellblock.Region2)
  |   |  Script: Prisoner_329.py
  |   |  Archetype branch:
  |   |    Jaffa (==8): dialog set 5866 -> choice 5021 -> complete
  |   |    Human (<5):  dialog set 2794 -> choice 2300 -> complete
  |   |    Others:      [NO DIALOG -- mission stuck]
  |   |  Tags: 329_CellDoorButton
  |   |  NPCs: Template 17 (Prisoner 329)
  |   v
  |   [639] "Find Ambernol"
  |   |  Trigger: interact ArmYourself_AmbernolVial
  |   |  Script: FindAmbernol.py
  |   |  Grants: Item 19 (ambernol vial)
  |   |  Removes: Item 19 (on use against PrisonerRetrievalUnit)
  |   |  Tags: ArmYourself_AmbernolVial, ArmYourself_PrisonerRetrievalUnit
  |   |  Region: Castle_Cellblock.Region11 (step advance trigger)
  |   |  Sequences: 10001
  |   |  Combat: Sets aggression on PrisonerRetrievalUnit, generates 1000 threat
  |   |  Chains to: auto-accept mission 640 on complete
  |   v
  |   [640] "Hack the Rings"
  |   |  Trigger: teleport::in region 2
  |   |  Script: HackTheRings.py (assumed, from space controller)
  |   |  Minigame: Livewire
  |   |  Tags: HackTheRings_Switch
  |   |  On complete: enables Col Marsh interaction (sets interaction type |= 8388608)
  |   v
  |   [641] "Preparation"
  |   |  Trigger: interact Preparation_SMG1A
  |   |  Script: Preparation.py
  |   |  Grants: Item 21 (SMG) to slot 3
  |   |  Tags: Preparation_SMG1A, Preparation_Terminal
  |   |  NPCs: Col Marsh (pet) -- interact by template name
  |   |  Archetype branch:
  |   |    Jaffa (==8): dialog 5023 -> choice -> advance
  |   |    Others:      dialog 3999 -> choice -> advance
  |   |  Minigame: Livewire (on terminal interact)
  |   |  On complete: dialog 3998, auto-accept mission 680
  |   v
  |   [680] "Escape the Cellblock"
  |   |  Trigger: interact Preparation_RingSwitch
  |   |  Script: EscapeTheCellblock.py
  |   |  Region: Region9 (ring transport)
  |   |  Tags: Preparation_RingSwitch
  |   v
  |   [681] "Mess Hall"
  |   |  Trigger: kill 2 guards
  |   |  Script: MessHall.py
  |   |  Tags: MessHall_Guard1, MessHall_Guard2
  |   |  Region: Region9 (guard area)
  |   |  On complete: auto-accept 682 (from space controller, Region3 exit)
  |   v
  |   [682] --> [683] --> [684] --> [685] --> [686]
  |   |  "Hallway Gauntlet" (5 hidden missions)
  |   |  is_hidden = true (not shown in quest log)
  |   |  Each: kill guards in hallway segment, auto-accept next on region entry
  |   |  Regions: Region3 exit, Region4, Region5, Region6 entries
  |   |  Space controller chains: 681 done -> 682 -> 683 -> 684 -> 685 -> 686 -> 687
  |   v
  |   [687] "Aftermath"
  |      Trigger: interact Cellblock_WoodenCrate (step 2354)
  |      Script: Aftermath.py
  |      Archetype branch:
  |        Jaffa (==8): dialog 3943, items 3482, 2797 (2 items)
  |        Human (<5):  dialog 3942, items 3347, 3359, 3372, 3387, 3401, 3325 (6 items)
  |        Others:      [NO ITEMS -- falls through both branches]
  |      Advance: step 2355
  |      END OF TUTORIAL
  |
  +-- System Communications (onSystemCommunication calls):
  |   Region11 entry: textId 5180
  |   Region2 entry:  textId 5040
  |   Region10 entry: textId 5181
  |   Region3 entry:  textId 5182
  |   Region12 entry: textId 5184
  |   Region13 entry: textId 5185
  |
  +-- Regions Used: Region2, Region3, Region4, Region5, Region6, Region8,
  |   Region9, Region10, Region11, Region12, Region13
  |
  +-- Sequences: 10000 (played on load if mission 622 complete)
  |
  +-- Abilities: 1372 (launched on player load via space controller)
```

**Confidence: HIGH** -- every line traced from source code.

### 5.2 SGC_W1 -- Complete Reconstruction

```
[Space Script: SGC_W1.py (480 lines)]
  |
  +-- Mission Chain (3 missions):
  |   [1559] "Security Office Part 1"
  |   |  Auto-accept on player.loaded
  |   |  Flow: GenHammond -> Tealc -> ElevatorButton1 -> FirearmBody
  |   |  NPCs: SGCW1_GenHammond, SGC_W1_Tealc, SGCW1_AirmanWalking,
  |   |        SGC_W1_ElevatorButton1, SGC_W1_FirearmBody
  |   |  Dialogs: 5354, 5355, 5356, 5357, 5358
  |   |  Sequences: 10005, 2319, 10002, 10013, 10012
  |   |  Grants: Item 55 (pistol) to slot 1
  |   |  Teleport: moveTo(-123.625, 1.311, -246.858) within SGC_W1
  |   |  Interaction type toggling: GenHammond (|= 16777216, &= ~16777216),
  |   |                             Tealc (|= 16777216, &= ~16777216),
  |   |                             ElevatorButton1 (|= 256)
  |   |  Waypoint: AirmanWalking moves to (237.326, 1.312, 17.921)
  |   v
  |   [1561] "Security Office Part 2"
  |   |  Script: SecurityOffice.py
  |   |  Trigger: dialog choice 5359 (after JaffaBomb entity dies)
  |   |  NPCs: SGCW1_AirmanBody, SGC_W1_NaqBomb, SGC_W1_ElevatorButton2
  |   |  Dialogs: 5359, 5362, 5363, 5365
  |   |  Grants: Item 5168 (radio) to slot 2
  |   |  Item use: 5168 (radio) -- used twice: once to advance, once to complete
  |   |  Minigame: Livewire (on NaqBomb interact)
  |   |  Entity dead trigger: SGC_W1_JaffaBomb
  |   v
  |   [1562] "Security Office Part 3"
  |      Trigger: dialog choice 5365 (chains from 1561 completion)
  |      NPCs: SGC_W1_ElevatorButton2
  |      Dialogs: 5366
  |      Sequences: 10009
  |      Teleport: moveTo(-53.86, 1.311, 62.136)
  |      Interaction type toggling: ElevatorButton2 (|= 256)
  |      END OF SGC_W1 CONTENT
  |
  +-- No spawnlist entries mapped (NPCs placed by other mechanism or
  |   hardcoded in space definition)
  |
  +-- No regions used (all progression via NPC interaction, not area triggers)
```

**Confidence: HIGH**

### 5.3 Harset -- Partial Reconstruction

```
[Space Script: Harset.py (71 lines)]
  |
  +-- Ring Transporters (5 interactions):
  |   ID 4: tag HarsetRingLeftBottom
  |   ID 5: tag HarsetRingRightBottom
  |   ID 6: tag HarsetRingLeft
  |   ID 7: tag HarsetRingLeftTop
  |   ID 8: tag HarsetRingRight
  |
  +-- Region: Harset.CommandCenterTransition
  |   --> Teleport to Harset_CmdCenter at (0, 0.355, -20)
  |
  +-- Spawnlist (22 entries):
  |   +-- Template 1 (DHD) --> static_interaction_sets {6891} [BROKEN]
  |   +-- Template 3 (Ring Switch) x multiple
  |   +-- Template 160 (Praxis Jaffa Guard) --> [MISSING: ability_set]
  |   |                                     --> [MISSING: loot_table]
  |   +-- Template 159 (Praxis Jaffa Lieutenant) --> [MISSING: ability_set]
  |
  +-- Scripted Mission (1 of 26 in DB for this zone):
  |   [742] "Giving the Walls Ears"
  |      NPCs: Template 163 (Petbe), 164, 43 (Anat), 53 (Nerus),
  |            FirstBug, SecondBug, ThirdBug
  |      Items: 2819 (disguise), 2820 (bugs x3), 2864 (map)
  |      Dialogs: 2636, 2637, 2638, 2639, 2640
  |      Dialog sets: 3129 (on 163), 1000000 (on 164), 3130 (on 43), 3131 (on 53)
  |      Entity interaction: template 43, set map 3127, gate: mission 742 not accepted
  |
  +-- [MISSING: 25 more mission scripts] (DB has 26 Harset-labeled missions)
  +-- [MISSING: NPC ability sets for guard templates 159, 160]
  +-- [MISSING: Loot tables for any Harset NPCs]
  +-- [MISSING: Vendor/trainer NPCs] (no templates with buy/sell/trainer lists)
  +-- [MISSING: Patrol paths for any NPCs] (0 patrol_path_id usage globally)
  +-- [MISSING: Dialog gating conditions] (all dialog_set_map conditions empty)
```

**Confidence: HIGH for present content; MEDIUM for MISSING tags (inferred from
DB labels and standard MMO zone expectations).**

### 5.4 SandBox -- Test Zone Reconstruction

```
[Space Script: SandBox.py]
  |
  +-- Entity Spawns:
  |   +-- Template 25 (Interaction Debug NPC) --> Buy list 1, Sell/Repair/Recharge list 2
  |   |                                       --> Trainer list 1 (debug abilities)
  |   |                                       --> static_interaction_sets {6891,7505,3851} [ALL BROKEN]
  |   +-- Template 166 (Sandbox Greeting NPC) --> interaction_set 100000
  |   +-- Template 167 (Sandbox Ba'al) --> interaction_set 100001
  |
  +-- The ONLY zone with:
  |   - A vendor NPC (template 25, buy list 1)
  |   - A trainer NPC (template 25, trainer list 1)
  |   - A repair NPC (template 25, list 2)
  |   - A recharge NPC (template 25, list 2)
  |   - DB-driven interaction sets (templates 166, 167)
  |
  +-- [MISSING: missions, combat NPCs, loot, progression]
```

**Confidence: HIGH**

### 5.5 Generic Shell Zone Template

This template represents the majority of zones (Tollana, Beta_Site, Dakara_E1,
Agnos, Lucia, etc.) that have server space definitions but minimal content.

```
[NO Space Script] or [Minimal space script -- ring transport only]
  |
  +-- Client Art: varies (some have hundreds of .umap tiles) [LOW]
  +-- Navmesh: varies (some have .navmesh files, some do not) [LOW]
  +-- Stargate: typically 1 gate with position [HIGH]
  +-- Spawnlist: 1-5 entries (DHD + ring switches) [HIGH]
  +-- Missions: [MISSING scripts -- missions labeled for zone exist in DB]
  +-- Combat NPCs: [MISSING -- no hostile NPCs spawned]
  +-- Dialog NPCs: [MISSING -- no dialog sets referenced]
  +-- Loot: [MISSING]
  +-- Abilities: [MISSING]
  +-- Events: event_set_id on spawned templates only
```

**Confidence: HIGH for listed items; LOW for client art (based on file names only).**

---

## Section 6: Content Connectivity Summary

### 6.1 System-to-System Connection Matrix

This matrix shows which content systems have verified FK or script connections
to each other. A cell marked "Y" means at least one connection exists between
the row system and column system. A number indicates how many connections.

|  | Templates | Missions | Items | Abilities | Effects | Dialogs | Dialog Sets | Loot | Speakers | Stargates | Worlds | Sequences | Regions | Minigames |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| **Templates** | -- | via script | 5 weapon | 3 sets | via ability | 2 interact | 3 static (BROKEN) | 2 tables | 1 FK | via spawnlist | via spawnlist | via event_set | via script | -- |
| **Missions** | via script tags | -- | ~20 grant | -- | -- | ~50 script | ~10 add/remove | -- | -- | -- | -- | ~8 play | ~10 regions | 3 (Livewire) |
| **Items** | 5 weapon FK | ~20 script | -- | -- | -- | -- | -- | 2 loot entries | -- | -- | -- | -- | -- | -- |
| **Abilities** | 3 sets | -- | -- | -- | ~961 link | -- | -- | -- | -- | -- | -- | -- | -- | -- |
| **Effects** | -- | -- | -- | ~961 link | -- | -- | -- | -- | -- | -- | -- | -- | -- | -- |
| **Dialogs** | 1 accepts_mission | 1 accepts | -- | -- | -- | -- | 4,670 maps | -- | via screen | -- | -- | -- | -- | -- |
| **Loot** | 2 template FK | -- | 2 entries | -- | -- | -- | -- | -- | -- | -- | -- | -- | -- | -- |
| **Blueprints** | -- | -- | ~840 refs | -- | -- | -- | -- | -- | -- | -- | -- | -- | -- | -- |
| **Stargates** | -- | -- | -- | -- | -- | -- | -- | -- | -- | -- | 28 world FK | -- | -- | -- |
| **Worlds** | via spawnlist | via label | -- | -- | -- | -- | -- | -- | -- | 28 gate FK | -- | -- | -- | -- |

### 6.2 Per-Content-Type Connectivity Scorecard

| Content Type | Total | Connected | Orphaned | % Connected | Confidence |
|---|---:|---:|---:|---:|---|
| Missions | 1,040 | 16 (scripted) | 1,024 | 1.5% | HIGH |
| Items | 6,059 | ~595 | ~5,464 | 9.8% | HIGH |
| Dialogs | 5,411 | ~50 (estimated via scripts) | ~5,361 | ~0.9% | MEDIUM |
| Dialog Sets | 1,178 | ~13 | ~1,165 | ~1.1% | MEDIUM |
| Effects | 3,216 | ~1,703 (ability refs) | ~1,513 | 53.0% | HIGH |
| Entity Templates | 153 | ~75-100 (spawned/scripted) | ~53-78 | ~49-65% | MEDIUM |
| Abilities | 1,886 | ~172 (in trees/sets) | ~1,714 | ~9.1% | HIGH |
| Speakers | 602 | 1 (FK) + ~134 (named in dialogs) | 468 | ~22.4% | MEDIUM |
| Loot Tables | 2 | 2 | 0 | 100% | HIGH |
| Loot Entries | 3 | 3 | 0 | 100% | HIGH |
| Blueprints | 498 | 498 (self-contained system) | 0 | 100% | HIGH |
| Worlds (DB) | 91 | 24 (in spaces.xml) | 67 | 26.4% | HIGH |
| Stargates | 28 | 20 (positioned) | 8 | 71.4% | HIGH |

### 6.3 The Connection Density Problem

The game has 6 major content types with 1,000+ entries each:

| Content Type | Entries | Avg Connections Per Entry |
|---|---:|---:|
| Items | 6,059 | 0.098 (most have zero) |
| Dialogs | 5,411 | ~0.009 (one in a hundred has a mission link) |
| Mission Tasks | 4,358 | 1.0 (all type=1 -- connected but identical) |
| Effects | 3,216 | ~0.53 (half referenced by abilities, but hollow) |
| Abilities | 1,886 | ~0.09 (only tree members are connected) |
| Missions | 1,040 | ~0.015 (only scripted ones are connected) |

For comparison, a shipped MMO at this scale would typically have 3-5 connections
per content entry on average (an item connected to loot tables, vendor lists,
crafting recipes, quest rewards, and visual assets). Cimmeria averages well
under 1 connection per entry across all types.

**Confidence: MEDIUM** -- comparison to shipped MMO norms is an ASSUMPTION.

---

## Section 7: The String Map

This section traces the FK chains that connect the most distant content types --
the "red strings" on the crazy wall that span the entire board.

### 7.1 Longest Verified FK Chain

The longest single chain of verified FK hops in the codebase:

```
Player loads into Castle_CellBlock
  --> Space script subscribes to player.loaded event
    --> Script checks mission 622 status
      --> Script calls missions.accept(622)
        --> Script calls addDialog(14, interaction_set_map 5229, 622)
          --> Dialog set map 5229 resolves to a dialog set
            --> Dialog set contains dialog mappings
              --> Dialog resolves to dialog_screens
                --> Dialog screen has dialog_screen_buttons
                  --> Button has text_id --> localization entry
                    --> Player sees localized dialog text
```

That is 10 hops from "player enters zone" to "player reads text."

**Confidence: HIGH** -- each hop verified in source code.

### 7.2 Longest Intended FK Chain (Including Broken Links)

The design intended even longer chains through mission-gated dialog conditions:

```
Player completes mission
  --> missions_completed[] condition on dialog_set_map [BROKEN -- all empty]
    --> New dialog set becomes available [NEVER HAPPENS]
      --> Dialog offers new mission via accepts_mission_id [BROKEN -- only 1 entry]
        --> Player accepts new mission
          --> Mission grants item reward [BROKEN -- 3 missions have rewards]
            --> Item links to container set [WORKS for those items]
              --> Item equips to slot
```

The intended chain was 7 hops. In practice, links 1-2 and 3-4 are broken,
reducing the working chain to individual mission-script-driven paths.

**Confidence: HIGH** -- the schema clearly supports this chain; the data does not.

### 7.3 Item 55 (Pistol) -- Most Connected Single Item

Item 55 is the single most cross-referenced content entry in the game:

```
Item 55 (pistol)
  +-- Entity template 15 (Cellblock Guard): weapon_item_id = 55
  +-- Loot table 1 entry (DEPRECATED): 100% drop chance, qty 1
  +-- Mission 622 script (ArmYourself.py): addItem(3, 55, 1)
  +-- Mission 1559 script (SGC_W1.py): addItem(1, 55, 1)
  +-- Starting ability 592 (Pistol Shot): uses this weapon type
  +-- Container set: weapon slot + bandolier + bank
```

No other item in the game has more than 3 inbound references.

**Confidence: HIGH**

### 7.4 Template 25 (Debug NPC) -- Most Connected Single Template

Template 25 is the only entity template connected to 5+ content systems:

```
Template 25 (Interaction Debug NPC)
  +-- Buy list 1 (vendor purchases)
  +-- Sell list 2 (vendor sales)
  +-- Repair list 2
  +-- Recharge list 2
  +-- Trainer list 1 (debug ability list -- all 169 Soldier+Commando abilities)
  +-- static_interaction_sets {6891, 7505, 3851} [ALL BROKEN]
  +-- Spawned in SandBox zone
```

This is a developer testing NPC with every NPC service type enabled. It is the
only template with vendor, trainer, repair, and recharge functionality. Its
broken static_interaction_sets suggest it was also intended to test dialog
interaction flows.

**Confidence: HIGH**

### 7.5 Mission 742 -- Most Connected Single Mission

Mission 742 ("Giving the Walls Ears") has the most cross-system connections
of any mission in the game:

```
Mission 742
  +-- Items: 2819 (disguise), 2820 (bugs x3), 2864 (map) -- grant + consume
  +-- Dialogs: 2636, 2637, 2638, 2639, 2640
  +-- Dialog sets: 3129 (add/remove), 1000000 (add), 3130 (add/remove), 3131 (add)
  +-- NPCs: templates 43, 53, 163, 164 + tags FirstBug, SecondBug, ThirdBug
  +-- Entity interactions table: template 43, set map 3127, gate {742}
  +-- Objectives: 2913 (FirstBug), 2914 (SecondBug), 2915 (ThirdBug)
  +-- Counter: 3-phase tracking (bugs placed)
  +-- Steps: 2503, 2504, 2505, 2506
  +-- Only mission with accepts_mission_id in a dialog (dialog 2636)
  +-- Only mission with entity_interactions table entry
```

**Confidence: HIGH**

### 7.6 Cross-Zone Connection Points

Very few connections span zone boundaries. The identified cross-zone links:

| Source Zone | Target Zone | Connection Type | Confidence |
|---|---|---|---|
| Castle_CellBlock | Castle | Implied progression (tutorial -> overworld) | MEDIUM |
| Harset | Harset_CmdCenter | Region teleport (Harset.CommandCenterTransition) | HIGH |
| SGC_W1 | (unknown next zone) | Mission 1562 teleport (internal, stays in SGC_W1) | HIGH |
| Any zone | Any zone | Stargate travel [BROKEN -- no addresses] | HIGH |

The stargate system was intended to be the primary inter-zone connection mechanism,
but with 0 stargate addresses in the database, no gate-based travel is possible.
The ring transporter regions (28 total) provide within-zone transport only.

Zone-to-zone progression is almost entirely undefined. After completing the
Castle_CellBlock tutorial, the intended player flow to the next zone is unknown --
there is no script or DB reference connecting the tutorial completion to the
Castle overworld or any other destination.

**Confidence: HIGH for listed connections; MEDIUM for the absence of others.**

---

## Summary

The cross-reference integrity of Stargate Worlds content is extremely low across
all content types. The data tells a consistent story:

1. **Content was authored in bulk** -- thousands of items, abilities, effects,
   dialogs, and missions were created in what appears to have been a systematic
   authoring pass.

2. **Systems integration barely started** -- the FK relationships that would
   connect content into playable gameplay loops (loot tables, vendor lists, mission
   rewards, dialog conditions, ability trees, patrol paths) are populated for
   fewer than 2% of entries.

3. **Script-driven wiring replaced DB-driven wiring** -- for the content that
   does work, Python scripts bypass the DB FK system entirely. Dialog sets are
   added via `addDialog()`, items are granted via `inventory.addItem()`, missions
   are accepted via `missions.accept()`. The DB FK columns exist but are not the
   operational mechanism.

4. **One zone received most of the integration effort** -- Castle_CellBlock
   accounts for 13 of 16 scripted missions, the majority of NPC ability sets
   and loot tables, and the only multi-mission chain with archetype branching.

5. **The game was cancelled before the wiring pass** -- the pipeline was:
   **define** (done for most types) --> **author** (done in bulk) -->
   **wire** (barely started) --> **script** (done for ~2%) --> **test** (1 zone).
   Cancellation occurred somewhere between "author" and "wire."

The overall content graph has ~30,000 nodes (across all tables) and fewer than
~5,000 verified edges. A shipped game at this content volume would typically have
100,000-150,000 edges. The graph is approximately 3-5% as connected as it needs
to be for a playable game.

**Confidence: MEDIUM** -- the per-system data is HIGH confidence, but the
aggregate "3-5% connected" figure involves estimated edge counts and comparison
to industry norms (ASSUMPTION).
