# Abilities + Effects System

> **Last updated**: 2026-05-26
> **Audience**: Engineers touching combat / abilities / effects on the cell
> **Type**: ADR + reference
> **Owner**: Combat systems
> **Status**: Accepted (shipped in PR #420)
> **Confidence**: High — every decision below is backed by code + tests in the same PR

## Context

PR #420 ([#47](https://github.com/SandboxServers/Cimmeria/issues/47), [#61](https://github.com/SandboxServers/Cimmeria/issues/61), [#331](https://github.com/SandboxServers/Cimmeria/issues/331), [#419](https://github.com/SandboxServers/Cimmeria/issues/419)) lit up the abilities system end-to-end: per-weapon resolution, hotbar population, trainer NPCs, effect-script dispatch, pulsing DoT/HoT, cone AoE, absorption shields, stun/suppression debuffs, channelled effects with movement-interrupt. Most of the engine code was new — but the cross-cutting design decisions deserve their own record so future contributors can extend the system without re-litigating them.

This doc captures **what** was decided and **why**, with pointers to the code that implements each piece. It is **not** a tutorial — for module-level walkthroughs read the inline docstrings on `cell::effects::mod.rs`, `cell::effects::pulsing.rs`, and `cell::abilities::cone_aoe.rs`.

## Decisions

### 1. EffectScript trait shape: `on_apply` + `on_remove`, no per-pulse callback

**Decision:** `EffectScript` exposes two methods — `on_apply(ctx)` and `on_remove(ctx)` with a default no-op for the latter.

**Why:** The Python reference (`AbilityManager.py`) had `on_pulse_begin`, `on_pulse_end`, `on_effect_init`, `on_effect_removed` — four lifecycle hooks. We collapsed to two because:

- **The initial pulse and every subsequent pulse run the same logic** for every script we've seen (heal re-heals, DoT re-damages, suppression re-chips). There's no use case yet for "do something different on pulse N vs pulse 1." Adding the split now would be speculative API surface.
- **`on_remove` is genuinely different** — it has to undo persistent state (Stun's `BSF_MOVEMENT_LOCK`, AbsorbShield's residual pool). Default empty impl means stat-mutation-only scripts (HealHealth, HealFocus, MeleeDamage, MeleePhysicalDamage) don't need to override.

**Reversibility:** Adding `on_pulse_begin` / `on_pulse_end` later is additive — existing scripts get default-empty impls, no migration. **Trapdoors:** none.

**Code:** [`crates/services/src/cell/effects/mod.rs`](../../crates/services/src/cell/effects/mod.rs) — trait definition + `dispatch_by_name` / `dispatch_on_remove` helpers.

### 2. Active-effect storage on the target, not the source

**Decision:** `ActiveEffectInstance` lives in `CellEntity.active_effects` on the **target**, with `invoker_id` pointing back at the source.

**Why:** Two real workloads decide this:

1. **Per-tick pulse fire** walks targets and fires their due effects. Storage-on-target makes this a single linear scan per cell tick over a Vec that's empty for most entities.
2. **"Cancel all channels by attacker X"** (channel-interrupt, channeller death) walks all targets and filters by `invoker_id`. O(N) over all entities, but N is small (~hundreds per cell) and channels are rare.

Alternative considered: storage-on-source with target_ids in the instance. Rejected because the per-tick "what should I tick right now" question dominates frequency over the "who did this come from" question.

**Reversibility:** Switching to source-side storage would require a single-pass migration of the `active_effects` field; both queries stay O(N) just over different sets. Not a permanent commitment.

**Code:** [`crates/entity/src/cell_entity/mod.rs`](../../crates/entity/src/cell_entity/mod.rs) (`ActiveEffectInstance` struct + `active_effects` field), [`crates/services/src/cell/effects/pulsing.rs`](../../crates/services/src/cell/effects/pulsing.rs).

### 3. Refcount lifecycle via existing `state_flag_counts`

**Decision:** Stun reuses the existing `set_state_flag` / `unset_state_flag` helpers (refcounted via `state_flag_counts`). No new per-stun-source tracking added.

**Why:** [`state-flag-conventions.md`](state-flag-conventions.md) already established the refcount pattern for `BSF_MOVEMENT_LOCK`, anticipating "future stun, cast, fear, knockback effects." The helpers do exactly what multi-source stun stacking needs:

- Two stuns from different invokers → counter = 2 → bit set
- First expiry → counter = 1 → bit STAYS set
- Second expiry → counter = 0 → bit cleared

The PR initially added a separate `movement_lock_reasons: HashSet<(u32, i32)>` field before realising the existing infra solved the problem. **The walked-back diff is a useful breadcrumb**: if someone hits the same false-positive read in the future, the answer is "the existing helpers work, write a regression test rather than a new mechanism."

**Reversibility:** Already the simpler design — no commitment to walk back.

**Code:** [`crates/entity/src/cell_entity/state_flags.rs`](../../crates/entity/src/cell_entity/state_flags.rs), `Stun::on_apply` / `on_remove` in [`crates/services/src/cell/effects/scripts.rs`](../../crates/services/src/cell/effects/scripts.rs).

### 4. Stacking semantics: same-source refresh, multi-source stack

**Decision:** `register_active_effect` does same-source refresh (same `invoker_id` + `effect_id` updates the existing instance) and multi-source stack (different `invoker_id` adds a new instance).

**Why:** Matches Python `AbilityManager.addEffect`. Prevents trivial DoT spam from a single attacker, while still letting multiple players DoT a boss in parallel. Refresh updates `remaining_pulses`, `next_pulse_at`, and `invoker_position_at_register` so the duration genuinely resets.

**Reversibility:** Trapdoor — content authored to assume same-source stacking would break if the rule changes. None of our seed currently assumes this either way, so we're free to revise. Document the rule clearly so content authors don't drift.

**Code:** `register_active_effect` in [`crates/services/src/cell/effects/pulsing.rs`](../../crates/services/src/cell/effects/pulsing.rs).

### 5. Pulsing model: initial pulse + N-1 follow-ups

**Decision:** When an effect with `pulse_count = N` lands, `damage_apply` fires the initial pulse synchronously, then `register_active_effect` registers an instance with `remaining_pulses = N - 1`. The per-tick loop fires the remaining pulses.

**Why:** Two reasons:

1. **Wire-side immediacy.** Players hitting "fire" expect the first damage tick to land on the same tick as the cast, not 0.5s later. The initial pulse goes through the same code path as a single-shot effect, so wire packets (`onEffectResults`, `onStatUpdate`) fire in the same order.
2. **Simpler scheduling.** `next_pulse_at = now + pulse_interval` always points to a FUTURE pulse. No special "first pulse is now" branch in the tick loop.

For channelled effects (`pulse_count = 0`), we register with `MAX_CHANNEL_PULSES = 60` as a safety cap so a missed cancellation event can't leak indefinitely.

**Reversibility:** Reversible — could move the initial pulse into the tick loop by setting `next_pulse_at = now` and skipping the synchronous fire. Would defer first damage by up to 100ms (one tick), which is noticeable in playtest.

**Code:** [`crates/services/src/cell/abilities/damage_apply/mod.rs`](../../crates/services/src/cell/abilities/damage_apply/mod.rs) (initial pulse), [`crates/services/src/cell/effects/pulsing.rs`](../../crates/services/src/cell/effects/pulsing.rs) (registration + tick).

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

**Code:** [`crates/services/src/cell/effects/pulsing.rs`](../../crates/services/src/cell/effects/pulsing.rs) (`cancel_channels_from_attacker`, `cancel_channels_for_invoker_ability`, `channel_interrupt_on_movement_tick`).

### 7. AF_CHANNEL_ALLOWS_MOVEMENT default = 0 (cancel-on-move)

**Decision:** The new `AF_CHANNEL_ALLOWS_MOVEMENT = 16384` ability flag defaults to 0 (off) across every authored ability. Operators flip it per-ability as content arrives that should be movement-tolerant.

**Why:** Cancel-on-move is the safe default — players who walk away from a channel expect it to stop. The inverse default would silently let channels persist across movement events the player doesn't realise are happening, which is a bug shape ("why is my buff still ticking after I rezoned?"). Opt-in to movement-tolerant via flag flip.

**Reversibility:** Per-ability flag, so changing one ability's behaviour is one DB row update. No engine commitment locked in.

**Code:** [`crates/entity/src/abilities.rs`](../../crates/entity/src/abilities.rs) (flag constant + docstring).

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

**Code:** [`crates/services/src/cell/abilities/cone_aoe.rs`](../../crates/services/src/cell/abilities/cone_aoe.rs), [`crates/services/src/cell/abilities/dispatch.rs`](../../crates/services/src/cell/abilities/dispatch.rs).

### 9. Absorption pool drain: elemental-specific first, generic catch-all second

**Decision:** When physical damage arrives, the drain order is `ABSORB_PHYSICAL` → `ABSORB_PHYSICAL_ENERGY` → `ABSORB_PHYSICAL_ITEM` → done (no overflow into untyped pools). Only HEALTH damage triggers absorption — FOCUS damage bypasses.

**Why:**

- **Elemental-specific first** matches player intent — a player who applied a "+200 physical absorb" buff expects it to consume on physical hits before generic shields drain
- **Three sub-pools per damage type** (the `_ENERGY` and `_ITEM` suffixes) come from the original game's stat schema; we honour the existing schema rather than collapse them
- **Only HEALTH absorbs** because the existing Python ref drains the same way; focus drains are typically resource-pressure mechanics (not damage to mitigate)

**Trapdoor:** The `_ENERGY` and `_ITEM` suffixed pools have no content driving them today — they'll only have non-zero `cur` once content adds effects that grant capacity to them.

**Reversibility:** Drain order is per-damage-type table inside `drain_absorption_pools`; trivially swapped. Changing the HEALTH-only rule means understanding the FOCUS-drain content semantics first.

**Code:** [`crates/services/src/cell/combat/damage.rs`](../../crates/services/src/cell/combat/damage.rs) — `drain_absorption_pools` + `calculate_damage`.

### 10. Script-name dispatch over flag-bit dispatch for effect categories

**Decision:** Effect categories (heal, damage, stun, suppression, shield) are selected via `EffectDef.script_name` — a string lookup in the registry. The `EffectDef.flags` bitmask is honoured for observability (`EF_STUN`, `EF_SUPPRESSION`, etc. log on apply) but does NOT drive dispatch.

**Why:** The original game's content has both — flags for category, script_name for behaviour. We chose script_name as the canonical dispatch key because:

- Adding a new script doesn't require allocating a new flag bit (32 bits is tight; the game already uses ~10)
- Scripts can take arbitrary NVPs without needing per-flag schema columns
- A single ability can have multiple effects each with different script_names; flag bits would conflate them

**Reversibility:** Could route flags into the dispatcher later (add a "if flags & EF_STUN, also run Stun" path) without breaking script_name routing.

**Code:** [`crates/services/src/cell/effects/registry.rs`](../../crates/services/src/cell/effects/registry.rs).

### 11. Channel-interrupt distance = 0.5m

**Decision:** `CHANNEL_INTERRUPT_DISTANCE = 0.5` world units.

**Why:** Walking is ~3 m/s — a 0.5m budget catches step-off-the-spot intent within ~150ms of a player input. Tighter (e.g. 0.1m) would fire on the ~3cm position jitter that comes from server-side movement smoothing; looser (e.g. 2m) would let players strafe through a substantial arc before the interrupt notices.

The 0.5m number is a guess pending playtest feedback — if it's too aggressive, it's one constant to bump.

**Reversibility:** Single constant, no schema commitment.

**Code:** [`crates/services/src/cell/effects/pulsing.rs`](../../crates/services/src/cell/effects/pulsing.rs).

### 12. Channel safety cap = 60 pulses

**Decision:** `MAX_CHANNEL_PULSES = 60` for `pulse_count == 0` channels.

**Why:** At a typical `pulse_duration = 0.5s`, this caps a channel at 30 seconds — well past any reasonable in-game channel duration (Sustained Sweep is 20 pulses / 10s, the longest in seed). Acts as a backstop if a cancellation event is missed (caster despawns without `apply_death_transition` running, etc.). Not a gameplay constraint — channels SHOULD be cancelled by one of the four explicit triggers; this is a leak guard.

**Reversibility:** Single constant.

**Code:** [`crates/services/src/cell/effects/pulsing.rs`](../../crates/services/src/cell/effects/pulsing.rs).

### 13. CellEntity.last_aoe_deaths: per-attacker scratchpad for AoE kill credit

**Decision:** Cone-AoE secondary kills are stashed in `CellEntity.last_aoe_deaths: Vec<u32>` on the attacker, then drained by `handle_use_ability_with_kill_credit` immediately after the call returns.

**Why:** `handle_use_ability` has many callers (NPC AI, auto-cycle tick, kill-credit wrapper). Most don't care about kill credit — adding a `Vec<u32>` return type would force every caller to handle it. Storing on the entity keeps the function signature stable; the one caller that cares (the kill-credit wrapper) drains the scratchpad after the call.

**Trapdoor:** If two `handle_use_ability` calls run on the same attacker between drains, the second one's deaths would join the first's set. In practice this can't happen because the kill-credit wrapper drains synchronously after each call, but the trapdoor is real if a future refactor batches calls.

**Reversibility:** Reversible — switch to a return-type if a batching refactor surfaces the race.

**Code:** [`crates/entity/src/cell_entity/mod.rs`](../../crates/entity/src/cell_entity/mod.rs) (field), [`crates/services/src/cell/abilities/use_ability/mod.rs`](../../crates/services/src/cell/abilities/use_ability/mod.rs) (stash + drain).

### 14. cone geometry: X/Z planar, ignoring Y

**Decision:** Cone collection is 2D in the X/Z plane (Y is up). An entity is inside the cone iff the X/Z distance ≤ length AND the X/Z angle off-axis ≤ half-angle. Y offset is not checked.

**Why:** The original game's cones are effectively cylindrical sections in 3D — anyone within the X/Z cone is "in the line of fire" regardless of vertical offset, short of going through a floor. We don't have navmesh LOS yet, so adding a Y check would only catch the trivial "enemy on a roof directly above me" case while missing the common "enemy on a ramp" case. Defer to navmesh LOS work.

**Reversibility:** Per-ability flag could add Y-bound checking later. Backward-compatible (default behaviour stays the same).

**Code:** [`crates/services/src/cell/abilities/cone_aoe.rs`](../../crates/services/src/cell/abilities/cone_aoe.rs) (`collect_cone_targets`).

### 15. Pulse tick cadence = 100ms (piggyback on AoI tick)

**Decision:** `effect_pulse_tick` and `channel_interrupt_on_movement_tick` both run every AoI tick (100ms).

**Why:** The cell's main loop already has 100ms cadence for AoI updates. Adding a separate timer for effects would either introduce drift between the two cadences or duplicate the timer infra. Piggybacking means pulse intervals down to 0.1s resolve correctly (which covers every pulse_duration in the DB — the smallest is 0.5s on Sustained Sweep) and the cost per tick is bounded by entities × active_effects, which is "small" for any real load (we tested up to ~10 entities × ~3 effects each with no measurable overhead).

**Reversibility:** Could split into a separate tick with its own cadence if effect frequency becomes a bottleneck. No content depends on the cadence — pulses fire at `pulse_duration` intervals regardless of how often the tick runs.

**Code:** [`crates/services/src/cell/service/message_loop.rs`](../../crates/services/src/cell/service/message_loop.rs).

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
- [`docs/protocol/client-method-dispatch-table.md`](../protocol/client-method-dispatch-table.md) — `onTimerUpdate` (12), `onEffectResults` (14), `onKnownAbilitiesUpdate` (101)
