# Abilities + Effects System

> **Last updated**: 2026-09-27
> **Audience**: Engineers touching combat / abilities / effects on the cell
> **Type**: ADR + reference
> **Owner**: Combat systems
> **Status**: Accepted (shipped in PR #420)
> **Confidence**: High — every decision below is backed by code + tests in the same PR

## Context

PR #420 ([#47](https://github.com/SandboxServers/Cimmeria/issues/47), [#61](https://github.com/SandboxServers/Cimmeria/issues/61), [#331](https://github.com/SandboxServers/Cimmeria/issues/331), [#419](https://github.com/SandboxServers/Cimmeria/issues/419)) lit up the abilities system end-to-end: per-weapon resolution, hotbar population, trainer NPCs, effect-script dispatch, pulsing DoT/HoT, cone AoE, absorption shields, stun/suppression debuffs, channelled effects with movement-interrupt. Most of the engine code was new — but the cross-cutting design decisions deserve their own record so future contributors can extend the system without re-litigating them.

This doc captures **what** was decided and **why**, with pointers to the code that implements each piece. It is **not** a tutorial — for module-level walkthroughs read the inline docstrings on `cell::effects::mod.rs`, `cell::effects::pulsing`, and `cell::abilities::cone_aoe`.

## Decisions

### 1. EffectScript trait shape: `on_apply` + `on_remove`, no per-pulse callback

**Decision:** `EffectScript` exposes two methods — `on_apply(ctx)` and `on_remove(ctx)` with a default no-op for the latter.

**Why:** The Python reference (`AbilityManager.py`) had `on_pulse_begin`, `on_pulse_end`, `on_effect_init`, `on_effect_removed` — four lifecycle hooks. We collapsed to two because:

- **The initial pulse and every subsequent pulse run the same logic** for every script we've seen (heal re-heals, DoT re-damages, suppression re-chips). There's no use case yet for "do something different on pulse N vs pulse 1." Adding the split now would be speculative API surface.
- **`on_remove` is genuinely different** — it has to undo persistent state (Stun's `BSF_MOVEMENT_LOCK`, AbsorbShield's residual pool). Default empty impl means stat-mutation-only scripts (HealHealth, HealFocus, MeleeDamage, MeleePhysicalDamage) don't need to override.

**Reversibility:** Adding `on_pulse_begin` / `on_pulse_end` later is additive — existing scripts get default-empty impls, no migration. **Trapdoors:** none.

**Code:** [`crates/cell-world/src/cell/effects/mod.rs`](../../crates/cell-world/src/cell/effects/mod.rs) — trait definition + `dispatch_by_name` / `dispatch_on_remove` helpers. The synchronous layer (trait, context, registry, scripts) is in `cimmeria-cell-world` because the spawn-time cover hold runs Cover Stance through it; the async pulsing scheduler ([`effects/pulsing/`](../../crates/cell-combat/src/cell/effects/pulsing/)) is combat's, in `cimmeria-cell-combat`, whose `cell::effects` re-exports the synchronous layer beside it; `cimmeria-services` re-exports that module at the same path ([services-crate-split.md](services-crate-split.md)).

### 2. Active-effect storage on the target, not the source

**Decision:** `ActiveEffectInstance` lives in `CellEntity.active_effects` on the **target**, with `invoker_id` pointing back at the source.

**Why:** Two real workloads decide this:

1. **Per-tick pulse fire** walks targets and fires their due effects. Storage-on-target makes this a single linear scan per cell tick over a Vec that's empty for most entities.
2. **"Cancel all channels by attacker X"** (channel-interrupt, channeller death) walks all targets and filters by `invoker_id`. O(N) over all entities, but N is small (~hundreds per cell) and channels are rare.

Alternative considered: storage-on-source with target_ids in the instance. Rejected because the per-tick "what should I tick right now" question dominates frequency over the "who did this come from" question.

**Reversibility:** Switching to source-side storage would require a single-pass migration of the `active_effects` field; both queries stay O(N) just over different sets. Not a permanent commitment.

**Code:** [`crates/entity/src/cell_entity/mod.rs`](../../crates/entity/src/cell_entity/mod.rs) (`ActiveEffectInstance` struct + `active_effects` field), [`crates/cell-combat/src/cell/effects/pulsing/mod.rs`](../../crates/cell-combat/src/cell/effects/pulsing/mod.rs).

### 3. Refcount lifecycle via existing `state_flag_counts`

**Decision:** Stun reuses the existing `set_state_flag` / `unset_state_flag` helpers (refcounted via `state_flag_counts`). No new per-stun-source tracking added.

**Why:** [`state-flag-conventions.md`](state-flag-conventions.md) already established the refcount pattern for `BSF_MOVEMENT_LOCK`, anticipating "future stun, cast, fear, knockback effects." The helpers do exactly what multi-source stun stacking needs:

- Two stuns from different invokers → counter = 2 → bit set
- First expiry → counter = 1 → bit STAYS set
- Second expiry → counter = 0 → bit cleared

The PR initially added a separate `movement_lock_reasons: HashSet<(u32, i32)>` field before realising the existing infra solved the problem. **The walked-back diff is a useful breadcrumb**: if someone hits the same false-positive read in the future, the answer is "the existing helpers work, write a regression test rather than a new mechanism."

**Reversibility:** Already the simpler design — no commitment to walk back.

**Code:** [`crates/entity/src/cell_entity/state_flags.rs`](../../crates/entity/src/cell_entity/state_flags.rs), `Stun::on_apply` / `on_remove` in [`crates/cell-world/src/cell/effects/scripts.rs`](../../crates/cell-world/src/cell/effects/scripts.rs).

### 4. Stacking semantics: same-source refresh, multi-source stack

**Decision:** `register_active_effect` does same-source refresh (same `invoker_id` + `effect_id` updates the existing instance) and multi-source stack (different `invoker_id` adds a new instance).

**Why:** Matches Python `AbilityManager.addEffect`. Prevents trivial DoT spam from a single attacker, while still letting multiple players DoT a boss in parallel. Refresh updates `remaining_pulses`, `next_pulse_at`, and `invoker_position_at_register` so the duration genuinely resets.

**Reversibility:** Trapdoor — content authored to assume same-source stacking would break if the rule changes. None of our seed currently assumes this either way, so we're free to revise. Document the rule clearly so content authors don't drift.

**Code:** `register_active_effect` in [`crates/cell-combat/src/cell/effects/pulsing/mod.rs`](../../crates/cell-combat/src/cell/effects/pulsing/mod.rs).

### 5. Pulsing model: initial pulse + N-1 follow-ups

**Decision:** When an effect with `pulse_count = N` lands, `damage_apply` fires the initial pulse synchronously, then `register_active_effect` registers an instance with `remaining_pulses = N - 1`. The per-tick loop fires the remaining pulses.

**Why:** Two reasons:

1. **Wire-side immediacy.** Players hitting "fire" expect the first damage tick to land on the same tick as the cast, not 0.5s later. The initial pulse goes through the same code path as a single-shot effect, so wire packets (`onEffectResults`, `onStatUpdate`) fire in the same order.
2. **Simpler scheduling.** `next_pulse_at = now + pulse_interval` always points to a FUTURE pulse. No special "first pulse is now" branch in the tick loop.

For channelled effects (`pulse_count = 0`), we register with `MAX_CHANNEL_PULSES = 60` as a safety cap so a missed cancellation event can't leak indefinitely.

**Reversibility:** Reversible — could move the initial pulse into the tick loop by setting `next_pulse_at = now` and skipping the synchronous fire. Would defer first damage by up to 100ms (one tick), which is noticeable in playtest.

**Code:** [`crates/cell-combat/src/cell/abilities/damage_apply/mod.rs`](../../crates/cell-combat/src/cell/abilities/damage_apply/mod.rs) (initial pulse), [`crates/cell-combat/src/cell/effects/pulsing/mod.rs`](../../crates/cell-combat/src/cell/effects/pulsing/mod.rs) (registration + tick).

### 6. Channel cancellation triggers

**Decision:** Channelled effects (DB `pulse_count == 0`) cancel on:

1. **Channeller fires a different ability** — `handle_use_ability` calls `cancel_channels_from_attacker(attacker, Some(ability_id), ...)` so the same ability re-fires as a refresh
2. **Channeller dies** — `apply_death_transition` calls `cancel_channels_from_attacker(target_eid, None, ...)`
3. **Channeller moves > `CHANNEL_INTERRUPT_DISTANCE` (0.5m)** — `channel_interrupt_on_movement_tick` runs before the pulse tick
4. **Safety cap of 60 pulses elapses** — instance simply ages out via the normal `remaining_pulses → 0` sweep

**NOT cancelled by:**

- Target moving (only the channeller's movement matters)
- Target dying (the channel completes its remaining pulses against a dead target, which no-op via the per-pulse dead-target guard)

**Why:** The four triggers match the conventional MMO model. Movement-cancel is the only one with a per-ability override (`AF_CHANNEL_ALLOWS_MOVEMENT`, default 0) — we expect ~all channels to cancel on move with rare exceptions; the inverse default would require flipping the flag for nearly every channel ability and would break sustained-stand-still designs.

**Reversibility:** Per-trigger thresholds (0.5m, 60 pulses) are tunable. Adding new cancel triggers is additive. Removing existing ones risks breaking content authored to rely on them.

**Code:** [`crates/cell-combat/src/cell/effects/pulsing/mod.rs`](../../crates/cell-combat/src/cell/effects/pulsing/mod.rs) (`cancel_channels_from_attacker`, `cancel_channels_for_invoker_ability`, `channel_interrupt_on_movement_tick`).

### 7. AF_CHANNEL_ALLOWS_MOVEMENT default = 0 (cancel-on-move)

**Decision:** The Cimmeria-side `AF_CHANNEL_ALLOWS_MOVEMENT` ability flag (bit 20, `1 << 20`) defaults to 0 (off) across every authored ability. Operators flip it per-ability as content arrives that should be movement-tolerant. It was first defined as 16384 (bit 14), but that bit is the client's `EAbilityFlags::SpeedPet` (`entities/defs/enumerations.xml:51`), which the seed sets on the 14 summon abilities. So those summons were exempt from the warmup move interrupt until pets PT-03 moved the flag above the client enum's highest token (`PetCommand` = 65536). No seed row sets bit 20.

**Why:** Cancel-on-move is the safe default — players who walk away from a channel expect it to stop. The inverse default would silently let channels persist across movement events the player doesn't realise are happening, which is a bug shape ("why is my buff still ticking after I rezoned?"). Opt-in to movement-tolerant via flag flip.

**Reversibility:** Per-ability flag, so changing one ability's behaviour is one DB row update. No engine commitment locked in.

**Code:** [`crates/entity/src/abilities/mod.rs`](../../crates/entity/src/abilities/mod.rs) (flag constant + docstring).

### 8. TCM dispatch routing: Single (always), Radius (ground-target), Cone (cone fan-out)

**Decision:** Three target-collection methods route through three different code paths:

| TCM | Effect rows (DB seed) | Route |
|---|---|---|
| `TCM_Single` | 2,795 (87%) | `apply_damage_to_target` (primary only) |
| `TCM_AERadius` | 300 (9%) | `handle_use_ability_on_ground` for ground-targeted; primary-only for everything else |
| `TCM_AECone` | 99 (3%) | `cone_aoe::fan_out_cone_effects` after primary commits |

**Why:** Each TCM has a different anchor and different geometry, so a unified "collect_targets(tcm, args)" entrypoint would push the dispatch one layer deeper without removing the per-TCM code. Three call sites match three real call paths.

**Caveat:** Single-target abilities with a `TCM_AERadius` effect attached (e.g. proximity-mine detonations) don't yet fan out at primary-cast time. Those need an explicit "detonate" trigger, not a fan-out on cast. Flagged as a follow-up.

**Reversibility:** Adding new TCM values is additive — add a fourth route. Re-routing existing TCMs is risky (changes content behaviour).

**Code:** [`crates/cell-combat/src/cell/abilities/cone_aoe/mod.rs`](../../crates/cell-combat/src/cell/abilities/cone_aoe/mod.rs), [`crates/cell-combat/src/cell/abilities/dispatch/mod.rs`](../../crates/cell-combat/src/cell/abilities/dispatch/mod.rs).

### 9. Absorption pool drain: elemental-specific first, generic catch-all second

**Decision:** When physical damage arrives, the drain order is `ABSORB_PHYSICAL` → `ABSORB_PHYSICAL_ENERGY` → `ABSORB_PHYSICAL_ITEM` → done (no overflow into untyped pools). Only HEALTH damage triggers absorption — FOCUS damage bypasses.

**Why:**

- **Elemental-specific first** matches player intent — a player who applied a "+200 physical absorb" buff expects it to consume on physical hits before generic shields drain
- **Three sub-pools per damage type** (the `_ENERGY` and `_ITEM` suffixes) come from the original game's stat schema; we honour the existing schema rather than collapse them
- **Only HEALTH absorbs** because the existing Python ref drains the same way; focus drains are typically resource-pressure mechanics (not damage to mitigate)

**Trapdoor:** The `_ENERGY` and `_ITEM` suffixed pools have no content driving them today — they'll only have non-zero `cur` once content adds effects that grant capacity to them.

**Reversibility:** Drain order is per-damage-type table inside `drain_absorption_pools`; trivially swapped. Changing the HEALTH-only rule means understanding the FOCUS-drain content semantics first.

**Code:** [`crates/cell-combat/src/cell/combat/damage/pipeline.rs`](../../crates/cell-combat/src/cell/combat/damage/pipeline.rs) — `drain_absorption_pools` + `calculate_damage`.

### 10. Script-name dispatch over flag-bit dispatch for effect categories

**Decision:** Effect categories (heal, damage, stun, suppression, shield) are selected via `EffectDef.script_name` — a string lookup in the registry. The `EffectDef.flags` bitmask is honoured for observability (`EF_STUN`, `EF_SUPPRESSION`, etc. log on apply) but does NOT drive dispatch.

**Why:** The original game's content has both — flags for category, script_name for behaviour. We chose script_name as the canonical dispatch key because:

- Adding a new script doesn't require allocating a new flag bit (32 bits is tight; the game already uses ~10)
- Scripts can take arbitrary NVPs without needing per-flag schema columns
- A single ability can have multiple effects each with different script_names; flag bits would conflate them

**Reversibility:** Could route flags into the dispatcher later (add a "if flags & EF_STUN, also run Stun" path) without breaking script_name routing.

**Code:** [`crates/cell-world/src/cell/effects/registry.rs`](../../crates/cell-world/src/cell/effects/registry.rs).

### 11. Channel-interrupt distance = 0.5m

**Decision:** `CHANNEL_INTERRUPT_DISTANCE = 0.5` world units.

**Why:** Walking is ~3 m/s — a 0.5m budget catches step-off-the-spot intent within ~150ms of a player input. Tighter (e.g. 0.1m) would fire on the ~3cm position jitter that comes from server-side movement smoothing; looser (e.g. 2m) would let players strafe through a substantial arc before the interrupt notices.

The 0.5m number is a guess pending playtest feedback — if it's too aggressive, it's one constant to bump.

**Reversibility:** Single constant, no schema commitment.

**Code:** [`crates/cell-combat/src/cell/effects/pulsing/mod.rs`](../../crates/cell-combat/src/cell/effects/pulsing/mod.rs).

### 12. Channel safety cap = 60 pulses

**Decision:** `MAX_CHANNEL_PULSES = 60` for `pulse_count == 0` channels.

**Why:** At a typical `pulse_duration = 0.5s`, this caps a channel at 30 seconds — well past any reasonable in-game channel duration (Sustained Sweep is 20 pulses / 10s, the longest in seed). Acts as a backstop if a cancellation event is missed (caster despawns without `apply_death_transition` running, etc.). Not a gameplay constraint — channels SHOULD be cancelled by one of the four explicit triggers; this is a leak guard.

**Reversibility:** Single constant.

**Code:** [`crates/cell-combat/src/cell/effects/pulsing/mod.rs`](../../crates/cell-combat/src/cell/effects/pulsing/mod.rs).

### 13. CellEntity.last_aoe_deaths: per-attacker scratchpad for AoE kill credit

**Decision:** Cone-AoE secondary kills are stashed in `CellEntity.last_aoe_deaths: Vec<u32>` on the attacker, then drained by `handle_use_ability_with_kill_credit` immediately after the call returns.

**Why:** `handle_use_ability` has many callers (NPC AI, auto-cycle tick, kill-credit wrapper). Most don't care about kill credit — adding a `Vec<u32>` return type would force every caller to handle it. Storing on the entity keeps the function signature stable; the one caller that cares (the kill-credit wrapper) drains the scratchpad after the call.

**Trapdoor:** If two `handle_use_ability` calls run on the same attacker between drains, the second one's deaths would join the first's set. In practice this can't happen because the kill-credit wrapper drains synchronously after each call, but the trapdoor is real if a future refactor batches calls.

**Reversibility:** Reversible — switch to a return-type if a batching refactor surfaces the race.

**Code:** [`crates/entity/src/cell_entity/mod.rs`](../../crates/entity/src/cell_entity/mod.rs) (field), [`crates/cell-combat/src/cell/abilities/use_ability/mod.rs`](../../crates/cell-combat/src/cell/abilities/use_ability/mod.rs) (stash + drain).

### 14. cone geometry: X/Z planar, ignoring Y

**Decision:** Cone collection is 2D in the X/Z plane (Y is up). An entity is inside the cone iff the X/Z distance ≤ length AND the X/Z angle off-axis ≤ half-angle. Y offset is not checked.

**Why:** The original game's cones are effectively cylindrical sections in 3D — anyone within the X/Z cone is "in the line of fire" regardless of vertical offset, short of going through a floor. We don't have navmesh LOS yet, so adding a Y check would only catch the trivial "enemy on a roof directly above me" case while missing the common "enemy on a ramp" case. Defer to navmesh LOS work.

**Reversibility:** Per-ability flag could add Y-bound checking later. Backward-compatible (default behaviour stays the same).

**Code:** [`crates/cell-combat/src/cell/abilities/cone_aoe/mod.rs`](../../crates/cell-combat/src/cell/abilities/cone_aoe/mod.rs) (`collect_cone_targets`).

### 15. Pulse tick cadence = 100ms (piggyback on AoI tick)

**Decision:** `effect_pulse_tick` and `channel_interrupt_on_movement_tick` both run every AoI tick (100ms).

**Why:** The cell's main loop already has 100ms cadence for AoI updates. Adding a separate timer for effects would either introduce drift between the two cadences or duplicate the timer infra. Piggybacking means pulse intervals down to 0.1s resolve correctly (which covers every pulse_duration in the DB — the smallest is 0.5s on Sustained Sweep) and the cost per tick is bounded by entities × active_effects, which is "small" for any real load (we tested up to ~10 entities × ~3 effects each with no measurable overhead).

**Reversibility:** Could split into a separate tick with its own cadence if effect frequency becomes a bottleneck. No content depends on the cadence — pulses fire at `pulse_duration` intervals regardless of how often the tick runs.

**Code:** [`crates/cell/src/cell/service/message_loop.rs`](../../crates/cell/src/cell/service/message_loop.rs).

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
  and its descendants (in practice `cell::content::executor`).
- Neither takes a client-supplied id. `ability_id` / `effect_id` come from a
  `content_actions` seed row loaded at startup; no wire field reaches them.

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
| At fire: target gone, dead, in another space, or no longer a valid target (#444) | Rust addition | `warmup_tick` |
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

### 23. A pet summon is a player cast with a `pet_summons` row, diverted at launch and fire (pets PT-03)

**Decision:** A player ability with a `resources.pet_summons` row (`SpaceManager::pet_summons`)
summons a pet. It rides the ordinary cast of decision 21, with three diversions in
[`use_ability/summon.rs`](../../crates/cell-combat/src/cell/abilities/use_ability/summon.rs):

- **Launch.** Straight after the weapon redirect, the client's `target_id` is replaced by 0.
  The summon is a Self ability, so the client's target plays no part in it. With target 0
  the #444 target-validity gate never sees the cast. The gate itself is unchanged, so any
  other ability aimed at the caster still fails there. Two refusals run before the cooldown
  is charged: the summon must be in the trained set (a weapon grant does not count), and its
  template must be in the startup cache. Each sends `onErrorCode` plus a `CHAN_FEEDBACK`
  chat line.
- **Warmup.** The spawn timer is the ability's own warmup, scaled by the caster's
  `speedPet` stat (111) when the ability has `SpeedPet` (16384), like the other speed flags
  (D-PT10). Every decision-21 interrupt applies. An interrupted warmup never reaches the
  fire, so nothing spawns.
- **Fire.** `fire::fire_cast` diverts to `fire_summon` before any ammo, channel or damage
  step. `fire_summon` re-checks that the pet can be spawned before it touches the current
  pet (caster alive, in a space, template cached). A refusal plays `Ability_Interrupt` and
  sends the feedback pair, and the cooldown stays charged. Otherwise it calls
  `spawn_pet_from_template` first. A spawn that still fails answers exactly like a refusal
  (`Ability_Interrupt`, the feedback pair, the cooldown stays charged), and the owner keeps
  its current pet. A spawn that succeeds plays `Ability_End`, despawns the owner's oldest
  pets down to `max_active - 1` (D-PT04, counting every pet the owner had before the spawn),
  and queues the target VFX.

The summon's phase sequences carry TargetID = caster, as python's
`targetId or ent.entityId` did. The target VFX is event set 1122 `Effect_Init` (2000),
sequence 2293. It is an `onSequence` on the pet, with source = owner, target = pet and
`InstanceId` 0, which is how python played an effect sequence on its target. It waits on the
pet registry
([`pets/arrival.rs`](../../crates/cell-world/src/cell/pets/arrival.rs)) until the owner
witnesses the pet. The drain runs after the AoI tick and sends the VFX to the pet's
witnesses, so it can never reach a client ahead of the pet's CREATE_ENTITY. It is dropped
after 2 s, and `forget_pet` scrubs it on every teardown path. It is also dropped when the
owner's entity id now belongs to a player who is not the pet's summoner
(`PetRegistry::summoner_matches`, #870). It is counted and logged as sent only when at least
one witness send succeeds. A summon carries `Deactivate_AutoCycle` and
`DoNotActivate_AutoCycle`, so it is not stashed as the last-fired ability, and a later
`setAutoCycle(1)` press cannot re-fire it. Neither flag lets any ability arm the loop:
1024 clears it, and 512 leaves it as it was (python passed `autoCycle = False`,
`SGWPlayer.py:1177`).

**Why:** The 2009 data never linked a summon to a template. The summon abilities carry no
effects, and the editor's "Spawn Mob" effects name no template (pets audit A-26), so there
is no effect script to run. Keying on the ability id keeps the damage pipeline and the #444
gate untouched, which is what the packet asked for. Discarding the target is safer than
rejecting it: a Self ability legitimately arrives with 0 or with the caster's own id. The
VFX waits for the intro because a client cannot play a sequence on an entity it has not
created.

**Consequences:** `AF_CHANNEL_ALLOWS_MOVEMENT` used to be bit 14, which is the client's
`SpeedPet`, so every seeded summon warmed up immune to the move interrupt. It is now bit 20
(decision 7). NPC casters never summon, because `player_summon` answers only for players.

**Code:** [`use_ability/summon.rs`](../../crates/cell-combat/src/cell/abilities/use_ability/summon.rs),
the hooks in `handle.rs`, `fire.rs` and `sequence.rs`, and
[`pets/arrival.rs`](../../crates/cell-world/src/cell/pets/arrival.rs). Tests are in
`use_ability/tests/summon.rs` and `pets/tests/arrival.rs`; the evidence and the
regression proofs are in the [PT-03 worknote](../analysis/pets/worknotes/pt-03.md).

### 24. One hostility rule for every gate; a duel partner is the only player target (social systems SS-D2)

**Decision:** `combat::player_may_attack(attacker, target, &duels)` (`crates/cell-world/src/cell/combat/aggression.rs`) is the single rule for what a player may damage. An NPC target must be a hostile-faction non-pet, as before. A player target is admitted only when `DuelRegistry::can_harm` says the two are an engaged duel pair, in the same space. The four hostility gates all call it: the single-target launch (`use_ability/handle.rs`), the warmup re-check at fire (`use_ability/warmup/tick.rs`), and the ground-AoE and cone collectors (`dispatch/mod.rs`, `cone_aoe/geometry.rs`). The two collectors scan `combat::area_candidates` (every NPC, plus the caster's engaged partner) and filter with `combat::may_hit_in_area`, which is `player_may_attack` for a player caster and the historical hostile-faction rule for an NPC caster.

**Why:** the gates had drifted into four inline copies of "hostile NPC only" (audit A-42), so a duel had to widen all four or leak through one. A candidate scan that adds only the partner means no filter mistake can reach a bystander. The client's PvP flag (`onEntityProperty(4, v)`, decision D-SS23) is never read back: a stuck flag cannot make anyone attackable.

**Consequences:** NPC-versus-player and pet targeting are unchanged; a pet never joins its owner's duel: `pet::fight_refusal` uses the no-duel form, `player_may_attack_pve`, which is also the NPC half of `player_may_attack`. Player-on-player damage creates no threat, so the duel supplies its own combat source (`cell::duel::combat`). The effect pulse does not re-run the rule, so the duel's single end (`duel::end_engaged`) strips every active effect the partner's engaged entity invoked on each duelist, with the normal `on_remove` and zero timer; an auto-cycle loop on a player the caster may no longer harm is cleared. Partner damage is non-lethal since SS-D3 (decision 26), with the clamp in the pulse seam as well.

**Code and tests:** `aggression.rs`, the four gates above, `crates/cell-world/src/cell/duel/`. `use_ability/tests/duel_gate.rs` (`duel_partner_damage_allowed_at_all_four_gates`, `bystander_untouchable_during_duel`) fails when any one gate is reverted; the proof is in the [SS-D2 worknote](../analysis/social-systems/worknotes/ss-d2.md).

### 25. Owner abilities that act on a pet are diverted to the owner's pet, and their state lives on the pet (pets PT-08)

**Decision:** An ability with an effect whose `script_name` is a pet script (`PetStatBuff`,
`PetDeathTimer`, `HealPetHealth`; `effects::pet_scripts::acts_on_owner_pet`) acts on the
caster's pet, never on the client's target. It rides the cast of decision 21 with the same
three diversions as decision 23, in
[`use_ability/owner_pet/`](../../crates/cell-combat/src/cell/abilities/use_ability/owner_pet/mod.rs):

- **Launch.** The client's `target_id` is replaced by 0, so the #444 gate never sees the cast
  and stays as strict for every other ability. The pet comes from
  `SpaceManager::owner_pet_targets`
  ([`pets/owner_target.rs`](../../crates/cell-world/src/cell/pets/owner_target.rs)): the
  registry's pets of the caster, each kept only when the summon-time identity says the caster
  summoned it (`PetRegistry::summoner_matches`), it is alive, and it is in the caster's
  space. A bare owner id is never enough. With no such pet the press is refused before the
  cooldown is charged, with `onErrorCode` plus a `CHAN_FEEDBACK` line: 190
  `EntityDoesNotHavePet` for no pet, a pet in another space or a reused owner id, 14
  `NotLiving` for a dead pet, and 133 `EffectMonikerOnEntity` for To The Death pressed while
  it already runs.
- **Warmup.** The ability's own, with every decision-21 interrupt.
- **Fire.** `fire::fire_cast` diverts to `fire_owner_pet` before any ammo or damage step. The
  pet is resolved again. A refusal plays `Ability_Interrupt` and sends the feedback pair, and
  the cooldown stays charged. Otherwise `Ability_End` plays with TargetID = the pet, and each
  pet script runs with source = owner and target = pet. A `TCM_Single` effect lands on one
  pet; any other collection method lands on every pet the owner has out (one today, D-PT04).
  A pulsing effect (Repair Turret: Regenerate) is registered on the pet, invoked by the owner.
  The pet's dirty stats go to its witnesses. Nothing enters the damage pipeline, threat or
  kill credit.

The state these abilities leave is on the pet, not in `active_effects`:

- **Buff ledger.** `register_active_effect` never registers a `pulse_count = 1` row, which is
  what the seed gives Holy Warrior (4220), To The Death (4121) and Lord's Concentration
  (350). `PetStatBuff` writes a `PetBuff` on `PetState::buffs` instead
  ([`pets/buffs.rs`](../../crates/cell-world/src/cell/pets/buffs.rs)). It records the delta
  each stat really moved, and removal takes back exactly that, as python's `statChanges` did
  (`AbilityManager.py:438-441`). Re-applying the same effect replaces it; it never stacks.
- **Bounds widen, a deliberate deviation.** `DEFENSE` and `INTERRUPT_RES` default to `[0, 0]`,
  so python's clamp would drop Holy Warrior's -100 Defense and Lord's Concentration's +50. The
  ledger widens that one pet's bound to admit the delta.
- **Toggle.** For an ability with `Toggled` (8, `AF_TOGGLED`), `PetStatBuff` takes the buff
  off when the pet has it and puts it on with no expiry when it has not. The owner gets a chat
  line with the new state ("Holy Warrior is on."), because the Ability window shows none.
- **Expiry and To The Death.** `owner_pet_tick` runs every AoI tick after the pet sweep. It
  takes expired buffs off, then kills each pet whose `PetState::doomed_at` has passed through
  `kill_npc_out_of_band(pet, pet, attacker_is_player = false, grant_xp = false)`, after
  zeroing its HEALTH. The kill pays nobody: no XP, no mission `EntityDeath` (only the
  kill-credit wrappers raise it), and a pet has no loot table. The corpse then follows the
  pet path of D-PT08. 4119 "Pet Death Timer" (`PetDeathTimer`) arms the doom; 4122 "Pet
  Death" has no script, because a script cannot await the death resolver. A re-cast while the
  pet is doomed is refused, another deliberate deviation: python's refresh would restart the
  60 s timer, and with a 30 s cooldown the +400 Accuracy would never end.
- **Passives.** An `EF_AlwaysPersist` (524288) effect whose script is a passive script
  (`pet_scripts::is_passive_script`, today only `PetSummonSpeed`) holds while its ability is
  known. [`effects/passives.rs`](../../crates/cell-world/src/cell/effects/passives.rs) runs it
  at `InitPlayerState`, `AbilityGranted` and `GmAbilityGranted` (the GM `.giveability` mirror),
  and runs its `on_remove` at `AbilitiesReset`.
  Heed Our Calling (2852 -> 4968) sets the owner's `speedPet` to its base plus 100, so a
  `SpeedPet` summon's warmup scales to 0 (D-PT10). The stat is server-side only: the passive
  leaves it clean, so no burst changes.

Holy Warrior's 4087 "Stance Removal" is "Remove Effect of moniker EFFECT_Stance", the
mutual-exclusion half every player stance carries. No player stance effect is active on this
server, and the seed links no effect to that moniker, so it has no script and removes nothing.

**Why:** The 2009 rows carry no `script_name` and no NVPs for these effects, so the scripts and
magnitudes are seed edits, each the number in the effect's own description
(`effect_nvps` 350-357). Keying the redirect on the scripts keeps it data-driven: a new
pet-acting ability is wired by naming the script on its effect. Keeping the state on the pet
means a despawn, a replacing summon or the owner's death (which despawns the pet) clears it,
and the owner carries no "buff on" flag. Lord's Concentration (1650) shipped with no effect at
all; effect 350 is server-only, and pets D-PT17 records its magnitude and duration as a
greenfield decision.

**Consequences:** Nothing reads `INTERRUPT_RES` yet. The server has no damage-driven warmup
interrupt (decision 21), so Lord's Concentration changes a stat that will matter only once
one lands. The Repair Turret heals redirect to whatever pet the owner has, which is a
Servant Lord pet until turrets exist (PT-12). Repair Turret: Restoration (1214, revive) is not
wired. The scripts are in their own file, `effects/pet_scripts.rs`, because `scripts.rs` is
over the file cap.

**Code:** [`use_ability/owner_pet/`](../../crates/cell-combat/src/cell/abilities/use_ability/owner_pet/mod.rs)
(launch, fire, tick, feedback), the hooks in `handle.rs`, `fire.rs` and `sequence.rs`,
[`effects/pet_scripts.rs`](../../crates/cell-world/src/cell/effects/pet_scripts.rs),
[`effects/passives.rs`](../../crates/cell-world/src/cell/effects/passives.rs),
[`pets/buffs.rs`](../../crates/cell-world/src/cell/pets/buffs.rs) and
[`pets/owner_target.rs`](../../crates/cell-world/src/cell/pets/owner_target.rs). Tests are in
`use_ability/owner_pet/tests/`, `pets/tests/owner_buffs.rs` and
`base_messages/tests/passive_abilities.rs`; the evidence and the regression proofs are in the
[PT-08 worknote](../analysis/pets/worknotes/pt-08.md).

### 26. Duel-partner damage is held at 1 HP in both damage seams (social systems SS-D3, D-SS20)

**Decision:** `cimmeria_cell_world::cell::duel::clamp_partner_lethal(mgr, attacker, target, source)` runs wherever a player's HEALTH is written by an attacker, before anything reads it for a death and before the stat flush:

- `apply_damage_to_target` (`damage_apply/mod.rs`), after the direct damage (so `target_died` sees 1) and again after the effect scripts (so the effect-driven death sweep sees 1);
- `fire_pulse` (`effects/pulsing/tick.rs`), after both the script and the NVP branch and after the surrender floor.

When the attacker and the target are an engaged duel's engaged entities and HEALTH is at or below 0, HEALTH becomes 1 and the hit is returned. The caller ends the duel with `duel::finish_clamped` (`EDUEL_DEFEAT_Health`, the clamped duelist losing) only after the rest of the resolution has run: in `apply_damage_to_target` at the very end, after the pulsing effects are registered, and in `fire_pulse` after the flush. `effect_pulse_tick` skips a due instance that an earlier pulse in the same tick removed.

**Why:** D-SS20 makes duels non-lethal, and a lethal duel would send duel kills down the loot and XP path. The pulse never re-checks hostility (decision 24), so a clamp only in `damage_apply` would let a partner's DoT kill. Ending the duel at once would strip the partner's effects and end `can_harm` before a script bleed or a newly registered DoT from the same hit had been clamped, and those would then kill after the duel. Ending last means the end's `strip_from` removes everything the hit registered.

**Consequences:** Damage from anyone else is untouched: a third party can still kill a duelist, and `resolve_death` reports that death to the duel (`duel::on_death`). A clamped duelist never reaches `resolve_death`: no corpse, loot, XP, Defeat Window or respawn. The client is told 1 HP, never 0. The pulse's `still_active` check also closes an older window, in which a channel cancel between awaits let a removed instance fire from the tick's snapshot. `apply_damage_to_target` re-runs the harm gate for player-on-player damage before anything else (PR #924 review): a multi-hit ability (two cones) collects its targets up front, so without the re-check the second cone would land on the ex-partner after the first had ended the duel. The gate (`player_may_attack`) also requires the exact engaged entities (`DuelRegistry::can_harm_entities`), the same pair the clamp keys on, so every hit the gate admits is one the clamp covers. A duelist already at 0 HP when a partner hit lands (a third-party DoT, which kills no player today) is raised to 1 by the clamp; accepted in review.

**Code and tests:** `crates/cell-world/src/cell/duel/paths.rs`, the two seams above, `death/mod.rs`. `use_ability/tests/duel_nonlethal.rs` (`lethal_partner_hit_clamps_to_one_hp`, `lethal_partner_bleed_clamps_to_one_hp`, `no_loot_xp_or_corpse_after_a_clamped_end`, `third_party_kill_is_normal_death`); the proof is in the [SS-D3 worknote](../analysis/social-systems/worknotes/ss-d3.md).

### 27. Ability ranges are UE3 units in the data and metres on `AbilityDef` (#919)

**Decision:** `load_ability_defs` divides `resources.abilities.min_range` / `max_range` by `ABILITY_RANGE_UNITS_PER_METRE` (100) once, and `AbilityDef::min_range` / `max_range` are `f32` metres. Every consumer resolves the reach through `AbilityDef::max_range_or_default` / `ability_max_range` (`crates/entity/src/abilities/range.rs`), which maps the `0` sentinel to `DEFAULT_ABILITY_MAX_RANGE` (30 m): the launch check in `handle_use_ability`, the warmup fire-time re-check, the ground-target primary check, the auto-cycle skip, the pet-order pre-check (CM 88) and the NPC/pet AI's `ability_ranges`.

**Why:** The data is the client's own. The 2009 `CookedDataAbilities.pak` ships the same numbers (1652 Jaffa: Double Blast `MaxRange="3000"`), and the client uses them in UE3 world space: the ground-target reticule clamps against them from the pawn's UE3 location, next to AoE radii it converts to UE3 units (Medium = 1000). Every non-zero seeded range is a multiple of 100. Compared raw with metre positions, every ranged ability with a real range reached 1 to 100 km. Evidence and addresses: [ability-resolution-pipeline.md § Range units](../reverse-engineering/findings/ability-resolution-pipeline.md#range-units-919-verified-2026-09-28).

**Consequences:** 1652 reaches 30 m, 1653 8 m, grenades 25 m, deployables 5 m; turret 1205's 300-unit minimum is 3 m. Weapon ranges (`resources.items.*_range`) are already metres and are not converted. The server still ignores `UseWeaponRange` (flag 4) and never checks `min_range` for a player's cast (python refuses inside it with `OutsideWeaponRange`); both are pre-existing gaps, not part of #919. Tests: `spawner::abilities::range_unit_tests`, `use_ability/tests/range_units.rs`, and the live-DB `spawner/tests/live_db_ability_ranges.rs` and `use_ability/tests/range_units_live_db.rs`.

## Cross-cutting follow-ups

These were considered and deliberately deferred:

- **Mental resist rolls** — `EF_MENTAL_RESIST_ROLL = 64` flag is parsed and observable but no roll mechanic. Needs a design pass: what's the formula (attacker PSIONIC vs defender MENTAL_RES?), how does it interact with QR, what's the wire surface for "resisted" results (new `SRC_*` code? new `onEffectResults` variant?). Picking a model now risks baking the wrong one into the 64 mental-resist effects in DB.
- **Stun stacking nuance** — multi-source stuns share one `BSF_MOVEMENT_LOCK` bit via refcount, but per-stun-duration tracking isn't on the wire. The client sees one buff icon for "stunned" even when two stuns are pulsing. Acceptable for v1; needs design + wire-format work to surface per-source durations.
- **Per-archetype tree content authoring** (~560 rows × 7 archetypes) — pure content design work, no engine blockers.
- **Effect VFX sequences** — most ranged abilities share a generic beam sequence; per-weapon polish.
- **AoE for radius effects on single-target abilities** — `TCM_AERadius` effects attached to a `TARGET_TARGET` ability (e.g. proximity-mine detonations) don't yet fan out at primary-cast. Needs an explicit detonation trigger pattern.

## Cross-references

- [`state-flag-conventions.md`](state-flag-conventions.md) — the refcount discipline that Stun reuses
- [`state-field-bits.md`](state-field-bits.md) — the `BSF_*` bit catalog
- [`negative-logging-convention.md`](negative-logging-convention.md) — the observability discipline applied across the effect dispatcher
- [`docs/game-systems.md`](../game-systems.md) — top-level systems overview (abilities + effects section gets updated alongside this ADR)
- [`docs/content/content-engine.md`](../content/content-engine.md) — the `entity_health_below` trigger's authoring shape and band-test semantics (decision 17), and the `launch_ability` / `apply_effect` action rows (decision 16)
- [`docs/protocol/client-method-dispatch-table.md`](../protocol/client-method-dispatch-table.md) — `onTimerUpdate` (12), `onEffectResults` (14), `onKnownAbilitiesUpdate` (101)
