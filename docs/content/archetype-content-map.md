# Archetype Content Availability Map

Per-archetype analysis of what content exists, what is placeholder, and what is missing
entirely in the Cimmeria server emulator. Based on cross-referencing the resource database
(`db/resources.sql`), Python game logic (`python/`), and entity definitions.

---

## Confidence Framework

| Level | Meaning |
|---|---|
| **CONFIRMED** | Verified in DB + code + tested |
| **HIGH** | Present in DB or code, cross-referenced against multiple sources |
| **MEDIUM** | Inferred from cross-referencing multiple sources |
| **LOW** | Based on client file names or structural inference |
| **ASSUMPTION** | MMO industry norms or developer intent guess |

---

## 1. Summary Table

Per-archetype completeness at a glance. Each column rates whether the system has
real, differentiated content for that archetype or just placeholder/clone data.

| System | Soldier | Commando | Scientist | Archeologist | Asgard | Goa'uld | Shol'va | Jaffa |
|---|---|---|---|---|---|---|---|---|
| **Base Stats** | Unique | Unique | Clone | Clone | Clone | Clone | Clone | Clone |
| **Ability Trees** | 84 abilities | 85 abilities | 0 | 0 | 0 | 0 | 0 | 0 |
| **Starting Abilities** | 3 | 3 | 0 | 0 | 0 | 0 | 0 | 0 |
| **Trainer List** | 84 | 85 | 0 | 0 | 0 | 0 | 0 | 0 |
| **Char Creation** | 4 defs | 4 defs | 4 defs | 4 defs | 1 def | 2 defs | 2 defs | 2 defs |
| **Tutorial Missions** | Branched | Branched | Generic | Generic | Generic | Generic | Generic | Branched |
| **Item Event Set** | 804 | 804 | 804 | 804 | **1455** | 804 | 804 | 804 |
| **Playable?** | Yes | Yes | No | No | No | No | No | No |

**Legend:** "Clone" = identical stats copied from Soldier. "0" = no data exists. "Branched" = archetype-specific dialog/rewards in tutorial scripts.

**Confidence:** HIGH -- all values taken directly from `resources.sql` INSERT statements and Python source.

---

## 2. Soldier Profile

**Status: FULLY IMPLEMENTED** (Confidence: CONFIRMED)

The Soldier is the primary archetype and the template from which all placeholder
archetypes were cloned. It is the only archetype with fully differentiated stats.

### Base Stats

| Stat | Value |
|---|---|
| Coordination | 5 |
| Engagement | 4 |
| Fortitude | 3 |
| Morale | 4 |
| Perception | 3 |
| Intelligence | 2 |
| Health | 760 |
| Focus | 1570 |
| HP/Level | 10 |
| Focus/Level | 70 |

Source: `resources.archetypes` WHERE archetype = 'ARCHETYPE_Soldier'

### Ability Trees (84 total)

| Tree | Count | Theme (inferred from ability names) |
|---|---|---|
| Tree 0 | 29 | Ranged combat -- Aimed Shot, Quick Burst, Snare Shot, Cover Fire, Pinning Fire |
| Tree 1 | 28 | Shared/utility -- Pistol Shot (592), Strike (594), Heal Focus (597) |
| Tree 2 | 27 | Defensive/tactical -- Stance abilities, ammunition types, area control |

All 84 abilities have `level = 1` and `prerequisite_abilities = '{}'`.
No prerequisite chains or level gating has been implemented.

**Confidence:** HIGH -- counts verified from `resources.archetype_ability_tree` inserts.

### Starting Abilities

All Soldier character definitions (def IDs 1, 2, 11, 12) receive:

| Ability ID | Name | Description |
|---|---|---|
| 592 | Pistol Shot | Ranged single-target attack (shared with Commando) |
| 594 | Strike | Melee attack (shared with Commando) |
| 597 | Heal Focus | Restores 35% Focus, 30s cooldown (shared with Commando) |

Source: `resources.char_creation_abilities`

### Character Creation Definitions

| Def ID | Faction | Gender | Body Set |
|---|---|---|---|
| 1 | Praxis | Male | BS_HumanMale |
| 2 | SGU | Male | BS_HumanMale |
| 11 | Praxis | Female | BS_HumanFemale |
| 12 | SGU | Female | BS_HumanFemale |

Starting world: Praxis -> Castle_CellBlock, SGU -> SGC_W1

### Trainer Access

84 abilities available through trainer list_id=1 ("Debug ability list").
Matches the full ability tree -- every tree ability is trainable.

---

## 3. Commando Profile

**Status: FULLY IMPLEMENTED** (Confidence: CONFIRMED)

The Commando is the only archetype with stats that differ from the Soldier baseline.
It has a distinct ability set focused on explosives, traps, and demolitions.

### Base Stats (differences from Soldier in bold)

| Stat | Value | vs Soldier |
|---|---|---|
| Coordination | **4** | -1 |
| Engagement | 4 | same |
| Fortitude | **2** | -1 |
| Morale | **3** | -1 |
| Perception | **5** | +2 |
| Intelligence | **3** | +1 |
| Health | 760 | same |
| Focus | 1570 | same |
| HP/Level | 10 | same |
| Focus/Level | 70 | same |

The Commando trades durability (lower Coordination, Fortitude, Morale) for
awareness and cunning (higher Perception, Intelligence). HP/Focus pools and
per-level scaling are identical across all archetypes.

Source: `resources.archetypes` WHERE archetype = 'ARCHETYPE_Commando'

### Ability Trees (85 total)

| Tree | Count | Theme (inferred from ability names) |
|---|---|---|
| Tree 0 | 28 | Demolitions -- Anti-Personnel Mine, C-4, Claymore, Tactical Nuke, EMP Grenade |
| Tree 1 | 30 | Shared/utility + stealth -- device disarm, flash mines, sticky bombs |
| Tree 2 | 27 | Tactical -- shockwave grenades, cluster mines, overloads |

All 85 abilities have `level = 1` and `prerequisite_abilities = '{}'`.

### Shared Abilities with Soldier

16 abilities appear in both Soldier and Commando trees:

| Ability ID | Notes |
|---|---|
| 523 | Shared utility |
| 592 | Pistol Shot (starting ability) |
| 594 | Strike (starting ability) |
| 597 | Heal Focus (starting ability) |
| 654, 656 | Shared combat |
| 716, 722 | Shared combat |
| 745, 780 | Shared combat |
| 847, 856 | Shared combat |
| 861, 867, 868 | Shared combat |
| 891 | Shared combat |

68 abilities are Soldier-unique. 69 abilities are Commando-unique.

**Confidence:** HIGH -- verified via set intersection of ability_id values from `archetype_ability_tree`.

### Starting Abilities

All Commando character definitions (def IDs 3, 4, 13, 14) receive the same 3
starting abilities as Soldier: Pistol Shot (592), Strike (594), Heal Focus (597).

### Character Creation Definitions

| Def ID | Faction | Gender | Body Set |
|---|---|---|---|
| 3 | Praxis | Male | BS_HumanMale |
| 4 | SGU | Male | BS_HumanMale |
| 13 | Praxis | Female | BS_HumanFemale |
| 14 | SGU | Female | BS_HumanFemale |

### Trainer Access

85 abilities available through trainer list_id=1.
Matches the full ability tree.

---

## 4. Placeholder Archetypes

All six remaining archetypes have database rows in `resources.archetypes` with stats
identical to Soldier, character creation definitions allowing them to be selected, but
zero ability content -- no ability trees, no starting abilities, no trainer abilities.
Any of them can be selected at character creation and will spawn correctly, but the
character will be combat-inert with no way to gain abilities.

### Character Creation Definitions

| Archetype | Def IDs | Factions | Genders | Body Set | Event Set |
|---|---|---|---|---|---|
| Scientist | 20, 21, 22, 23 | SGU + Praxis | M + F | BS_Human | 804 |
| Archeologist | 5, 6, 15, 16 | SGU + Praxis | M + F | BS_Human | 804 |
| Asgard | 9 | SGU only | **M only** | **BS_Asgard** | **1455** |
| Goa'uld | 10, 19 | Praxis only | M + F | **BS_Goauld** | 804 |
| Shol'va | 8, 18 | SGU only | M + F | BS_Jaffa | 804 |
| Jaffa | 7, 17 | Praxis only | M + F | BS_Jaffa | 804 |

### Notable Differences

**Asgard** is the most restricted and the most unique placeholder:
- SGU faction only, male only, unique body set (BS_Asgard)
- **Only** placeholder with a unique item event set (1455 vs 804 for all others),
  suggesting intended distinct equip/unequip animations for the Asgard body type
- Source: `python/common/Constants.py` line 109

**Shol'va / Jaffa** are the same race split by faction. Shol'va = rebel Jaffa
allied with Earth (SGU). Jaffa = Goa'uld-loyal (Praxis). Both use BS_Jaffa body sets.

**Jaffa** is the only placeholder with archetype-specific tutorial branching
(see Section 5).

---

## 5. Archetype-Specific Content

### 5.1 Tutorial Mission Branching

The Castle_CellBlock tutorial (Praxis starting area) contains the only
archetype-aware mission logic in the codebase. Three scripts branch on the
player's archetype enum value using a consistent pattern:

**Branching pattern** (found in Castle_CellBlock.py, Prisoner_329.py, Preparation.py, Aftermath.py):

```python
# Pseudocode of the deobfuscated branching logic:
archetype_value = entity.archetype  # EArchetype enum ordinal

if archetype_value == 8:    # ARCHETYPE_Jaffa
    # Jaffa-specific path
elif archetype_value < 5:   # ARCHETYPE_Any through ARCHETYPE_Archeologist
    # Human archetype path (Soldier, Commando, Scientist, Archeologist)
```

The enum ordinal values are:

| Ordinal | Archetype | Branch |
|---|---|---|
| 0 | Any | Human path (< 5) |
| 1 | Soldier | Human path (< 5) |
| 2 | Commando | Human path (< 5) |
| 3 | Scientist | Human path (< 5) |
| 4 | Archeologist | Human path (< 5) |
| 5 | Asgard | **Neither branch** |
| 6 | Goa'uld | **Neither branch** |
| 7 | Shol'va | **Neither branch** |
| 8 | Jaffa | Jaffa path (== 8) |

**Critical gap:** Asgard (5), Goa'uld (6), and Shol'va (7) fall through both
checks -- they are neither == 8 nor < 5. These archetypes would receive no
dialog or rewards from the branching tutorial missions.

**Confidence:** HIGH -- verified by reading the branching constants directly
from the Python source. The values 8 and 5 are hardcoded as comparison constants
in the `__init__` methods of Castle_CellBlock.py (lines 313-316),
Prisoner_329.py (lines 164-167), Preparation.py (lines 101-102),
and Aftermath.py (lines 187-190).

### 5.2 Per-Script Branching Details

#### Castle_CellBlock.py (Space Controller)

Two archetype branch points:

1. **n161_trigger_In** (Col. Marsh interaction, mission 641 pre-acceptance):
   - Jaffa (== 8): Display dialog 5022
   - Human (< 5): Display dialog 4001
   - Others: No dialog displayed

2. **n172_trigger_In** (Prisoner_329 region entry, mission 638 acceptance):
   - Jaffa (== 8): Add dialog set 5866 (interaction_set_map)
   - Human (< 5): Add dialog set 2794 (interaction_set_map)
   - Others: No dialog set added

#### Prisoner_329.py

**n144_trigger_In** (Cell door button interaction):
- Jaffa (== 8): Add dialog set 5865 -> choice 5021 -> complete mission 638
- Human (< 5): Add dialog set 2793 -> choice 2300 -> complete mission 638
- Others: No dialog, mission stuck

The Jaffa and Human paths converge at mission completion (638) but use different
dialog IDs: Jaffa gets dialog choice 5020, Human gets 2299.

#### Preparation.py

**n36_trigger_In** (Col. Marsh pet interaction, mission 641 step 3563):
- Jaffa (== 8): Display dialog 5023 -> choice advances mission
- Human (< 5): Display dialog 3999 -> choice advances mission
- Others: No dialog, mission stuck

#### Aftermath.py

**n7_trigger_In** (Wooden crate interaction, mission 687 step 2354):
- Jaffa (== 8): Receive items 3482, 2797 (2 items)
- Human (< 5): Display dialog 3942, receive items 3347, 3359, 3372, 3387, 3401, 3325 (6 items)
- Both paths: Advance mission 687, step 2355

This is the most significant gameplay branch -- Jaffa receive different (fewer)
equipment rewards than human archetypes, likely reflecting lore-appropriate gear
differences.

### 5.3 Item Event Sets

The only archetype-keyed data structure outside the database:

```python
# python/common/Constants.py, line 103
ARCHETYPE_ITEM_EVENT_SETS = {
    ARCHETYPE_Any:          804,
    ARCHETYPE_Soldier:      804,
    ARCHETYPE_Commando:     804,
    ARCHETYPE_Scientist:    804,
    ARCHETYPE_Archeologist: 804,
    ARCHETYPE_Asgard:       1455,  # <-- unique
    ARCHETYPE_Goauld:       804,
    ARCHETYPE_Sholva:       804,
    ARCHETYPE_Jaffa:        804,
}
```

Event set 1455 (Asgard) likely references different equip/unequip/reload animation
sequences for the Asgard body type. All other archetypes share event set 804.

**Confidence:** CONFIRMED -- direct code reference.

---

## 6. Faction x Archetype Matrix

Which archetypes are available for each faction, based on `resources.char_creation`:

| Archetype | SGU (Earth) | Praxis (Offworld) | Body Set |
|---|---|---|---|
| Soldier | Male + Female | Male + Female | BS_Human |
| Commando | Male + Female | Male + Female | BS_Human |
| Scientist | Male + Female | Male + Female | BS_Human |
| Archeologist | Male + Female | Male + Female | BS_Human |
| Asgard | Male only | -- | BS_Asgard |
| Goa'uld | -- | Male + Female | BS_Goauld |
| Shol'va | Male + Female | -- | BS_Jaffa |
| Jaffa | -- | Male + Female | BS_Jaffa |

**Faction-exclusive archetypes:**
- **SGU only:** Asgard, Shol'va
- **Praxis only:** Goa'uld, Jaffa
- **Both factions:** Soldier, Commando, Scientist, Archeologist

**Shol'va/Jaffa symmetry:** These are the same race (Jaffa) split by faction
allegiance. Shol'va = rebel Jaffa allied with Earth (SGU). Jaffa = Goa'uld-loyal
Jaffa (Praxis). Both use BS_Jaffa body sets.

**Gender restrictions:**
- Asgard: Male only (1 character definition). Lore-consistent -- Asgard in
  Stargate canon are a genderless species that appears uniformly similar.
- All others: Male and Female variants available.

**Total character definitions:** 23 (across 8 archetypes, 2 factions, 2 genders)

**Confidence:** CONFIRMED -- complete enumeration from `resources.char_creation` inserts.

---

## 7. Reconstruction Potential

What would it take to make each placeholder archetype playable?

### 7.1 Minimum Viable Archetype (any placeholder)

To bring a placeholder archetype from "shell" to "minimally playable":

1. **Differentiate base stats** -- Design unique stat distributions in
   `resources.archetypes`. Currently all 6 placeholders are Soldier clones.
   (Confidence: HIGH that this is all that is needed for stats)

2. **Create ability trees** -- Add rows to `resources.archetype_ability_tree`.
   Need ~80-85 abilities across 3 trees to match Soldier/Commando depth.
   Each ability also needs a corresponding entry in `resources.abilities`.
   (Confidence: HIGH -- the loading code in `Archetype.py` is generic)

3. **Add starting abilities** -- Add rows to `resources.char_creation_abilities`
   for each char_def_id of the archetype. Minimum 3 abilities (ranged, melee, heal)
   to match Soldier/Commando pattern.
   (Confidence: HIGH)

4. **Add trainer abilities** -- Add rows to `resources.trainer_abilities` for
   list_id=1 with the new archetype's abilities.
   (Confidence: HIGH -- `TrainerAbilityList.py` loads by archetype automatically)

5. **Fix tutorial branching** -- Ensure Castle_CellBlock scripts handle the
   archetype's enum ordinal. Currently Asgard/Goa'uld/Shol'va fall through.
   (Confidence: HIGH)

### 7.2 Per-Archetype Specific Needs

| Archetype | Unique Needs | Effort Estimate |
|---|---|---|
| **Scientist** | New ability theme (tech/healing?), no body set issues | Medium |
| **Archeologist** | New ability theme (relics/knowledge?), no body set issues | Medium |
| **Asgard** | Ability theme (psionic/tech), event set 1455 already exists, single-gender OK | Medium-High |
| **Goa'uld** | Ability theme (symbiote/host powers), unique body set exists | Medium-High |
| **Shol'va** | Same as Jaffa but SGU-aligned, tutorial already handles "human" path | Medium |
| **Jaffa** | Tutorial branching exists, needs ability theme (staff weapon/melee) | Medium |

### 7.3 Effort Breakdown

| Task | Scale | Notes |
|---|---|---|
| Stat design | Trivial | 6 rows x 11 columns in `resources.archetypes` |
| Ability definitions | **Large** | ~80-85 abilities per archetype, each needing effects, animations, balance |
| Ability tree layout | Small | Populate `archetype_ability_tree` once abilities exist |
| Starting abilities | Trivial | 3 rows per char_def_id in `char_creation_abilities` |
| Trainer list | Small | Mirror the ability tree into `trainer_abilities` |
| Tutorial fixes | Small | Fix branching in 4 Python scripts for 3 missing cases |
| Testing | Medium | Each archetype needs combat, leveling, trainer flow testing |

**The bottleneck is ability creation.** The framework (loading, training, trees,
combat) is archetype-agnostic and works for any archetype with populated data.
The missing content is ~500 unique ability definitions across 6 archetypes.

**Confidence:** MEDIUM -- framework generality is verified in code, but ability
design effort is an estimate based on Soldier/Commando depth.

### 7.4 Racial Paradigms

The `resources.racial_paradigm` table defines 5 paradigms:

| ID | Name | Likely Archetype Mapping |
|---|---|---|
| 1 | Common | All archetypes |
| 2 | Human | Soldier, Commando, Scientist, Archeologist |
| 3 | Goa'uld | Goa'uld |
| 4 | Asgard | Asgard |
| 5 | Ancient | Unknown -- no archetype maps to Ancient |

Racial paradigms gate access to disciplines (the crafting/applied science tree).
The `resources.disciplines` table references `racial_paradigm_id` and
`racial_paradigm_level`, meaning some disciplines are race-locked. This system
works independently of archetypes and is already populated with data for all
5 paradigms.

**Confidence:** HIGH -- paradigms verified in DB, discipline references verified
via grep of `resources.disciplines` inserts.

---

## 8. Key Findings

### 8.1 The Two-Archetype Reality

Despite 9 enum values and 8 named archetypes (plus "Any"), only **Soldier and
Commando** have functional content. The remaining 6 archetypes are database
shells -- they have rows in `archetypes`, `char_creation`, and the enum, but
zero ability trees, zero starting abilities, and zero trainer content.

### 8.2 All Stats Are Identical (Except Commando)

The Commando is the only archetype with differentiated stats. The other 8 entries
(including "Any") share identical stat blocks. HP, Focus, and per-level gains
are the same across **all** 9 entries with no exception.

### 8.3 No Prerequisite System

All `prerequisite_abilities` arrays in `archetype_ability_tree` are empty (`{}`).
All `level` values are `1`. The ability tree system supports prerequisites and
level gating in its schema and loading code, but no content uses these features.

### 8.4 Tutorial Is The Only Archetype-Branching Content

The Castle_CellBlock tutorial scripts are the only place in 164 Python files
where gameplay branches on archetype. The branching covers only two cases:
Jaffa (ordinal 8) and "Human" (ordinal < 5). Three archetypes (Asgard, Goa'uld,
Shol'va) are not handled and would experience broken tutorial progression.

### 8.5 The Framework Is Ready

The archetype system itself is well-designed and fully generic:
- `Archetype.py` loads any archetype with populated DB rows
- `TrainerAbilityList.py` filters by archetype automatically
- `CharacterCreation.py` handles any char_def_id with abilities
- `SGWPlayer.py` initializes stats from the archetype definition
- `SGWBeing.py` sends ability trees to the client generically

**The code does not need changes to support new archetypes.** The gap is
purely data: ability definitions, stat tuning, and tutorial script fixes.

### 8.6 Open Questions

| Question | Confidence | Notes |
|---|---|---|
| Were the 6 placeholder archetypes ever functional in the retail client? | LOW | No client-side data has been analyzed for ability definitions |
| What ability themes were intended for each archetype? | ASSUMPTION | Scientist = tech/healing, Archeologist = relics, Asgard = psionic, Goa'uld = symbiote powers, Jaffa/Shol'va = staff weapons |
| Do client .pak files contain ability data for placeholder archetypes? | LOW | The `data/cache/*.pak` files have not been fully decoded |
| What is event set 1455 (Asgard)? | MEDIUM | Presumed to be equip/unequip animation sequences for the Asgard body type |
| Why do Jaffa get 2 items and Humans get 6 in Aftermath.py? | MEDIUM | Likely different equipment load-outs reflecting lore (Jaffa use staff weapons, humans use Earth gear) |
