---
name: crafting-induction-engine-seams
description: Crafting induction engine (base-session crafting/session, transaction) - process-wide registry, drop hooks, lock order, stale world_name across gate travel, base-session cannot reach base-methods inventory helpers
metadata:
  type: project
---

Facts from building the crafting induction engine (branch craft/cr06-induction, 2026-09-27):

- `cimmeria-base-session` sits BELOW `cimmeria-base-methods`, so `send_full_inventory_update`,
  `send_on_remove_item`, `reserve_free_inventory_slots` and `INVENTORY_ITEM_SELECT` are not reachable
  from `base/crafting/`. `transaction/client_sync.rs` and `transaction/grant.rs` carry filtered copies.
- The induction registry is a process-wide `LazyLock<Arc<CraftingSessions>>`
  (`crafting_sessions()`); tests build their own with `ManualScheduler`. Tests that touch the global
  must use a private entity id (4290-4295 are taken).
- Drop hooks: `destroy_client_entities` (all disconnects), `handle_log_off` (crates/base
  dispatch/session.rs) and `handle_gate_travel`. `ConnectedClientState.world_name` is set only in
  `play_character.rs` and is NOT updated by gate travel, so any "same world" check built on it misses
  gate travel; `active_player_id` is the reliable "same character" check.
- Inventory lock orders differ per path: move takes `pg_advisory_xact_lock(player, 0)` first; grant
  and trade take `(player, bag)` before rows/`sgw_player`; vendor purchase locks rows first. The
  crafting transaction takes every advisory lock (0, then bags sorted) first; since the #897 review it
  only reads `sgw_player` (no `FOR UPDATE`), and vendor purchase now takes `(player, 0)` first too.
- `i32::div_ceil` / `i64::div_ceil` are unstable (int_roundings) on the pinned 1.98.1; use the
  unsigned form.

**Why:** the verb packets (craft, research, reverse engineer, alloy) plug into this engine.
**How to apply:** reuse `apply_craft_transaction` and `crafting_sessions().submit`; do not add a
second inventory lock order. See [[sqlx-dynamic-sql-string]] for the `&'static str` query rule.
