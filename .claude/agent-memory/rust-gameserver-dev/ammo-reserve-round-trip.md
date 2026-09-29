---
name: ammo-reserve-round-trip
description: AM-02 special-ammo reload/switch is a cell->base->cell round trip; base counts rounds from the locked weapon row (cell flushes first on the ordered channel); rounds load at draw commit, not at warmup end
metadata:
  type: project
---

Built 2026-09-28 (PR for AM-02, campaign #1026). Worknote: `docs/analysis/ammo/worknotes/AM-02.md`.

- **Base cell-message loop is sequential** (`crates/base/src/base/service.rs`, one `while let` awaiting each `handle_cell_message`). So a `BandolierAmmoUpdate` flush sent right before a request is applied before it: the base can trust the weapon row. That is how a duplicated/replayed reserve request moves nothing — never trust the cell's `clip_before`/`rounds` for arithmetic.
- **Shots are not persisted per round** (`bandolier_ammo_dirty`, flushed on swap/reload/logout), so `sgw_inventory.ammo` lags the cell unless you flush first.
- **Load rounds when the draw commits, not in the completion tick**: `active_slot` swap and death (`clear_weapon_action_state`) cancel `reload_complete_at`, so a tick-time refill of drawn rounds loses them. The tick keeps the clip via `reload_reserve::completion_target` (matched on the warmup deadline, so a stale marker never blocks a free refill).
- Per-player cell state goes in `CellEntity::extensions` (typed map, #962) — no struct field, no `construction.rs` edit.
- A type-5 race here: the advisory lock serializes the drawers, so the forced-overlap SHARE-gate recipe ([[forced-db-race-share-lock]]) counts gate-blocked A plus advisory-blocked B = 2.
- `ammo_feature::finite_special()` is a process global: read it at the entrypoint and pass a bool (`handle_reload_with`, `begin_switch_return_with`) so tests never flip it.
