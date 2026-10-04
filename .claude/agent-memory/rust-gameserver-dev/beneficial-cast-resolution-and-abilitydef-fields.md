---
name: beneficial-cast-resolution-and-abilitydef-fields
description: AB-01 beneficial-cast resolver lives in use_ability/beneficial.rs (with the #444 gate); adding an AbilityDef field touches ~70 struct literals; seed has a Heal-typed attack (2228)
metadata:
  type: project
---

**Where the rule lives (AB-01, 2026-10-03).** `cell-combat/src/cell/abilities/use_ability/beneficial.rs` holds `resolve_cast_target`, `target_gate` (the moved #444 gate plus the support-shot and beneficial reversals) and `fire_beneficial`. handle.rs (launch: `launch_target` rewrites target_id to the resolved entity), fire.rs and warmup/tick.rs all call it. `PendingCast::wire_target_id` keeps the client's target; the fire re-resolves from it before `Ability_End`. D-AB02's fallback-vs-refusal is the one const `FALLBACK_TO_CASTER`. Only player casts are resolved; NPC casts keep the wire target.

**Seed traps.** `ability_is_beneficial` (entity crate) = ≥1 implemented effect, and every implemented effect carries EF_Beneficial_Effect (1) or (Heal-typed only) runs a `HEAL_SCRIPTS` script. The Heal type alone is NOT enough: ~200 seed abilities are Heal-typed debuffs/CC (1874, 1988, 2154...); review S1 caught it. Two vetoes added beyond the ledger's contract: damage NVPs veto (2228 `MS020_080818_CallTarget` is ABILITY_TYPE_Heal with a `RangedPhysicalDamage` effect), and "≥1 implemented" (an unimplemented attack must not be vacuously beneficial). 597/1646/1218 are beneficial by type only: effect 659 is flags 16, no bit 1. Heal warmups are 2-5 s, so the warmup tick path matters.

**Adding an `AbilityDef` field** breaks ~70 struct literals across ~60 files (tests mostly). A script inserting `field: Default::default(),` before each literal's closing brace works if it skips `-> AbilityDef {` / `-> cimmeria_entity::abilities::AbilityDef {` fn signatures (`'->' in prefix`) and literals with a top-level `..` spread. The `type_id` loader reads the Postgres enum as `type_id::text`.

**Why:** next AB packets (AB-07, AB-12) edit the same files; knowing the resolver is the single seam avoids re-deriving the target rule. **How to apply:** route any new friendly/hostile target decision through `beneficial.rs`; never add it to handle.rs (hard cap 700).

Related: [[support-shot-inverse-gate]], [[ability-launch-fire-split]].
