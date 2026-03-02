# Content Inventory — Stargate Worlds Emulator

> **Last updated**: 2026-03-01
> **Purpose**: Statistical audit of all game content data across the database (`resources.sql`), Python scripts, entity definitions, and client assets
> **Scope**: Counts, distributions, cross-references, and completeness for every content type in the emulator

This document inventories every category of authored game content in the Cimmeria server
emulator. The numbers come from direct queries against the resource database schema
(`db/resources.sql`), cross-referenced with Python scripts (`python/`), entity definitions
(`entities/`), and client data files (`data/`). Each section states its confidence level
explicitly.

---

## Confidence Framework

Every claim in this document carries a confidence tag:

| Tag | Meaning |
|-----|---------|
| **CONFIRMED** | Verified in database + code + tested at runtime |
| **HIGH** | Present in database or code, cross-referenced against at least one other source |
| **MEDIUM** | Inferred from cross-referencing multiple sources (DB schema, file names, entity defs) |
| **LOW** | Based on client file names, directory structure, or structural inference only |
| **ASSUMPTION** | Based on MMO industry norms or inferred developer intent |

---

## 1. Summary Dashboard

Quick-reference table of all content types. "Populated" means the entry has meaningful
data beyond a bare-minimum placeholder row. "Placeholder" means the row exists but lacks
functional data (empty scripts, zero rewards, no references from other systems, etc.).

| Content Type | Total Count | Populated | Placeholder | Completeness | Confidence |
|---|---:|---:|---:|---:|---|
| Missions | 1,040 | ~16 scripted | ~1,024 | 1.5% | HIGH |
| Mission Steps | 3,475 | 3,475 | — | 100%* | HIGH |
| Mission Objectives | 4,033 | 4,033 | — | 100%* | HIGH |
| Mission Tasks | 4,358 | 0 varied | 4,358 | 0% | HIGH |
| Items | 6,059 | 595 referenced | 5,464 | 9.8% | HIGH |
| Abilities | 1,886 | ~169 trainable | ~1,717 | 9.0% | HIGH |
| Effects | 3,216 | 4 scripted | 3,212 | 0.12% | HIGH |
| Dialogs | 5,411 | ~5,411 | — | ~100%* | HIGH |
| Dialog Screens | 13,466 | 13,466 | — | 100%* | HIGH |
| Dialog Buttons | 4,350 | 4,350 | — | 100%* | HIGH |
| Dialog Sets | 1,178 | 1,178 | — | 100%* | HIGH |
| Dialog Set Maps | 4,670 | ~2 conditional | ~4,668 | <0.1% | HIGH |
| Entity Templates (NPCs) | 153 | ~153 | — | ~100%* | HIGH |
| Speakers | 602 | 134 named | 468 | 22.3% | HIGH |
| Loot Tables | 2 | 2 | — | 100%* | HIGH |
| Loot Entries | 3 | 3 | — | 100%* | HIGH |
| Blueprints | 498 | 498 | — | 100%* | HIGH |
| Blueprint Components | 2,556 | 2,556 | — | 100%* | HIGH |
| Crafting Disciplines | 78 | 78 | — | 100%* | HIGH |
| Stargates | 28 | 20 positioned | 8 | 71.4% | HIGH |
| Worlds | 91 | ~50 real | ~41 test/empty | ~55% | HIGH |
| Texts (Localization) | 29,126 | 29,126 | — | 100%* | HIGH |
| Sequences | 1,973 | 1,973 | — | 100%* | HIGH |
| Event Sets | 675 | 675 | — | 100%* | HIGH |
| Static Meshes | 5,152 | 5,152 | — | 100%* | LOW |
| Skeletal Meshes | 2,141 | 2,141 | — | 100%* | LOW |
| Body Components | 2,320 | 2,320 | — | 100%* | MEDIUM |
| Visual Variants | 3,657 | 3,657 | — | 100%* | MEDIUM |
| Character Configs | 23 | 23 | — | 100%* | HIGH |
| Ring Transport Regions | 28 | 28 | — | 100%* | HIGH |
| Generic Trigger Regions | 28 | 28 | — | 100%* | HIGH |
| Respawners | 7 | 7 | — | 100%* | HIGH |
| Special Words (Chat Filter) | 280 | 280 | — | 100%* | HIGH |
| Containers (Equip Slots) | 20 | 20 | — | 100%* | HIGH |

> \* "100% populated" means every row has data in its required columns — it does **not**
> mean the content is functionally complete. Many systems have rows with valid structure
> but no behavioral variety (e.g., all 4,358 mission tasks are task_type=1).

---

## 2. Missions

**Total: 1,040 mission definitions** | **Confidence: HIGH**

Missions are the primary player-facing content driver. The database holds a substantial
volume of mission definitions, but the vast majority lack scripted behavior, rewards, or
differentiated task types.

### 2.1 Volume Breakdown

| Table | Row Count | Notes |
|---|---:|---|
| `missions` | 1,040 | Top-level mission definitions |
| `mission_steps` | 3,475 | Ordered steps within missions (~3.3 steps/mission avg) |
| `mission_objectives` | 4,033 | Objectives within steps (~1.2 objectives/step avg) |
| `mission_tasks` | 4,358 | Individual tasks within objectives (~1.1 tasks/objective avg) |

### 2.2 Task Type Distribution

| task_type | Count | Percentage | Description |
|---:|---:|---:|---|
| 1 | 4,358 | 100.0% | Single placeholder type |
| 2+ | 0 | 0.0% | No other task types exist |

Every single mission task in the database is `task_type=1`. There is **zero behavioral
variety** across all 4,358 tasks. This strongly suggests that the task type system was
stubbed out with a default value during content authoring and never received the
implementation pass that would have introduced kill-count, collect, interact, escort, or
other standard MMO task types.

**Confidence: HIGH** — direct query, zero ambiguity.

### 2.3 Scripted Missions

| Category | Script Count | Missions |
|---|---:|---|
| Castle_CellBlock | 13 | Castle Cellblock zone mission scripts |
| Harset | 1 | Harset zone mission |
| SGC_W1 | 1 | SGC Wing 1 mission |
| General / test | 1 | Test or general-purpose mission script |
| **Total** | **16** | **1.5% of all missions** |

Only 16 of 1,040 missions (1.5%) have associated Python scripts. The Castle Cellblock
zone accounts for 81% of all scripted missions, reflecting that it was the primary
test/demo zone.

**Confidence: HIGH**

### 2.4 Rewards

| Reward Type | Missions with Nonzero Value | Notes |
|---|---:|---|
| `reward_xp` > 0 | 0 | No mission grants experience points |
| `reward_naq` > 0 | 0 | No mission grants currency (Naquadah) |
| Item rewards | 3 | Missions 1001 (2 reward groups), 40 (1 group) |
| Item reward entries | 8 | Total individual item reward rows |

The reward system is effectively empty. Zero missions grant XP. Zero missions grant
currency. Only 3 missions out of 1,040 have any item rewards at all, with a combined
total of 8 item reward entries. This means a player could complete every mission in the
game and receive essentially nothing.

**Confidence: HIGH**

### 2.5 Level Distribution

| Level | Mission Count | | Level | Mission Count |
|---:|---:|---|---:|---:|
| 1 | 204 | | 26 | 10 |
| 2 | 15 | | 27 | 10 |
| 3 | 15 | | 28 | 10 |
| 4 | 15 | | 29 | 10 |
| 5 | 31 | | 30 | 30 |
| 6 | 10 | | 31 | 10 |
| 7 | 10 | | 32 | 10 |
| 8 | 10 | | 33 | 10 |
| 9 | 10 | | 34 | 10 |
| 10 | 30 | | 35 | 23 |
| 11 | 10 | | 36 | 10 |
| 12 | 10 | | 37 | 10 |
| 13 | 10 | | 38 | 10 |
| 14 | 10 | | 39 | 10 |
| 15 | 22 | | 40 | 85 |
| 16 | 10 | | 41 | 10 |
| 17 | **0** | | 42 | 10 |
| 18 | 10 | | 43 | 10 |
| 19 | 10 | | 44 | 10 |
| 20 | 31 | | 45 | 22 |
| 21 | 10 | | 46 | 10 |
| 22 | 10 | | 47 | 10 |
| 23 | 10 | | 48 | 10 |
| 24 | 10 | | 49 | 10 |
| 25 | 30 | | 50 | 21 |

Notable patterns:
- **Level 1**: 204 missions (19.6% of all missions) — heavily front-loaded
- **Level 17**: 0 missions — the only level with zero content
- **Milestone levels** (5, 10, 15, 20, 25, 30, 35, 40, 45, 50): consistently higher counts (21-85)
- **Level 40**: 85 missions — second highest after level 1
- **Non-milestone levels**: most have exactly 10 missions, suggesting batch-generated placeholders

**Confidence: HIGH**

### 2.6 Mission Labels

78 distinct `mission_label` values. Top labels by frequency:

| Label | Count | Percentage |
|---|---:|---:|
| General | 430 | 41.3% |
| NO MISSION LABEL | 202 | 19.4% |
| Men'fa | 38 | 3.7% |
| Harset | 26 | 2.5% |
| Dakara E2 | 26 | 2.5% |
| Dakara E3 | 24 | 2.3% |
| Ihpet Crater | 22 | 2.1% |
| *(72 other labels)* | 272 | 26.2% |

Over 60% of missions are either "General" (the default label) or explicitly marked
"NO MISSION LABEL", confirming that most mission definitions are unfinished stubs
awaiting zone assignment and content design.

**Confidence: HIGH**

### 2.7 Special Flags

| Flag | Description | Notes |
|---|---|---|
| `is_story` | Marks main story arc missions | Exact count unknown; subset of total |
| `is_hidden` = true | Hallway controller missions | 682-686 missions flagged hidden |

Approximately 65-66% of all missions are flagged `is_hidden=true`. These appear to be
hallway controller missions — internal logic missions that gate zone transitions or
trigger events, not player-visible quest log entries.

**Confidence: HIGH** (count range due to possible off-by-one in query boundary)

---

## 3. Items

**Total: 6,059 item definitions** | **Confidence: HIGH**

The item database is the largest single content table. Despite the volume, the vast
majority of items are not connected to any game system.

### 3.1 Quality Distribution

| Quality | Count | Percentage |
|---|---:|---:|
| Normal | 5,808 | 95.9% |
| Good | 68 | 1.1% |
| Great | 61 | 1.0% |
| Poor | 18 | 0.3% |
| Fantastic | 3 | <0.1% |
| *Unset / other* | 101 | 1.7% |

The quality curve is extremely bottom-heavy. Nearly 96% of items are Normal quality.
Only 3 items in the entire game reach Fantastic tier. This distribution is consistent with
early-development bulk item generation before tuning passes.

**Confidence: HIGH**

### 3.2 Tier Distribution

| Tier | Count | Percentage |
|---|---:|---:|
| T1 | 2,710 | 44.7% |
| T2 | 1,045 | 17.2% |
| T3 | 1,034 | 17.1% |
| T4 | 591 | 9.8% |
| T5 | 578 | 9.5% |
| *Unset / other* | 101 | 1.7% |

T1 items dominate at nearly 45% of all items. The drop-off from T1 to T2 is sharp
(2,710 to 1,045), then T2 and T3 are roughly equal, with T4 and T5 trailing. This
suggests content authoring focused on the early game.

**Confidence: HIGH**

### 3.3 Container Sets (Slot Categories)

Container sets define which equipment/inventory slots an item can occupy. The largest
sets by item count:

| Container Set / Slot Category | Item Count | Notes |
|---|---:|---|
| Weapon + Bandolier + Bank | 1,469 | Weapons usable in bandolier and bankable |
| Mission Items | 749 | Quest/mission-specific items |
| Crafting Materials | 747 | Blueprint components and materials |
| Equippable (various slots) | 426-437 each | Head, chest, legs, feet, hands, etc. |

**Confidence: HIGH**

### 3.4 Item Reference Analysis

This is the critical metric: how many items are actually used by any game system?

| Reference Source | Unique Items Referenced | Notes |
|---|---:|---|
| Blueprint products | 494 | Items created by crafting |
| Blueprint components | 346 | Items consumed by crafting |
| Mission reward items | 8 | Items given as quest rewards |
| Vendor list items | 6 | Items available for purchase (2 vendor lists) |
| Loot table items | 2 | Items dropped by NPCs |
| **Total unique referenced** | **595** | **9.8% of all items** |
| **Orphaned items** | **5,464** | **90.2% of all items** |

Over 90% of items in the database are **orphaned** — they exist as definitions but are
not referenced by any mission reward, vendor list, loot table, blueprint, or other game
system. A player has no way to obtain these items through normal gameplay.

The crafting system accounts for the majority of referenced items (494 products + 346
components), but even crafting references less than 14% of the total item pool.

**Confidence: HIGH**

---

## 4. Abilities

**Total: 1,886 ability definitions** | **Confidence: HIGH**

The ability system defines all combat and non-combat actions available to players and
NPCs. While the raw count is large, only a fraction of abilities are wired into trainable
progression trees.

### 4.1 Type Distribution

| Ability Type | Count | Percentage |
|---|---:|---:|
| Undefined / Unclassified | ~854 | ~45.3% |
| Buff | 263 | 13.9% |
| Heal | 218 | 11.6% |
| DD (Direct Damage) | 211 | 11.2% |
| Debuff | 49 | 2.6% |
| DOT (Damage Over Time) | 14 | 0.7% |
| Other typed | 277 | 14.7% |

Nearly half of all abilities have no defined type, which limits the combat system's
ability to apply type-specific rules (e.g., interrupt effects on DD abilities, dispel
logic for Buffs/Debuffs).

**Confidence: HIGH**

### 4.2 Effect Linkage

| Metric | Count | Percentage |
|---|---:|---:|
| Abilities with at least one `effect_id` | ~961 | ~51% |
| Abilities with empty `effect_ids` | ~791 | ~42% |
| Abilities with ambiguous linkage | ~134 | ~7% |

Approximately half of all abilities have at least one effect linked. The other half are
ability shells with no mechanical effect — using them would do nothing.

**Confidence: HIGH**

### 4.3 Starting Abilities

| Ability ID | Name | Given To |
|---:|---|---|
| 592 | Pistol Shot | Soldier, Commando |
| 594 | Strike | Soldier, Commando |
| 597 | Heal Focus | Soldier, Commando |

Only 3 starting abilities exist, and they are assigned only to Soldier and Commando
archetypes. The remaining 6 archetypes (Scientist, Asgard, Goa'uld, Jaffa, Archaeologist,
and any others) start with **zero abilities**.

**Confidence: HIGH**

### 4.4 Archetype Ability Trees

| Archetype | Tree Entries | Distinct Trees | Notes |
|---|---:|---:|---|
| Soldier | 84 | 3 | Fully populated tree structure |
| Commando | 85 | 3 | Fully populated tree structure |
| Scientist | 0 | 0 | No ability tree data |
| Asgard | 0 | 0 | No ability tree data |
| Goa'uld | 0 | 0 | No ability tree data |
| Jaffa | 0 | 0 | No ability tree data |
| Archaeologist | 0 | 0 | No ability tree data |
| *(Other)* | 0 | 0 | No ability tree data |

Only **2 of 8 archetypes** have any ability tree data. Soldier and Commando together
account for all 169 trainer ability entries. The 16 abilities shared between both trees
suggest some cross-class utility or shared faction abilities.

**Confidence: HIGH**

### 4.5 Trainer Configuration

| Metric | Value |
|---|---|
| Total trainer ability entries | 169 |
| Soldier entries | 84 |
| Commando entries | 85 |
| Trainer lists | 1 (single shared list) |
| Prerequisite abilities populated | 0 (all empty) |
| Tree levels assigned | All set to 1 |

The trainer system has entries but no prerequisite chains and no level differentiation.
Every trainable ability is at tree level 1, meaning there is no unlock progression — all
abilities would be available immediately.

**Confidence: HIGH**

---

## 5. Effects

**Total: 3,216 effect definitions** | **Confidence: HIGH**

Effects are the mechanical backbone of the ability system — they define what actually
happens when an ability fires (damage, healing, buffs, debuffs, etc.).

### 5.1 Scripted Effects

| Effect ID | Name | Script Purpose |
|---:|---|---|
| 264 | RangedEnergyDamage | Ranged energy damage calculation |
| 658 | Reload | Weapon reload mechanic |
| 659 | TestEffect | Test/debug effect |
| 3091 | RangedPhysicalDamage | Ranged physical damage calculation |

Only **4 of 3,216 effects (0.12%)** have Python scripts. The remaining 3,212 effects
rely entirely on data-driven parameters — but as shown below, the parameter system is
also nearly empty.

**Confidence: HIGH**

### 5.2 Target Collection Methods

The target collection method (TCM) determines how an effect selects its targets.

| TCM | Count | Percentage | Description |
|---|---:|---:|---|
| TCM_Single | 1,953 | 60.7% | Single target |
| TCM_AERadius | 124 | 3.9% | Area effect (radius) |
| TCM_AECone | 14 | 0.4% | Area effect (cone) |
| TCM_Group | 9 | 0.3% | Group-wide effect |
| TCM_Aura | 8 | 0.2% | Persistent aura |
| TCM_RandomSingle | 2 | 0.1% | Random single target |
| *Unset* | 1,106 | 34.4% | No TCM assigned |

Over a third of effects have no target collection method set. Among those that do,
single-target effects dominate at 60.7%. AoE effects (radius, cone, aura combined) total
only 146 entries (4.5%), and the server-side AoE implementation is noted as incomplete in
the gap analysis.

**Confidence: HIGH**

### 5.3 Effect Parameters (NVPs)

| Metric | Value |
|---|---|
| Total effect NVP entries | 17 |
| Effects with NVPs | 7 |
| Parameter types used | HealthDamage, FocusDamage, HealPercentage |

Only 17 name-value-pair parameter entries exist across 7 effects. The 3,209 remaining
effects have **zero parameters**. This means the data-driven effect system has almost no
data to drive — effects without scripts or parameters are inert.

**Confidence: HIGH**

### 5.4 Effect-Ability Linkage

| Metric | Value |
|---|---|
| Unique abilities referencing effects | 1,703 |
| Percentage of abilities with effect links | ~90.3% |
| Effects referenced by abilities | varies |

While 90.3% of abilities point to at least one effect, most of those effects are empty
shells (no script, no parameters, no TCM). The link exists but the destination is hollow.

**Confidence: HIGH**

---

## 6. Dialogs

**Total: 5,411 dialog definitions** | **Confidence: HIGH**

The dialog system is one of the largest content pools by raw row count. Dialog structures
(screens, buttons, sets) are well-populated, but the conditional logic that would make
dialogs context-sensitive is almost entirely unused.

### 6.1 Volume Breakdown

| Table | Row Count | Notes |
|---|---:|---|
| `dialogs` | 5,411 | Top-level dialog definitions |
| `dialog_screens` | 13,466 | Individual dialog pages (~2.5 screens/dialog avg) |
| `dialog_screen_buttons` | 4,350 | Player response options (~0.3 buttons/screen avg) |
| `dialog_sets` | 1,178 | Named sets grouping related dialogs |
| `dialog_set_maps` | 4,670 | Mappings from sets to specific dialogs (~4.0 maps/set avg) |

### 6.2 Conditional Logic Usage

| Condition Type | Rows Using It | Percentage | Notes |
|---|---:|---:|---|
| `accepts_mission_id` set | 1 | <0.02% | Essentially zero |
| `missions_completed` conditions | 0 | 0% | Completely unused |
| `missions_not_accepted` conditions | 0 | 0% | Completely unused |
| Alignment gating (non-empty) | 2 | <0.05% | Sandbox test entries with SGU/Praxis alignment |

The dialog condition system exists in the schema but is functionally unused. This means:
- NPCs cannot change their dialog based on mission progress
- No dialog is gated behind mission completion
- Faction alignment has no effect on available conversations (except 2 test entries)

This is a significant content gap — in a working MMO, dialog conditions are the primary
mechanism for making NPCs feel responsive to player progress.

**Confidence: HIGH**

### 6.3 Entity Interactions

| Metric | Value |
|---|---|
| `entity_interactions` rows | 1 |
| Template ID | 43 (Anat) |
| Interaction set map | 3127 |
| `missions_not_accepted` filter | {742} |

The entire entity interaction table contains a single row: NPC template 43 (Anat) with
one conditional interaction that checks whether mission 742 has not been accepted. This is
the only example of a mission-gated NPC interaction in the database.

**Confidence: HIGH**

---

## 7. NPCs / Entity Templates

**Total: 153 entity templates** | **Confidence: HIGH**

Entity templates define NPC archetypes — their combat stats, abilities, loot, dialog, and
patrol behavior.

### 7.1 Template Type Distribution

| Type | Count | Percentage |
|---|---:|---:|
| Mob (hostile) | 133 | 86.9% |
| Being (friendly/neutral) | 14 | 9.2% |
| Spawnable | 6 | 3.9% |

### 7.2 Foreign Key Usage

This table shows how many entity templates actually reference other content systems via
their foreign keys:

| FK Field | Templates Using It | Percentage | Notes |
|---|---:|---:|---|
| `event_set_id` | 129 | 84.3% | Most templates have event sets |
| `weapon_item_id` | 5 | 3.3% | Only 5 NPCs have equipped weapons |
| `ability_set_id` | 3 | 2.0% | Only 3 NPCs have ability sets |
| `static_interaction_sets` | 3 | 2.0% | Only 3 NPCs have static interactions |
| `loot_table_id` | 2 | 1.3% | Only 2 NPCs drop loot (see Section 8) |
| `interaction_set_id` | 2 | 1.3% | Only 2 NPCs have interaction sets |
| `buy_list_id` | 1 | 0.7% | 1 vendor NPC |
| `sell_list_id` | 1 | 0.7% | 1 vendor NPC |
| `repair_list_id` | 1 | 0.7% | 1 repair NPC |
| `recharge_list_id` | 1 | 0.7% | 1 recharge NPC |
| `trainer_list_id` | 1 | 0.7% | 1 trainer NPC |
| `speaker_id` | 1 | 0.7% | 1 NPC with explicit speaker link |
| `patrol_path_id` | 0 | 0.0% | No NPCs patrol |

Event sets are well-populated (84.3%), but nearly every other system link is at or near
zero. No NPCs patrol. Only 2 drop loot. Only 3 have abilities. Only 1 each for vendor,
repair, recharge, and trainer functionality.

**Confidence: HIGH**

### 7.3 Speakers

| Metric | Value |
|---|---|
| Total speaker entries | 602 |
| Named speakers | 134 (22.3%) |
| Empty / placeholder speakers | 468 (77.7%) |

Notable named speakers from the Stargate franchise:

| Speaker Name | Notes |
|---|---|
| Vala Mal Doran | SG-1 team member |
| Daniel Jackson | SG-1 team member |
| Jack O'Neill | SG-1 commander |
| Teal'c | SG-1 team member (appears at IDs 1333, 2226, 2316) |
| General Hammond | SGC commander |
| Col. Carter | SG-1 team member |
| Thor | Asgard Supreme Commander |
| Ba'al | Goa'uld System Lord |
| Nerus | Goa'uld minor lord |
| Anat | Goa'uld |
| Master Bra'tac | Jaffa leader |
| Ra | Goa'uld (historical) |

Some characters have duplicate speaker entries across different IDs (e.g., Teal'c
appears three times: IDs 1333, 2226, and 2316). This likely represents the same character
in different zones or with different dialog states.

**Confidence: HIGH**

### 7.4 Spawn Lists

| Metric | Value |
|---|---|
| Total spawnlist entries | 167 |
| Worlds with spawns | 10+ |

Spawn entries place entity templates into specific world locations. The 167 entries are
distributed across at least 10 worlds, concentrated in the zones that received the most
development attention (Castle Cellblock, Harset, Men'fa, etc.).

**Confidence: HIGH**

---

## 8. Loot

**Total: 2 loot tables, 3 loot entries** | **Confidence: HIGH**

The loot system is the most conspicuously empty content category relative to its
importance in an MMO.

### 8.1 Loot Tables

| Table ID | Name | Status |
|---:|---|---|
| 1 | Cpl. Frost body - DEPRECATED | Deprecated (but still referenced) |
| 2 | Cellblock NID guard default | Active |

### 8.2 Loot Entries

| Table ID | Item (design_id) | Drop Chance | Quantity | Notes |
|---:|---:|---:|---:|---|
| 1 | 3730 | 100% | 1 | From deprecated Cpl. Frost table |
| 1 | 55 | 100% | 1 | From deprecated Cpl. Frost table |
| 2 | NULL | 80% | 5-50 | No item — likely currency drop |

### 8.3 Entity Template References

| Template ID | Loot Table ID | Notes |
|---:|---:|---|
| 15 | 2 | Cellblock NID guard |
| 24 | 2 | Cellblock NID guard variant |

Only 2 of 153 entity templates (1.3%) reference a loot table. Both point to loot table 2
(Cellblock NID guard default). The deprecated table 1 is no longer referenced by any
active template but persists in the database.

In practical terms: killing enemies in the game drops essentially nothing. The entire loot
economy consists of one active table with a single entry that has a NULL item ID (likely
mapping to a currency drop of 5-50 units at 80% chance).

**Confidence: HIGH**

---

## 9. Blueprints / Crafting

**Total: 498 blueprints, 2,556 components** | **Confidence: HIGH**

The crafting system is the most structurally complete content category. While we cannot
confirm runtime functionality, the data relationships are well-formed.

### 9.1 Volume Breakdown

| Table | Row Count | Notes |
|---|---:|---|
| Blueprints | 498 | Crafting recipes |
| Blueprint component entries | 2,556 | Input materials (~5.1 components/blueprint avg) |
| Crafting disciplines | 78 | Skill categories |
| Applied science categories | 4 | Top-level science groupings |
| Racial paradigms | 5 | Craft traditions by race |

### 9.2 Racial Paradigms

| Paradigm | Notes |
|---|---|
| Common | Shared across all races |
| Human | Tau'ri/Earth crafting |
| Goa'uld | Goa'uld technology |
| Asgard | Asgard technology |
| Ancient | Ancient technology |

### 9.3 Item References

| Reference Type | Unique Items | Notes |
|---|---:|---|
| Blueprint products (output) | 494 | Items created by crafting |
| Blueprint components (input) | 346 | Items consumed by crafting |
| **Total unique items in crafting** | **~840** | Some overlap between products and components |

The crafting system references more unique items than all other game systems combined.
Blueprint products alone (494) exceed the total of mission rewards (8) + vendor items (6)
+ loot items (2) by a factor of 30x.

**Confidence: HIGH**

---

## 10. Stargates

**Total: 28 stargates** | **Confidence: HIGH**

Stargates are the signature travel mechanism of the Stargate franchise and the primary
inter-zone transport system in the game.

### 10.1 Positioning Status

| Status | Count | Percentage |
|---|---:|---:|
| Positioned (non-zero coordinates) | 20 | 71.4% |
| Placeholder position (0,0,0) | 8 | 28.6% |

### 10.2 Event Set Configuration

| Status | Count | Percentage |
|---|---:|---:|
| Has `event_set_id` | 12 | 42.9% |
| No event set | 16 | 57.1% |

### 10.3 Address System

| Table | Row Count | Notes |
|---|---:|---|
| `stargates` | 28 | Gate placement definitions |
| `stargate_addresses` | **0** | **Empty — no dial addresses exist** |

The `stargate_addresses` table is completely empty. Without addresses, stargates cannot
function as a player-initiated travel system. Travel between zones would require
server-side scripting or hardcoded transitions rather than the canonical DHD (Dial Home
Device) interface.

**Confidence: HIGH**

### 10.4 Gate Locations

| World | World ID | Gate Notes |
|---|---:|---|
| The Castle | 8 | Castle Cellblock zone |
| Harset | 57 | Harset zone |
| Tollana | 19 | Tollan homeworld |
| Omega Site | 18 | Military outpost |
| Beta Site E1 | 23 | Beta Site variant |
| Lucia | 15 | Lucia zone |
| Agnos | 10 | Agnos zone |
| SGC W1 | 58 | Stargate Command Wing 1 |
| Dakara E1 | 61 | Dakara variant |
| Men'fa (Praxis) | — | Faction-specific gate |
| Men'fa (SGU) | — | Faction-specific gate |
| Ihpet Crater (Praxis) | — | Faction-specific gate |
| Ihpet Crater (SGU) | — | Faction-specific gate |

Several zones have faction-specific stargate variants (Men'fa Praxis/SGU, Ihpet Crater
Praxis/SGU), indicating that the two player factions (SGU and Praxis) had separate
gate access points in contested zones.

**Confidence: HIGH**

---

## 11. Worlds

**Total: 91 world definitions** | **Confidence: HIGH**

### 11.1 World Categories

| Category | Count | Notes |
|---|---:|---|
| Real game worlds (various stages) | ~50 | Actual playable zones |
| MissionTest* worlds | 26 | Empty test maps |
| Pocket instances | 5 | Instanced zones (Tol-Alpha, Ca-Alpha, etc.) |
| Other / unknown | ~10 | Uncategorized |

### 11.2 Configuration Sources

| Source | Count | Notes |
|---|---:|---|
| Database world definitions | 91 | All worlds |
| `spaces.xml` entries | 24 | Worlds with server-side space config |
| — Persistent spaces | 16 | Always-loaded zones |
| — Instanced spaces | 8 | Created on demand |
| Worlds with Python scripts | 10 | Zones with scripted behavior |

The gap between database definitions (91) and `spaces.xml` entries (24) means 67 worlds
exist as database rows only — they have no server-side spatial configuration and cannot
be loaded.

**Confidence: HIGH**

---

## 12. Other Content

### 12.1 Localization

| Metric | Value | Confidence |
|---|---:|---|
| Text entries | 29,126 | HIGH |

The localization table is the single largest content table by row count. It holds all
player-facing strings: dialog text, item names/descriptions, ability names, UI labels,
mission text, and system messages.

### 12.2 Sequences and Events

| Table | Count | Confidence |
|---|---:|---|
| Sequences | 1,973 | HIGH |
| Sequence NVPs | 2,042 | HIGH |
| Event sets | 675 | HIGH |

Sequences drive scripted events (camera movements, NPC animations, cinematic moments).
With 2,042 NVPs across 1,973 sequences, most sequences have approximately 1 parameter
each — minimal complexity.

Event sets are referenced by 129 of 153 entity templates (84.3%) and 12 of 28 stargates
(42.9%), making them one of the better-connected content types.

### 12.3 Visual Assets (Client-Side)

| Asset Type | Count | Confidence |
|---|---:|---|
| Static meshes | 5,152 | LOW |
| Skeletal meshes | 2,141 | LOW |
| Body components | 2,320 | MEDIUM |
| Visual variants (body component) | 3,657 | MEDIUM |

These counts come from the resource database and represent asset references. The actual
3D model files reside in the client `.pak` archives. The LOW confidence for mesh counts
reflects that we are trusting the DB entries without independently verifying the
referenced files exist in the client data.

### 12.4 Character Creation

| Metric | Value | Confidence |
|---|---:|---|
| Character creation configurations | 23 | HIGH |

23 configurations map to the combinations of faction, archetype, and race available
during character creation. The `Account.py` character creation flow (~300 lines)
validates against these configurations.

### 12.5 World Infrastructure

| System | Count | Confidence |
|---|---:|---|
| Ring transport regions | 28 | HIGH |
| Generic trigger regions | 28 | HIGH |
| Respawners | 7 | HIGH |

Ring transports are the secondary travel system (short-range, within-zone). The 28 ring
transport regions roughly match the 28 stargates, suggesting a 1:1 correspondence
between zones and ring transport setups.

Generic trigger regions (28) handle area-based events: zone transitions, cutscene
triggers, ambush spawns, etc.

Only 7 respawners exist, meaning most zones have no configured respawn points. Players
dying in zones without respawners would need fallback logic (e.g., returning to zone
entry point).

### 12.6 Social / Utility

| System | Count | Confidence |
|---|---:|---|
| Special words (chat filter) | 280 | HIGH |
| Containers (equipment slot types) | 20 | HIGH |

The chat filter contains 280 blocked words/phrases. The 20 container types define the
equipment and inventory slot system (head, chest, legs, feet, hands, weapon, bandolier,
bank, mission items, crafting, etc.).

---

## 13. Cross-Reference Integrity Summary

This table shows how connected each content type is to the rest of the game. "Inbound
references" means other systems point to this content. "Outbound references" means this
content points to other systems.

| Content Type | Total | Referenced by Other Systems | Orphaned | Orphan Rate | Confidence |
|---|---:|---:|---:|---:|---|
| Items | 6,059 | 595 | 5,464 | 90.2% | HIGH |
| Abilities | 1,886 | 169 (trainable) | 1,717 | 91.0% | HIGH |
| Effects | 3,216 | ~1,703 (via abilities) | ~1,513 | 47.0% | HIGH |
| Missions | 1,040 | 16 (scripted) | 1,024 | 98.5% | HIGH |
| Mission Tasks | 4,358 | 4,358 (all type=1) | 0* | 0%* | HIGH |
| Dialogs | 5,411 | 1 (with conditions) | 5,410 | 99.98% | HIGH |
| Dialog Set Maps | 4,670 | 2 (with conditions) | 4,668 | 99.96% | HIGH |
| Entity Templates | 153 | 167 (spawn entries) | varies | — | HIGH |
| Loot Tables | 2 | 2 (template refs) | 0 | 0% | HIGH |
| Loot Entries | 3 | 3 (in tables) | 0 | 0% | HIGH |
| Stargates | 28 | 0 (no addresses) | 28 | 100% | HIGH |
| Blueprints | 498 | — (standalone) | — | — | HIGH |
| Speakers | 602 | 134 (named) | 468 | 77.7% | HIGH |
| Worlds (DB) | 91 | 24 (in spaces.xml) | 67 | 73.6% | HIGH |

> \* Mission tasks have 0% orphan rate because all tasks are referenced by objectives,
> but all 4,358 are `task_type=1` — structurally linked but behaviorally identical.

### Referential Integrity Issues

| Issue | Severity | Details |
|---|---|---|
| Stargate addresses empty | High | 28 gates exist but 0 dial addresses — gates cannot be used |
| Loot table deprecation | Low | Table 1 ("Cpl. Frost body") marked DEPRECATED but not removed |
| Duplicate speakers | Low | Same character names at multiple IDs (Teal'c x3) |
| NULL loot item | Medium | Loot entry in table 2 has NULL `design_id` — relies on fallback behavior |
| 67 worlds without spaces.xml | Medium | Exist in DB but cannot be loaded by the server |

---

## 14. Key Findings

### Finding 1: Massive Authoring, Minimal Wiring

The database contains an impressive volume of authored content:
- 6,059 items
- 5,411 dialogs
- 3,216 effects
- 1,886 abilities
- 1,040 missions

But the **systems integration** that would connect this content into a playable game is
almost entirely absent. Content was authored in isolation — items were created but not
placed in loot tables, abilities were defined but not assigned to trainers, missions
were written but not given rewards.

**Confidence: HIGH**

### Finding 2: Mission Scripts Cover 1.5% of Content

Only 16 of 1,040 missions have Python scripts. The Castle Cellblock zone alone accounts
for 13 of those 16 scripts. Outside of one test zone, the mission system is pure data
with no scripted behavior.

**Confidence: HIGH**

### Finding 3: Effect Scripts Cover 0.12% of Content

Only 4 of 3,216 effects have Python scripts. Combined with only 17 parameter entries
across 7 effects, the effect system is structurally present but mechanically inert for
99.88% of its entries.

**Confidence: HIGH**

### Finding 4: 90.2% of Items Are Unreachable

Of 6,059 items, only 595 (9.8%) are referenced by any game system. The remaining 5,464
items have no path to the player — they cannot be looted, purchased, crafted, or earned.

**Confidence: HIGH**

### Finding 5: Zero Missions Grant XP or Currency

Not a single mission in the database has `reward_xp > 0` or `reward_naq > 0`. Players
cannot level up or earn currency through questing. Only 3 missions offer any item rewards
at all (8 total item entries).

**Confidence: HIGH**

### Finding 6: The Loot System Has 3 Entries

The entire loot economy of the game consists of 2 loot tables with 3 entries total. One
table is deprecated. The active table drops a NULL item (likely currency) from exactly
2 NPC templates. For an MMO, this is the most incomplete system by the widest margin.

**Confidence: HIGH**

### Finding 7: Dialog Conditions Are Unused

The dialog system has 5,411 dialogs and 4,670 set maps, but conditional logic
(mission gating, completion checks, faction requirements) is used in exactly 0-2 entries.
NPCs say the same things regardless of player progress.

**Confidence: HIGH**

### Finding 8: Only 2 of 8 Archetypes Have Ability Trees

Soldier (84 entries, 3 trees) and Commando (85 entries, 3 trees) are the only archetypes
with trainer data. The remaining 6 archetypes have zero trainable abilities, zero
ability trees, and zero starting abilities. Players choosing those archetypes would have
an empty action bar.

**Confidence: HIGH**

### Finding 9: All Mission Tasks Are Identical

Every one of the 4,358 mission tasks is `task_type=1`. There are no kill tasks, no
collection tasks, no escort tasks, no interaction tasks — just 4,358 copies of a single
placeholder type. The task type system was defined in schema but never populated with
behavioral variety.

**Confidence: HIGH**

### Finding 10: Mid-Development Snapshot

The data tells a consistent story: **Stargate Worlds was in mid-development when work
stopped.** Content designers had completed a massive bulk-authoring pass, creating
thousands of items, abilities, effects, and dialog entries. But the systems integration
pass — wiring content into loot tables, reward structures, ability trees, dialog
conditions, and scripted behavior — was in its earliest stages, concentrated almost
entirely on the Castle Cellblock test zone.

The game's content pipeline was: **define** (done for most types) -> **wire** (barely
started) -> **script** (done for <2% of content) -> **test** (done for 1 zone).

**Confidence: MEDIUM** — this interpretation is consistent with all observed data but
represents an inference about development process rather than a directly measurable fact.

---

## Appendix A: Content Type Definitions

| Term | Definition |
|---|---|
| Mission | A player quest with steps, objectives, and tasks |
| Step | An ordered stage within a mission |
| Objective | A goal within a step (can have multiple) |
| Task | An atomic action the player must perform to complete an objective |
| Effect | A mechanical outcome (damage, heal, buff) applied by an ability |
| TCM | Target Collection Method — how an effect selects its targets |
| NVP | Name-Value Pair — a parameter entry for an effect |
| Blueprint | A crafting recipe defining inputs and outputs |
| Entity Template | A server-side NPC archetype definition |
| Speaker | A named or unnamed NPC dialog identity |
| Dialog Set | A group of related dialogs assigned to an NPC |
| Dialog Set Map | A mapping entry linking a dialog set to specific dialogs with optional conditions |
| Event Set | A collection of triggered events (animations, sounds, spawns) |
| Sequence | A scripted cinematic or event sequence |
| Container | An equipment or inventory slot type |
| Paradigm | A racial crafting tradition (Common, Human, Goa'uld, Asgard, Ancient) |

## Appendix B: Methodology Notes

All counts in this document were derived from:

1. **Database queries** against the schema defined in `db/resources.sql` — this is the
   primary source for all content counts
2. **Python script inventory** from `python/` — cross-referenced with DB entries for
   scripted content percentages
3. **Entity definitions** from `entities/` (XML) — used for entity template and space
   configuration counts
4. **Client data references** from `data/cache/*.pak` — used for mesh and visual asset
   counts (lower confidence)
5. **Configuration files** from `config/` — used for server-side space and world counts

Where a count is given as exact (e.g., "1,040 missions"), it comes from a direct
database query. Where a count is approximate (e.g., "~50 real game worlds"), it involves
categorization judgment.

Cross-reference percentages (e.g., "9.8% of items referenced") were computed by querying
all foreign key relationships that point to the item table across all other tables and
deduplicating the resulting item ID set.
