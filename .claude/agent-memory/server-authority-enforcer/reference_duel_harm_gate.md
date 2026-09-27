---
name: reference-duel-harm-gate
description: Where the duel PvP harm gate lives (SS-D2, PR 911) and the side paths that do NOT re-check hostility — pulses, auto-cycle validity, pet defend sweep, launch-time same-space
metadata:
  type: reference
---

Authority: `combat::player_may_attack(attacker, target, &DuelRegistry)` in `crates/cell-world/src/cell/combat/aggression.rs`; player targets only via `DuelRegistry::can_harm` (player_id keyed, engaged state, same space). The PvP flag (`onEntityProperty(4, v)`) is presentation only and never read back (D-SS23). Area collectors use `area_candidates` + `may_hit_in_area`. Single end clear: `duel::end_engaged` (`duel/end.rs`), idempotent (removes the duel first).

Paths that apply harm or state WITHOUT re-running the gate (review these on every PvP change):
- `effects/pulsing/tick.rs::fire_pulse` — DoTs/debuffs keep ticking on the ex-partner after the duel ends; `end_engaged` strips no effects. Also bypasses any future damage clamp unless the clamp sits in the pulse seam too.
- `ticks/auto_cycle.rs` validity is `is_auto_cycle_target_valid` (dead/submit only) — no hostility check, so a loop on the ex-partner re-invokes `handle_use_ability` every 100 ms and hits the #444 WARN each time.
- `npc_ai/pet/defend.rs::sync_owner_combat` stale filter treats every `threatened_mobs` entry as a mob with a threat_list; any non-mob combat source (the duel opponent) is dropped on the next pet turn.
- `use_ability/handle.rs` launch gate has no same-space check for NPC targets (fire_los returns `NotChecked("other_space")`); only the warmup re-check tests space. Duel targets are safe because `player_may_attack` checks space itself.

`threatened_mobs` now holds a player entity id during a duel — any new code that iterates it as "mobs" must tolerate that. See [[reference-combat-exploit-classes]], [[exploit-entity-id-recycling]].

**Status (2026-09-27, PR 911 follow-up):** the first three side paths are closed in the same PR: `end_engaged` strips every active effect the partner's engaged entity invoked (`duel/effects.rs`), the auto-cycle tick stops a loop on a player the caster may not harm (`duel.auto_cycle_stopped`), and `sync_owner_combat`'s stale filter prunes only NPC combat sources, never a player source. `engaged_opponent_entity` now re-checks `connected_player`, so a recycled id is never offered as the partner. Mid-duel pulses still don't re-run the gate (they can't target a non-partner; SS-D3's clamp must still sit in the pulse seam). The launch same-space note is unchanged.

**SS-D3 review (2026-09-27, branch social/d3-duel-end-paths):** the non-lethal clamp `duel::clamp_partner_lethal` (`duel/paths.rs`) sits in `damage_apply` (after direct damage, again after scripts) and in `fire_pulse`; `finish_clamped` ends the duel last so same-resolution DoTs get stripped. Two gaps to re-check on any PvP change:
- `apply_damage_to_target` never re-runs the harm gate, so a target list built before the end still lands after it. `cone_aoe/fan_out.rs` collects per-effect targets up front, so a two-cone ability (seed: 3216 Multi-Dart, 4683+4684) clamps the partner on cone 1, ends the duel, then cone 2 kills the ex-partner through `resolve_death`. The fix is to re-check `player_may_attack` for player targets at the top of `apply_damage_to_target`.
- The clamp matches exact engaged entities, but `can_harm` is keyed on player_id. For about one tick (a new entity of a duelist before the sweep), the gate allows a hit that the clamp won't hold. The rule: the clamp predicate must be a superset of the gate predicate.
