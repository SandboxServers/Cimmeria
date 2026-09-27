---
name: per-session-player-state-lifecycle
description: Where to hang per-player UI/session state on the cell (vault session, BV-02) - a CellEntity field dies on every space change and logout because no path moves an entity between spaces; every interact pin goes through one chokepoint; interact range is double-gated.
metadata:
  type: project
---

Learned building the BV-02 vault session (2026-09-27, branch `bank/bv02-banker-open`).

- **A `CellEntity` never changes space in place.** The only `entity_space.insert` sites are `insert_entity_into_space`, `spawn_npc` and `spawn_npc_from_record_into`, each building a fresh entity; cross-world, `.goto`, gate travel and logout all go through `destroy_entity` / `disconnect_entity`. So any per-player state stored as a `CellEntity` field is cleared on space change and logout *by construction*. A test of that can only pin the construction, not a revertible line.
- **Every `interact` pin goes through `cell::interactions::pin_interaction_target`** (cell-interactions `bank/mod.rs`, which calls `CellEntity::pin_interaction_target`). There are two pin sites: the outer dispatcher in `cell-methods .../interaction/interact.rs` and `handle_interact`'s fall-through. Hang "a new target ends X" logic there, not at either site.
- **Interact range is double-gated**: the outer `interact_target_in_range` (space + distance) and `handle_interact`'s own distance check. A "too far sends nothing" test passes with either gate removed alone; a regression proof must remove both.
- **The pure rule is `interact_range(...) -> Result<(), InteractRangeFail>`**; `interact_target_in_range` is it plus "interact: ..." INFO logs. Reuse the pure one from non-interact checks.
- **Adding a `SpawnRecord` field touches ~13 hand-built test literals** across cell, cell-console, cell-content, cell-world (no `Default`). A `sed` that appends after each literal's `use_cover:` line does it; new tests can use `test_fixtures::npc_spawn_record`.
- **Removing a column DEFAULT fails the seed reload itself** (`live-db-test.sh` stops at psql), because seed INSERTs list columns and omit new ones.

Related: [[debug-hub-npc-authoring-traps]], [[destroy-entity-vs-despawn-npc]], [[cross-world-transfer-flow]].
