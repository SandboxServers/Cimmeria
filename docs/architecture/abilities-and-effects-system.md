# Abilities + Effects System

> **Last updated**: 2026-10-03 (decisions 16-33 moved to two sibling files)
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

**Code:** [`crates/cell-world/src/cell/effects/mod.rs`](../../crates/cell-world/src/cell/effects/mod.rs) — trait definition + `dispatch_by_name` / `dispatch_on_remove` helpers. The synchronous layer (trait, context, the registry type) is in `cimmeria-cell-world` because the spawn-time cover hold runs Cover Stance through it; the scripts themselves are in the leaf crate `cimmeria-cell-effect-scripts` and register at startup (decision 33); the async pulsing scheduler ([`effects/pulsing/`](../../crates/cell-combat/src/cell/effects/pulsing/)) is combat's, in `cimmeria-cell-combat`, whose `cell::effects` re-exports the synchronous layer beside it; `cimmeria-services` re-exports that module at the same path ([services-crate-split.md](services-crate-split.md)).

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

**Code:** [`crates/entity/src/cell_entity/state_flags.rs`](../../crates/entity/src/cell_entity/state_flags.rs).

**Extended (ability mechanics AB-09a, 2026-10-03): one reference per timed-effect entry, not per `on_apply`.** The counter was right; the old `Stun` script used it wrong. It took a reference on every `on_apply` and released one on `on_remove`, and the pulse layer calls `on_apply` on every pulse and on a same-source re-hit, so a pulsing or refreshed stun left the counter above zero and the bit set for good; a `pulse_count = 1` stun never got an instance and never released at all. Stun and knockdown are now entries on the timed effect ledger (decision 28) with a **state-flag payload**: `TimedEffectSpec::state_flags` / `TimedEffect::state_flags` name the `BSF_*` bits the entry holds, and the ledger takes one counted reference per bit when the entry goes on and releases it when the entry comes off ([`stat_buff_flags.rs`](../../crates/entity/src/cell_entity/stat_buff_flags.rs)). Entries are keyed by `(effect, invoker)`, so a script that runs again replaces its own entry, releasing before it retakes. Two casters' stuns still hold two references. `clear_all_state_flags` (respawn, revive) forfeits the entries' holds so a stale entry cannot release a newer stun's reference. The ledger records the `state_field` from before its first unsent change, and `flush_stat_buff_timers` sends `onStateFieldUpdate` to the entity and its witnesses only when the value really changed, so a refresh sends nothing. Duration is the effect's `CcDuration` NVP (the generator's `cc` family), else its pulse span; a channelled stun is held until its instance's `on_remove`. The NPC fight tick holds (`decision_outcome = stunned`) and the NPC movement tick freezes the route with zero velocity while the bit is set. Python's `SGWBeing` lists `PLAYER_STATE_Stun` as "BSF_MovementLock + No ability/item use", and the client reads bit 6 for movement only, so the server enforces the rest: a landed stun or knockdown queues an unrolled interrupt (`InterruptCause::Incapacitated`, decision 21's extension) that ends the target's warmup and channels; `handle_use_ability` refuses a stunned caster before any cost (`use_ability/incapacitated.rs`: a player gets `onErrorCode` 15 and "You cannot do that while stunned.", the auto-cycle relaunch waits silently, an NPC is refused silently); a native consumable is refused the same way. The test is a ledger entry holding the lock (`holds_ledger_flag`), not the bare bit, which death and ring transport also set. The duel-end strip removes the partner's ledger entries too (`duel/effects.rs`, reason `duel_ended`). Resist rolls are not modelled (D-AB13): a stun that is not a miss lands. Scripts: `Stun`, `Knockdown` in [`crowd_control.rs`](../../crates/cell-effect-scripts/src/cell/effects/crowd_control.rs). Guards: `stat_buff_flags::tests`, `crowd_control::tests`, `pulsing::stun_tests` (the issue's three-pulse repro and the state-field broadcast), `npc_ai::crowd_control` and `npc_movement::cc_tests` in `cimmeria-cell`.

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
| `TCM_AERadius` | 300 (9%) | `handle_use_ability_on_ground` for ground-targeted (its secondaries take the area part only); a beneficial one of a player's non-ground cast fans out to the caster's allies; primary-only for everything else (decision 35) |
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

**Code:** [`crates/cell-combat/src/cell/combat/damage/absorb.rs`](../../crates/cell-combat/src/cell/combat/damage/absorb.rs) — `drain_absorption_pools`, called from `calculate_damage` in [`pipeline.rs`](../../crates/cell-combat/src/cell/combat/damage/pipeline.rs).

**Addendum (ability mechanics AB-10, 2026-10-03).** Two changes:

- **Focus damage absorbs too.** SGW's Focus is the outer damage pool, so a shield behind it did nothing until Focus was gone, and every authored shield ("Absorption: 500 Physical") was inert. The drain now applies to Focus and Health damage alike, in the pool's own points, Focus first where a seam deals both (a Focus-gated script, `RangedPhysicalDamage` or `MeleePhysicalDamage`, passes only its Focus half, since its Health half lands only when Focus breaks; a scripted pulse takes its damage type from its script): the pipeline (NVP hits and unscripted DoT pulses), and `absorb_damage_nvps`, which runs a damage script's `FocusDamage` / `HealthDamage` through the shields before the script writes the pools (scripted hits and scripted pulses). This departs from the Python reference on purpose.
- **A shield is a timed effect ledger entry.** `AbsorbShield` puts one entry per `(effect, caster)` on the ledger with one mutable pool per damage type, and adds each pool to its `absorb*` stat (the client's view). Every seam that drains the stats then calls `SpaceManager::settle_absorb_shields`, which charges the drain to the pools, oldest entry first, and takes off an empty shield (`StatBuffRemoval::Drained`, logged `drained`). Removing an entry for any other reason (expiry, death, refresh, a cleanse) takes its unspent pool back off the stat, so a timed shield no longer outlives itself (audit B-33). Capacity on the stat that no pool owns is spent first. [Decision 28](abilities-and-effects-decisions-23-33.md#28-native-consumables-the-base-consumes-before-the-cell-applies-and-timed-stat-buffs-live-in-their-own-ledger) has the ledger side.

### 10. Script-name dispatch over flag-bit dispatch for effect categories

**Decision:** Effect categories (heal, damage, stun, suppression, shield) are selected via `EffectDef.script_name` — a string lookup in the registry. The `EffectDef.flags` bitmask is honoured for observability (`EF_STUN`, `EF_SUPPRESSION`, etc. log on apply) but does NOT drive dispatch.

**Why:** The original game's content has both — flags for category, script_name for behaviour. We chose script_name as the canonical dispatch key because:

- Adding a new script doesn't require allocating a new flag bit (32 bits is tight; the game already uses ~10)
- Scripts can take arbitrary NVPs without needing per-flag schema columns
- A single ability can have multiple effects each with different script_names; flag bits would conflate them

**Reversibility:** Could route flags into the dispatcher later (add a "if flags & EF_STUN, also run Stun" path) without breaking script_name routing.

**Code:** the registry type in [`crates/cell-world/src/cell/effects/registry.rs`](../../crates/cell-world/src/cell/effects/registry.rs), the script table in [`crates/cell-effect-scripts/src/cell/effects/registry.rs`](../../crates/cell-effect-scripts/src/cell/effects/registry.rs) (decision 33).

**Addendum (2026-10-03, ability-mechanics AB-06, D-AB07).** Three changes refine this decision:

- **A damage script is its effect's only damage path.** An effect whose `script_name` is `RangedPhysicalDamage`, `MeleePhysicalDamage`, `RangedEnergyDamage` or `MeleeDamage` no longer also feeds its `HealthDamage`/`FocusDamage` NVPs to the legacy QR pipeline; before, both ran and Pistol Shot and Strike hit twice. The script runs where the NVP damage runs (before the hit's death check), with its two NVPs scaled by the hit's cover, special-ammo and splash scale, and its HEALTH change is the `onEffectResults` entry. Every other script still runs after the hit.
- **A miss lands nothing.** A QR-rolled effect whose roll is `RC_MISS` deals no NVP damage, runs no script and registers no pulses.
- **One flag bit now drives behaviour: `EF_DontUseQR` (16).** An effect carrying it never misses; a hit whose every effect carries it takes no roll (`RC_Hit`, the authored base damage). The old category constants (`EF_STUN = 12`, `EF_DOT = 516`, ...) were not client bits and are gone; the flag log names the client's `EEffectFlag` tokens.

Starter damage drops as a result. Code: [`damage_apply/effect_scripts.rs`](../../crates/cell-combat/src/cell/abilities/damage_apply/effect_scripts.rs) and [`damage_apply/qr_gate.rs`](../../crates/cell-combat/src/cell/abilities/damage_apply/qr_gate.rs).

**Addendum (2026-10-03, ability-mechanics AB-03, B-21).** The NVP path resolves each `TCM_Single` effect on its own instead of keeping the last positive `HealthDamage`/`FocusDamage` of any effect, so a direct hit and its DoT both land, one `onEffectResults` HEALTH entry each. `EF_DontUseQR` is read per effect: in a mixed ability the flagged effect resolves at the unrolled QR (its base) whatever the hit rolled, a miss included. Cone and radius effects keep the old collapse on a hit and land only when no direct (non-pulsing) `TCM_Single` damage effect does; their secondaries get them through the fan-outs as before. Code: [`damage_apply/nvp_damage.rs`](../../crates/cell-combat/src/cell/abilities/damage_apply/nvp_damage.rs).

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

### Decisions 16-33

Later decisions live in two sibling files, with their numbers and text unchanged: [decisions 16-22](abilities-and-effects-decisions-16-22.md) and [decisions 23-33](abilities-and-effects-decisions-23-33.md).

- [16. Content-initiated effects use a separate entry point, not `handle_use_ability`](abilities-and-effects-decisions-16-22.md#16-content-initiated-effects-use-a-separate-entry-point-not-handle_use_ability)
- [17. `entity_health_below` samples at the damage seams and drains at the engine holders](abilities-and-effects-decisions-16-22.md#17-entity_health_below-samples-at-the-damage-seams-and-drains-at-the-engine-holders)
- [18. A surrendered NPC is immune to the killing blow from an automatic damage source, not to damage](abilities-and-effects-decisions-16-22.md#18-a-surrendered-npc-is-immune-to-the-killing-blow-from-an-automatic-damage-source-not-to-damage)
- [19. Every death resolves through one function, including an effect script's killing blow](abilities-and-effects-decisions-16-22.md#19-every-death-resolves-through-one-function-including-an-effect-scripts-killing-blow)
- [20. A player's targeted ability needs line of sight at fire time (NA31, D-NA14)](abilities-and-effects-decisions-16-22.md#20-a-players-targeted-ability-needs-line-of-sight-at-fire-time-na31-d-na14)
- [21. Warmup is a pending cast per caster, fired by the 100 ms tick (AT-10)](abilities-and-effects-decisions-16-22.md#21-warmup-is-a-pending-cast-per-caster-fired-by-the-100-ms-tick-at-10)
- [22. Timer expiries are absolute on one server-wide game clock (CR-02)](abilities-and-effects-decisions-16-22.md#22-timer-expiries-are-absolute-on-one-server-wide-game-clock-cr-02)
- [23. A pet summon is a player cast with a `pet_summons` row, diverted at launch and fire (pets PT-03)](abilities-and-effects-decisions-23-33.md#23-a-pet-summon-is-a-player-cast-with-a-pet_summons-row-diverted-at-launch-and-fire-pets-pt-03)
- [24. One hostility rule for every gate; a duel partner is the only player target (social systems SS-D2)](abilities-and-effects-decisions-23-33.md#24-one-hostility-rule-for-every-gate-a-duel-partner-is-the-only-player-target-social-systems-ss-d2)
- [25. Owner abilities that act on a pet are diverted to the owner's pet, and their state lives on the pet (pets PT-08)](abilities-and-effects-decisions-23-33.md#25-owner-abilities-that-act-on-a-pet-are-diverted-to-the-owners-pet-and-their-state-lives-on-the-pet-pets-pt-08)
- [26. Duel-partner damage is held at 1 HP in both damage seams (social systems SS-D3, D-SS20)](abilities-and-effects-decisions-23-33.md#26-duel-partner-damage-is-held-at-1-hp-in-both-damage-seams-social-systems-ss-d3-d-ss20)
- [27. Ability ranges are UE3 units in the data and metres on `AbilityDef` (#919)](abilities-and-effects-decisions-23-33.md#27-ability-ranges-are-ue3-units-in-the-data-and-metres-on-abilitydef-919)
- [28. Native consumables: the base consumes before the cell applies, and timed stat buffs live in their own ledger](abilities-and-effects-decisions-23-33.md#28-native-consumables-the-base-consumes-before-the-cell-applies-and-timed-stat-buffs-live-in-their-own-ledger)
- [29. A deployable is a player ground cast with a `deployables` row: the object is an owned `SGWBeing` that pulses as its owner (deployables Phase 0)](abilities-and-effects-decisions-23-33.md#29-a-deployable-is-a-player-ground-cast-with-a-deployables-row-the-object-is-an-owned-sgwbeing-that-pulses-as-its-owner-deployables-phase-0)
- [30. A player's cast honours `min_range`, and a `UseWeaponRange` ability reaches as far as the weapon (#1016, #1017)](abilities-and-effects-decisions-23-33.md#30-a-players-cast-honours-min_range-and-a-useweaponrange-ability-reaches-as-far-as-the-weapon-1016-1017)
- [31. Special ammo modifies the shot directly, from `resources.ammo_modifiers` (ammo campaign AM-04, D-AM07)](abilities-and-effects-decisions-23-33.md#31-special-ammo-modifies-the-shot-directly-from-resourcesammo_modifiers-ammo-campaign-am-04-d-am07)
- [32. NPC-vs-NPC: an NPC's area ability hits the NPCs it would target, and an NPC-only kill pays nobody (#1009)](abilities-and-effects-decisions-23-33.md#32-npc-vs-npc-an-npcs-area-ability-hits-the-npcs-it-would-target-and-an-npc-only-kill-pays-nobody-1009)
- [33. The scripts live in a leaf crate and register with the cell at startup (#962 step 4)](abilities-and-effects-decisions-23-33.md#33-the-scripts-live-in-a-leaf-crate-and-register-with-the-cell-at-startup-962-step-4)
- [34. A beneficial cast lands on the caster or an ally, never on a hostile (ability mechanics AB-01)](abilities-and-effects-decisions-23-33.md#34-a-beneficial-cast-lands-on-the-caster-or-an-ally-never-on-a-hostile-ability-mechanics-ab-01)
- [35. Each effect of a cast lands where its routing says (ability mechanics AB-07)](abilities-and-effects-decisions-23-33.md#35-each-effect-of-a-cast-lands-where-its-routing-says-ability-mechanics-ab-07)
- [36. GM god mode puts Health and Focus back at the two damage seams (ability mechanics AB-N2)](abilities-and-effects-decisions-23-33.md#36-gm-god-mode-puts-health-and-focus-back-at-the-two-damage-seams-ability-mechanics-ab-n2)

## Cross-cutting follow-ups

These were considered and deliberately deferred:

- **Mental resist rolls** — no roll mechanic. (The old `EF_MENTAL_RESIST_ROLL = 64` constant was not a client bit, 64 is `EF_SequenceOnFinish`, and AB-06 removed it; the resists are their own "Resist Roll" effects, ability-mechanics AB-09d.) Needs a design pass: what's the formula (attacker PSIONIC vs defender MENTAL_RES?), how does it interact with QR, what's the wire surface for "resisted" results (new `SRC_*` code? new `onEffectResults` variant?). Picking a model now risks baking the wrong one into the 64 mental-resist effects in DB.
- **Stun stacking nuance** — multi-source stuns share one `BSF_MOVEMENT_LOCK` bit via refcount, but per-stun-duration tracking isn't on the wire. The client sees one icon per stun effect (the ledger's one icon per `effect_id`, with the latest expiry of its entries). Acceptable; per-source durations would need design + wire-format work.
- **Stunned players' movement** — the client disables its own movement input on `BSF_MovementLock`; the server refuses a stunned player's casts and consumables (AB-09a) but not its position updates.
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
