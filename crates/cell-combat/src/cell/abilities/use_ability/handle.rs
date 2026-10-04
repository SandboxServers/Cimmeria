//! The launch half of `useAbility(abilityId, targetId)`.
//!
//! Validates the call, starts the cooldown and sends its timer, arms or
//! clears auto-cycle, then either fires the cast in the same pass (zero
//! warmup, [`super::fire::fire_cast`]) or starts its warmup
//! ([`super::warmup::begin_warmup`], AT-10). The validation phase stays one
//! function because the borrow ordering between immutable and mutable
//! space_manager access is delicate and a hand-rolled split here would just
//! trade lines for `&mut` plumbing.

use tokio::sync::mpsc;

use cimmeria_entity::abilities::{
    caster_range_bounds, serialize_timer_update, AF_DEACTIVATE_AUTO_CYCLE,
    AF_DO_NOT_ACTIVATE_AUTO_CYCLE, TIMER_ABILITY_COOLDOWN,
};

use super::super::super::combat;
use super::super::super::messages::CellToBaseMsg;
use super::super::super::space_manager::SpaceManager;
use crate::mercury::game_clock;

use super::super::auto_cycle_state::send_auto_cycle_state;
use super::super::timer_update::send_timer_update;

use super::gate_rows::{LaunchRefusal, LaunchRow};
use super::weapon_redirect::resolve_weapon_redirect;

/// Handle a `useAbility(abilityId, targetId)` cell method call.
///
/// Flow:
/// 1. Look up entity in space manager
/// 2. Check entity is alive, not already warming up, has the ability
/// 3. Check ability not on cooldown
/// 4. Start cooldown timer (cooldown + warmup, as python did)
/// 5. Send `onTimerUpdate` to client
/// 6. Warmup > 0: send `Ability_Begin` and park the cast for the warmup
///    tick. Warmup = 0: fire now (`fire::fire_cast` — ammo, `Ability_End`,
///    damage, `onEffectResults`, `onStatUpdate`, death).
///
/// Returns `true` when the cast committed (validation passed and the
/// cooldown took effect). With a zero warmup the target's damage
/// resolution and wire packets have fired by then; with a positive warmup
/// they fire later from the warmup tick, or never if it is interrupted.
/// Returns `false` when any pre-consume guard rejected the call (entity
/// missing/dead, already warming up, no ability, a player's ability with no
/// mechanic yet (AB-12, `no_mechanics`), on cooldown, reload in
/// flight, no ammo, out-of-range, a target in another space (#906), or no
/// fire-time line of sight for an explicit target). Ground-target AoE
/// callers gate secondary-target damage on this return value.
#[tracing::instrument(
    name = "combat.use_ability",
    level = "info",
    skip_all,
    fields(entity_id, ability_id, target_id, cast_id = tracing::field::Empty)
)]
pub async fn handle_use_ability(
    entity_id: u32,
    ability_id: i32,
    target_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    // ── Look up ability definition from DB (before mutable borrow) ──
    let ability_def = space_mgr.ability_defs.get(&ability_id).cloned();

    // ── Archetype-default weapon redirect (read-only) ──
    //
    // Resolves the archetype-default ranged starter (Pistol Shot, 592)
    // to the active weapon's RANGED binding before validation runs. See
    // `weapon_redirect::resolve_weapon_redirect` for the full rationale
    // + scope limits.
    let (ability_id, ability_def) =
        resolve_weapon_redirect(entity_id, ability_id, ability_def, space_mgr);
    // Rule 5: every row below names the player (`None` for an NPC caster).
    let who = space_mgr.player_identity(entity_id);
    // Every early return logs one row through it (AB-T2, `gate_rows`).
    let mut row = LaunchRow {
        who,
        entity_id,
        ability_id,
        ability_name: ability_def.as_ref().map_or("unknown", |d| d.name.as_str()),
        wire_target_id: target_id,
        target_id,
    };

    // ── Pet summon (pets PT-03) ──
    //
    // A summon is a Self ability: the client's target is discarded here,
    // before anything reads it, so the #444 gate below never sees a summon
    // and stays exactly as strict for every other ability. See `summon`.
    let summon = super::summon::player_summon(space_mgr, entity_id, ability_id);
    // ── Owner ability on the owner's pet (pets PT-08) ──
    //
    // Same shape: the pet is resolved from the registry (summoner-checked),
    // never from the client's target, so the target is discarded here and
    // the #444 gate never sees the cast. See `owner_pet`.
    let owner_pet = super::owner_pet::player_owner_pet_ability(space_mgr, entity_id, ability_id);
    // A deployable aims at its staged ground point (`abilities::deployable`).
    let deploy = super::super::deployable::player_deployable(space_mgr, entity_id, ability_id);
    // A beneficial cast (a heal, a buff) lands on the caster or an ally,
    // whatever the client named (AB-01). See `beneficial`.
    let diverted = summon.is_some() || owner_pet || deploy.is_some();
    let client_target_id = target_id;
    let Some((target_id, beneficial)) = super::beneficial::launch_target(
        entity_id,
        ability_def.as_ref(),
        target_id,
        diverted,
        tx,
        space_mgr,
    )
    .await
    else {
        row.refused(LaunchRefusal::NoBeneficialTarget);
        return false;
    };
    row.target_id = target_id;
    // The fire re-resolves a beneficial cast from what the client sent.
    let wire_target_id = if beneficial {
        client_target_id
    } else {
        target_id
    };

    // ── Auto-cycle manual-override gate ──
    //
    // If the player has auto-cycle armed for one ability and manually
    // fires a different ability, cancel the loop before validation.
    // Matches python `AbilityManager.useAbility` (line 1019:
    // `self.autoCycle = False`) — the manual click is intent to break
    // the cycle. Tick-driven re-fires always invoke with the stashed
    // ability_id, so this never trips for loop-driven shots. Same-
    // ability manual fire is NOT a cancel: clicking the same weapon
    // on a different target should let the loop continue and let the
    // next tick redirect via `current_target_id`.
    let override_clears_loop = space_mgr.get_entity(entity_id).is_some_and(|e| {
        e.is_player
            && e.abilities.auto_cycle
            && e.abilities
                .auto_cycle_ability_id
                .is_some_and(|id| id != ability_id)
    });
    if override_clears_loop {
        if let Some(new_state) = combat::clear_auto_cycle(space_mgr, entity_id) {
            row.auto_cycle_overridden();
            send_auto_cycle_state(entity_id, new_state, tx, space_mgr).await;
        }
    }

    // ── Validation (immutable checks first to avoid borrow conflicts) ──

    // Pre-checks with immutable borrows
    let mut out_of_range = None;
    // A player pressed a server-known ability they do not know: answered
    // with `onErrorCode` below, once the entity borrow ends.
    let mut not_known = false;
    // A known ability with no mechanic yet (AB-12): answered below, before
    // any target or range check could refuse the press silently.
    let mut no_mechanics = false;
    let mut shield_full = false;
    // Stunned or knocked down (AB-09a): answered below, before any cost.
    let mut incapacitated = false;
    // Beneficial ammo (AM-11d): a support shot may land on an ally or the
    // shooter and never on a hostile target. `None` for every other cast,
    // whose targeting is exactly the #444 rule below.
    let support = super::support_shot::beneficial_shot(space_mgr, entity_id, ability_def.as_ref());
    // A support shot at an ally or a beneficial cast: never arms auto-cycle.
    let mut friendly_cast = false;
    let mut support_hostile = false;
    'validate: {
        let Some(entity) = space_mgr.get_entity(entity_id) else {
            row.refused(LaunchRefusal::CasterMissing);
            return false;
        };
        if combat::is_dead_state(entity.state_field) {
            row.refused(LaunchRefusal::CasterDead);
            return false;
        }
        if super::incapacitated::is_incapacitated(entity) {
            incapacitated = true;
            break 'validate;
        }
        // One cast at a time: python `canUseAbility` refused a launch while
        // `currentAbility` (an ability in its warmup) was set. The refusal
        // is silent, like the cooldown refusal below (AT-10).
        if let Some(pending) = entity.pending_cast.as_ref() {
            row.refused(LaunchRefusal::AlreadyWarming {
                ability_id: pending.ability_id,
                cast_id: pending.cast_id(),
            });
            return false;
        }
        if !entity.abilities.has_ability(ability_id) {
            // Weapon-granted abilities are resolved at fire time from
            // `items_event_sets` (see `resolve.rs` and the hostile-NPC
            // right-click path in `interaction.rs`). They are NOT
            // injected into `entity.abilities` on equip — that field
            // carries the player's trained / known / archetype-starter
            // set, distinct from weapon-granted IDs. Without this
            // fallback every weapon fire is rejected with "entity does
            // not have ability".
            let granted_by_weapon = super::super::resolve::is_ability_granted_by_active_weapon(
                space_mgr, entity_id, ability_id,
            );
            if !granted_by_weapon {
                // Severity split keyed on "does the server know this
                // ability id?" — `ability_def` is `Some` only when the
                // id is in `space_mgr.ability_defs`:
                //
                // - server-known + not granted → WARN. Real wiring
                //   issue: the player tried to fire an ability the
                //   server understands but the active weapon doesn't
                //   bind it. Operator-actionable.
                // - server-unknown → DEBUG. Almost certainly a forged
                //   or buggy client packet — the server has no def for
                //   this id at all. WARN here would let any client
                //   spam-burn the log index just by sending bogus
                //   ability ids on this client-controlled path.
                if ability_def.is_some() {
                    row.refused(LaunchRefusal::NotKnown);
                    // A stale action-bar button after a respec (AT-08)
                    // lands here: the bar is client-side and keeps the
                    // binding. The press gets feedback (project rule). A
                    // forged id with no server def stays silent (below).
                    if entity.is_player {
                        not_known = true;
                        break 'validate;
                    }
                } else {
                    row.refused(LaunchRefusal::UnknownAbilityId);
                }
                return false;
            }
        }
        if entity.abilities.is_on_cooldown(ability_id) {
            row.refused(LaunchRefusal::OnCooldown);
            return false;
        }
        if super::no_mechanics::lacks_mechanics(
            entity_id,
            ability_id,
            ability_def.as_ref(),
            space_mgr,
        ) {
            no_mechanics = true;
            break 'validate;
        }
        if super::shield_full::shield_has_no_room(
            entity_id,
            ability_def.as_ref(),
            target_id,
            space_mgr,
        ) {
            shield_full = true;
            break 'validate;
        }

        // Range + target validation
        if target_id > 0 {
            if let Some(target) = space_mgr.get_entity(target_id as u32) {
                // Don't attack dead targets
                if combat::is_dead_state(target.state_field) {
                    row.refused(LaunchRefusal::TargetDead);
                    return false;
                }
                // The #444 gate and its two inversions (support shots,
                // beneficial casts) live in `beneficial::target_gate`.
                match super::beneficial::target_gate(
                    entity,
                    target,
                    ability_id,
                    support.is_some(),
                    beneficial,
                    space_mgr,
                ) {
                    super::beneficial::TargetGate::SupportHostile => {
                        support_hostile = true;
                        break 'validate;
                    }
                    super::beneficial::TargetGate::Refused => return false,
                    super::beneficial::TargetGate::Admitted { friendly } => {
                        friendly_cast = friendly
                    }
                }
                // Range check, in metres: the loader converted the
                // ability's UE3-unit ranges (#919). A player is also held
                // to the ability's `min_range` (#1016), and a `UseWeaponRange`
                // ability to its weapon's reach (#1017). See `cast_range`.
                out_of_range = super::cast_range::check_cast_range(
                    caster_range_bounds(ability_def.as_ref(), entity, &space_mgr.weapon_ranges),
                    entity.position.distance_to(&target.position),
                    entity.is_player,
                );
            }
        }
    }

    if support_hostile {
        if let Some(shot) = support.as_ref() {
            super::support_shot::refuse(
                entity_id,
                target_id as u32,
                ability_id,
                shot,
                "launch",
                super::support_shot::REASON_HOSTILE_TARGET,
                tx,
                space_mgr,
            )
            .await;
        }
        // A loop re-firing support rounds at a hostile would only repeat
        // the refusal every cooldown.
        if let Some(new_state) = combat::clear_auto_cycle(space_mgr, entity_id) {
            send_auto_cycle_state(entity_id, new_state, tx, space_mgr).await;
        }
        return false;
    }

    if incapacitated {
        super::incapacitated::refuse_while_incapacitated(entity_id, ability_id, tx, space_mgr)
            .await;
        return false;
    }

    if not_known {
        // An untrained summon gets the summon refusal (onErrorCode, the
        // CHAN_FEEDBACK line and `summon_refused`), not the bare
        // onErrorCode every other unknown ability gets (pets PT-03).
        if let Some(summon) = summon {
            if super::summon::refuse_summon_launch(entity_id, ability_id, summon, tx, space_mgr)
                .await
            {
                return false;
            }
        }
        super::not_known::send_not_known_feedback(entity_id, who, ability_id, tx).await;
        return false;
    }

    // A shield whose every pool is already full: feedback, no cooldown.
    if shield_full {
        if let Some(def) = ability_def.as_ref() {
            super::shield_full::refuse_shield_full(entity_id, def, tx, space_mgr).await;
        }
        return false;
    }

    // A known ability with no mechanic yet: feedback, no cooldown (AB-12).
    if no_mechanics {
        if let Some(def) = ability_def.as_ref() {
            super::no_mechanics::refuse_without_mechanics(entity_id, def, tx, space_mgr).await;
        }
        return false;
    }

    if let Some(failure) = out_of_range {
        super::cast_range::refuse_out_of_range(
            entity_id,
            ability_id,
            target_id as u32,
            failure,
            "launch",
            tx,
            space_mgr,
        )
        .await;
        return false;
    }

    if let Some(summon) = summon {
        if super::summon::refuse_summon_launch(entity_id, ability_id, summon, tx, space_mgr).await {
            return false;
        }
    }
    if owner_pet
        && super::owner_pet::refuse_owner_pet_launch(entity_id, ability_id, tx, space_mgr).await
    {
        return false;
    }
    if deploy.is_some()
        && super::super::deployable::refuse_unstaged_launch(entity_id, ability_id, tx, space_mgr)
            .await
    {
        return false;
    }

    // Fire-time target checks, players only: a target in another space is
    // refused with onErrorCode 0 (#906), a wall between the eyes with 39
    // (NA31, D-NA14). See `fire_los` for where they apply.
    if target_id > 0
        && super::fire_los::refuse_without_line_of_sight(
            entity_id,
            ability_id,
            target_id as u32,
            ability_def.as_ref(),
            tx,
            space_mgr,
        )
        .await
    {
        return false;
    }

    // Weapon attacks: the holstered-draw queue and the slot-swap lockout.
    if super::weapon_gate::hold_weapon_attack(
        entity_id,
        ability_id,
        target_id,
        ability_def.as_ref(),
        tx,
        space_mgr,
    )
    .await
    {
        return false;
    }

    // Mutable borrow for state changes
    let Some(entity) = space_mgr.get_entity_mut(entity_id) else {
        row.refused(LaunchRefusal::CasterVanished);
        return false;
    };

    // Check ammo for ranged abilities (players only — NPCs have infinite ammo).
    // Stage C: read through the bandolier helpers; Stage B's reload tick is the
    // sole refill path, so the eager promotion that used to live here is gone.
    // The ammo itself is consumed when the cast fires (`fire::fire_cast`),
    // after any warmup, as python's `afterWarmup` did.
    let required_ammo = ability_def.as_ref().map_or(0, |d| d.required_ammo);

    // Block firing while a reload is in flight — checked as `is_some()`, not
    // `now < deadline`. Between the deadline elapsing and the next 100 ms
    // `reload_completion_tick`, the warmup is "over" by clock but the magazine
    // hasn't been refilled yet; allowing fire in that window would decrement
    // against pre-refill ammo and then be silently overwritten by the tick,
    // effectively granting free ammo. The tick is the sole authority that
    // clears `reload_complete_at`, so we gate on its presence.
    if required_ammo > 0 && entity.is_player && entity.reload_complete_at.is_some() {
        row.refused(LaunchRefusal::Reloading);
        return false;
    }

    let current_ammo = entity.active_ammo();
    if required_ammo > 0 && entity.is_player && current_ammo < required_ammo {
        row.refused(LaunchRefusal::NoAmmo {
            current: current_ammo,
            required: required_ammo,
        });
        return false;
    }

    // Warmup after the speed-stat modifiers (python `launch()`). Zero for
    // most abilities, which then fire in this same pass.
    let warmup_secs = super::warmup::effective_warmup(ability_def.as_ref(), entity);

    let cooldown_secs =
        ability_def
            .as_ref()
            .map_or(2.0, |d| if d.cooldown > 0.0 { d.cooldown } else { 0.5 });
    // Python `launch()` starts the cooldown at launch and makes it cover the
    // warmup: `cooldown = now + abilityCooldown + abilityWarmup`. With a zero
    // warmup this is the plain cooldown, unchanged.
    let charged_secs = cooldown_secs + warmup_secs;
    let cooldown_duration = std::time::Duration::from_secs_f32(charged_secs);
    entity
        .abilities
        .start_ability_cooldown(ability_id, cooldown_duration);

    // Stash the just-fired ability so `setAutoCycle(1)` can fire it
    // immediately on the next button press. Distinct from
    // `auto_cycle_ability_id` (the LOOP's committed ability, cleared
    // on stop): this field persists across auto-cycle on/off cycles
    // for the whole session. NPCs use `chooseAbility` per-fire and
    // don't need the stash. An ability flagged out of auto-cycle
    // (`DoNotActivate_AutoCycle` / `Deactivate_AutoCycle`, e.g. a pet
    // summon) is not stashed, or the next `setAutoCycle(1)` press would
    // re-fire it (python kept 512 abilities out, `SGWPlayer.py:1177`).
    let auto_cycle_excluded = ability_def
        .as_ref()
        .is_some_and(|d| d.flags & (AF_DO_NOT_ACTIVATE_AUTO_CYCLE | AF_DEACTIVATE_AUTO_CYCLE) != 0);
    if entity.is_player && !auto_cycle_excluded {
        entity.abilities.last_fired_ability_id = Some(ability_id);
    }

    // The cast's sequence id: the `InstanceId` of its sequences, the effect
    // id the client receives, and its telemetry `cast_id` (AB-T1). Every row
    // the cast causes, here or later from a tick, carries it.
    let effect_seq = entity.abilities.next_effect_id();
    tracing::Span::current().record("cast_id", effect_seq);

    tracing::info!(
        target: "abilities",
        event = "ability_launched",
        stage = "launch",
        account_id = who.account_id,
        player_id = who.player_id,
        entity_id,
        cast_id = effect_seq,
        ability_id,
        target_id,
        wire_target_id,
        cooldown_secs,
        warmup_secs,
        ability_name = ability_def.as_ref().map_or("unknown", |d| &d.name),
        "useAbility: launched"
    );

    // ── Send cooldown timer to the attacker's own client ──
    //
    // `BigWorldTimeComplete` is absolute, on the clock the client was told
    // at login: the client's cooldown manager (`FUN_00c6d1c0`) shows
    // `complete - clock`, clamped to 0, so a 0.0 here showed no cooldown.
    // Python: `now + abilityCooldown + abilityWarmup` (AbilityManager.py:605).
    let timer_args = serialize_timer_update(
        ability_id,
        TIMER_ABILITY_COOLDOWN,
        entity_id as i32,
        0,
        charged_secs,
        game_clock::game_time_secs() + charged_secs,
    );

    // Owner only: the client binds onTimerUpdate on SGWPlayer alone, so an
    // NPC's cooldown sent to its witnesses was dropped on every shot.
    send_timer_update(entity_id, timer_args, tx, space_mgr).await;

    // ── Auto-cycle commit: arm or DEACTIVATE-flag clear ──
    super::auto_cycle_commit::commit_auto_cycle(
        super::auto_cycle_commit::CommitCast {
            entity_id,
            ability_id,
            target_id,
            friendly_cast,
        },
        ability_def.as_ref(),
        tx,
        space_mgr,
    )
    .await;

    if let Some(summon) = summon {
        super::summon::log_summon_launched(
            space_mgr,
            entity_id,
            ability_id,
            summon,
            warmup_secs,
            charged_secs,
        );
    }

    // Note on BSF_InCombat (bit 3): intentionally NOT set here. The bit is
    // derived from `threatened_mobs` and flips on via
    // `combat::generate_threat` → `enter_player_combat` when this attack
    // actually generates threat on a surviving NPC target (handled in
    // `damage_apply::apply_damage_to_target`). Setting it raw here used to
    // strand it for one-shot kills (target dies before generate_threat
    // runs) and target-less casts (early-return before damage_apply).

    // ── Warmup (AT-10) ──
    //
    // A positive warmup sends `Ability_Begin` and parks the cast; the
    // warmup tick fires it (`fire::fire_cast`) when the timer expires, or
    // interrupts it. The cast has committed either way (the cooldown is
    // charged), so callers see `true`.
    if warmup_secs > 0.0 {
        super::warmup::begin_warmup(
            entity_id,
            super::warmup::WarmupStart {
                ability_id,
                target_id,
                wire_target_id,
                effect_seq,
                warmup_secs,
                event_set_id: ability_def.as_ref().and_then(|d| d.event_set_id),
            },
            tx,
            space_mgr,
        )
        .await;
        return true;
    }

    // Zero warmup: fire in this pass. Cooldown + ammo are consumed and the
    // cast committed even when no target resolves; ground-target callers
    // see this as "primary succeeded" and proceed with any AoE secondaries.
    // The cast scope stamps `cast_id` on whatever the fire lands (AB-T1).
    let outer_cast = space_mgr.enter_cast_scope(Some(effect_seq));
    super::fire::fire_cast(
        entity_id,
        ability_id,
        target_id,
        wire_target_id,
        effect_seq,
        &ability_def,
        tx,
        space_mgr,
    )
    .await;
    space_mgr.exit_cast_scope(outer_cast);
    true
}
