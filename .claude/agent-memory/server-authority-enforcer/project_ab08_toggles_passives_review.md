---
name: project-ab08-toggles-passives-review
description: PR #1161 AB-08 held toggles/stances/passives review (2026-10-03) — cleared shape, and the passive-cast-goes-hostile hole
metadata:
  type: project
---

Reviewed PR #1161 (ability-mechanics AB-08) on 2026-10-03.

Cleared shape: toggle state = presence of the ability's last held TimedStat entry (anchor) on the caster, `(effect, invoker)` PerSource keyed, so forged repeat presses only flip, never stack. `held_not_self` refuses any held entry where source != target. Every seeded toggle/passive is `target_type_id = 1` (TargetSelf), so `resolve_cast_target` retargets beneficial ones to the caster. Ledger reset in `InitPlayerState` makes passive re-apply once per world entry (relog, zone). No server expiry at `HELD_ICON_SECS` (86,400 s is icon-only).

Open finding: passives can still be sent through useAbility. 1574 (effect 4782, flags 524304, no EF_Beneficial bit, cooldown 0, max_range 0 = default range) is non-beneficial, so the cast goes to `apply_damage_to_target`: QR roll, threat, BSF_InCombat, with zero damage and no cooldown. Before AB-08 it was refused as `no_mechanics`. `passive_yn` is not loaded into AbilityDef, so nothing refuses a passive at launch. Recommended fix: refuse useAbility when `passive_yn` is set or every effect is EF_AlwaysPersist.

Hardening notes: `RemoveByMoniker` has no source==target check (reachable today only via TargetSelf 859/857). A weapon-granted toggle or passive revoked by `swap_weapon_granted_abilities` would leave its held entry behind (none are item-bound today).

**Why:** future held-effect or passive packets (AB-10/11 cleanses, mini-game passive 809) reuse this machinery.
**How to apply:** check any new passive or toggle for its beneficial bit and its launch refusal, and check new unlearn paths for the passive removal. Related: [[exploit-use-ability-no-faction]], [[reference-combat-exploit-classes]].
