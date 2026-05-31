---
name: project-crafting-system
description: Crafting system deep-dive findings: DB schema, wire formats, Python logic, item flags, expertise formulas, and implementation phases for issue #53
metadata:
  type: project
---

Full deep-dive completed 2026-05-27 for issue #53. All evidence is from Python source + dispatch table + DB SQL, not guessed.

**Why:** Crafting was never ported from Python to Rust. A 59-line stub exists at `crates/services/src/cell/cell_methods/player/crafting.rs`. This is the largest unported Python system (~533 lines in `deprecated/python/cell/Crafter.py`).

**How to apply:** Use this as the authoritative reference before implementing issue #53 work.

## Inbound cell methods (client → server)

| Const | Index | Args |
|---|---|---|
| `SPEND_APPLIED_SCIENCE_POINTS` | 95 | `disciplineId: i32` |
| `CRAFT` | 96 | `blueprintId: i32, itemIds: ARRAY<i32>, quantity: i32` |
| `RESEARCH` | 97 | `itemId: i32, kickerIds: ARRAY<i32>` |
| `REVERSE_ENGINEER` | 98 | `itemId: i32` |
| `ALLOYING` | 99 | `blueprintId: i32, currentTierItemId: i32, lowerTierItems: ARRAY<i32>` |
| `RESPEC_CRAFTING` | 100 | none |

**Critical bug:** Index 95 (`SPEND_APPLIED_SCIENCE_POINTS`) is NOT in the current dispatch range `CRAFT..=RESPEC_CRAFTING` (96–100). It will silently drop.

## Outbound methods (server → client)

Indices from `docs/protocol/client-method-dispatch-table.md`:

| Constant name (to add) | Index | Wire payload |
|---|---|---|
| `ON_CRAFTING_RESPEC_PROMPT` | 112 | `i32 CostToRespec` |
| `ON_UPDATE_DISCIPLINE` | 136 | `i32 disciplineSeqId, i32 expertise` |
| `ON_DISCIPLINE_RESPEC` | 137 | none |
| `ON_UPDATE_RACIAL_PARADIGM_LEVEL` | 138 | `i32 racialParadigmId, i8 level` |
| `ON_UPDATE_KNOWN_CRAFTS` | 139 | `ARRAY<i32> blueprint_ids` — **already in method_idx** |
| `ON_UPDATE_CRAFTING_OPTIONS` | 140 | `CraftingOptions` — **type unverified; Ghidra needed** |

Applied Science Points use `onEntityProperty(type=2, value)` [idx 7] — not a dedicated message.
Craft induction timer uses `onTimerUpdate(blueprintId, type=16, entityId, 0, 3.0, completeTime)` [idx 12].

## DB schema

**`sgw_player` already has crafting state columns** — no new columns needed:
- `discipline_ids integer[]`
- `racial_paradigm_levels integer[]`
- `applied_science_points integer`
- `blueprint_ids integer[]`

**Missing:** expertise-per-discipline storage. Need new table:
```sql
CREATE TABLE sgw_player_discipline_expertise (
    player_id     integer NOT NULL,
    discipline_id integer NOT NULL,
    expertise     integer NOT NULL DEFAULT 1 CHECK (expertise >= 0 AND expertise <= 100),
    PRIMARY KEY (player_id, discipline_id),
    FOREIGN KEY (player_id) REFERENCES sgw_player(player_id) ON DELETE CASCADE
);
```

Resources tables (read-only): `resources.disciplines`, `resources.racial_paradigm`, `resources.blueprints`, `resources.blueprints_components`, `resources.applied_science` (4 branches: 1=Biomedical, 2=Materials, 3=Power Systems, 4=Electronic Engineering).

## Item flag bits (from `deprecated/python/Atrea/enums.py`)

```
ITEM_FLAG_Kicker               = 32     (0x0020) — research kicker
ITEM_FLAG_Craft_Research       = 128    (0x0080) — researchable
ITEM_FLAG_Craft_RevEng         = 256    (0x0100) — reverse-engineerable
ITEM_FLAG_ElementaryComponent  = 32768  (0x8000) — alloying elementary component
```

These live in `resources.items.flags` column. Query as bitwise AND.

## Expertise formulas

- Craft/alloy completion: `gainExpertise(+1)`, capped at 100
- Research success: `gainExpertise(+5)` if roll succeeds
- Research roll: `chance = (100 - expertise) + len(kickers) * 5` percent
- Prerequisite threshold: expertise >= 50 required in all prerequisites before ASP spend
- DB UPDATE: use `SET expertise = LEAST(expertise + $1, 100)` to avoid read-modify-write race

## Alloying elementary count table

```
Fantastic (5000) → 1 elementary component
Great (4000)     → 2
Good (3000)      → 3
Normal (2000)    → 5
Poor (1000)      → 10
```

Source: `deprecated/python/common/Constants.py::ALLOYING_ELEMENTARY_COUNTS`

## Discipline tree shape

DAG (not linear). `required_discipline_ids integer[]` on each discipline. 0 entries = root node. Multiple parents allowed (e.g., discipline 38 requires {33, 34, 35}). All prerequisites must have expertise >= 50.

## Key risk: consume-before-grant TOCTOU

Craft/alloy/reveng all consume items BEFORE the 3-second timer fires. A crash between consume and grant permanently loses items. Mitigation: `pending_crafts` table row inserted atomically with consume, cleared atomically with grant.

## Confidence gaps (need Ghidra)

1. `CraftingOptions` struct layout for `onUpdateCraftingOptions` [140]
2. Respec confirm/cancel flow and cost formula
3. `onEntityProperty` sends delta vs. new total for ASP

Gates only Phase 4–5; Phases 1–3 are fully evidenced.
