---
name: combat-telemetry-footprint
description: Combat telemetry gaps found 2026-10-10 (telemetry-gaps campaign, reviews/combat.md): player.respawn DEBUG unexported, target-scan guard blind after `#[cfg(test)] mod x;`, NPC staff melee 710 inert, ClickHouse query traps
metadata:
  type: project
---

Review: `docs/analysis/telemetry-gaps/reviews/combat.md` (packets TG-CMB-01..10). The remaining bullets are the 2026-10-10 report and were not re-checked; verify before acting.

- FIXED since 2026-10-10: `player.respawn=debug` is now in the OTEL filter (`crates/server/src/logging/filters.rs`) and `target_scan_tests::strip_test_module` keeps code after a `mod tests;` declaration (test `strip_test_module_keeps_code_after_a_test_mod_declaration`).
- Ability 710 Staff Melee AA / effect 736 Strike Damage plans `path=skipped reason=no_script`, so the
  Praxis Jaffa Guard's melee does nothing (DEBUG only).
- `effect_script_unregistered` startup WARN fires every boot on the pinned known set (`""`/2907, Reload/658).
- 97% of `useAbility: launched` INFO rows are NPC casts (Debug Area sparring).

**Why:** the combat targets look complete, but these are the blind spots that stayed hidden anyway.
**How to apply:** on a "respawn did nothing" report, the refusal is missing from SigNoz until TG-CMB-01
lands. In ClickHouse, `attributes_number[...]` is Float64, so compare it with `toFloat64(710)`, not a
bare integer (BAD_GET error).

Related: [[colo-combat-forensics]], [[ability-mechanics-gaps]].

INVALIDATED WHEN: effect 736 / ability 710 get a script or the effect_script_unregistered pinned set changes
checked: re-verified first bullet only; ability 710, 736 and 97% claims unverified 2026-11-01
