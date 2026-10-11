---
name: combat-telemetry-footprint
description: Combat telemetry gaps found 2026-10-10 (telemetry-gaps campaign, reviews/combat.md): player.respawn DEBUG unexported, target-scan guard blind after `#[cfg(test)] mod x;`, NPC staff melee 710 inert, ClickHouse query traps
metadata:
  type: project
---

Review: `docs/analysis/telemetry-gaps/reviews/combat.md` (packets TG-CMB-01..10). Verify before acting.

- `player.respawn` is not in `OTEL_FILTER` and its two refusal rows (`respawn_not_dead`,
  `respawner_not_offered`) are DEBUG, so they never reach SigNoz. `target_scan_tests::strip_test_module`
  cuts a file at the first `#[cfg(test)] mod`, including a `mod tests;` declaration at the top of the
  file, so the guard missed it (25 files have log sites below such a declaration).
- Ability 710 Staff Melee AA / effect 736 Strike Damage plans `path=skipped reason=no_script`, so the
  Praxis Jaffa Guard's melee does nothing (DEBUG only).
- `effect_script_unregistered` startup WARN fires every boot on the pinned known set (`""`/2907, Reload/658).
- 97% of `useAbility: launched` INFO rows are NPC casts (Debug Area sparring).

**Why:** the combat targets look complete, but these are the blind spots that stayed hidden anyway.
**How to apply:** on a "respawn did nothing" report, the refusal is missing from SigNoz until TG-CMB-01
lands. In ClickHouse, `attributes_number[...]` is Float64, so compare it with `toFloat64(710)`, not a
bare integer (BAD_GET error).

Related: [[colo-combat-forensics]], [[ability-mechanics-gaps]].
