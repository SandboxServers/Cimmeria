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
