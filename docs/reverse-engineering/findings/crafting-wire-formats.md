# Crafting System Wire Formats

> **Date**: 2026-03-01
> **Phase**: 3 — Missing Systems RE
> **Confidence**: HIGH (derived from `.def` files + `alias.xml` + universal RPC dispatcher architecture)
> **Sources**: `SGWPlayer.def`, `alias.xml`, Ghidra decompilation of universal RPC dispatcher (`0x00c6fc40`)

---

## Overview

The crafting system encompasses four distinct activities: **crafting** (combining materials into items), **research** (analyzing items for patterns), **reverse engineering** (breaking items down), and **alloying** (tiered item upgrades). It also includes a discipline/expertise progression system and applied science point spending.

Crafting methods are defined directly on `SGWPlayer.def` — there is no separate crafting interface `.def` file.

---

## Client → Server Messages

### Cell Methods (Exposed)

#### `craft` — Craft an Item

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `aCraftId` | `INT32` | 4B | Craft recipe ID |
| `aItems` | `ARRAY<ItemID>` | 4B count + N×4B | Input item instance IDs |
| `aQuantity` | `INT32` | 4B | Number to craft |

**Wire size**: 1B header + 12B + 4B per item

**Note**: `ItemID` is aliased to `INT32` in `alias.xml`.

#### `research` — Research an Item

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `aItemId` | `ItemID` (INT32) | 4B | Item to research |
| `aKickers` | `ARRAY<ItemID>` | 4B count + N×4B | Kicker items (boost research quality) |

**Wire size**: 1B header + 8B + 4B per kicker

#### `reverseEngineer` — Break Down an Item

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `aItemId` | `ItemID` (INT32) | 4B | Item to reverse engineer |

**Total wire size**: 1B header + 4B = **5 bytes**

#### `alloying` — Tier Upgrade

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `aCraftId` | `INT32` | 4B | Alloy recipe ID |
| `aCurrentTierItemId` | `ItemID` (INT32) | 4B | Current tier item |
| `aLowerTierItems` | `ARRAY<ItemID>` | 4B count + N×4B | Lower tier ingredient items |

**Wire size**: 1B header + 12B + 4B per ingredient

#### `spendAppliedSciencePoints` — Spend Science Points

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `aDisciplineSeqId` | `INT32` | 4B | Discipline to invest in |

**Total wire size**: 1B header + 4B = **5 bytes**

#### `respecCrafting` — Respec Crafting Disciplines

*(No arguments)*

**Total wire size**: 1B header = **1 byte**

---

## Server → Client Messages

### `onUpdateDiscipline` — Discipline Progress Update

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `aDisciplineSeqId` | `INT32` | 4B | Discipline ID |
| `aExpertise` | `INT32` | 4B | Current expertise level |

**Total wire size**: 1B header + 8B = **9 bytes**

### `onDisciplineRespec` — Discipline Respec Complete

*(No arguments)*

**Total wire size**: 1B header = **1 byte**

### `onUpdateKnownCrafts` — Known Recipe List

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `aCraftList` | `ARRAY<INT32>` | 4B count + N×4B | Full list of known craft IDs |

**Wire size**: 1B header + 4B + 4B per craft

### `onUpdateCraftingOptions` — Available Crafting Activities

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `aOptions` | `CraftingOptions` | FIXED_DICT | Available items and entities per activity |

**`CraftingOptions` FIXED_DICT layout**:

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `crafting` | `CraftingInfo` | FIXED_DICT |
| `research` | `CraftingInfo` | FIXED_DICT |
| `reverseEngineering` | `CraftingInfo` | FIXED_DICT |
| `alloying` | `CraftingInfo` | FIXED_DICT |

**`CraftingInfo` FIXED_DICT layout**:

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `items` | `ARRAY<INT32>` | 4B count + N×4B — eligible item IDs |
| `entities` | `ARRAY<INT32>` | 4B count + N×4B — eligible entity IDs |

**Minimum wire size**: 1B header + 32B (4 activities × 2 empty arrays × 4B count)

### `onCraftingRespecPrompt` — Respec Cost Dialog

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `CostToRespec` | `INT32` | 4B | Naquadah cost |

**Total wire size**: 1B header + 4B = **5 bytes**

---

## Related Crafting Properties (SGWPlayer.def)

| Property | Type | Flags | Notes |
|----------|------|-------|-------|
| `craftingEntityFlags` | `INT32` | `CELL_PRIVATE` | Crafting entity state flags |
| `craftingOptions` | `PYTHON` | `CELL_PRIVATE` | Available crafting options |
| `craftTimer` | `INT32` | `CELL_PRIVATE` | Active craft timer |
| `craftQueue` | `PYTHON` | `CELL_PRIVATE` | Queued craft operations |
| `numRespecCrafting` | `INT32` | `CELL_PRIVATE` | Number of crafting respecs used |

All crafting properties are `CELL_PRIVATE` — none synced to client via property updates.

---

## Server-Only Methods (Not on Wire)

| Method | Category | Args | Notes |
|--------|----------|------|-------|
| `gainExpertise` | Cell | `INT32 expertise`, `INT32 disciplineSeqId`, `INT8 withTransaction` | Grant expertise points |
| `gainAppliedSciencePoints` | Cell | `INT32 points`, `INT8 withTransaction` | Grant applied science points |
| `updateCraftingFlags` | Cell | `INT32 entityId`, `INT8 flags` | Update crafting entity flags |

---

## Implementation Notes

1. **Four crafting activities**: Craft, Research, Reverse Engineer, Alloy — each with distinct input patterns.
2. **Craft uses item instance IDs** (not definition IDs) — the server resolves actual items from inventory.
3. **Research "kickers"** are optional bonus items that improve research outcomes.
4. **Alloying** requires a current-tier item plus lower-tier ingredients for upgrade.
5. **Discipline system**: Applied science points are spent into discipline sequences. `onUpdateDiscipline` pushes current expertise levels to the client.
6. **CraftingOptions** is a complex FIXED_DICT with nested arrays — it tells the client which items/entities can be used for each activity type.
7. **Respec flow**: Client sends `respecCrafting` → server shows cost via `onCraftingRespecPrompt` → client confirms → server processes → sends `onDisciplineRespec`.
