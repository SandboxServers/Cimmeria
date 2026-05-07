-- Content chains for consumable items.
--
-- Triggered by `useItem(item_id)`, scoped globally (no mission gate).
-- Each chain runs a `change_stat` (the heal/buff effect) followed by a
-- `remove_item` (consume the stack by 1). Pattern mirrors the ambernol
-- chain at castle_cellblock_chains.sql:287-316 — `useItem` events fire
-- without consuming, so per-item chains decide whether to remove.
--
-- Chain ID range: 4000-4099 reserved for consumable-use chains.
-- Picked above the mission ranges (1xxx Castle_CellBlock allocates up
-- through 1130, 2xxx effect chains, 3xxx sgc_w1) and the matching
-- 5xxx dialog set. 4xxx is otherwise unused.
--
-- Stat IDs: 7 = HEALTH (see crates/entity/src/stats/stat_ids.rs).

SET search_path = resources, pg_catalog;

-- ============================================================
-- Item 2893 — Health Slappack TC1 (+500 HP)
-- ============================================================

-- Chain 4001: useItem(2893) when HP < max → +500 HP, consume 1 slappack.
--
-- The `stat_below_max` condition gates the chain on actual HP headroom:
-- using a slappack at full HP fizzles silently rather than burning the
-- stack. Stat values are populated into the chain context by
-- `populate_stats_context` (called from `fire_item_use`); the condition
-- reads `stat_7_cur` and `stat_7_max` and fires only when cur < max.
--
-- A user-facing "already at full health" message is intentionally not
-- emitted here — `Action::SystemMessage` is currently a stub (correct
-- client wire format unknown; see executor.rs::SystemMessage). Until
-- that's wired up, the failure mode is "click does nothing, stack is
-- preserved" which is correct-but-quiet.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (4001, 'Health Slappack TC1: +500 HP + consume (HP < max)', 'global', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (4001, 'item_use', '2893', 'player', false, 0);

-- `operator` and `value` are unused by `stat_below_max` (the condition is
-- structural: cur < max). Pass the column-default 'eq' for operator and
-- NULL for value so the loader's required-not-null operator parse
-- succeeds and the matcher branch ignores both.
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (4001, 'stat_below_max', 7, NULL, 'eq', NULL, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  -- Heal first, then consume. Order matters: if `remove_item` failed
  -- (channel saturated, etc.) the player at least sees the heal land
  -- rather than losing the stack to a silent no-op.
  (4001, 'change_stat', NULL, NULL, '{"stat_id": 7, "amount": 500}', 0, 0),
  (4001, 'remove_item', 2893, NULL, '{"qty": 1}', 0, 1);
