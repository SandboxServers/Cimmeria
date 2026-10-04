# Abilities + Effects System — decisions 16-22

> **Last updated**: 2026-10-03
> **Audience**: Engineers touching combat / abilities / effects on the cell
> **Type**: ADR + reference
> **Status**: Accepted
> **Part of**: [abilities-and-effects-system.md](abilities-and-effects-system.md), which holds the context, decisions 1-15 and the index of every decision. This file holds decisions 16-22 (content-initiated effects, health-band triggers, surrender, death resolution, line of sight, warmup and timers), split out on 2026-10-03 with their numbers and text unchanged.

## Decisions

### 16. Content-initiated effects use a separate entry point, not `handle_use_ability`

**Decision:** Content chains that apply an ability or an effect
(`Action::LaunchAbility`, `Action::ApplyEffect`) call
[`cell/content/effect_apply.rs`](../../crates/cell-content/src/cell/content/effect_apply.rs),
which resolves the effect defs and calls `dispatch_by_name` +
`register_active_effect` directly. They do **not** route through
`cell::abilities::use_ability::handle_use_ability`.

**Why:** `handle_use_ability` is the *client* combat entry point, and its gates
are correct for client input and wrong for content. A scripted debuff — the
Castle Cellblock wake-up "Stasis Sickness" (ability 1372) is the motivating
case — is rejected by it three independent ways:

1. `has_ability` fails. The player never trained the ability and it is not
   weapon-granted, so the check returns false with a WARN.
2. Self-target trips the friendly-fire gate — a player entity may only
   single-target a hostile NPC.
3. The path resolves unconditionally as QR damage. There is no
   offensive-vs-supportive branch, so "apply a debuff" has nowhere to land.

Loosening any of those to admit content would loosen them for forged client
packets too. A second entry point keeps the client gates intact.

**The security constraint — this is the load-bearing part.** The helper
bypasses the combat gates by design, so it is only safe while nothing
client-driven can reach it. Three properties enforce that and none may be
widened without re-deciding this ADR:

- `mod effect_apply;` is declared **without `pub`**, so the path does not
  exist outside `cell::content`. A `use` from a cell method or a Mercury
  handler is a compile error, not a lint.
- Both entry points are `pub(super)` — nameable only from `cell::content`
  and its descendants (in practice `cell::content::executor` and, since
  decision 28, `cell::content::consumable_use`, both private).
- Neither takes a client-supplied id. `ability_id` / `effect_id` come from a
  `content_actions` seed row loaded at startup, or (decision 28) from the
  `items_event_sets` row of an item the base has just consumed from the
  player's own inventory; no wire field reaches them.

Do not re-export these from `cell::content`, and do not add a cell method
that forwards to them. That would turn this into an "apply arbitrary effect
to arbitrary entity" primitive drivable by any forged packet.

**Reversibility:** High. If an ability ever needs both client and content
entry with shared warmup/cooldown/animation semantics, the right move is to
factor the *effect-application tail* of `handle_use_ability` into a shared
function that both call — not to route content back through the gated front
door.

**Code:** [`crates/cell-content/src/cell/content/effect_apply.rs`](../../crates/cell-content/src/cell/content/effect_apply.rs),
dispatched from [`executor/mod.rs`](../../crates/cell-content/src/cell/content/executor/mod.rs).

### 17. `entity_health_below` samples at the damage seams and drains at the engine holders

**Decision:** `pct_before` is sampled inside the two health-application seams —
[`abilities/damage_apply`](../../crates/cell-combat/src/cell/abilities/damage_apply/) (single
target, AoE secondary, cone secondary, and the effect scripts it dispatches) and
[`effects/pulsing/tick.rs`](../../crates/cell-combat/src/cell/effects/pulsing/tick.rs)
(`fire_pulse`) — by
[`combat::note_pre_damage_health`](../../crates/cell-combat/src/cell/combat/damage_credit.rs),
which queues it on the `SpaceManager`. The queue is drained immediately after the hit by
`content::fire_pending_health_below`: the kill-credit wrapper and the pulse tick reach it
through `ContentEvents::pending_health_below` (combat takes `&dyn ContentEvents`, and the
cell passes `EngineEvents(&engine)`; services-crate-split.md §2E), and the
`useAbilityOnGroundTarget` handler and a per-tick safety drain in the cell message loop call
it directly. The pure percentage arithmetic lives in
[`cell/combat/health_threshold.rs`](../../crates/cell-world/src/cell/combat/health_threshold.rs).

**Why a queue rather than a threaded handle.** The trigger needs three things at once: the
target's health on **both** sides of the hit, the attacking player as the acting entity, and
a `&ChainEngine`. The seams have the first two and must not grow the third —
`apply_damage_to_target` is reused by NPC AI, and threading a content handle through the
damage stack would put the content engine in the middle of combat resolution. The queue is
the same scratchpad shape the cone path already uses for kill credit
(`CellEntity::last_aoe_deaths`, decision 13).

**Why not at the kill-credit wrapper.** That is where this originally lived (Harset H04), and
it covered the single-target player path only. Because the predicate is a stateless downward
band, a crossing made on any *other* path is lost **forever** rather than merely late: every
later hit arrives with `pct_before <= threshold`, so the band can never be satisfied again
short of a heal. Missing a sample is therefore not a gap in coverage but a permanent
disarming of the chain — which is why the sample has to sit at the seam every path shares.

**Death is read from `BSF_DEAD`, not `health.cur <= 0`.** This is the load-bearing detail.
An effect script runs after the NVP damage path and outside its `target_died` guard, so a
heal script on a killing blow can leave a corpse sitting at positive health. A health-based
liveness check would then fire a threshold chain on that corpse. The dispatcher
(`fire_health_below_for_hit`) drops any hit whose target ends dead, keeping the zero-health
check alongside the flag for an entity that is at zero but not yet marked. The `!just_died`
branch in the wrapper is defence in depth, not the enforcement — the authority is at the
dispatcher, at the one place that knows the hit was lethal.

**A DoT kill is a real kill.** Closing the pulse gap exposed a second one: `fire_pulse`
writes the HEALTH stat directly, so none of `apply_damage_to_target`'s death machinery ran.
A mob finished by a tick sat at zero health, never flipped `BSF_DEAD`, dropped no loot and
fired no `entity_dead_tag` — a kill-count mission stalled whenever the killing blow happened
to be a tick rather than a shot. The pulse tick now routes through the canonical
`abilities::kill_npc_out_of_band` (the GM-kill primitive, renamed and given an
`attacker_is_player` flag) and then fires `EntityDeath` for the effect's invoker. Ordering is
load-bearing: death first, threshold drain second, so the corpse already carries `BSF_DEAD`
when the dispatcher decides whether to suppress the threshold chain.

**Death is still read from `BSF_DEAD`, not `health.cur <= 0`** — see below; the exclusivity
contract ("exactly one of `entity_dead_tag` and `entity_health_below` per hit") now holds on
the pulse and AoE paths too.

**Reversibility:** Moderate. The queue is one `Vec` on `SpaceManager` and two call sites; a
future path that mutates health without going through either seam needs its own
`note_pre_damage_health` call, and the per-tick safety drain bounds how long a sample from a
forgotten drain site can sit unfired.

### 18. A surrendered NPC is immune to the killing blow from an automatic damage source, not to damage

**Decision:** `fire_pulse` floors an `AiState::Submit` target's `HEALTH.cur` at 1 instead
of letting a pulse take it to zero. The pulse still lands its full damage; only the killing
blow is refused. Direct hits are untouched, so a single deliberate shot still kills a
surrendered NPC.

**Why it is needed at all.** Decision 17's second half made a damage-over-time kill a real
kill. Before that, a lethal pulse left the mob standing at zero health, which was a bug but
was also, accidentally, harmless to Harset H08: the packet that makes an NPC's surrender
stick. After it, a DoT the player applied *before* the surrender walks the NPC to a corpse
a few seconds later. H08 stops the auto-attack loop, and the mob dies anyway on a different
clock.

**Why the line is drawn at "automatic", and where that line is.** H08's rule is that the
surrender survives everything the *server* re-delivers on its own cadence, and changes
nothing about what a player does deliberately. A pulse is on the automatic side by the same
definition
[`combat::is_auto_cycle_target_valid`](../../crates/cell-combat/src/cell/combat/auto_cycle.rs)
uses for the auto-fire loop: the deliberate act was applying the effect, and every tick
after it is the scheduler's. So the two guards are one rule applied at two seams, and
neither of them touches a direct hit.

This does mean a player who *wants* to finish a surrendered NPC cannot do it with a DoT.
That is a real, narrow change to explicit-attack behaviour, taken knowingly: the alternative
(strip hostile pulsing effects at the moment of surrender) leaves a ~2 s window, because the
surrender handler runs on the AI tick while pulses run at 100 ms.

**Why in `fire_pulse` rather than in `dot_kill_credit`.** The dirty-stat flush at the bottom
of `fire_pulse` is the same pulse's `onStatUpdate` broadcast, so clamping above it means the
client is told `1` and never renders a zero-health frame. It also collapses two guards into
one: `dot_kill_credit`'s existing `cur > 0` probe early-outs on the floored value without
needing its own `ai_state` check that could drift out of sync.

**Not a faction flip, and not an immunity flag.** `faction` gates whether an offensive
ability may target the entity at all, so flipping it would make the surrendered NPC
unattackable and would read it as an ally to every other mob's idle scan. There is no
`BSF_*` bit for "cannot be finished"; `ai_state == AiState::Submit` is the durable fact and
both guards read it directly.

**What is still lethal to a surrendered NPC** — all deliberate, all recorded rather than
changed: a direct single-target hit, a ground- or cone-AoE secondary (one ability press, so
the splash that catches a surrendered bystander is deliberate at the press), a content chain
that applies damage through `effect_apply`, and the GM kill primitive. Harset H21's duel
carries an `entity_dead_tag` fallback chain for exactly this reason (H04 worknote,
integration request A).

**Reversibility:** High. One `if` in `fire_pulse` and one sentence of the `dot_kill_credit`
doc comment; deleting both restores the pre-H08 behaviour and fails
`a_dot_cannot_finish_a_surrendered_npc`.

### 19. Every death resolves through one function, including an effect script's killing blow

**Decision:** [`abilities::death::resolve_death`](../../crates/cell-combat/src/cell/abilities/death/mod.rs)
is the only place a death happens. It owns the kill-site state mutations, the ordered wire
burst, the threat drain, the death animation, kill XP, and the player Defeat Window.
`damage_apply` calls it twice per hit — once for direct damage, once as a sweep after the
effect scripts have run — and `kill_npc_out_of_band` (GM `.kill`, DoT pulse) is a thin
NPC-only wrapper over it. A `BSF_DEAD` probe at the top makes it idempotent, so the second
call costs one lookup when the first already killed.

**The bug it closes.** Effect scripts write `HEALTH` directly. `RangedPhysicalDamage`'s
Focus-pierce bleed, `MeleePhysicalDamage`, `RangedEnergyDamage` and `Suppression` all end in
`stat.update(min, (cur - damage).max(0), max)` with no death check — and they are dispatched
at the *bottom* of `apply_damage_to_target`, long after its own `target_died` probe. A shot
whose direct damage left the NPC standing could therefore take it to zero with nothing
noticing. Playtest 2026-09-19 (Castle Mess Hall): pistol auto attack (ability 579, effect
641) bled `MessHall_Guard1` to 0 HP. The mission's `entity_dead_tag` fired — the kill-credit
wrapper reads the health stat, so *it* saw the death — but no death transition ran. The
guard stayed at 0 HP with `ai_state == Fighting`, hit the player for 86 damage 0.6 s later,
and only became a corpse 1.5 s on, when the player's next shot re-entered the direct-damage
arm. Loot, XP, the `BSF_InCombat` clear and the auto-cycle stop were all 1.5 s late; the
death event and the death transition were credited to different shots.

**Why the sweep sits in `damage_apply` and not in the scripts.** A script holds
`&mut SpaceManager` through a synchronous [`EffectContext`](../../crates/cell-world/src/cell/effects/mod.rs)
and cannot await the wire burst. Pushing lethality handling into each script would also mean
every future HEALTH-touching script has to remember it — the same omission that produced
this bug, re-armed nine times over. The sweep is unconditional rather than gated on "did a
script run": any post-`calculate_damage` step that zeroes HEALTH should produce a corpse,
and a target arriving at the end of the function at 0 HP without `BSF_DEAD` is something to
fail safe on.

**Kill credit is unaffected.** `handle_use_ability_with_kill_credit` and
`fan_out_cone_effects` both detect deaths by comparing the HEALTH stat before and after the
whole ability resolution, which already saw effect-driven zeroes — that is why the mission
event fired on time while the transition did not. The fix moves the transition onto the same
shot; it adds no second credit window, and the corpse's `BSF_DEAD` still suppresses credit
on a follow-up shot.

**Defence in depth in the AI tick.** `npc_ai_tick` and `npc_ai_retry_sweep` now drop any NPC
whose HEALTH is at or below zero, before the `ai_state` filter. `mark_npc_dead` stamping
`ai_state = Dead` remains the primary mechanism and this filter should be redundant; it is
not free redundancy, because the playtest symptom was precisely an NPC acting on a state the
combat layer already considered finished. A zeroed NPC *without* `BSF_DEAD` warns on the way
out, per [`negative-logging-convention.md`](negative-logging-convention.md).

**Behaviour changes folded in, same class of bug.** `kill_npc_out_of_band` previously ran the
transition but skipped the death animation and the XP grant, so a DoT kill produced a silent
corpse worth nothing. Routing it through `resolve_death` fixes both. Because a GM `.kill`
shares that entry point, `grant_xp` is an explicit parameter — the DoT path passes `true`,
the GM path `false`, so an admin command cannot mint levels.

**Reversibility:** High. The sweep is one `if` in `damage_apply`; the AI filter is one
`.filter(...)`. Deleting either fails
`effect_script_bleed_to_zero_runs_death_transition_in_same_resolution` /
`zero_health_npc_without_dead_bit_gets_no_ai_turn` respectively.
(`npc_killed_by_an_effect_bleed_does_not_shoot_back` is the end-to-end cover for
both together; on its own it cannot isolate the AI filter, because the death it
resolves also stamps `AiState::Dead`.)

### 20. A player's targeted ability needs line of sight at fire time (NA31, D-NA14)

**Decision:** `handle_use_ability` runs `fire_los::refuse_without_line_of_sight` straight
after the range check. The check covers a player attacker using an ability aimed at another
entity (`target_type_id` not `TargetSelf` or `TargetGround`) in a world with a
collision-geometry occluder. When the eye ray and every tolerance ray are blocked, the
ability is refused with `onErrorCode(0, ability_id, 39)`. The tolerance rays are the target
one tick back, the shooter one tick ahead, and 0.35 m to each side of the target. The
refusal comes before the holster queue, the cooldown and the ammo check, so a refused shot
costs nothing.

**Why these limits:** the navmesh ray reads furniture as walls (NA16), so no occluder, or an
eye off its grid, never refuses. NPC launches are not re-checked, because the fight tick
checked them in the same tick. Ground-target and AoE collection are unchanged: they aim at a
point or a volume, not an entity's eyes. The gameplay rules are in
[combat-system.md](../gameplay/combat-system.md#fire-time-line-of-sight).

**Reversibility:** High. The gate is one `if` in `handle.rs` and one pre-gate in the
auto-cycle tick. Removing it fails `a_shot_through_the_hallway_walls_is_refused_with_error_39`.

**Same space first (#906).** The same fire-time check refuses a player's cast whose target is
not in the caster's space, before any ray and whatever the ability's target type or the
world's occluder. `get_entity` searches every space, so a target id from another instance at
nearby coordinates used to pass the `distance_to` range check and take damage and threat. The
refusal is `onErrorCode(0, ability_id, 0)` (`CONDITION_FEEDBACK_InvalidEntity`, as the pet
bar's `target_other_space`) and an `abilities` DEBUG row `event=cast_refused
reason=target_other_space` with the caster's `account_id` and `player_id`. The auto-cycle tick
treats a target in another space as gone and stops the loop. Removing the check fails
`a_target_in_another_space_is_refused_at_launch`.

### 21. Warmup is a pending cast per caster, fired by the 100 ms tick (AT-10)

**Decision:** `handle_use_ability` is the launch half of a cast. It validates, charges the
cooldown for `cooldown + warmup`, and sends the cooldown timer. With a zero warmup it then
calls `fire::fire_cast` in the same pass, and the wire is unchanged from before AT-10. With a
positive warmup it sends `Ability_Begin` and the `AbilityWarmup` (type 1) timer, and parks a
`PendingCast` on the caster (`CellEntity.pending_cast`, indexed by
`SpaceManager.pending_casts`). `warmup::warmup_tick` runs every AoI tick. It interrupts a
caster that has moved, re-validates each cast whose warmup has expired, and fires it through
the same `fire_cast`: ammo, `Ability_End`, channel cancel, damage, cone fan-out, auto-reload.
Each of those runs once per cast, in the fire phase. NPC casters use the same path, and the
NPC fight tick holds while its NPC is casting. A ground-target cast parks its ground point
with the primary, and its secondaries are collected when it fires.

One primitive, `warmup::interrupt_pending_cast`, cancels a warmup. It refunds the cooldown
and sends the player a zeroed warmup timer and a zeroed cooldown timer, then sends
`Ability_Interrupt` (1002) to the caster and witnesses. It also stops the auto-cycle loop if
the interrupted ability is the loop's. The triggers are:

| Trigger | Source | Where |
|---|---|---|
| Caster death | python `onDead` → `interruptAbility` | `death::apply_death_transition`, beside the channel cancel |
| Active bandolier slot change | python `onBandolierSlotChange` | `handle_request_active_slot_change`, when the slot differs |
| Caster moves ≥ 0.5 m (planar), unless `AF_CHANNEL_ALLOWS_MOVEMENT` | channel rule (decision 11); `SGWAbilityManager.def` pairs `lastWarmUpInterruptTime` with `lastChannelInterruptTime` | `warmup_tick` |
| Caster in another space (whatever the flag) | Rust addition | `warmup_tick` |
| At fire: a player's active-slot weapon is not the one it launched with | python `onBandolierSlotChange` also covered "the active item was swapped/removed" | `warmup_tick` |
| At fire: target gone, dead, in another space, or no longer a valid target (#444; a support shot's ally stays valid, AM-11d) | Rust addition | `warmup_tick` |
| At fire: target beyond range (sends `onErrorCode` 42) | Rust addition, the launch's own check | `warmup_tick` |
| At fire: no line of sight for a player (sends `onErrorCode` 39) | Rust addition, decision 20 | `warmup_tick` |
| At fire: a player's weapon is reloading or short of ammo | Rust addition | `warmup_tick` |

A second launch while a cast warms up is refused silently, whatever the ability. Python
`canUseAbility` refused while `currentAbility` was set.

**Why:** Python (`AbilityInstance.launch` / `afterWarmup` / `interrupt`) is the only
reference for the split, and it is followed where it speaks: the cooldown starts at launch
and covers the warmup, ammo is spent at the fire, and the speed stats shorten the warmup.
Death and slot change interrupt, the cooldown is refunded, and the cancel is the zeroed
warmup timer plus `Ability_Interrupt`. Python's `afterWarmup` re-checked nothing and applied
the effects to a dead or distant target. The fire-time checks are the conservative
server-authoritative choice: a cast the launch would refuse is refused at the fire, with the
same error codes. The zeroed cooldown timer is an addition, because python refunded the
server cooldown without telling the client. The loop stop is an addition too, because
without it a refunded auto-cycle ability relaunches on the next tick. A stun does not
interrupt: nothing in python or in the Rust launch path gates on a stun, and a stun is only
`BSF_MOVEMENT_LOCK`, which ring transport and death also set.

**Reversibility:** High for the triggers: each is one call. The launch/fire split itself is
load-bearing. `warmup_damage_waits_for_the_warmup_and_lands_once` fails if the fire goes back
into the launch pass, and `zero_warmup_wire_is_unchanged` pins the zero-warmup bytes.

**Code:** [`use_ability/handle.rs`](../../crates/cell-combat/src/cell/abilities/use_ability/handle.rs)
(launch), [`use_ability/fire.rs`](../../crates/cell-combat/src/cell/abilities/use_ability/fire.rs)
(fire), [`use_ability/warmup/`](../../crates/cell-combat/src/cell/abilities/use_ability/warmup/mod.rs)
(park, tick, interrupt), [`dispatch/mod.rs`](../../crates/cell-combat/src/cell/abilities/dispatch/mod.rs)
(`fire_ground_cast_after_warmup`). Evidence and test list:
[AT-10 worknote](../analysis/ability-trees/worknotes/at10.md).

**Extended (ability mechanics AB-09c, 2026-10-03): an interrupt effect is a fourth trigger.**
An effect script cannot reach this cancel: it is synchronous and below combat. The
`Interrupt` script ("Interrupts target", effect 723 of Interrupting Shot) and the EMP round's
`EmpDisrupt` (at its `InterruptChance`, 25 %) queue an `InterruptRequest` on the
`SpaceManager` ([`interrupt_request.rs`](../../crates/cell-world/src/cell/effects/interrupt_request.rs)).
Combat resolves the queue in `flush_stat_buff_timers`, which every caller of a script already
awaits, so the interrupt reaches the client in the hit's burst, and on the stat-buff tick as a
safety net ([`effects/interrupt.rs`](../../crates/cell-combat/src/cell/effects/interrupt.rs)).
A target with neither a warmup nor a channel is not rolled. Otherwise it resists with
probability `(interruptRes + coordination) / 1000`, clamped to 0..=1: `alias.xml` makes
`interruptRes` a resistance to every interrupt but movement and gives coordination "+0.1%
resistance to interrupts per point", and D-AB09's 10 points per 1 % puts both in one unit.
`P(interrupt) = InterruptChance x (1 - resist)`, rolled from a seed of the source, the target,
the effect and the target's warmup instance. DESIGN, not recovered. When it lands the target's
warmup goes through `interrupt_pending_cast` with reason `interrupt_effect` (refund, zeroed
timers, `Ability_Interrupt`, no lockout, exactly as above) and its channels through
`cancel_channels_from_attacker`. One `abilities` `interrupt_effect` row per request:
`decision_outcome` `interrupted`, `resisted` or `nothing_to_interrupt`, the interrupter as
actor, the target as subject. Guards: `use_ability/tests/interrupt_effect.rs`.

### 22. Timer expiries are absolute on one server-wide game clock (CR-02)

**Decision:** Every `onTimerUpdate` that starts a timer sends
`BigWorldTimeComplete = game_clock::game_time_secs() + duration`: the ability cooldown
(`TotalTime = cooldown + warmup`), the warmup timer, the reload timer, the duration-effect
timer and `.net_timer`. A timer that clears sends `0.0`. The clock lives in
[`crates/wire/src/mercury/game_clock/`](../../crates/wire/src/mercury/game_clock/mod.rs): one
epoch pinned at server start, 10 ticks per second, and the same tick count in the login
bundle (`TICK_SYNC`, `SET_GAME_TIME`) and in every heartbeat.

**Why:** The client's game clock is `TICK_SYNC.gameTime / hertz` seconds, and its cooldown,
effect, reload and crafting handlers all compare `BigWorldTimeComplete` against it
([system-protocol-wire-formats.md](../reverse-engineering/findings/system-protocol-wire-formats.md#the-client-game-clock)).
Before CR-02 each session counted ticks from its own login and the login bundle sent 0, so
no absolute expiry could mean the same thing to two clients, and the senders passed 0.0 (no
cooldown shown) or the relative duration (no effect icon once the clock passed it). Python
sent `Atrea.getGameTime() + duration` (`AbilityManager.py:605`, `Net.py:93`). The epoch is
server start, not Unix time, because the field is an `f32`.

**Reversibility:** High per sender, one expression each. The clock's rate is pinned by
`declared_frequency_tick_period_and_send_interval_agree` and the byte-exact time-sync tests.

**Known limits:** the client's clock runs about one tick ahead of the server's, so a client
cooldown ends up to 0.1 s early. A server restart resets the clock; never persist an
absolute expiry. Category (type 8) cooldown timers are still not sent. Evidence and test
list: [CR-02 worknote](../analysis/crafting/worknotes/cr-02.md).

**Routing (2026-09-29): the owning player's client only.** Every cell `onTimerUpdate` goes
through `send_timer_update`
([`timer_update.rs`](../../crates/cell-combat/src/cell/abilities/timer_update.rs)), which
sends to the entity's own client when it is a player and sends nothing otherwise. The client
binds `Event_NetIn_TimerUpdate` on `SGWPlayer` alone (see
[the binding table](../protocol/client-method-dispatch-table.md#client-handler-bindings)), so
an NPC or pet cooldown, and a duration timer on an NPC target, sent to that NPC's witnesses
was dropped by the dispatcher on arrival: 191 `client.dispatch.method_dropped` rows (method 12,
type 4) in one colo session on 2026-09-29, one per NPC shot. The witness-fanout helpers refuse
method 12 for a non-player entity with a WARN (`reason = no_client_binding`,
target `cimmeria_cell_combat::cell::abilities::messaging`), so a new caller that bypasses the helper is visible in SigNoz.
Python sent ability timers to `ent.client` only (`AbilityManager.py:611-652`); its
`updateEffectTimer` also sent effect timers to `ent.witnesses` (`:825`), which only a player
target's witnesses could use. The drop never cost an animation: a dropped method is skipped
whole and the NPC's `onSequence` in the same bundle still dispatches.
