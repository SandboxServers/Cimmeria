-- Migration: add the Health Slappack TC1 (item 2893) consumable chain.
--
-- Wires `useItem(2893)` to a +500 HP heal followed by remove_item(2893,1).
-- Mirrors the seed file `db/resources/Content/Seed/consumables_chains.sql`
-- one-for-one so a fresh load and an in-place migration converge to the
-- same chain shape.
--
-- Chain id is 4001 — review feedback on the original PR moved consumable
-- chains out of the Castle_CellBlock 1xxx range. The DELETE on 1100 below
-- cleans up any local DB that already ran the pre-feedback migration.
--
-- Convergent upsert (DO UPDATE) on 4001: if it already exists with a
-- different shape (e.g. an aborted earlier run), this migration corrects
-- it rather than silently leaving the wrong values in place.
--
-- All work runs inside a single BEGIN/COMMIT so a mid-script failure
-- rolls back cleanly. `ON_ERROR_STOP` halts at the first error but
-- without the explicit transaction, partial deletes/inserts would
-- persist on failure.
--
-- Safe to re-run.

\set ON_ERROR_STOP on

BEGIN;

-- Cleanup: remove chain 1100 if a pre-feedback run left it. Order is
-- (children → parent) to avoid FK violations: triggers, conditions,
-- counters, actions, then chains. Both content_conditions and
-- content_counters reference content_chains via FK — missing them in
-- the cleanup would cascade-fail the chain delete or leave stale
-- gating/counter state on the renumbered 4001 chain.
DELETE FROM resources.content_triggers   WHERE chain_id = 1100;
DELETE FROM resources.content_conditions WHERE chain_id = 1100;
DELETE FROM resources.content_counters   WHERE chain_id = 1100;
DELETE FROM resources.content_actions    WHERE chain_id = 1100;
DELETE FROM resources.content_chains     WHERE chain_id = 1100;

INSERT INTO resources.content_chains
    (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES
    (4001, 'Health Slappack TC1: +500 HP + consume', 'global', NULL, true, 0)
ON CONFLICT (chain_id) DO UPDATE SET
    description = EXCLUDED.description,
    scope_type  = EXCLUDED.scope_type,
    scope_id    = EXCLUDED.scope_id,
    enabled     = EXCLUDED.enabled,
    priority    = EXCLUDED.priority;

-- Triggers, conditions, counters, actions for chain 4001: delete-then-
-- insert across all child tables so a stale row with a different shape
-- can't survive the migration. The seed defines no conditions or
-- counters for this chain, but the cleanup runs unconditionally to
-- absorb any drift from prior migration runs.
DELETE FROM resources.content_triggers   WHERE chain_id = 4001;
DELETE FROM resources.content_conditions WHERE chain_id = 4001;
DELETE FROM resources.content_counters   WHERE chain_id = 4001;
DELETE FROM resources.content_actions    WHERE chain_id = 4001;

INSERT INTO resources.content_triggers
    (chain_id, event_type, event_key, scope, once, sort_order)
VALUES
    (4001, 'item_use', '2893', 'player', false, 0);

INSERT INTO resources.content_actions
    (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
    (4001, 'change_stat', NULL, NULL, '{"stat_id": 7, "amount": 500}'::jsonb, 0, 0),
    (4001, 'remove_item', 2893, NULL, '{"qty": 1}'::jsonb,                    0, 1);

COMMIT;
