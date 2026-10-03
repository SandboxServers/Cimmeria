---
name: ability-mechanics-gaps
description: Why most player abilities do nothing (2026-10-03 audit, docs/analysis/ability-mechanics/) - Self target never substituted, NVP-less damage is 0, timed single-pulse effects never register, regen stats are percentages, scripted damage double-applies
metadata:
  type: project
---

Audit of the 423 player-reachable abilities (5 starters + 419 tree ids) against `main` @ b40ef76d2.
Full rows: `docs/analysis/ability-mechanics/audit.md` (B-nn), packets AB-nn. Verify line numbers before acting.

**Why:** these are the traps that make an ability "look implemented" (script registered, NVP present,
tests green) yet do nothing in play. **How to apply:** before calling any ability family working, check
all five; before proposing regen numbers, cite B-51/B-52 (no retail rate exists).

1. **Client sends its current target for Self abilities** (Ability.lua `useAbility(id, Unit.Target)`,
   native emit 0x00d2ae40 branches only on Ground). Server never substitutes the caster
   (python did, AbilityManager.py:527). Result: target 0 -> `fire.rs` no-op after cooldown;
   self/ally -> #444 refusal; hostile -> the heal script heals the HOSTILE. No ability heal fired in
   30 days of SigNoz; only consumables (648, 2206) and stimpacks.
2. **Damage comes only from `HealthDamage`/`FocusDamage` NVPs.** 229 reachable effects state damage
   only in `effect_desc` ("-200F / -20H") -> 0-damage hit that still creates threat. Only 592/594 have NVPs.
3. **Scripted damage applies twice**: NVP pipeline + the `RangedPhysicalDamage`/`MeleePhysicalDamage`
   script, and scripts run on RC_MISS (no result check before dispatch in damage_apply).
4. **`pulse_count = 1` with `pulse_duration > 0` never registers** (`is_pulsing` false) - that is the
   authored shape of every timed buff/debuff/snare (135 reachable effects). The stimpack `stat_buffs`
   ledger (ADR decision 28) is the seam to generalise, not the pulsing layer.
5. **Regen stats are % modifiers** (alias.xml: healthRegen/focusRegen "+1% rate", morale +0.5% focus
   regen); `regen.rs` reads them as points/s with a floor of 1 -> Focus (1570) refills in ~26 min.
   No artefact holds a base rate; python never ticked regen. Content (Leadership 858 "+50% Focus Regen
   20 s") implies Focus regenerates in combat.

Also: no ability Focus cost exists anywhere (Focus is the damage shield, not mana); the `EF_*`
"category" constants (EF_STUN=12 etc.) are not client bits; `EF_ResolveOnAbilityUser` appears only on
Disguise 1081-1083 and Incinerate Area 1889. When :5433 Postgres is down, the seed INSERTs load fine
into SQLite for read-only classification (strip `::type` casts).

Related: [[shipped-data-combat-evidence]], [[health-mutation-paths]], [[qr-direction-and-cover]].
