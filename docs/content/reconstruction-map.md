# Reconstruction Map

What can be rebuilt from existing data, what has gaps requiring guesswork, and what was
never built and requires new assets or code.

> **Last updated**: 2026-03-01
> **Scope**: All gameplay content and server systems in the Cimmeria emulator
> **Sources**: `db/resources.sql`, `db/sgw.sql`, `python/`, `entities/`, `data/`, `config/`
> **Related documents**:
> - [Content Inventory](content-inventory.md) -- raw counts and distributions
> - [Zone Audit](zone-audit.md) -- per-zone completeness scorecards
> - [Archetype Content Map](archetype-content-map.md) -- per-archetype analysis
> - [Gap Analysis](../gap-analysis.md) -- per-feature status tracking
> - [Project Status](../project-status.md) -- system-level status summary

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

## Organization

This document divides the project into three categories:

1. **RECONSTRUCTABLE** -- Sufficient data exists in the database, code, or client files to
   rebuild the system without inventing new content. The work is wiring existing data
   together, not designing new gameplay.

2. **HOLES** -- Partial data exists but not enough to reconstruct without design decisions
   or informed guesswork. Filling these gaps requires choosing values, formulas, or
   behaviors that the original developers had not finalized.

3. **NEVER BUILT** -- No meaningful data exists. Building these systems requires new creative
   work: game design, content authoring, and potentially new client-side assets.

Within each category, items are ordered roughly by effort (lowest first).

---

## Category 1: RECONSTRUCTABLE

These items have sufficient data in the database, code, or client assets to reconstruct
without guesswork. They need code or scripts to wire existing data together.

---

### 1.1 Stargate Address Population

**Effort: TRIVIAL** | **Confidence: HIGH**

The `resources.stargate_addresses` table is empty, but 28 stargates exist with inline
`gate_address` strings (e.g., `"3-22-4-20-9-7/18"`). DHD interaction code works, the
client-side UI is functional, and address grant/revoke methods exist.

**What is needed:** Populate `stargate_addresses` from inline fields via SQL migration or
modify the loading code to use the inline format directly.

---

### 1.2 Navigation Mesh Generation

**Effort: LOW** | **Confidence: HIGH**

Only 5 of 24 zones have `.nav` files. NavBuilder compiles, uses Recast/Detour, and has
already produced working navmeshes for the 2 PLAYABLE zones. 4,618 `.umap` tiles
provide terrain geometry for all zones.

**What is needed:** Run NavBuilder on the remaining 19 zones.

**Priority:** Castle (upgrades to PLAYABLE -- only missing navmesh), Menfa_Dark (25
spawns, 38 DB missions), Lucia (892 tiles, 11 transporters), Omega_Site (5 beam pads).

---

### 1.3 Mission Reward Generation (XP and Currency)

**Effort: LOW** | **Confidence: MEDIUM**

All 1,040 missions have `reward_xp = 0` and `reward_naq = 0`. But every mission has a
level (1-50), `giveExperience()` is fully implemented, and reward fields are loaded from
DB and paid out on completion. The placeholder XP table covers 20 levels (MAX_LEVEL = 20).

**What is needed:** Extend XP table to 50 levels. Define XP/Naquadah reward formulas
based on mission level. SQL UPDATE to populate all 1,040 missions.

**Approach:** With ~10 missions per level, each should grant ~10-15% of the XP needed to
level. Standard exponential MMO curve. Values are guesswork but structurally sound.

---

### 1.4 Effect Script Generation

**Effort: LOW-MEDIUM** | **Confidence: HIGH** (structure) / **MEDIUM** (balance)

3,216 effects exist with full definitions (TCM, pulse, duration, flags). Only 4 have
scripts: `RangedEnergyDamage.py`, `RangedPhysicalDamage.py`, `Reload.py`, `TestEffect.py`.
The effect framework is fully implemented (application, removal, pulsing, stat mod, QR
combat damage, Kismet sequences). A stub generator tool exists. TCM distribution: 1,953
single-target (61%), 124 AoE radius (4%), 1,106 unset (34%).

**What is needed:** Generate scripts for 3,212 effects. Classify by ability type (DD,
Heal, Buff, Debuff, DOT) and TCM, then expand from the 4 existing templates. Effects
without NVPs (3,209 of 3,216) start as data-driven stubs using the effect definition's
parameters directly.

---

### 1.5 Loot Table Generation

**Effort: LOW-MEDIUM** | **Confidence: HIGH** (structure) / **MEDIUM** (balance)

Only 2 loot tables with 3 entries exist, yet 6,059 items span 5 quality tiers and 5
item tiers, and 153 entity templates span levels 1-50. The loot generation algorithm
(`Lootable.randomizeLoot`) is fully implemented. Item tier distribution: T1 (2,710),
T2 (1,045), T3 (1,034), T4 (591), T5 (578).

**What is needed:** Generate loot tables matching mob level brackets to item tiers
(T1 for L1-10, T2 for L11-20, etc.). Assign quality-based probability curves (Normal
~70%, Good ~20%, Great ~8%, Fantastic ~2%). Link `loot_table_id` to all 153 templates.
Add Naquadah drops. The 5,464 orphaned items (90.2%) become reachable.

---

### 1.6 Dialog Condition Population

**Effort: LOW-MEDIUM** | **Confidence: MEDIUM**

4,670 `dialog_set_maps` have `missions_completed[]` and `missions_not_accepted[]` fields
-- all empty. Only 1 dialog has `accepts_mission_id` set (dialog 2636 -> mission 742).
Only 1 `entity_interactions` row exists. The schema supports conditions, the loading
code works, but the data is unpopulated.

**What is needed:** Populate condition arrays based on mission chain data. Link
`accepts_mission_id` for quest-giving NPCs. Analyze mission step text for NPC references
("Report to Anat") and match against speaker/template names. Infer chains from level
ordering within zone labels. The mapping is inferrable but requires interpretation.

---

### 1.7 Mission Script Generation (~1,024 unscripted missions)

**Effort: MEDIUM per mission** | **Confidence: HIGH**

The database contains full step/objective/task structure for all 1,040 missions. Only
20 have Python scripts (13 Castle_CellBlock + 1 Harset + 1 SGC_W1 + 2 Castle + 3 test).
The existing scripts provide proven templates for the major mission patterns.

**What exists:**
- 1,040 missions with 3,475 steps, 4,033 objectives, and 4,358 tasks
- Step text containing NPC names, action descriptions, and location hints
- Mission labels grouping missions by zone/story arc (78 distinct labels)
- Mission levels providing progression ordering
- 20 existing mission scripts demonstrating the full pattern:
  - Region trigger entry/exit (`nXXX_trigger_In/Out`)
  - NPC dialog display and choice handling
  - Mission acceptance, advancement, and completion
  - Item grants and removal
  - Entity spawning and despawning
  - Minigame integration
  - Archetype branching
- MissionManager fully implemented: accept, advance, complete, fail, abandon, persist

**What is needed:**
- Python scripts for each mission implementing step progression, dialog triggers, and
  NPC interactions
- Most missions follow simple patterns: talk to NPC -> receive task -> complete
  objective -> return to NPC

**Reconstruction approach:** Template-based generation. Classify missions by their step
text patterns:
- "Speak to X" / "Report to X" -> dialog trigger mission
- "Kill X" / "Defeat X" -> combat objective mission (requires task type implementation)
- "Collect X" / "Retrieve X" -> item collection mission
- "Defend X" / "Protect X" -> escort/defense mission
- Hallway controllers (682-686 hidden missions) -> gate scripts (5 existing examples)

The Castle_CellBlock tutorial chain (missions 622-687) provides 13 complete examples
covering all major patterns. Each new mission script follows the same structure:
`__init__` creates triggers, trigger handlers display dialogs and advance steps.

**Scalability note:** The 430 "General" label missions and 202 unlabeled missions
cannot be zone-assigned without individual inspection. Zone-labeled missions (e.g.,
38 Men'fa, 26 Harset, 26 Dakara_E2) are more immediately actionable because their
zone context is known.

---

### 1.8 Spawn Placement from Existing Data

**Effort: MEDIUM** | **Confidence: MEDIUM**

167 spawnlist entries exist, concentrated in the more developed zones. Entity templates
define 153 NPC archetypes with levels, factions, event sets, and visual data. The spawn
system architecture is fully designed in the entity `.def` files, with 20 base methods
declared across `SGWSpawnRegion` and `SGWSpawnSet`.

**What exists:**
- 167 `spawnlist` entries with world coordinates, headings, template IDs, and tags
- 153 entity templates with levels, factions, and visual data
- `SGWSpawnRegion.def` and `SGWSpawnSet.def` with complete property and method sets
- Named spawn tags used by mission scripts (e.g., `MessHall_Guard1`, `Prisoner_329`)
- Space scripts that place some NPCs via `spawnMob()` calls
- Spawn distribution across zones: Castle (33), SGC_W1 (26), Menfa_Dark (25),
  Harset (22), Lucia (12), etc.

**What is needed:**
- Implement `SGWSpawnRegion.py` and `SGWSpawnSet.py` (currently 100% empty stubs)
- Expand spawnlist entries for zones with few or zero spawns
- Add spawn coordinates for NPCs in zones where only the template exists but no
  placement data was authored

**Reconstruction approach:** The spawn system design is documented in `spawn-system.md`
based on the `.def` file properties and method signatures. Implementation follows the
standard pattern: SpawnRegion manages SpawnSets, SpawnSets manage individual mob
entities, mobs report back to their set on death for respawn scheduling. The 167
existing entries provide placement data; new entries require zone geometry knowledge
from client `.umap` files or navmesh data.

---

### 1.9 Crafting System Verification

**Effort: LOW** | **Confidence: HIGH**

The most structurally complete content category. 498 blueprints, 2,556 components, 78
disciplines, 5 racial paradigms. `Crafter.py` (575 lines) covers ~95% of features.
Crafting references 840 unique items -- more than all other systems combined.

**What is needed:** End-to-end testing with a live client. This is a verification task,
not a reconstruction task. The data and code already exist.

---

## Category 2: HOLES

These items have some data but not enough to reconstruct without design decisions or
informed guesswork. The gap is not "missing code" but "missing design intent."

---

### 2.1 Mission Task Types

**Effort: MEDIUM** | **Confidence: MEDIUM**

All 4,358 mission tasks are `task_type=1`. Zero behavioral variety exists. But mission
step text strongly implies diverse task types: "Kill 3 Jaffa", "Collect 4 corpses",
"Defend Private Burgin", "Activate the console".

**What exists:**
- 4,358 tasks, all `task_type=1`
- `onTaskUpdate` client method defined (TaskID, Status, Count) -- designed for progress
  tracking
- Step text with action verbs implying task types
- MissionManager handles task state tracking

**What is missing:**
- Task type enum definition (kill, collect, interact, escort, defend, activate, etc.)
- Per-type completion logic (kill counter, item pickup, distance check, NPC alive check)
- Task-to-objective completion propagation

**The gap:** The task type system was stubbed with a default value during content
authoring and never received the implementation pass. Step text reveals what the types
should be, but extracting types from natural-language text requires interpretation.

**Recommended approach:** Define a task type enum based on MMO conventions and the
action verbs found in step text. Implement per-type handlers in MissionManager. Classify
existing tasks by parsing their step text. The classification is inferrable but requires
manual review for ambiguous cases.

---

### 2.2 Stat Scaling and Derived Formulas

**Effort: HIGH** | **Confidence: LOW**

60+ stats per combatant with a 6-tier structure. Stat class fully implemented (mutation,
clamping, callbacks). 6 base attributes defined. HP/Focus pools identical across all
archetypes (760/1570, +10/+70 per level). Only Soldier and Commando have differentiated
base attributes.

**What is missing:** Level-based scaling formulas, derived stat formulas (how attributes
affect combat), stat contribution weights, diminishing returns, HP/Focus regen loop,
per-archetype differentiation for 6 placeholder archetypes.

**The gap:** The framework is solid but the math is absent. Stat names suggest intent
(e.g., `fortitude` -> health resistance) but conversion formulas were never coded.
Server-side formulas are not in the client binary. Any values are guesswork.

---

### 2.3 XP Curve and Leveling

**Effort: MEDIUM** | **Confidence: ASSUMPTION**

Current XP table covers 20 levels with placeholder values (irregular -- L12 requires
less per-level than L11). Missions span L1-50, implying a 50-level game.
`giveExperience()` works with multi-level-up. MAX_LEVEL = 20.

**What is missing:** XP table for levels 21-50, corrected L1-20 curve, XP-per-kill
formulas, XP modifiers (group bonus, rest), level-50 progression.

**Approach:** Standard exponential MMO curve calibrated so ~10 missions/level provide
60-80% of required XP. Reference similar-era MMOs (2008-2010). ~1.5x multiplier per
5 levels based on existing placeholder shape.

---

### 2.4 Combat Formulas

**Effort: HIGH** | **Confidence: LOW**

Single-target combat works via the QR (Quality Rating) system using a beta distribution.
Damage pipeline (base -> resist -> QR -> AF -> absorb) is implemented. 4 effect scripts
use `qrCombatDamage()`. Effect NVP values: HealthDamage 8-25, FocusDamage 80-250.

**What is missing:** Base attribute -> QR modifier formulas, armor mitigation, resistance
per damage type, level-based ability scaling, AoE/cone targeting (only single-target
works), channeled abilities, diminishing returns, cover system modifiers.

**Approach:** Start with linear scaling (stat * coefficient) and iterate. Use existing NVP
values as baselines. Test with Soldier/Commando. RE docs cover wire format but not
server-side formulas.

---

### 2.5 Vendor Pricing and Economy

**Effort: MEDIUM** | **Confidence: LOW**

6,059 items lack prices. Only 6 vendor items have Naquadah prices (50-1000). Vendor
buy/sell/repair/recharge code works in `Vendor.py`. Only 1 each of vendor/repair/
recharge/trainer NPCs exist.

**What is missing:** Base price formulas by tier/quality, sell-back ratios, repair costs,
vendor inventories per zone, NPC vendor placement. A functioning economy requires price
curves, vendor placement, and balanced sink/faucet ratios -- none finalized.

---

### 2.6 NPC Patrol Paths

**Effort: MEDIUM-HIGH per zone** | **Confidence: LOW**

`patrol_path_id` field exists on templates (all NULL). `patrol_point_delay` field exists.
AI states Patrol (8) and Wander (9) defined but unimplemented. C++ nav primitives
(`findPathTo`, `addWaypoint`) available. No `patrol_paths` table exists in schema.

**The gap:** Complete absence of patrol data. Designing routes requires zone layout
knowledge, NPC role analysis, and navmesh geometry. This is a design task per zone.

---

### 2.7 Archetype Ability Design (6 archetypes)

**Effort: HIGH** | **Confidence: LOW**

Soldier (84 abilities, 3 trees) and Commando (85 abilities, 3 trees) are fully
defined. The remaining 6 archetypes have zero ability trees, zero starting abilities,
and identical stats cloned from Soldier. The framework is fully generic and would work
with any archetype that has populated data.

**What exists:**
- 1,886 total abilities in DB (many unassigned to any archetype)
- Working ability framework: loading, training, trees, combat -- all archetype-agnostic
- 16 shared abilities between Soldier and Commando (cross-class utility)
- Ability type distribution: ~854 unclassified, 263 Buff, 218 Heal, 211 DD,
  49 Debuff, 14 DOT, 277 other
- `TrainerAbilityList.py` filters by archetype automatically
- Character creation handles any archetype with populated data

**What is missing:**
- Ability tree definitions for Scientist, Archeologist, Asgard, Goa'uld, Shol'va, Jaffa
- Starting abilities for 6 archetypes (currently only Soldier and Commando get 3 each)
- Differentiated base stats for 6 archetypes (all are Soldier clones)
- Prerequisite chains within ability trees (all current entries have empty prerequisites)
- Tree level assignments (all current entries are level 1)
- ~500 new ability definitions (~85 per archetype x 6)

**The gap:** The ~1,700 unassigned abilities in the database may include some intended for
other archetypes, but there is no mapping. Ability themes must be designed:
- Scientist: technology/healing
- Archeologist: relics/knowledge
- Asgard: psionic/technology (event set 1455 already unique)
- Goa'uld: symbiote/host powers (unique body set exists)
- Shol'va: staff weapon/melee (SGU-aligned Jaffa)
- Jaffa: staff weapon/melee (Praxis-aligned, tutorial branching exists)

**Why this is a HOLE, not NEVER BUILT:** The framework exists and is tested. The
bottleneck is ability content, not code. New abilities need DB entries, effect links,
and balance testing -- but no new Python code is required to support them. The
distinction between "design the abilities" (HOLE) and "build the ability system"
(NEVER BUILT) is important: the system is done, the content is not.

---

## Category 3: NEVER BUILT

These items have no meaningful data in any source and require new creative work: game
design, content authoring, and potentially new client-side support.

---

### 3.1 SHELL Zone Content (14 zones)

**Effort: VERY HIGH per zone** | **Confidence: N/A**

14 of 24 defined zones have a SHELL verdict: client art exists, a server space
definition exists, but zero gameplay content (no missions, no NPCs, no scripted
behavior).

**SHELL zones by client art size:**

| Zone | Type | Tiles | Stargate | Spawns | DB Missions |
|------|------|-------|----------|--------|-------------|
| Lucia | Persistent | 892 | YES | 12 | Unknown |
| Beta_Site_Evo_1 | Persistent | 631 | YES | 5 | ~6 |
| Tollana | Persistent | 505 | YES | 3 | Unknown |
| Dakara_E1 | Persistent | 401 | YES | 0 | Unknown |
| Menfa_Light | Persistent | 271 | YES | 0 | Unknown |
| Ihpet_Crater_Dark | Persistent | 155 | YES | 0 | ~22 (shared) |
| Ihpet_Crater_Light | Persistent | 155 | YES | 0 | ~22 (shared) |
| Sewer_Falls | Persistent | 73 | NO | 0 | Unknown |
| Agnos_Library | Persistent | 37 | NO | 0 | Unknown |
| SGC | Instanced | 17 | YES | 0 | Unknown |
| Harset_Market | Instanced | 10 | NO | 0 | Unknown |
| Harset_StorageRm | Instanced | 10 | NO | 0 | Unknown |
| Dakara_E1_StoryRm | Instanced | 2 | NO | 0 | Unknown |
| Tollana_Curia | Instanced | 2 | NO | 0 | Unknown |

**What is needed per zone:**
- Space script (Python) defining entity spawns, triggers, and event handling
- NPC placement with coordinates from client map geometry
- Mission chains with dialog trees
- Zone-specific loot and vendor content
- Navigation mesh (Category 1 task, prerequisite)

**Scale of effort:** The 2 PLAYABLE zones (Castle_CellBlock at 511 lines + 13 mission
scripts, SGC_W1 at 480 lines + 1-4 mission scripts) represent the reference standard.
Each SHELL zone would need comparable investment. With 14 SHELL zones, this represents
months of content design work.

**Priority ordering:** See the zone audit's development priority recommendations. Castle,
Harset, and Agnos are Tier 1 (closest to PLAYABLE). Menfa_Dark, Lucia, and Omega_Site
are Tier 2 (transport infrastructure exists, needs content).

---

### 3.2 DB-Only Worlds (67 worlds)

**Effort: HIGH per world** | **Confidence: LOW**

67 worlds exist as DB records but have no `spaces.xml` entry. Most notable: Dakara_E2
(26 missions), Dakara_E3 (24 missions), SGC_W2 (sequel to PLAYABLE SGC_W1), Hebridan,
Sokars_Moon, Egypt, Kheb, Yotunheim, CombatSim, PrimHatak. Each needs space definition,
client art verification, navmesh, and full zone content. The episodic naming (E1/E2/E3,
W1/W2) reveals the planned content delivery model. Planned scope was ~3x what was
server-implemented.

---

### 3.3 Spawn System Implementation

**Effort: HIGH** | **Confidence: HIGH** (design known) / **N/A** (code)

Zero lines of implementation. `SGWSpawnRegion.py` and `SGWSpawnSet.py` are empty stubs.
Complete `.def` files exist (20 base methods: population limits, respawn timers,
time-of-day hooks, mission hooks). Design documented in `docs/gameplay/spawn-system.md`.
DB has 167 spawnlist entries and 153 templates.

**What is needed:** Full Python implementation (~500-800 lines). SpawnRegion manages
SpawnSets, enforces population caps, schedules respawns. SpawnSet selects mob types
from weighted tables, creates entities, tracks live mobs.

**Priority:** Highest-priority NEVER BUILT item. Without it, no mobs exist dynamically.
Item #1 on the critical path for playability.

---

### 3.4 NPC AI States (8 of 12 unimplemented)

**Effort: HIGH** | **Confidence: MEDIUM**

12 AI states defined; 4 work (Spawning, Idle, Fighting, Dead). 8 need implementation:
Patrol, Wander, Leash, Investigate, Follow, Despawn, Submit, Error. `doAiAction()` tick
loop and threat system work. C++ nav primitives (`findPathTo`, `addWaypoint`) exist but
are never called.

**The gap:** Movement is the core missing capability. Mobs stand still during combat.
Implementing movement states requires per-state handlers and navmesh pathfinding
integration.

---

### 3.5 Stub-Only Social and Economy Systems

The following systems have entity `.def` files, wire format documentation from reverse
engineering, and Python stubs -- but zero functional code. Each requires full
implementation from the wire format documentation inward.

| System | Effort | Stubs | RE Docs | Notes |
|--------|--------|-------|---------|-------|
| **PvP / Dueling** | Very High | 6 methods, all `pass` | duel-wire-formats.md | SGWDuelMarker entity empty. Faction system (SGU/Praxis) provides foundation |
| **Organizations** | High | 15+ methods, all `pass` | organization-wire-formats.md | SGWPlayerGroupAuthority empty shell. Needs DB schema |
| **Black Market** | High | 6 methods, all `pass` | black-market-wire-formats.md | Auction listing, bidding, buyout, fees. Needs DB schema |
| **Mail** | Medium-High | sendMailMessage = `pass` | mail-wire-formats.md | Some read-only methods partial. Needs DB table |
| **Contact Lists** | Medium | 6 methods, all `pass` | contact-list-wire-formats.md | Friend/ignore list, online status |
| **Groups** | Medium | All methods empty | group-wire-formats.md | SGWPlayerGroupAuthority empty shell |

---

### 3.6 Minigame Server Logic (6 of 9 types)

**Effort: MEDIUM per type** | **Confidence: LOW**

9 minigame types exist in `python/base/minigame/`. 3 have logic (Livewire, Hack,
GoauldCrystals). 6 are placeholders (Activate, Analyze, Bypass, Converse, Alignment,
Placeholder). Needs server-side validation, implementation of 6 placeholder types,
and scoring/rewards.

---

### 3.7 Pet System

**Effort: MEDIUM** | **Confidence: LOW**

`SGWPet` entity exists with basic init but no behavior. Pet commands defined but
unimplemented. Follow action stub exists. Needs pet AI states, owner binding,
combat integration, summoning/dismissal.

---

### 3.8 End-Game, Cinematics, and Raid Content

**Effort: VERY HIGH** | **Confidence: N/A**

No end-game systems, raid mechanics, reputation systems, or alternative advancement
exists. 1,973 sequences with NVPs exist in DB and space scripts trigger some, but no
cinematic camera system or cutscene scripting framework exists beyond basic event
triggers. These are entirely new game design work.

---

### 3.9 Server Infrastructure Systems

**Effort: MEDIUM-HIGH total** | **Confidence: MEDIUM**

Eight systems documented in `docs/architecture/server-systems.md`. Most have partial
foundations. See that document for full details.

| System | Current State | Priority |
|--------|---------------|----------|
| GM Tools Backend | 50+ commands work; console disabled (empty password) | HIGH |
| Session Management | Basic timeout exists; no reconnection grace | HIGH |
| Rate Limiting | Per-ability cooldowns only | MEDIUM |
| Anti-Cheat | Position bounds only; no speed validation | MEDIUM |
| World State Persistence | Player pos persists; no world object state | MEDIUM |
| Economy / Scheduler / Metrics | Minimal | LOW |

**Immediate action:** Set `<py_console_password>` in `config/BaseService.config` to
enable 50+ GM commands. Zero code changes required.

---

## Summary Table

| Category | Items | Effort Range | Data Available | Key Metric |
|----------|-------|-------------|----------------|------------|
| **RECONSTRUCTABLE** | 9 items | Trivial to Medium | 70-100% | Data exists, needs wiring |
| **HOLES** | 7 items | Medium to High | 20-50% | Partial data, needs design decisions |
| **NEVER BUILT** | 9 items | Medium to Very High | 0-10% | No data, needs new creative work |

### Category 1 (RECONSTRUCTABLE) by effort

| Item | Effort | Impact |
|------|--------|--------|
| 1.1 Stargate addresses | Trivial | Enables gate travel UI |
| 1.2 Navmesh generation | Low | Upgrades Castle to PLAYABLE; enables NPC navigation |
| 1.3 Mission rewards | Low | Makes 1,040 missions grant XP/currency |
| 1.4 Effect scripts | Low-Medium | Activates 3,212 inert effects |
| 1.5 Loot tables | Low-Medium | Connects 5,464 orphaned items to gameplay |
| 1.6 Dialog conditions | Low-Medium | Makes NPCs responsive to player progress |
| 1.7 Mission scripts | Medium each | Brings 1,024 missions to life |
| 1.8 Spawn placement | Medium | Populates zones with NPCs |
| 1.9 Crafting verification | Low | Validates the most complete system |

### Category 2 (HOLES) by effort

| Item | Effort | Blocker? |
|------|--------|----------|
| 2.1 Task types | Medium | Blocks mission variety |
| 2.2 Stat scaling | High | Blocks meaningful leveling |
| 2.3 XP curve | Medium | Blocks 50-level progression |
| 2.4 Combat formulas | High | Blocks balanced combat |
| 2.5 Vendor pricing | Medium | Blocks economy |
| 2.6 Patrol paths | Medium-High | Blocks NPC movement |
| 2.7 Archetype abilities | High | Blocks 6 of 8 archetypes |

### Category 3 (NEVER BUILT) by priority

| Item | Effort | Critical Path? |
|------|--------|---------------|
| 3.3 Spawn system | High | YES -- #1 on critical path |
| 3.4 NPC AI states | High | YES -- mobs cannot move without this |
| 3.9 Server infrastructure | Medium-High | YES -- session management, GM tools |
| 3.1 SHELL zone content | Very High | Per zone -- gates playable area |
| 3.6 Minigame logic | Medium each | Blocks mission types using minigames |
| 3.5 Stub social/economy | Medium-Very High | Quality of life and feature completeness |
| 3.7 Pet system | Medium | Feature completeness |
| 3.2 DB-only worlds | High each | Content scope expansion |
| 3.8 End-game/cinematics | Very High | Post-launch feature |

---

## Priority Recommendations

### Quick Wins (days, not weeks)

1. **Stargate addresses** (1.1) -- Single SQL script, trivial
2. **Enable GM console** (3.9) -- Single config value, 50+ commands become available
3. **Navmesh for Castle** (1.2) -- One NavBuilder run, upgrades zone to PLAYABLE
4. **Crafting verification** (1.9) -- Test existing system, no new code

### Medium Investment (weeks)

5. **Mission reward generation** (1.3) -- Algorithmic XP/currency population
6. **Loot table generation** (1.5) -- Algorithmic item distribution
7. **Effect script stubs** (1.4) -- Template expansion from 4 existing scripts
8. **Dialog condition population** (1.6) -- SQL updates with mission chain analysis
9. **Spawn system implementation** (3.3) -- Critical path, ~500-800 lines Python
10. **XP curve expansion** (2.3) -- Extend 20-level table to 50 levels

### Long-Term Design (months)

11. **NPC AI movement states** (3.4) -- Patrol, Wander, Leash, Chase
12. **Mission script generation** (1.7) -- Template-based, ~1,024 missions
13. **Combat formula design** (2.4) -- Core balance system
14. **Stat scaling formulas** (2.2) -- Level progression math
15. **Archetype ability design** (2.7) -- 6 archetypes x ~85 abilities each
16. **SHELL zone content** (3.1) -- Per-zone content design, 14 zones

### Out of Scope for Near-Term

17. PvP/Dueling/Organizations/Black Market (3.5) -- Social and economy features, not on critical path
18. End-game/Cinematics (3.8) -- Post-launch concern
19. DB-only worlds (3.2) -- 67 worlds, each requiring full zone build-out

---

## The Core Loop

The minimum viable gameplay loop is:

```
Spawn mobs (3.3) -> Player fights mobs (exists) -> Mobs drop loot (1.5) ->
Player gains XP (1.3 + 2.3) -> Player levels up (exists) -> Stats scale (2.2) ->
Player takes missions (1.7) -> Missions reward (1.3) -> Player travels (1.1) ->
New zone, repeat
```

Items on the critical path for this loop, in dependency order:

1. Spawn system (3.3) -- without it, no mobs
2. Loot tables (1.5) -- without it, combat is unrewarding
3. Mission rewards (1.3) -- without it, missions are unrewarding
4. XP curve (2.3) -- without it, progression caps at 20
5. Stat scaling (2.2) -- without it, leveling is cosmetic
6. Stargate addresses (1.1) -- without it, players cannot travel between zones
7. Navmesh generation (1.2) -- without it, NPCs cannot navigate

Everything else enhances the loop (combat formulas make it feel better, patrol paths
make NPCs look alive, dialog conditions make story feel responsive) but is not strictly
required for the loop to function.

---

## Appendix: Reconstruction Dependency Graph

Items that must be completed before others can begin or function correctly.

```
Stargate Addresses (1.1) <- no dependencies, immediate
Navmesh Generation (1.2) <- client .umap files (exist)
Effect Scripts (1.4)      <- no dependencies, templates exist
Loot Tables (1.5)         <- no dependencies, algorithmic
Mission Rewards (1.3)     <- XP Curve (2.3) for correct values (can use placeholder)
Dialog Conditions (1.6)   <- mission chain analysis (step text interpretation)
Spawn System (3.3)        <- Navmesh (1.2) for entity movement
NPC AI States (3.4)       <- Spawn System (3.3) + Navmesh (1.2)
Spawn Placement (1.8)     <- Spawn System (3.3) + Navmesh (1.2)
Mission Scripts (1.7)     <- Dialog Conditions (1.6) + Task Types (2.1) + Spawn System (3.3)
Combat Formulas (2.4)     <- Stat Scaling (2.2) + Effect Scripts (1.4)
Archetype Abilities (2.7) <- Combat Formulas (2.4) + Effect Scripts (1.4)
SHELL Zone Content (3.1)  <- Navmesh (1.2) + Spawn System (3.3) + NPC AI (3.4) + Missions (1.7)
```

The critical dependency chain for the core gameplay loop:

```
Navmesh (1.2) -> Spawn System (3.3) -> Loot Tables (1.5) + Mission Rewards (1.3)
                                         -> XP Curve (2.3) -> Stat Scaling (2.2)
```

Everything else branches off this chain or is independent of it.
