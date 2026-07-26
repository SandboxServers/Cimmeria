---
title: "Character Visual Components"
type: reference
audience: engineers
last_updated: 2026-07-25
---

# Character Visual Components

> **Last updated**: 2026-07-25
> **RE Status**: Verified via database schema, Python scripts, and Ghidra
> **Sources**: `db/resources/Archetypes/`, `db/resources/Items/`, `db/sgw/Players/Tables/sgw_player.sql`, `deprecated/python/cell/SGWPlayer.py`, `deprecated/python/cell/Inventory.py`, `deprecated/python/cell/Bag.py`

---

## Overview

Each character's visual appearance is defined by a **bodyset** (skeleton/mesh group name) and a list of **components** (attachable mesh parts). Components come from two sources: the character's body (created at character creation) and equipped items (loaded from inventory). Both are merged into a single list before being sent to the client via `BeingAppearance`.

## Database Schema

The player table is **`sgw_player`** (`db/sgw/Players/Tables/sgw_player.sql`). Earlier revisions of this doc called it `sgw_characters`; no such table exists.

### Character Body Components

Stored in `sgw_player.components` as a PostgreSQL array (`sgw_player.sql:27`):

```sql
components character varying(200)[] DEFAULT '{}'::character varying[] NOT NULL
```

Set during character creation from the visual groups associated with the character definition.

### Character Body Set

Stored in `sgw_player.bodyset` (`sgw_player.sql:16`) — note this is **64** chars, not 200:

```sql
bodyset character varying(64) DEFAULT NULL::character varying NOT NULL
```

Determined by the character definition (chardef) — maps to a `.BS` file in game data.

### Item Visual Components

Stored in `resources.items.visual_component` (`db/resources/Items/Tables/items.sql:20`):

```sql
visual_component character varying(255) DEFAULT NULL::character varying
```

NULL for non-visual items (consumables, quest items, etc.). Non-NULL for equipment that changes appearance.

## Character Creation Flow

### 1. Visual Group Resolution

This is a **two-table** model, and the split matters. `char_creation_visgroups` holds only the named *slots* for a chardef; the actual component strings and item bindings live in `char_creation_choices`, one or more rows per slot.

```sql
-- resources.char_creation_visgroups — the slots (one row per slot per chardef)
CREATE TABLE char_creation_visgroups (
    vis_group_id integer NOT NULL,
    char_def_id  integer NOT NULL,
    text         character varying(255) NOT NULL,   -- slot name: 'Torso', 'Head', ...
    vis_type     "EVisualGroupType" NOT NULL        -- 'VIS_Forced' | 'VIS_Optional'
);

INSERT INTO char_creation_visgroups (vis_group_id, char_def_id, text, vis_type)
    VALUES (1, 1, 'Torso',     'VIS_Forced');
INSERT INTO char_creation_visgroups (vis_group_id, char_def_id, text, vis_type)
    VALUES (2, 1, 'TorsoBase', 'VIS_Forced');
INSERT INTO char_creation_visgroups (vis_group_id, char_def_id, text, vis_type)
    VALUES (5, 1, 'Head',      'VIS_Optional');

-- resources.char_creation_choices — the components (one or more rows per slot)
CREATE TABLE char_creation_choices (
    choice_id       integer NOT NULL,
    vis_group_id    integer NOT NULL,
    component       character varying(255) NOT NULL,
    item_id         integer,                        -- NULL = body component
    item_bound      boolean DEFAULT false NOT NULL,
    item_durability integer DEFAULT (-1)
);

-- Item components have item_id set:
INSERT INTO char_creation_choices VALUES (1, 1, 'AR_Global.Prisoner_Torso',         3440, false, -1);
INSERT INTO char_creation_choices VALUES (3, 3, 'AR_Global.Prisoner_Legs',          3437, false, -1);
INSERT INTO char_creation_choices VALUES (17, 8, 'AR_H_Ballistic00.AR_HM_BB1_BH100', 3438, false, -1);

-- Body components have item_id IS NULL:
INSERT INTO char_creation_choices VALUES (2, 2, 'BS_HumanMale.BS_HM_Torso_00', NULL, false, -1);
INSERT INTO char_creation_choices VALUES (4, 4, 'BS_HumanMale.BS_HM_Legs_00',  NULL, false, -1);
```

Two things to note. The body/item discriminator is `char_creation_choices.item_id IS NULL` — it is **not** a column on `char_creation_visgroups`, which has no `item_id`, no `visual_component`, and no `choice_id`. And component strings are stored **fully qualified** (`BS_HumanMale.BS_HM_Torso_00`, `AR_H_Ballistic00.AR_HM_BB1_BH100`), not as the bare leaf names used loosely elsewhere in this doc.

### 2. Separation at Creation Time

During character creation (`character.rs` / `Account.py`):

1. Query all visual groups for the chardef
2. **Body components** (`char_creation_choices.item_id IS NULL`): stored in `sgw_player.components`
3. **Item components** (`char_creation_choices.item_id IS NOT NULL`): inserted into `sgw_inventory` as starter equipment

This separation matters because:
- Body components are permanent (never change)
- Item components change when equipment changes
- The client needs the merged list for rendering

### 3. PostgreSQL Array Format

Python reads the components array from PostgreSQL as a string literal:

```python
# deprecated/python/cell/SGWPlayer.py:170
self.components = [comp for comp in player['components'][1:-1].split(',') if comp]
```

This strips the PostgreSQL `{comp1,comp2,...}` array braces and splits on commas.

## World Entry Visual Assembly

### Python Reference Flow

```
SGWPlayer.onPlayerLoaded()
  ├── self.bodySet = player['bodyset']           # "BS_HumanMale.BS_HumanMale"
  ├── self.components = parse(player['components']) # [BS_HM_Torso_00, BS_HM_Legs_00, ...]
  └── self.inventory.loadItems()
        └── for each equipped item:
              loadItem() → addDbItem() → onSlotUpdate()
                └── if item has visual_component:
                      self.components.append(visual_component)  # AR_Global.Prisoner_Torso, etc.

# Later, in mapLoaded():
self.client.BeingAppearance(self.bodySet, self.components)
# components now has BOTH body and item visuals
```

### Rust Flow

```
query_player_load_data()
  ├── Query sgw_player → bodyset, components (body only)
  ├── Query equipped items' visual_component from resources.items
  │     WHERE container_id = ANY([4..14] + [3])
  │       AND ((container_id <> 3 AND slot_id = 0)
  │            OR (container_id = 3 AND slot_id = bandolier_slot))
  └── components.extend(item_visuals)  # merge in-place

build_map_loaded()
  └── append_entity_method(BEING_APPEARANCE, [bodyset, components])
```

Both produce the same merged component list.

## Bodyset Mapping

### Chardef → Bodyset Table

Complete, from `resources.char_creation` (`db/resources/Archetypes/Seed/char_creation.sql`). All 23 chardefs, with the column actually named `body_set`:

| CharDefId | Alignment | Archetype | Gender | Bodyset | Starting world |
|-----------|-----------|-----------|--------|---------|----------------|
| 1 | Praxis | Soldier | Male | `BS_HumanMale.BS_HumanMale` | Castle_CellBlock |
| 2 | SGU | Soldier | Male | `BS_HumanMale.BS_HumanMale` | SGC_W1 |
| 3 | Praxis | Commando | Male | `BS_HumanMale.BS_HumanMale` | Castle_CellBlock |
| 4 | SGU | Commando | Male | `BS_HumanMale.BS_HumanMale` | SGC_W1 |
| 5 | Praxis | Archeologist | Male | `BS_HumanMale.BS_HumanMale` | Castle_CellBlock |
| 6 | SGU | Archeologist | Male | `BS_HumanMale.BS_HumanMale` | SGC_W1 |
| 7 | Praxis | Jaffa | Male | `BS_JaffaMale.BS_JaffaMale` | Castle_CellBlock |
| 8 | SGU | Sholva | Male | `BS_JaffaMale.BS_JaffaMale` | SGC_W1 |
| 9 | SGU | Asgard | Male | `BS_Asgard.BS_Asgard` | SGC_W1 |
| 10 | Praxis | Goauld | Male | `BS_GoauldMale.BS_GoauldMale` | Castle_CellBlock |
| 11 | Praxis | Soldier | Female | `BS_HumanFemale.BS_HumanFemale` | Castle_CellBlock |
| 12 | SGU | Soldier | Female | `BS_HumanFemale.BS_HumanFemale` | SGC_W1 |
| 13 | Praxis | Commando | Female | `BS_HumanFemale.BS_HumanFemale` | Castle_CellBlock |
| 14 | SGU | Commando | Female | `BS_HumanFemale.BS_HumanFemale` | SGC_W1 |
| 15 | Praxis | Archeologist | Female | `BS_HumanFemale.BS_HumanFemale` | Castle_CellBlock |
| 16 | SGU | Archeologist | Female | `BS_HumanFemale.BS_HumanFemale` | SGC_W1 |
| 17 | Praxis | Jaffa | Female | `BS_JaffaFemale.BS_JaffaFemale` | Castle_CellBlock |
| 18 | SGU | Sholva | Female | `BS_JaffaFemale.BS_JaffaFemale` | SGC_W1 |
| 19 | Praxis | Goauld | Female | `BS_GoauldFemale.BS_GoauldFemale` | Castle_CellBlock |
| 20 | Praxis | Scientist | Male | `BS_HumanMale.BS_HumanMale` | Castle_CellBlock |
| 21 | SGU | Scientist | Male | `BS_HumanMale.BS_HumanMale` | SGC_W1 |
| 22 | Praxis | Scientist | Female | `BS_HumanFemale.BS_HumanFemale` | Castle_CellBlock |
| 23 | SGU | Scientist | Female | `BS_HumanFemale.BS_HumanFemale` | SGC_W1 |

The two alignments are `ALIGNMENT_Praxis` and `ALIGNMENT_SGU` — there is no "System Lords" alignment in the data. The ordering is not grouped by alignment: 1–10 are male, 11–19 female, then 20–23 append the Scientist archetype for both. Archetype spellings are the enum's own: `ARCHETYPE_Archeologist` (one `a`), `ARCHETYPE_Goauld`, `ARCHETYPE_Sholva`.

### Bodyset Naming Convention

Format: `BS_<Race><Gender>.<same>` — the dotted name references a `.BS` file in the game data that defines the skeleton and available attachment points.

- `BS_HumanMale` / `BS_HumanFemale` — human skeletons
- `BS_JaffaMale` / `BS_JaffaFemale` — Jaffa skeletons (larger frame); used by both the Praxis `Jaffa` and SGU `Sholva` archetypes
- `BS_GoauldMale` / `BS_GoauldFemale` — Goa'uld skeletons
- `BS_Asgard` — Asgard skeleton (small frame); the only bodyset with no gender split

There is no bare `BS_Jaffa.BS_Jaffa` bodyset — Jaffa are gender-split like the other races.

## Equipment Bag IDs

Items in equipment bags contribute visual components:

Names are from `resources.containers` (`db/resources/Items/Seed/containers.sql`); `is_equipped = true` is exactly the 3–14 range.

| Container ID | Name | Notes |
|-------------|------|-------|
| 3 | `BANDOLIER` | Weapon slots — the active `bandolier_slot` selects which one is visible |
| 4 | `HEAD` | |
| 5 | `FACE` | |
| 6 | `NECK` | |
| 7 | `CHEST` | |
| 8 | `HANDS` | |
| 9 | `WAIST` | |
| 10 | `BACK` | |
| 11 | `LEGS` | |
| 12 | `FEET` | |
| 13 | `ARTIFACT1` | |
| 14 | `ARTIFACT2` | |

Containers 1–2 (`MAIN`, `MISSION`) and 15–20 (`CRAFTING`, `BUYBACK`, `BANK`, `AUCTION`, `TEAMBANK`, `COMMANDBANK`) are `is_equipped = false` and contribute no visuals.

The Rust constants mirror this exactly: `CONTAINER_BANDOLIER = 3` and `EQUIPMENT_CONTAINERS = &[4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14]` (`crates/services/src/base/world_entry/methods/player_load/core/mod.rs:30-31`). The visuals query (`crates/services/src/base/character/mod.rs:204-213`) takes `slot_id = 0` from every non-bandolier equipment container, plus `container_id = 3 AND slot_id = bandolier_slot` for the active weapon, and filters to `visual_component IS NOT NULL`.

## Component Name Format

Components use a `Package.Name` dotted format that maps to asset paths. The package prefix (left of the dot) is what distinguishes the categories; the leaf name often carries its own abbreviated race/gender tag (`BS_HM_*` for Human Male, and so on), but the leaf alone is not a valid component string.

These are the packages actually present in `char_creation_choices`, with their row counts:

| Package prefix | Description | Rows | Example |
|---------|-------------|-----:|---------|
| `BS_HumanMale` / `BS_HumanFemale` | Human body parts | 44 / 34 | `BS_HumanMale.BS_HM_Torso_00` |
| `BS_JaffaMale` / `BS_JaffaFemale` | Jaffa body parts | 44 / 34 | `BS_JaffaMale.BS_JM_Torso_00` |
| `BS_GoauldMale` / `BS_GoauldFemale` | Goa'uld body parts | 44 / 34 | |
| `BS_Asgard` | Asgard body parts (no gender split) | 24 | |
| `ACC_*` | Accessories, per race/gender | 61 total | `ACC_HumanMale.*` |
| `AR_Global` | Race/gender-agnostic armor | 2 | `AR_Global.Prisoner_Torso` |
| `AR_H_*` | Human-specific armor | 4 | `AR_H_Ballistic00.AR_HM_BB1_BH100` |
| `AR_J_*` / `AR_G_*` | Jaffa- / Goa'uld-specific armor | 3 / 3 | `AR_J_Standard.*` |

## Inventory Visual Update Flow

When equipment changes after world entry:

```
Python: onSlotUpdate(bag, slotId, oldItem, newItem)
  ├── Remove old item's visual_component from self.components (if any)
  ├── Add new item's visual_component to self.components (if any)
  └── Set visualsDirty = True

flushUpdates() [called after all inventory changes]
  └── if visualsDirty:
        onVisualsUpdated()
          └── self.client.BeingAppearance(self.bodySet, self.components)
```

This sends a fresh BeingAppearance with the updated component list whenever equipment changes.

## Related Documents

- [Client Visual System](client-visual-system.md) — Client-side handling of BeingAppearance
- [Entity Type Catalog](entity-type-catalog.md) — Entity class definitions
- [CME Framework](cme-framework.md) — Entity scripting framework
