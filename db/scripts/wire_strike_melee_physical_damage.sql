-- ============================================================================
-- wire_strike_melee_physical_damage.sql
--
-- Wires ability 594 (Strike) up to the new `MeleePhysicalDamage` effect
-- script by:
--
--   1. Setting effect 656's `script_name` to `'MeleePhysicalDamage'`
--   2. Inserting the `FocusDamage` / `HealthDamage` NVPs the script reads
--
-- Pre-fix state (per seed):
--   - effects row 656: script_name = NULL, desc = '-100F -10H'
--   - effect_nvps for effect 656: none
--
-- The legacy authoring stored the damage numbers in `effect_desc` as a
-- free-text hint (`-100F -10H` = -100 Focus, -10 Health) and relied on
-- per-effect script logic to do the actual application. Without the
-- script wired, pressing Strike fires the animation + cooldown but
-- never applies damage to the target — the symptom user-reported as
-- "Strike doesn't do anything."
--
-- Strike's hit values come from the legacy effect_desc convention. We
-- pick the canonical -100/-10 split here; once we have a real combat
-- balance pass, the values live in this row and only this row.
--
-- Idempotent — value-gated UPDATEs + an INSERT … ON CONFLICT guard so
-- repeated runs are no-ops.
--
-- See:
--   - crates/services/src/cell/effects/scripts.rs::MeleePhysicalDamage
--   - crates/services/src/cell/effects/registry.rs
--   - db/resources/Effects/Seed/effects.sql row for effect 656
-- ============================================================================

BEGIN;

-- Wire the script. Gated against NULL (the unfixed state) and against
-- the target name (so a re-run is a no-op). Anything else in
-- `script_name` is a deliberate override and we leave it alone.
UPDATE resources.effects
SET script_name = 'MeleePhysicalDamage'
WHERE effect_id = 656
  AND (script_name IS NULL OR script_name = 'MeleePhysicalDamage');

-- FocusDamage = 100 — the -100F half of the legacy effect_desc.
INSERT INTO resources.effect_nvps (nvp_id, effect_id, name, value)
VALUES (
  -- nvp_id is locally unique; pick a 600-series id that's free in the
  -- seed (the highest seeded nvp_id is in the low 100s today; 600
  -- leaves headroom for future Strike-adjacent NVPs).
  600, 656, 'FocusDamage', '100'
)
ON CONFLICT (nvp_id) DO NOTHING;

-- HealthDamage = 10 — the -10H half. Spillover from any remaining
-- focus_overflow stacks on top of this base value per the script's
-- two-step legacy formula.
INSERT INTO resources.effect_nvps (nvp_id, effect_id, name, value)
VALUES (601, 656, 'HealthDamage', '10')
ON CONFLICT (nvp_id) DO NOTHING;

-- Sanity check — list the rows after update so operators can eyeball.
SELECT effect_id, ability_id, name, script_name
FROM resources.effects
WHERE effect_id = 656;

SELECT nvp_id, effect_id, name, value
FROM resources.effect_nvps
WHERE effect_id = 656
ORDER BY nvp_id;

COMMIT;
