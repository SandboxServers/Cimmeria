---
name: combat-exit-tail-parity
description: The canonical "combat ends for this NPC" cleanup tail — which steps apply_death_transition does, which are death-only, and which every non-death combat-exit path (submit/surrender, leash, no-threat-idle) must also do
metadata:
  type: project
---

# Combat-exit cleanup tail — death vs. non-death parity

`apply_death_transition` (`crates/services/src/cell/abilities/death.rs`) is the only
fully-correct combat-exit tail in the tree. Any NEW path that ends an NPC's combat
while the NPC stays alive (surrender/submit, scripted pacify, future yield) must
copy a **subset** of it. The subset boundary, as of 2026-09:

**MUST copy (player-side leaks otherwise):**

1. `combat::clear_dead_npc_from_all_player_threat(space_mgr, npc_id)` — death.rs:148.
   It snapshots `npc.threat_list.keys()` (`threat/player_combat.rs:145-148`) and
   explicitly does NOT drain the list (doc at :131-132), so it **must run before**
   the NPC's own `threat_list.clear()`. Same deferred-drain contract as
   `gm_kill_npc` and the NPC death path, which drains `threat_list` immediately
   after `apply_death_transition` returns (`damage_apply/mod.rs:324-329`).
   Then broadcast `ON_STATE_FIELD_UPDATE` per returned `(player, state)` pair
   (death.rs:149-169). Do NOT force an appearance refresh — `exit_player_combat`
   stamps `combat_exit_at` and `holster_timer_tick` handles it.
   Skipping this = `threatened_mobs` leak = `BSF_InCombat` stuck, weapon drawn,
   and `regen_tick` (`ticks/regen.rs:60-69`, gated on
   `threatened_mobs.is_empty()`) never regens that player again.
2. `combat::clear_auto_cycle_for_target(space_mgr, npc_id)` + per-pair broadcast —
   death.rs:179-196. Filters on the player's LIVE `current_target_id`
   (`combat/auto_cycle.rs:162-166`).
3. `space_mgr.cover.release_for_entity(EntityId(npc_id as i32))` — death.rs:136-138.
   `npc_ai_fight` also releases cover on its `Fighting -> Idle` and
   `Fighting -> Leashing` transitions. Those transitions clear the NPC threat
   list, but do not run player-threat cleanup or the auto-cycle sweep. A
   combat-exit path that omits it leaks the cover slot for the instance's life.
4. `effects::cancel_channels_from_attacker(npc_id, None, ...)` — death.rs:110-114.
   Otherwise a surrendering channeller keeps pulsing its debuff.

**Death-ONLY — do NOT copy to a live-NPC exit:**
`onTargetUpdate(0)` reticle drop (:117-126), `generate_loot_on_death` (:233),
the `INTERACTION_TYPE` push (:235-245), the dead-bit
`send_entity_method_to_self_and_witnesses` (:252-259), Discord/ContactList
emits (:66-102), and `combat::mark_npc_dead` (`combat/state.rs:109` sets
`AiState::Dead` + stamps `respawn_at`). Step 2c `clear_auto_cycle` on the dying
entity's own loop (:212-228) is `target_is_player`-only.

**Bit discipline:** existing NPC-side `BSF_IN_COMBAT` clears are raw `&= !MASK`
in the death paths and Submit handler. `Fighting -> Idle` and
`Fighting -> Leashing` do not currently clear that bit. There is no NPC-side
ref-counted `set_state_flag(BSF_IN_COMBAT)` enter path, so routing a raw clear
through `unset_state_flag` would be a no-op decrement.

**Known gap (unclosed as of 2026-09):** no `npc_ai/*.rs` handler ever sends
`ON_STATE_FIELD_UPDATE` for the NPC itself. The NPC-side `BSF_IN_COMBAT` clear on
submit/leash/idle is **server-only** — witnesses keep the last broadcast state.
Death broadcasts the dead-state update, and the respawn tick broadcasts the
cleared state to witnesses. Verify against
`docs/reverse-engineering/findings/state-flag-broadcast.md` before "fixing" it.

## Tick cadences that set the leak window

`crates/services/src/cell/service/message_loop.rs`, AoI tick = 100 ms:
- `holster_timer_tick` — every tick (:77)
- `auto_cycle_tick`, `pending_attack_tick`, `npc_ai_retry_sweep` — every tick
- `regen_tick` — every 10th tick (:144-145), i.e. 1 s
- `npc_ai_tick` — every 20th tick (:126-127), i.e. **2 s**

So any AI-tick-driven cleanup has a worst case of ~2 s of continued auto-fire at
`1 / ability cooldown` shots per second.

## `aggression` is NOT persisted

No entity-template or player-persistence column. Defaults `0`
(`entity/src/cell_entity/construction.rs:88`). Content parses `set_aggression` in
`content-engine/src/loader/action.rs` and applies it in
`cell/content/executor/world/mod.rs`; GM `.aggression` writes the same in-memory
field in `console/net.rs`. Clearing it is instance-scoped — **but `npc_respawn`
(`ticks/npc_respawn/mod.rs:220-233`) does
not restore it**, so a cleared `aggression` survives every respawn for the space's
life. Contrast `original_interaction_type_flags` (:200-204), which does get
restored.

## `generate_threat` fires on a QR MISS

`apply_damage_to_target` (`abilities/damage_apply/mod.rs`) has exactly three early
returns — :72, :86, :153 — all before or independent of the QR roll (:93-100).
`generate_threat` at :476 runs for every non-lethal resolution, passing
`_total_health_damage as f32` (0.0 on a miss). `threat/aggro.rs:91`
(`*threat_list.entry(a).or_insert(0.0) += amount`) is **outside** the `preemptable`
branch, so a 0.0 add still creates the key. "Did anyone attack since last
cleanup?" is therefore reliably probed by `!threat_list.is_empty()` — misses count.

## `AiState::Submit` is not preemptable

`threat/aggro.rs:69-76` admits only Idle/Patrol/Wander/Investigating/Follow into
`Fighting`. Submit/Leashing/Despawning/Error/Dead cannot be pushed back into
combat by damage. But `set_npc_ai_state` (`executor/world/mod.rs:212`),
`set_follow_target`'s no-target arm (:191), GM console (`console/net.rs:424`) and
`npc_respawn` (:228) can all flip an NPC to `Idle` — which is why an
`aggression = 0` belt-and-braces on a Submit NPC is load-bearing rather than
redundant.
