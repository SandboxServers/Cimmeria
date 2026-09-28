---
name: native-consumable-round-trip
description: Item use for heals/stimpacks is a cell->base->cell round trip (ConsumeItemForUse / ItemUseConsumed), consume first; timed stat buffs need their own ledger because pulse_count=1 never registers; where each piece lives across four crates
metadata:
  type: project
---

Built 2026-09-28 (branch `feat/item-use-abilities`). Design record: decision 27 of `docs/architecture/abilities-and-effects-system.md`.

- **Pay before apply.** A chain's `change_stat` + `remove_item` (and the naive "apply then send RemoveInventoryItem") double-heals on a fast double-click of the last unit: both `ItemUsed` pass the gate before either removal commits. The native path sends `CellToBaseMsg::ConsumeItemForUse`; the base's `remove_instance` (FOR UPDATE, optional `expected_type_id`) returns `bool` committed, and only then sends `BaseToCellMsg::ItemUseConsumed` **directly, not via the outbox** (a replayed outbox row would apply twice for one unit).
- **Where it lives:** classify/refuse/apply in `cimmeria-cell-content` `content/consumable_use.rs` (a second private caller of `effect_apply`, same security properties); consume in `cimmeria-base-methods` `inventory/core/consume_for_use.rs` (dispatched from `base-world-entry` `inventory_dispatch`); ledger math on `CellEntity` in `cimmeria-entity` `cell_entity/stat_buff.rs`; `StatBuff` script + logged `SpaceManager` wrappers in `cimmeria-cell-world` `effects/stat_buff/`; tick/timers/death strip in `cimmeria-cell-combat` `effects/stat_buffs/`. The full round trip test is `cimmeria-services` `consumable_round_trip_tests.rs` (the only crate above both tracks).
- **Sync scripts cannot send.** The ledger queues start timers (`timer_sent = false`) and clears (`pending_timer_clears`); `effect_apply::apply_effect` flushes right after the script, and `stat_buff_tick` is the safety net. Only flush *stats* for entities whose buffs expired this tick; flushing every buffed entity each tick would steal other systems' dirty bits.
- **Primary attributes sit at cur == max**, so `Stat::change(+5)` clamps to nothing; widen max and record the shift, restore it exactly on removal (`shift_stat_widening` / `unshift_stat`).
- **`InitPlayerState` reruns `apply_archetype`**, which would leave a stale ledger entry to restore below base; it clears `stat_buffs` first.

Related: [[owner-pet-effects-and-passives]] (the pet-side version of the pulse_count=1 trap), [[session-scoped-cell-state-hooks]].
