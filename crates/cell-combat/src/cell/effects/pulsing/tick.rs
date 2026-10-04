//! The per-cell pulse scheduler.
//!
//! [`effect_pulse_tick`] runs at the cell's AoI cadence (100ms), walks
//! every entity's active-effect list, fires due pulses via
//! `super::pulse::pulse_one`, decrements `remaining_pulses`, and sweeps
//! expired instances.

use std::time::Instant;

use tokio::sync::mpsc;

use cimmeria_entity::abilities::{serialize_timer_update, EffectDef, TIMER_DURATION_EFFECT};
use cimmeria_entity::cell_entity::{ActiveEffectInstance, PlayerIdentity};

use crate::cell::abilities::wire_ledger::{self, WireCtx};
use crate::cell::abilities::{send_timer_update_ctx, WireRoute};
use crate::cell::content_events::ContentEvents;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Per-cell tick — fire any due pulses on any entity's active-effect
/// list. Runs at the cell's AoI cadence (100ms) so pulse intervals
/// down to 0.1s resolve correctly.
///
/// For each entity with active effects:
///   1. Take a snapshot of indices of effects whose `next_pulse_at`
///      has elapsed.
///   2. For each due effect, clone enough state to fire (effect def,
///      invoker), drop the borrow, fire the pulse, re-acquire to
///      decrement `remaining_pulses` / reschedule `next_pulse_at`.
///   3. Sweep removed instances (remaining_pulses == 0) after the
///      pulse-fire loop completes.
///
/// `events` is what makes a DoT tick content-visible (the chain engine in
/// production, as `&EngineEvents(&engine)`; §2E of
/// docs/architecture/services-crate-split.md). Each fired pulse is followed
/// immediately by two content hooks, in this order, both of which were
/// missing before the PR #662 review:
///
/// - [`super::pulse`]'s `dot_kill_credit`, because a pulse that takes a mob to zero used to
///   leave it standing at 0 HP with no death transition and no
///   `entity_dead_tag` credit at all.
/// - `ContentEvents::pending_health_below` (`content::fire_pending_health_below`),
///   draining the pre-pulse health sample so `entity_health_below` fires for a
///   threshold a DoT crossed. Draining per pulse rather than per tick
///   keeps `pct_after` exact when two DoTs land on the same target in the
///   same 100ms tick.
pub async fn effect_pulse_tick(
    events: &dyn ContentEvents,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let now = Instant::now();

    // Snapshot entities with at least one active effect so we don't
    // hold a borrow on `space_mgr` across the await. Walks every
    // entity (not just players + NPCs) so DoTs on turrets, destructibles,
    // and any future entity types fire correctly.
    let entities_with_effects: Vec<u32> = space_mgr
        .all_entity_ids()
        .into_iter()
        .filter(|&eid| {
            space_mgr
                .get_entity(eid)
                .is_some_and(|e| !e.active_effects.is_empty())
        })
        .collect();

    for entity_id in entities_with_effects {
        // Identify which effects are due and clone enough to fire
        // without holding the borrow across awaits.
        //
        // **Key by (effect_id, invoker_id) rather than Vec index.** The
        // earlier version re-acquired the entity by Vec index after the
        // await — but `cancel_channels_from_attacker` / `_for_invoker_ability`
        // can `retain()` between the await and the re-acquire, which
        // shifts indices and lands the post-pulse decrement on the wrong
        // effect. Keying by the instance's stable identity tuple avoids
        // the race.
        let due: Vec<(ActiveEffectInstance, EffectDef)> = {
            let Some(entity) = space_mgr.get_entity(entity_id) else {
                continue;
            };
            entity
                .active_effects
                .iter()
                .filter(|inst| inst.next_pulse_at <= now && inst.remaining_pulses > 0)
                .filter_map(|inst| {
                    space_mgr
                        .effect_defs
                        .get(&inst.effect_id)
                        .cloned()
                        .map(|def| (inst.clone(), def))
                })
                .collect()
        };

        if due.is_empty() {
            continue;
        }

        // Fire each due pulse. We re-look up the entity every iteration
        // because between awaits another tick could mutate.
        for (inst, effect_def) in &due {
            // An earlier pulse this tick may have removed this instance: a
            // duel the pulse ended strips the partner's effects (SS-D3), and
            // a channel cancel can `retain()` between awaits. A removed
            // instance must not fire from the stale snapshot.
            let still_active = space_mgr.get_entity(entity_id).is_some_and(|e| {
                e.active_effects
                    .iter()
                    .any(|a| a.effect_id == inst.effect_id && a.invoker_id == inst.invoker_id)
            });
            if !still_active {
                continue;
            }
            super::pulse::pulse_one(entity_id, inst, effect_def, events, tx, space_mgr).await;
            // Update schedule + decrement on the matching instance,
            // located by (effect_id, invoker_id) — index would be unsafe.
            if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                if let Some(active) = entity
                    .active_effects
                    .iter_mut()
                    .find(|a| a.effect_id == inst.effect_id && a.invoker_id == inst.invoker_id)
                {
                    active.remaining_pulses = active.remaining_pulses.saturating_sub(1);
                    if active.remaining_pulses > 0 {
                        active.next_pulse_at =
                            now + std::time::Duration::from_secs_f32(active.pulse_interval_secs);
                    }
                }
            }
        }

        // Sweep completed instances after the fire loop, sending
        // `onTimerUpdate` with `total_time = 0` so the client clears
        // the buff/debuff icon. Without the clear, the icon would
        // stick at "expiring in 0s" forever.
        let cleared_ids: Vec<(i32, u32, i32, Option<i32>, PlayerIdentity)> = {
            let Some(entity) = space_mgr.get_entity_mut(entity_id) else {
                continue;
            };
            let mut cleared = Vec::new();
            entity.active_effects.retain(|inst| {
                if inst.remaining_pulses <= 0 {
                    cleared.push((
                        inst.effect_id,
                        inst.invoker_id,
                        inst.ability_id,
                        inst.cast_id,
                        inst.invoker_identity,
                    ));
                    false
                } else {
                    true
                }
            });
            cleared
        };
        for (cleared_effect, invoker, ability_id, cast_id, who) in cleared_ids {
            // AB-T3: `pulse_ended` (was `active_effect_ended`), the natural
            // end. A strip (death, duel end, cleanse, channel cancel) logs
            // its own row where it removes the instance.
            tracing::debug!(
                target: "abilities.pulse",
                event = "pulse_ended",
                stage = "end",
                reason = "natural_end",
                account_id = who.account_id,
                account_name = who.account_name,
                player_id = who.player_id,
                player_name = who.player_name,
                entity_id = invoker,
                entity_name = who.player_name,
                target_id = entity_id,
                target_name = space_mgr.entity_label(entity_id),
                target_player_id = space_mgr.player_identity(entity_id).player_id,
                target_player_name = space_mgr.player_identity(entity_id).player_name,
                invoker_id = invoker,
                invoker_name = who.player_name,
                cast_id, // nt:id-only per-cast sequence number, no name exists
                effect_id = cleared_effect,
                effect_name = cimmeria_names::book().effect(cleared_effect),
                ability_id,
                ability_name = cimmeria_names::book().ability(ability_id),
                "pulsing effect ran its last pulse; clearing its icon"
            );
            // Phase I: script on_remove first so stateful effects (Stun
            // clearing BSF_MOVEMENT_LOCK, AbsorbShield draining residual
            // pool) get their cleanup. The effect-def lookup needs to
            // happen here because the active-effect Vec only stores
            // effect_id, not the full def.
            if let Some(effect_def) = space_mgr.effect_defs.get(&cleared_effect).cloned() {
                if let Some(script_name) = effect_def.script_name.clone() {
                    // The cast's scope closed long ago: the removal names the
                    // instance's snapshotted cast and invoker (`cast_scope`).
                    let outer = space_mgr.enter_effect_scope(cast_id, invoker, who);
                    let mut ctx = crate::cell::effects::EffectContext {
                        source_id: invoker,
                        target_id: entity_id,
                        effect: &effect_def,
                        space_mgr,
                    };
                    crate::cell::effects::dispatch_on_remove(&script_name, &mut ctx);
                    space_mgr.exit_effect_scope(outer);
                }
                // Flush any stat dirty bits the on_remove produced (e.g.
                // ABSORB_PHYSICAL drained by AbsorbShield::on_remove) so
                // the client picks up the change.
                if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                    let dirty = entity.stats.serialize_dirty();
                    entity.stats.clear_dirty();
                    if !dirty.is_empty() {
                        wire_ledger::send(
                            entity_id,
                            crate::mercury::method_idx::ON_STAT_UPDATE,
                            dirty,
                            WireRoute::EntityDefault,
                            WireCtx::new("pulse_end").cast(cast_id).ability(ability_id),
                            tx,
                            space_mgr,
                        )
                        .await;
                    }
                }
            }
            let zero_timer = serialize_timer_update(
                cleared_effect,
                TIMER_DURATION_EFFECT,
                invoker as i32,
                cleared_effect,
                0.0,
                0.0,
            );
            send_timer_update_ctx(
                entity_id,
                zero_timer,
                WireCtx::new("pulse_end").cast(cast_id).ability(ability_id),
                tx,
                space_mgr,
            )
            .await;
        }
    }
}
