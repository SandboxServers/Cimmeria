-- ============================================================================
-- seed_effect_script_names.sql
--
-- Wires the new `cell::effects` script dispatcher to existing content by
-- setting `effects.script_name` on a handful of canonical effects:
--
--   - Heal Focus (effect 659 / ability 597)    → 'HealFocus'
--   - Dragon Fist stun (effect 1422 / 1252)    → 'Stun'
--   - Cover Fire suppression (effect 1475/720) → 'Suppression'
--
-- Idempotent — uses explicit value gating so repeated runs are no-ops.
--
-- Effect 659 currently carries the placeholder `script_name = 'TestEffect'`
-- (left over from fan-server era); flip it to the real registered script.
-- The other two are NULL today; assigning a script_name is what makes the
-- dispatcher pick them up.
--
-- See:
--   - crates/services/src/cell/effects/scripts.rs (script catalog)
--   - crates/services/src/cell/effects/registry.rs (name → script lookup)
--   - crates/services/src/cell/effects/mod.rs      (dispatch entry)
-- ============================================================================

BEGIN;

UPDATE resources.effects
SET script_name = 'HealFocus'
WHERE effect_id = 659
  AND (script_name IS NULL OR script_name IN ('TestEffect', 'HealFocus'));

UPDATE resources.effects
SET script_name = 'Stun'
WHERE effect_id = 1422
  AND (script_name IS NULL OR script_name = 'Stun');

-- Effect 1475 is labeled "Cover Fire Cone Suppression" but ships with
-- target_collection_method = 'TCM_Single' in the original seed — fix the
-- mismatch so the cone fan-out actually engages.
UPDATE resources.effects
SET script_name = 'Suppression',
    target_collection_method = 'TCM_AECone'
WHERE effect_id = 1475
  AND (script_name IS NULL OR script_name = 'Suppression');

-- Sanity check — list the rows after update so operators can eyeball.
SELECT effect_id, ability_id, name, script_name, pulse_count, pulse_duration
FROM resources.effects
WHERE effect_id IN (659, 1422, 1475)
ORDER BY effect_id;

COMMIT;
