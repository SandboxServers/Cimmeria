---
name: project-278-combat-death-fanout
description: #278 combat+death witness-fanout — which emit paths were converted, the idbase fix, and tests
metadata:
  type: project
---

Implemented on branch `feat/278-witness-fanout` in worktree `Cimmeria-278`. Commit `163be645`.

## Emit paths converted (send_entity_method → send_entity_method_to_self_and_witnesses)

All five paths are in `crates/services/src/cell/abilities/`:

1. `damage_apply/mod.rs`: `onEffectResults` for attacker — now fans to self+witnesses.
2. `damage_apply/mod.rs`: `onEffectResults` for player target (keyed on `target_eid`) — now fans to self+witnesses of target (not just own-client).
3. `damage_apply/mod.rs`: `onStatUpdate` for target — now fans to self+witnesses.
4. `damage_apply/mod.rs`: `BSF_InCombat onStateFieldUpdate` — now fans to self+witnesses.
5. `damage_apply/mod.rs`: death `onSequence` animation — now fans to self+witnesses.
6. `death.rs`: final dead-state `onStateFieldUpdate` — now fans to self+witnesses.

## Player-ghost idbase fix

`CellToBaseMsg::WitnessEntityMethod` gained field `entity_is_player: bool`.
Stamped at every construction site (messaging.rs, space_manager/aoi.rs, ring_transport, being.rs, gm/world.rs, content/executor/world+dialog).
`aoi.rs::witness_entity_method` now selects `IDBASE_SGW_PLAYER` (61) vs `IDBASE_NPC_DEFAULT` (62) based on this flag.
Method indices ≥61 encode differently; previous unconditional NPC idbase would have corrupted wire for high-index player methods once fanout went live.

## The audit-missed emit path

The task spec only called for the attacker's `onEffectResults` to fan out. The on-target `onEffectResults` (keyed on `target_eid`) is a SEPARATE send that also needed converting — without it, a spectator cannot see the hit land on a player target (only sees the attacker fire). Both were converted.

## Tests added

- `damage_apply/tests.rs`: `spectator_receives_effect_results_and_stat_update_from_pvp_hit`
- `damage_apply/tests.rs`: `spectator_receives_death_state_and_sequence_on_player_kill`
- `tests_dispatch_arms/witness_broadcast.rs`: `witness_entity_method_player_ghost_uses_idbase_61_npc_uses_62`

**Why:** Guarantees witness delivery, not just local state mutation. Fails if emit paths revert to own-client-only.

## Deferred

BeingAppearance equip/holster recomposite (#279) is base-side property fanout — different plumbing, deferred. `onBeginAidWait` (Defeat Window) stays own-player — by design.
