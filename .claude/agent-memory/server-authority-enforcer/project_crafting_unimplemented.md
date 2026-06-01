---
name: project-crafting-unimplemented
description: CAT-F crafting/research/alloy/RE/spendASP are Phase-1 stubs; only TrainAbility is wired; future implementer's must-validate checklist
metadata:
  type: project
---

CAT-F (Crafting / R&D / Training) audit complete on 2026-05-31. Trust posture:

**TrainAbility (method 77)** — fully implemented, well-validated.
- Cell side: archetype-tree membership + level + prereqs + already-known guard,
  at `crates/services/src/cell/cell_methods/player/vendor.rs:491`.
- Base side: atomic `UPDATE … WHERE training_points > 0 AND NOT (abilities @>
  ARRAY[$1])` at `crates/services/src/base/world_entry/methods/progression/mod.rs:456`.
- **BUT** missing trainer-NPC interaction state + distance check — Python
  required `self.trainerEntity != None` AND `distanceTo(trainerEntity) <=
  MAX_INTERACT_DISTANCE`; Rust dropped both. Filed as CAT-F-01 Medium. Fix:
  add `trainer_entity: Option<u32>` on entity, parallel to `vendor_entity`,
  set by `try_open_trainer` at `crates/services/src/cell/interactions/trainer.rs:55`.

**The other five RPCs** — all stubs:
- `spendAppliedSciencePoints` (95), `craft` (96), `research` (97),
  `reverseEngineer` (98), `alloying` (99), `respecCrafting` (100) — all in
  `crates/services/src/cell/cell_methods/player/crafting.rs:23-86`. All
  return `true` (handled) with `tracing::info!(... "UNIMPLEMENTED")`.
- Persistence layer EXISTS:
  - `crates/services/src/base/crafting/persistence.rs` — load/save round-trip
    with `RowNotFound` guard on save; `applied_science_points`,
    `discipline_ids`, `blueprint_ids`, `racial_paradigm_levels`,
    `expertise[]` all persisted.
  - `crates/entity/src/crafting.rs` — `CraftingState` struct + serializer for
    `onUpdateDiscipline` (method 136).
- World-entry already sends `onUpdateKnownCrafts` from server-authoritative
  `blueprint_ids` at `crates/services/src/mercury/world_data/map_loaded.rs:375`.

**Wire shapes (from `entities/defs/SGWPlayer.def` lines 911-948, Ghidra-confirmed):**
- `spendAppliedSciencePoints(INT32 aDisciplineSeqId)` — method 95
- `craft(INT32 aCraftId, ARRAY<ItemID> aItems, INT32 aQuantity)` — method 96
- `research(ItemID aItemId, ARRAY<ItemID> aKickers)` — method 97
- `reverseEngineer(ItemID aItemId)` — method 98
- `alloying(INT32 aCraftId, ItemID aCurrentTierItemId, ARRAY<ItemID> aLowerTierItems)` — method 99
- `respecCrafting()` — method 100 (empty)

**The future implementer's MUST-validate checklist** (per finding):
1. **Blueprint known**: `aCraftId ∈ sgw_player.blueprint_ids` for caller.
2. **Item ownership**: every `ItemID` in payload → `sgw_inventory.player_id =
   caller`. NEVER trust the item_id alone.
3. **Bag location**: `bag_id IN (INV_Main, INV_Crafting)` — Python
   `Crafter.py:212-215`. Equipped/bandolier/mail items are NOT eligible.
4. **Item type matches recipe**: lookup `resources.blueprints_components`
   keyed by `(blueprint_id, component_set_id)`; verify supplied item types
   cover requirement.
5. **Quantity sufficient**: `sum(item.quantity ...) >= component.quantity *
   aQuantity`. Bound `aQuantity ≥ 1` and protect against overflow.
6. **Researchable / kicker / reverseEngineerable type flags** — resources lookup;
   reject quest/soulbound items.
7. **Outcome rolls server-side** — `research` chance + `reverseEngineer`
   component quantities use server RNG, never client-supplied seed/result.
8. **Atomic consume + produce**: single `BEGIN…COMMIT` with `FOR UPDATE` on
   input rows, parallel to TrainAbility's `WHERE training_points > 0` pattern.
9. **Alloy specifics**: `blueprint.alloy = true` check (Python `Crafter.py:464`
   — `if not blueprint.alloy: reject`); elementary count matches
   `ALLOYING_ELEMENTARY_COUNTS[quality]`; every elementary at `tier == current.tier - 1`.
10. **Busy gate**: no `busy` flag exists in Rust today — add per-entity
    `crafting_busy: Option<CraftTimer>` parallel to Python `@mustBeIdle`; reject
    overlapping craft requests.
11. **Validation BEFORE any mutation** — no `busy = true`, no
    `onCraftingStarted`, no in-memory state change before the validation
    chain completes.
12. **Python bug NOT to port**: `Crafter.py:469` precedence error
    `requirement['item'].id != component.typeId is None` evaluates as
    `(x != y) and (y is None)` = always False; never actually validates
    current-tier type. Rust must compare type ids directly.
13. **Python bug NOT to port**: `randint(0, len(list))` is inclusive-upper;
    Rust must use `rng.gen_range(0..n)`.

**ASP balance** for `spendAppliedSciencePoints` — read from
`sgw_player.applied_science_points`; decrement guarded by `WHERE
applied_science_points >= 1 AND NOT (discipline_ids @> ARRAY[$1])`.

**Cross-disciplinary**: items-systems-advisor owns the consume/produce path
itself (UPDATE/INSERT against `sgw_inventory`); this enforcer owns the
"client-supplied item_id must be validated" gate.

Confidence: high — five separate exploit shapes filed (CAT-F-02 through
CAT-F-05 + CAT-F-07 busy-gate). Ghidra confirmed all wire shapes.
