# HA-03b: combat-side contract for the 1325 / Mala'c duel

Packet HA-03b, advisor note. Docs only. Input: [HA-03a design](HA-03a-rinla-duel-design.md),
[owner decisions](owner-decisions-2026-10-18.md) (O18-10), and
[work-packets.md](../work-packets.md) (rules R1-R9). Output feeds HA-03c (the
`remove_effect` implementation) and the HA-03a maintainer decisions.

Labels: FACT = read in the code in this worktree on 2026-10-18. INFERENCE =
follows from facts, not run. PROVISIONAL = a choice HA-03c or the maintainer may
overturn. Nothing here was verified in game. No ID is allocated: where one is
needed the text says `ID needed: <kind>`, and the only candidates cited are the
chains 6311-6330 already reserved for HA-03 / 1325 in id-allocation.md.

## 1. Verdict

1. FACT: `remove_effect` is parsed (loader) but has no executor arm. It falls
   through to the "Content: unhandled action" arm. HA-03c adds the arm
   (section 2). It is self-only: it acts on the chain's `entity_id`.
2. FACT: Mala'c immobility needs no ability. `spawn_entity` already takes
   `is_stationary`. Use it (section 3.1).
3. PROVISIONAL: no Mala'c invulnerability. Nothing in the engine can target
   Mala'c's health from a chain, and a Stun would stop him attacking. The duel
   resolves by the 30 percent yield and by `entity_dead_tag` as the fallback.
4. FACT: HA-03a's player "movement lock via apply_effect" cannot be a Stun-style
   effect. A ledger-held `BSF_MOVEMENT_LOCK` makes the player incapacitated and
   blocks `useAbility`. The player could not fight. See section 3.2.
5. MAINTAINER DECISION: the `player_died` trigger (HA-03a option A). Without it,
   use the `EF_CLEAR_ON_DEATH` flag and the relog repair (section 4).

## 2. The `remove_effect` executor arm (HA-03c spec)

### 2.1 Current facts

- FACT: `crates/content-engine/src/actions.rs:62` has `RemoveEffect { effect_id: i32 }`.
  `ApplyEffect { effect_id, duration_secs: Option<f32> }` is at line 56.
- FACT: `crates/content-engine/src/loader/action.rs:358-367` builds it as
  `Some(Action::RemoveEffect { effect_id: row.target_id? })`. No loader change.
- FACT: `crates/cell-content/src/cell/content/executor/dispatch.rs` (631 lines):
  `execute_one(chain_id: i64, action: Action, entity_id: u32, player_id: i32,
  params, tx, space_mgr, engine)`. `LaunchAbility` is at 550-587, `ApplyEffect`
  at 588-606, `TriggerChain` at 607, the `other =>` unhandled fallthrough at
  620-629. The new arm goes immediately before `other =>`.
- FACT: `dispatch.rs` is 631 lines, over the 500 soft cap and under the 700 hard
  cap. The arm must be a thin call (under 12 lines). All logic goes in
  `effect_apply.rs`.
- FACT: the `ApplyEffect` arm carries a stale comment (597-601) claiming the
  action is unreachable. HA-03c deletes that comment while it is in the file.
- FACT: `crates/cell-content/src/cell/content/effect_apply.rs` (241 lines) is a
  private module with a security-boundary doc. It holds
  `pub(super) async fn apply_ability_effects(ability_id, target_id, invoker_id,
  chain_id, tx, space_mgr) -> usize` and `pub(super) async fn apply_effect(
  effect_id: i32, target_id: u32, invoker_id: u32, chain_id: i64, tx:
  &mpsc::Sender<CellToBaseMsg>, space_mgr: &mut SpaceManager) -> bool`. Its log
  events are `content_apply_effect_unknown` and `content_apply_effect_no_target`.
  The new helper mirrors `apply_effect`.
- FACT: both `fire_entity_death` (`event_dispatch/lifecycle/mod.rs:189-245`) and
  `fire_entity_health_below` (431-488) execute with the attacker / killer player
  as `entity_id`. So in the duel chains a self-only `remove_effect` hits the
  player. That is what the duel needs (clear the player's root and marker).
- FACT: the seed already has two test chains, 2023 and 2024
  (`db/resources/Content/Seed/effects_chains.sql:95-105`), that call
  `remove_effect` 1635. They are effect-scoped and unreachable today. They become
  live when the arm lands. HA-03c must check what 1635 is and that those chains
  stay harmless (expected: no-op when the effect is absent).

### 2.2 Existing removal machinery to reuse (do not re-implement)

A timed effect lives in one of two places. The arm must clear both.

| Store | Where | Removal call | Evidence |
| --- | --- | --- | --- |
| Pulsing instances | `active_effects` on the entity | `dispatch_on_remove(name, ctx)` per instance, then push `pending_timer_clears` | `crates/cell-world/src/cell/effects/mod.rs:229`; `crates/cell-console/src/cell/console/abilities/clear_effects.rs` `strip_pulses` 99-130 |
| Timed-effect ledger | `TimedEffect` entries (`effect_id`, `ability_id`, `invoker_id`, `effect_flags`, `moniker_ids`, `state_flags`, `duration_secs`, `stacking`) | `strip_timed_effects(entity_id, why, pred, tx, space_mgr) -> usize` | `crates/cell-combat/src/cell/effects/stat_buffs/mod.rs:267`; ledger `remove_timed_effects` at `crates/cell-world/src/cell/effects/stat_buff/ledger.rs:349` |

- FACT: `strip_timed_effects` removes the ledger entries, then calls
  `flush_stat_buff_timers` (121) and `flush_ledger_state_field` (135), then
  stats. It is re-exported through `crates/cell-combat/src/cell/effects/mod.rs`.
  It already releases the counted state-flag references, so a Stun-style flag
  drains by its refcount and not by a bit clear.
- FACT: `StatBuffRemoval` (`ledger.rs:31+`) is the reason enum. The duel code
  uses `DuelEnded` (`crates/cell-world/src/cell/duel/effects.rs`). HA-03c adds a
  variant named `Removed` for this action unless a suitable one exists; check the
  enum first and reuse if one fits (INFERENCE: none named for chain removal).
- FACT: `clear_effects.rs` is the full "strip everything" precedent and its
  `flush_stats` sends to self and witnesses. Copy that fan-out for the pulse path.

### 2.3 Signature and behaviour

```rust
// crates/cell-content/src/cell/content/effect_apply.rs
pub(super) async fn remove_effect(
    effect_id: i32,
    target_id: u32,
    invoker_id: u32,
    chain_id: i64,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> usize   // number of instances + ledger entries removed
```

Dispatch arm (shape only):

```rust
Action::RemoveEffect { effect_id } => {
    let n = effect_apply::remove_effect(effect_id, entity_id, entity_id,
        chain_id, tx, space_mgr).await;
    // log in the helper; arm does nothing else
}
```

Rules (PROVISIONAL, HA-03c may tighten):

1. Self-only. `target_id` is the chain's `entity_id`. No cross-entity removal.
   Reason: matches `apply_effect` and `ChangeStat` (both self-only), and the
   executor's security-boundary doc.
2. Idempotent. Absent effect returns 0, never an error. This is mandatory: the
   yield, dead, abandon and relog chains all call it and any of them may run when
   the effect is already gone.
3. Unknown `effect_id` (no def): log `content_remove_effect_unknown` at WARN and
   return 0. Do not treat as a hard failure.
4. Entity not found or not a cell entity: log `content_remove_effect_no_target`
   at DEBUG and return 0, mirroring `content_apply_effect_no_target`.
5. Order: pulses first (run script `on_remove`, queue timer clears), then ledger
   via `strip_timed_effects` with a predicate on `effect_id`, then one flush.
   Never send two `onStateFieldUpdate` for one removal.
6. No client-visible work beyond what the flushes already send (icon clear via
   the timer clear, `onStateFieldUpdate`, `onStatUpdate`). No new opcode.
7. Do not call `remove_effect` machinery for effects with `EF_` flags that
   forbid removal if the engine has such a flag. HA-03c must grep
   `effect_flags` for a "cannot be removed" bit and report; INFERENCE: none.

### 2.4 Files HA-03c touches

| File | Change |
| --- | --- |
| `crates/cell-content/src/cell/content/executor/dispatch.rs` | Add the `RemoveEffect` arm before `other =>`. Delete the stale comment at 597-601. |
| `crates/cell-content/src/cell/content/effect_apply.rs` | Add `remove_effect`. Watch the 500-line cap (241 now). |
| `crates/cell-world/src/cell/effects/stat_buff/ledger.rs` | Only if a new `StatBuffRemoval` variant is needed. |
| `crates/cell-content/src/cell/content/executor/tests/effects.rs` | New tests (section 6). 375 lines now; if it passes 500, add `tests/remove_effect.rs` and list it in `tests/mod.rs`. |

HA-03c adds no seed, no wire change and no `.def` change.

## 3. Mala'c and the player: the choices

### 3.1 Mala'c immobility: use `spawn_entity` `is_stationary`

- FACT: `SpawnEntity { template_id, position, heading, tag, is_stationary:
  Option<bool>, aggression: Option<i32>, allow_shared: Option<bool> }`
  (`actions.rs`). Guards (`executor/spawn/mod.rs:23-285`): player-source only,
  template cached, refusal in a shared world unless `allow_shared`, same-tag
  idempotence that also matches corpses, and a warning when hostile aggression is
  combined with a faction-0 template.
- FACT: stationary NPC behaviour (`crates/cell-combat/src/cell/service/npc_ai/
  fight.rs:280-320`): it faces its target, holds fire out of range or without
  line of sight, and attacks in range. It never steps.
- FACT: a locked NPC (movement lock) holds and does not launch
  (`fight.rs:104-125`; `crates/cell/src/cell/service/ticks/npc_movement.rs:110-118`
  gives it zero velocity). So a Stun on Mala'c would stop his attacks. Wrong for
  a duel.
- Choice: spawn Mala'c with `is_stationary: Some(true)`. No ability, no effect.
  `ID needed: npc template` and `ID needed: chain` (candidates 6311-6330 only).
- Consequence (INFERENCE): the player can kite out of his range or break line of
  sight and he will not chase. That is acceptable for a duel arena; HA-03d may
  bound the arena by geometry. Do not add a leash change.
- Neutral aggression value: FACT, `MobAggression` (`crates/entity/src/cell_entity/
  aggression.rs:19-70`): 1 Hostile, 2 Suspicious, 3 Neutral, 4 Friendly,
  5 Default (low is hostile). HA-03a's "neutral" is therefore 3. Pre-duel
  Mala'c (the talk stage) is `aggression: Some(3)`; the duel Mala'c is hostile
  (1) and a different tag (see 3.3).
- Warning: a faction-0 template plus hostile aggression logs the spawn warning.
  The template (`ID needed: npc template`) must carry a non-zero faction.

### 3.2 Player root: the ledger lock is the wrong tool

- FACT: `is_incapacitated` is `holds_ledger_flag(BSF_MOVEMENT_LOCK)`
  (`crates/cell-combat/src/cell/abilities/use_ability/incapacitated.rs`). It
  returns error code 15, "You cannot do that while stunned." Stun and Knockdown
  (`crates/cell-effect-scripts/src/cell/effects/crowd_control.rs`) take that
  lock through the ledger (`apply_lock`, `remove_lock`, `CcDuration` NVP).
- Therefore a player held by the ledger lock cannot use abilities, so cannot
  duel. HA-03a's "movement lock via apply_effect self" cannot be implemented as
  a Stun-style effect. FACT.
- Options for "keep the player in the arena", smallest first:
  1. PROVISIONAL, recommended: no player root. Bound the arena by Mala'c being
     stationary and by the yield / abandon chains (section 4). Nothing to clean
     up on the player, so the cleanup matrix is simplest. Cost: the player can
     walk away mid-duel; the duel then stalls until abandon.
  2. A `MovementSpeedMod` snare effect (`ID needed: effect`). INFERENCE: the
     server applies it to speed validation; whether the 2009 client honors the
     speed stat for a locally predicted avatar is UNVERIFIED. Do not claim it
     roots a player.
  3. MAINTAINER DECISION: a bare-flag root action (a content action that holds
     `BSF_MOVEMENT_LOCK` without the incapacitation semantics). It needs a new
     action and a separate "locked but may cast" check. This is a new
     engine primitive; do not build it for HA-03 without a decision.
- Recommendation: option 1. If a feel pass later wants a root, take option 3 as
  its own decision, not inside HA-03.

### 3.3 Mala'c invulnerability: none

- FACT: `ChangeStat` is self-only (`crates/cell-content/src/cell/content/
  executor/stats.rs`) and `apply_effect` is self-only, so no chain action can
  buff Mala'c. No action targets another entity's health.
- Triggers available (FACT): `entity_health_below`
  (`fire_entity_health_below`) and `entity_dead_tag` (`fire_entity_death`), both
  run as the attacking player.
- PROVISIONAL design: the yield fires at 30 percent health via
  `entity_health_below`; the C-YIELD chain removes the player's effects, despawns
  or retags Mala'c by tag, and advances the step. `entity_dead_tag` is the
  fallback for a kill that skips the threshold in one hit (a high-QR crit). Both
  chains must be safe to run in either order (rule 2, idempotence).
- INFERENCE: one hit can skip below 30 percent straight to dead, and the engine
  fires `entity_dead_tag` on that path. HA-03c or HA-03d should still prove the
  threshold chain does not double-advance the step; the mission `advance_step`
  back-step guard (`executor/mission.rs:262-355`) is UNREAD and must be checked.
- If the design owner wants Mala'c unkillable, that is a MAINTAINER DECISION with
  the smallest option: a one-line `health floor` clamp in the damage path keyed
  on a tag. It is a combat change with its own tests. Not recommended for HA-03.

### 3.4 Player death during the duel

- FACT: there is no `player_died` trigger today (HA-03a option A would add one).
- Without it, the death-side cleanup is flag-driven: give every duel effect
  `EF_CLEAR_ON_DEATH` (`clear_stat_buffs_on_death`,
  `crates/cell-combat/src/cell/effects/stat_buffs/mod.rs:290`). The numeric flag
  value and whether death also strips pulsing `active_effects` are UNVERIFIED;
  HA-03c reads the enum and the death path and reports. If pulses survive death,
  the relog repair (section 4) is the backstop.
- MAINTAINER DECISION: add a `player_died` trigger so a chain can reset the
  mission step and despawn Mala'c at the moment of death. Smallest option: one
  new `TriggerType` fired from the death path with the player as `entity_id`,
  mirroring `fire_entity_death`. Until decided, the duel does not reset on
  death; it resets on respawn-to-relog repair or on the abandon path.

## 4. Cleanup on yield, dead, player death, abandon and relog

Role names are HA-03a's (C-YIELD, C-DEAD, C-DEATH, C-ABANDON, C-RELOG).

| Exit | Trigger | What must run | Source of the player's `entity_id` |
| --- | --- | --- | --- |
| Yield (Mala'c at 30 percent) | `entity_health_below` | `remove_effect` for each duel effect on the player; retag or despawn the duel Mala'c by tag; advance step | the attacker player |
| Mala'c killed | `entity_dead_tag` | the same removal set, idempotent; advance only if the step has not advanced | the killer player |
| Player dies | none today | `EF_CLEAR_ON_DEATH` on the effects; on respawn the relog repair resets the step. MAINTAINER DECISION for a real `player_died` chain | n/a |
| Abandon (leave the zone, quit step) | a chain on the abandon trigger HA-03a names | removal set; despawn duel Mala'c by tag; restore the talk-stage Mala'c (`aggression` 3) | the player |
| Relog / reconnect | `player_loaded` | removal set (a stale effect can persist if it was saved); despawn any duel Mala'c by tag; respawn the talk-stage Mala'c per step state | the player |
| Zone 4036 or leaving the instance | whatever HA-03a assigns | same removal set | the player |

Rules:

- FACT: `despawn_by_tag` (`executor/spawn/mod.rs:300`) uses
  `despawn_npc_releasing_combat`, so it also releases `BSF_InCombat` for the
  player. Prefer despawn by tag over killing Mala'c on every non-kill exit.
- FACT: the spawn guard's same-tag idempotence also matches corpses. A killed
  duel Mala'c with the same tag blocks a respawn until the corpse is gone. Use a
  distinct tag per duel round or despawn the corpse by tag before respawning
  (`ID needed: tag`; the tag is a string, not an ID, but must be unique per
  stage and recorded in the HA-03a note).
- Do not mutate shared world 57 / 68 actors per player (R-rule). The duel Mala'c
  must live in a per-player or instanced space, or be spawned with
  `allow_shared` only if the design proves it is invisible to others.
  UNVERIFIED which space 1325 occupies; HA-03d must settle it before seeding.
- Removal set means: every `effect_id` the duel applies (`ID needed: effect`
  per effect, none allocated here). Each exit calls `remove_effect` once per
  effect. Idempotence makes the repeated calls safe.
- Every button press gets visible feedback on first press (CLAUDE.md): the yield
  and abandon exits should send a chat or dialog line through an existing
  action, not silence.

## 5. Decisions needed

| ID | Decision | Smallest option | Default if no answer |
| --- | --- | --- | --- |
| M1 | `player_died` trigger | One `TriggerType` fired from the death path | Not added; relog repair only |
| M2 | Bare-flag player root | New action holding `BSF_MOVEMENT_LOCK` without incapacitation | No player root |
| M3 | Mala'c unkillable | Tag-keyed health floor in the damage path | Not added |
| M4 | Snare as soft root | `MovementSpeedMod` effect, client honor unverified | Not added |

None blocks HA-03c. HA-03c implements only section 2.

## 6. Tests the implementation owes

HA-03c, in `crates/cell-content/src/cell/content/executor/tests/effects.rs` (or
`tests/remove_effect.rs`), using `make_mgr_with_defs()`, `make_pulsing_effect`,
`resolved(Action)` and `execute_actions(resolved(..), 1, 42, &tx, &mut mgr,
&engine).await`. Follow the pattern at lines 285-315. Each test must fail when
the arm is reverted to the unhandled fallthrough.

1. Pulsing instance: apply a pulsing effect, run `RemoveEffect`, assert the
   instance is gone from `active_effects`, the script `on_remove` ran once, and
   a timer clear was queued.
2. Ledger entry: apply a timed ledger effect that holds a counted flag, run
   `RemoveEffect`, assert the entry is gone and the flag drained.
3. Two sources: a second effect holds the same flag; removing the first leaves
   the flag set (refcount, not bit clear). This guards the ref-counted trap.
4. Idempotent: run `RemoveEffect` twice; second returns 0, no error, no second
   `onStateFieldUpdate`.
5. Absent effect: never applied; returns 0 and sends nothing.
6. Unknown `effect_id`: returns 0 and logs `content_remove_effect_unknown`.
7. Self-only: with two entities holding the effect, removal changes only
   `entity_id`.
8. No double flush: one removal sends at most one `onStateFieldUpdate`.
9. Seed chains 2023 / 2024 still execute without panic against a harness with no
   effect 1635.

HA-03d (seed packet) owes a content test that loads the real duel chains and
checks every `remove_effect` target resolves to an effect row, and that every
exit in section 4 calls the full removal set.

## 7. Telemetry the implementation owes

Target `abilities`; use `cimmeria_names::book()` for names, and register new
event names there. Debuggable from SigNoz alone.

| Event | Level | Fields |
| --- | --- | --- |
| `content_remove_effect` | INFO | `chain_id`, `effect_id`, `entity_id`, `pulses_removed`, `ledger_removed` |
| `content_remove_effect_unknown` | WARN | `chain_id`, `effect_id`, `entity_id` |
| `content_remove_effect_no_target` | DEBUG | `chain_id`, `effect_id`, `target_id` |
| `content_remove_effect_noop` | DEBUG | `chain_id`, `effect_id`, `entity_id` |

- NPC-only warnings stay silent: only the player-sourced path logs at WARN.
- Counters: reuse an existing content-action counter if one exists (grep
  `record_` in `cell-content` metrics); do not add a high-cardinality label
  (no per-entity label). INFERENCE: none exists for `remove_effect`.
- Log the removal reason (`yield`, `dead`, `abandon`, `relog`) only if the chain
  can pass it. The action carries only `effect_id`, so the reason comes from
  `chain_id`, which is enough to filter.
