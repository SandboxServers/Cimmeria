-- Migration: fix swapped dialog_set_map IDs in Mission 638's archetype
-- gating chains (Prisoner 329 dialog).
--
-- Chains 1011 (non-Jaffa) and 1012 (Jaffa) had their `add_dialog_set`
-- target_id values swapped relative to the archetype condition, so a
-- Tau'ri / Human player would see the Jaffa "My symbiote will cure me"
-- dialog (5021 via dialog_set_map 5866) and a Jaffa player would see
-- the Human "stasis sickness" dialog (2300 via dialog_set_map 2794).
--
-- Correct mapping for Mission 638 / dialog_set_id 603:
--   dialog_set_map 2794 → dialog 2300 (Human "Free Prisoner 329")
--   dialog_set_map 5866 → dialog 5021 (Jaffa "Free Prisoner 329")
--
-- The chain_id pinpoints the exact action row via (chain_id,
-- action_type='add_dialog_set'); the existing seed has exactly one
-- such action per chain so the WHERE clause matches a single row.
--
-- Idempotent: if the seed already has the correct values (e.g. from a
-- fresh load post-fix), the UPDATE is a no-op.

\set ON_ERROR_STOP on

-- Chain 1011 (non-Jaffa archetype) must add the Human dialog set (2794).
UPDATE resources.content_actions
SET    target_id = 2794
WHERE  chain_id = 1011
  AND  action_type = 'add_dialog_set';

-- Chain 1012 (Jaffa archetype) must add the Jaffa dialog set (5866).
UPDATE resources.content_actions
SET    target_id = 5866
WHERE  chain_id = 1012
  AND  action_type = 'add_dialog_set';
