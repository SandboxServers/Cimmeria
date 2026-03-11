# Character Visual Components

> **Last updated**: 2026-03-08
> **RE Status**: Verified via database schema, Python scripts, and Ghidra
> **Sources**: `db/resources/Archetypes/Seed/`, `python/cell/SGWPlayer.py`, `python/cell/Inventory.py`, `python/cell/Bag.py`

---

## Overview

Each character's visual appearance is defined by a **bodyset** (skeleton/mesh group name) and a list of **components** (attachable mesh parts). Components come from two sources: the character's body (created at character creation) and equipped items (loaded from inventory). Both are merged into a single list before being sent to the client via `BeingAppearance`.

## Database Schema

### Character Body Components

Stored in `sgw_characters.components` as a PostgreSQL `character varying(200)[]` array.

```sql
-- From sgw_characters table
components character varying(200)[] DEFAULT '{}'::character varying[]
```

Set during character creation from the visual groups associated with the character definition.

### Character Body Set

Stored in `sgw_characters.bodyset` as `character varying(200)`.

```sql
-- From sgw_characters table
bodyset character varying(200) NOT NULL
```

Determined by the character definition (chardef) — maps to a `.BS` file in game data.

### Item Visual Components

Stored in `resources.items.visual_component` as `character varying(200)`.

```sql
-- From resources.items table
visual_component character varying(200)
```

NULL for non-visual items (consumables, quest items, etc.). Non-NULL for equipment that changes appearance.

## Character Creation Flow

### 1. Visual Group Resolution

Each character definition (chardef) has associated visual groups in `char_creation_visgroups`:

```sql
-- char_creation_visgroups columns:
-- chardef_id, visgroup_id, visual_component, item_id, choice_id

-- Body components have item_id IS NULL:
INSERT INTO char_creation_visgroups VALUES (1, 1, 'BS_HM_Torso_00', NULL, NULL);
INSERT INTO char_creation_visgroups VALUES (1, 2, 'BS_HM_Legs_00', NULL, NULL);
INSERT INTO char_creation_visgroups VALUES (1, 4, 'BS_HM_Boots_00', NULL, NULL);

-- Item components have item_id set:
INSERT INTO char_creation_visgroups VALUES (1, 5, 'AR_Global.Prisoner_Torso', 3440, NULL);
INSERT INTO char_creation_visgroups VALUES (1, 6, 'AR_Global.Prisoner_Legs', 3437, NULL);
INSERT INTO char_creation_visgroups VALUES (1, 7, 'AR_HM_BB1_BH100', 3438, NULL);

-- Player-choice components (e.g., head) have choice_id set:
INSERT INTO char_creation_choices VALUES (1, 3, 'BS_HM_Head_00', NULL);
INSERT INTO char_creation_choices VALUES (1, 3, 'BS_HM_Head_01', NULL);
-- etc.
```

### 2. Separation at Creation Time

During character creation (`character.rs` / `Account.py`):

1. Query all visual groups for the chardef
2. **Body components** (`item_id IS NULL`): stored in `sgw_characters.components`
3. **Item components** (`item_id IS NOT NULL`): inserted into `sgw_inventory` as starter equipment

This separation matters because:
- Body components are permanent (never change)
- Item components change when equipment changes
- The client needs the merged list for rendering

### 3. PostgreSQL Array Format

Python reads the components array from PostgreSQL as a string literal:

```python
# SGWPlayer.py line 170
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
  ├── Query sgw_characters → bodyset, components (body only)
  ├── Query equipped items' visual_component from resources.items
  │     WHERE container_id IN (3..14) AND slot_id = 0
  │     OR container_id = 3 AND slot_id = bandolier_slot
  └── components.extend(item_visuals)  # merge in-place

build_map_loaded()
  └── append_entity_method(BEING_APPEARANCE, [bodyset, components])
```

Both produce the same merged component list.

## Bodyset Mapping

### Chardef → Bodyset Table

| CharDefId | Alignment | Archetype | Gender | Bodyset |
|-----------|-----------|-----------|--------|---------|
| 1 | Praxis | Soldier | Male | `BS_HumanMale.BS_HumanMale` |
| 2 | Praxis | Scientist | Male | `BS_HumanMale.BS_HumanMale` |
| 3 | Praxis | Commando | Male | `BS_HumanMale.BS_HumanMale` |
| 4 | Praxis | Archaeologist | Male | `BS_HumanMale.BS_HumanMale` |
| 5 | Praxis | Soldier | Female | `BS_HumanFemale.BS_HumanFemale` |
| 6 | Praxis | Scientist | Female | `BS_HumanFemale.BS_HumanFemale` |
| 7 | Praxis | Commando | Female | `BS_HumanFemale.BS_HumanFemale` |
| 8 | Praxis | Archaeologist | Female | `BS_HumanFemale.BS_HumanFemale` |
| 9 | System Lords | Soldier | Male | `BS_HumanMale.BS_HumanMale` |
| 10 | System Lords | Scientist | Male | `BS_HumanMale.BS_HumanMale` |
| 11 | System Lords | Goa'uld | Male | `BS_HumanMale.BS_HumanMale` |
| 12 | System Lords | Jaffa | Male | `BS_Jaffa.BS_Jaffa` |
| 13 | System Lords | Soldier | Female | `BS_HumanFemale.BS_HumanFemale` |
| 14 | System Lords | Scientist | Female | `BS_HumanFemale.BS_HumanFemale` |
| 15 | System Lords | Goa'uld | Female | `BS_HumanFemale.BS_HumanFemale` |
| 16 | System Lords | Jaffa | Female | `BS_Jaffa.BS_Jaffa` |
| 17 | Praxis | Asgard | N/A | `BS_Asgard.BS_Asgard` |
| 18-23 | (additional variants) | | | |

### Bodyset Naming Convention

Format: `BS_<Race><Gender>.<same>` — the dotted name references a `.BS` file in the game data that defines the skeleton and available attachment points.

- `BS_HumanMale` — human male skeleton
- `BS_HumanFemale` — human female skeleton
- `BS_Jaffa` — Jaffa skeleton (larger frame)
- `BS_Asgard` — Asgard skeleton (small frame)

## Equipment Bag IDs

Items in equipment bags contribute visual components:

| Container ID | Bag Name | Visual? |
|-------------|----------|---------|
| 3 | Primary Weapon | Yes (bandolier slot matters) |
| 4 | Head | Yes |
| 5 | Chest | Yes |
| 6 | Legs | Yes |
| 7 | Feet | Yes |
| 8 | Hands | Yes |
| 9 | Shoulders | Yes |
| 10 | Back | Yes |
| 11 | Accessory 1 | Sometimes |
| 12 | Accessory 2 | Sometimes |
| 13 | Accessory 3 | Sometimes |
| 14 | Secondary Weapon | Yes |

The query joins `sgw_inventory.container_id IN (3..14)` with `slot_id = 0` (first slot in each bag), plus `container_id = 3 AND slot_id = bandolier_slot` for the active weapon.

## Component Name Format

Components use a `Category.Name` dotted format that maps to asset paths:

| Pattern | Description | Example |
|---------|-------------|---------|
| `BS_HM_*` | Human Male body part | `BS_HM_Torso_00` |
| `BS_HF_*` | Human Female body part | `BS_HF_Head_01` |
| `BS_JM_*` | Jaffa Male body part | `BS_JM_Torso_00` |
| `AR_Global.*` | Race/gender-agnostic armor | `AR_Global.Prisoner_Torso` |
| `AR_H_*.*` | Human-specific armor | `AR_H_Ballistic00.AR_HM_BB1_BH100` |
| `AR_J_*.*` | Jaffa-specific armor | (similar pattern) |

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
