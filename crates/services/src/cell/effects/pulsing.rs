//! Active-effect pulsing for DoT, HoT, and timed-buff effects (#47, #419).
//!
//! When an effect's `pulse_count > 1`, the initial apply in
//! `damage_apply` fires the first pulse and then registers an
//! [`ActiveEffectInstance`] on the target. A per-cell tick walks every
//! entity's active-effect list, fires due pulses, decrements
//! `remaining_pulses`, and removes the instance when it hits zero.
//!
//! ## What a "pulse" does
//!
//! - If the effect has a `script_name`, re-dispatches the script (the
//!   script decides whether to heal again, do more damage, refresh a
//!   buff, etc.).
//! - Else, applies the legacy `HealthDamage` / `FocusDamage` NVPs as
//!   raw stat mutations (no QR roll on subsequent pulses — DoT/HoT use
//!   the QR result from the initial cast for replay determinism).
//! - Sends a follow-up `onStatUpdate` to the target so the client
//!   updates their stat bars.
//!
//! ## What's NOT here (v1 scope)
//!
//! - **No stacking dedup.** Two casters DoT-ing the same target both
//!   register their own instances and both pulse. Same caster casting
//!   the same DoT twice creates two instances — refresh-vs-stack semantics
//!   land in Phase E (stacking rules).
//! - **No channelled effects.** `pulse_count == 0` (infinite until
//!   removed) is recognised but not registered; the cell has no channel-
//!   release path yet (interrupt, movement, ability switch). Channelled
//!   abilities currently fire one initial pulse and stop.
//! - **No interrupt cancellation.** If the target dies, the instance
//!   sticks until the next pulse tick clears it via the dead-entity
//!   guard. Mid-tick burst interrupt (e.g. stun) is a Phase G follow-up.

use std::time::Instant;

use tokio::sync::mpsc;

use cimmeria_entity::abilities::{
    serialize_timer_update, EffectDef, DT_PHYSICAL, TIMER_DURATION_EFFECT,
};
use cimmeria_entity::cell_entity::ActiveEffectInstance;
use cimmeria_entity::stats::{FOCUS, HEALTH};

use crate::cell::abilities::send_entity_method;
use crate::cell::client_methods::being::ON_TIMER_UPDATE;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Register a pulsing effect on a target. Called once per effect right
/// after the initial pulse fires in `damage_apply`. Returns `true` if
/// an instance was registered (effect was actually pulsing), `false`
/// for single-shot effects.
///
/// Implements **same-source refresh** stacking semantics (Phase E):
/// if `target_id` already has an active instance with the same
/// `effect_id` AND `invoker_id`, the existing instance is refreshed
/// (remaining_pulses + next_pulse_at restored) instead of stacking a
/// duplicate. Different invokers each get their own instance (multi-
/// source stack). Matches the Python `AbilityManager.addEffect`
/// behavior for DoT/HoT — re-applying the same caster's bleed
/// refreshes the duration, two different casters' bleeds both tick.
///
/// `now` is passed in so tests can use a fixed instant without driving
/// the wall clock. Production callers pass `Instant::now()`.
///
/// When the effect is registered (or refreshed), sends an
/// `onTimerUpdate(TIMER_DURATION_EFFECT)` so the client renders the
/// buff/debuff icon with a duration countdown. The clear-on-expiry
/// timer comes from `effect_pulse_tick` when remaining_pulses hits 0.
pub async fn register_active_effect(
    space_mgr: &mut SpaceManager,
    target_id: u32,
    invoker_id: u32,
    effect: &EffectDef,
    now: Instant,
    tx: &mpsc::Sender<CellToBaseMsg>,
) -> bool {
    if !effect.is_pulsing() {
        return false;
    }
    // pulse_count == 0 means "channel until removed" — we don't have a
    // remove path yet, so skip registration. The initial pulse already
    // fired so the player saw something happen; the rest of the channel
    // is a Phase G follow-up.
    if effect.pulse_count == 0 {
        tracing::debug!(
            target: "abilities",
            event = "channelled_effect_skipped_registration",
            target_id,
            invoker_id,
            effect_id = effect.effect_id,
            "Channelled effect (pulse_count=0) — channel-release path not yet wired; one pulse fired and stopping"
        );
        return false;
    }

    // remaining_pulses = total - 1 (the initial pulse already fired).
    let remaining = effect.pulse_count - 1;
    if remaining <= 0 {
        return false;
    }
    let pulse_secs = effect.pulse_duration.max(0.1);
    let next_at = now + std::time::Duration::from_secs_f32(pulse_secs);

    let was_refresh = {
        let Some(target) = space_mgr.get_entity_mut(target_id) else {
            return false;
        };

        // Same-source refresh: re-applying the SAME effect from the SAME
        // invoker refreshes the existing instance instead of stacking.
        // Different invoker = stack (separate instance). This matches the
        // Python reference and prevents trivial DoT stacking from a single
        // attacker spam-casting the same bleed.
        if let Some(existing) = target
            .active_effects
            .iter_mut()
            .find(|i| i.effect_id == effect.effect_id && i.invoker_id == invoker_id)
        {
            existing.remaining_pulses = remaining;
            existing.next_pulse_at = next_at;
            existing.pulse_interval_secs = pulse_secs;
            true
        } else {
            target.active_effects.push(ActiveEffectInstance {
                effect_id: effect.effect_id,
                ability_id: effect.ability_id,
                invoker_id,
                remaining_pulses: remaining,
                total_pulses: effect.pulse_count,
                next_pulse_at: next_at,
                pulse_interval_secs: pulse_secs,
            });
            false
        }
    };

    tracing::info!(
        target: "abilities",
        event = if was_refresh { "active_effect_refreshed" } else { "active_effect_registered" },
        target_id,
        invoker_id,
        effect_id = effect.effect_id,
        ability_id = effect.ability_id,
        remaining_pulses = remaining,
        total_pulses = effect.pulse_count,
        pulse_interval = pulse_secs,
        "Active effect registered/refreshed"
    );

    // Send onTimerUpdate(TIMER_DURATION_EFFECT) so the client renders a
    // buff/debuff icon with the duration countdown. `effect_id` is the
    // packet's `id` field — the client uses it to correlate with the
    // clear packet when the effect expires.
    let total_time = effect.total_duration();
    let timer_bytes = serialize_timer_update(
        effect.effect_id,
        TIMER_DURATION_EFFECT,
        invoker_id as i32,
        total_time,
        total_time,
    );
    send_entity_method(target_id, ON_TIMER_UPDATE, timer_bytes, tx, space_mgr).await;

    true
}

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
pub async fn effect_pulse_tick(tx: &mpsc::Sender<CellToBaseMsg>, space_mgr: &mut SpaceManager) {
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
        // Identify which effects are due and clone enough to fire without
        // holding the borrow across awaits. Each due fire is followed by
        // a re-acquire to commit the schedule update.
        let due: Vec<(usize, ActiveEffectInstance, EffectDef)> = {
            let Some(entity) = space_mgr.get_entity(entity_id) else {
                continue;
            };
            entity
                .active_effects
                .iter()
                .enumerate()
                .filter(|(_, inst)| inst.next_pulse_at <= now && inst.remaining_pulses > 0)
                .filter_map(|(idx, inst)| {
                    space_mgr
                        .effect_defs
                        .get(&inst.effect_id)
                        .cloned()
                        .map(|def| (idx, inst.clone(), def))
                })
                .collect()
        };

        if due.is_empty() {
            continue;
        }

        // Fire each due pulse. We re-look up the entity every iteration
        // because between awaits another tick could mutate.
        for (idx, inst, effect_def) in &due {
            fire_pulse(entity_id, inst, effect_def, tx, space_mgr).await;
            // Update schedule + decrement on the matching instance.
            if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                if let Some(active) = entity.active_effects.get_mut(*idx) {
                    // Defensive: someone else may have already cleaned it.
                    if active.effect_id == inst.effect_id && active.invoker_id == inst.invoker_id {
                        active.remaining_pulses = active.remaining_pulses.saturating_sub(1);
                        if active.remaining_pulses > 0 {
                            active.next_pulse_at = now
                                + std::time::Duration::from_secs_f32(active.pulse_interval_secs);
                        }
                    }
                }
            }
        }

        // Sweep completed instances after the fire loop, sending
        // `onTimerUpdate` with `total_time = 0` so the client clears
        // the buff/debuff icon. Without the clear, the icon would
        // stick at "expiring in 0s" forever.
        let cleared_ids: Vec<(i32, u32)> = {
            let Some(entity) = space_mgr.get_entity_mut(entity_id) else {
                continue;
            };
            let mut cleared = Vec::new();
            entity.active_effects.retain(|inst| {
                if inst.remaining_pulses <= 0 {
                    cleared.push((inst.effect_id, inst.invoker_id));
                    false
                } else {
                    true
                }
            });
            cleared
        };
        for (cleared_effect, invoker) in cleared_ids {
            let zero_timer = serialize_timer_update(
                cleared_effect,
                TIMER_DURATION_EFFECT,
                invoker as i32,
                0.0,
                0.0,
            );
            send_entity_method(entity_id, ON_TIMER_UPDATE, zero_timer, tx, space_mgr).await;
        }
    }
}

/// Apply a single pulse to `target_id`. Re-dispatches the effect's
/// script if it has one, otherwise applies the legacy NVP damage path
/// (HealthDamage / FocusDamage as raw stat mutations).
async fn fire_pulse(
    target_id: u32,
    inst: &ActiveEffectInstance,
    effect: &EffectDef,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // Bail on dead target — pulses on corpses shouldn't fire.
    if space_mgr
        .get_entity(target_id)
        .and_then(|e| e.stats.get(HEALTH))
        .is_some_and(|s| s.cur <= 0)
    {
        tracing::debug!(
            target: "abilities",
            event = "pulse_skipped_dead_target",
            target_id,
            effect_id = inst.effect_id,
            "Skipping pulse — target is dead"
        );
        return;
    }

    // Script path takes precedence over NVP path so a registered
    // script can fully decide what happens on each pulse.
    if let Some(script_name) = effect.script_name.clone() {
        let mut ctx = super::EffectContext {
            source_id: inst.invoker_id,
            target_id,
            effect,
            space_mgr,
        };
        super::dispatch_by_name(&script_name, &mut ctx);
    } else {
        // Legacy path — read HealthDamage / FocusDamage NVPs and
        // mutate the target's stats directly. No QR roll per pulse
        // (the initial cast's QR result already determined hit type;
        // re-rolling per pulse would make DoT unpredictable).
        let h_dmg = effect.param_i32("HealthDamage");
        let f_dmg = effect.param_i32("FocusDamage");
        if let Some(target) = space_mgr.get_entity_mut(target_id) {
            if h_dmg > 0 {
                if let Some(stat) = target.stats.get_mut(HEALTH) {
                    let cur = stat.cur;
                    let new_cur = (cur - h_dmg).max(0);
                    stat.update(stat.min, new_cur, stat.max);
                }
            }
            if f_dmg > 0 {
                if let Some(stat) = target.stats.get_mut(FOCUS) {
                    let cur = stat.cur;
                    let new_cur = (cur - f_dmg).max(0);
                    stat.update(stat.min, new_cur, stat.max);
                }
            }
        }
    }

    // Flush any stat changes the pulse produced so the client renders
    // the bar update. Pulses don't generate effect-results packets in
    // v1 — that's a wire-format addition tracked alongside per-pulse
    // tick observability in a follow-up.
    let dirty = space_mgr.get_entity_mut(target_id).map(|t| {
        let d = t.stats.serialize_dirty();
        t.stats.clear_dirty();
        d
    });
    if let Some(bytes) = dirty {
        if !bytes.is_empty() {
            send_entity_method(
                target_id,
                crate::mercury::method_idx::ON_STAT_UPDATE,
                bytes,
                tx,
                space_mgr,
            )
            .await;
        }
    }

    tracing::debug!(
        target: "abilities",
        event = "effect_pulse_fired",
        target_id,
        invoker_id = inst.invoker_id,
        effect_id = inst.effect_id,
        ability_id = inst.ability_id,
        damage_type = DT_PHYSICAL,
        remaining_before_decrement = inst.remaining_pulses,
        "Effect pulse fired"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_entity::abilities::EffectDef;
    use std::collections::HashMap;
    use std::time::Duration;

    fn make_mgr() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="W" Instanced="false" MinX="-100" MaxX="100" MinY="-100" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="W" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr.create_entity(1, "W", [0.0; 3], [0.0; 3]).unwrap();
        mgr.create_entity(2, "W", [5.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(100);
            if let Some(s) = e.stats.get_mut(HEALTH) {
                s.update(0, 100, 100);
            }
        }
        if let Some(e) = mgr.get_entity_mut(2) {
            if let Some(s) = e.stats.get_mut(HEALTH) {
                s.update(0, 100, 100);
            }
        }
        mgr
    }

    fn make_dot_effect(pulse_count: i32, pulse_secs: f32, dmg: i32) -> EffectDef {
        let mut params = HashMap::new();
        params.insert("HealthDamage".to_string(), dmg.to_string());
        EffectDef {
            effect_id: 7777,
            ability_id: 1234,
            pulse_count,
            pulse_duration: pulse_secs,
            params,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn register_pulsing_effect_stores_remaining_pulse_count_minus_one() {
        let mut mgr = make_mgr();
        let effect = make_dot_effect(5, 1.0, 10);
        let (tx, _rx) = mpsc::channel(64);
        let now = Instant::now();
        let registered = register_active_effect(&mut mgr, 2, 1, &effect, now, &tx).await;
        assert!(registered);
        let active = &mgr.get_entity(2).unwrap().active_effects;
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].remaining_pulses, 4);
        assert_eq!(active[0].total_pulses, 5);
    }

    #[tokio::test]
    async fn register_single_shot_effect_does_not_register() {
        let mut mgr = make_mgr();
        let effect = make_dot_effect(1, 0.0, 10);
        let (tx, _rx) = mpsc::channel(64);
        let registered = register_active_effect(&mut mgr, 2, 1, &effect, Instant::now(), &tx).await;
        assert!(!registered);
        assert!(mgr.get_entity(2).unwrap().active_effects.is_empty());
    }

    #[tokio::test]
    async fn register_channelled_effect_skipped_until_release_path_lands() {
        let mut mgr = make_mgr();
        let effect = make_dot_effect(0, 1.0, 10);
        let (tx, _rx) = mpsc::channel(64);
        let registered = register_active_effect(&mut mgr, 2, 1, &effect, Instant::now(), &tx).await;
        assert!(!registered, "pulse_count=0 (channelled) skipped in v1");
        assert!(mgr.get_entity(2).unwrap().active_effects.is_empty());
    }

    #[tokio::test]
    async fn register_same_invoker_refreshes_existing_instance() {
        // Phase E stacking semantics: same source re-cast = refresh duration.
        let mut mgr = make_mgr();
        let effect = make_dot_effect(5, 1.0, 10);
        let (tx, _rx) = mpsc::channel(64);
        register_active_effect(&mut mgr, 2, 1, &effect, Instant::now(), &tx).await;
        // Burn a few pulses so we can detect a refresh.
        if let Some(t) = mgr.get_entity_mut(2) {
            if let Some(inst) = t.active_effects.first_mut() {
                inst.remaining_pulses = 1;
            }
        }
        register_active_effect(&mut mgr, 2, 1, &effect, Instant::now(), &tx).await;
        let active = &mgr.get_entity(2).unwrap().active_effects;
        assert_eq!(active.len(), 1, "same-source re-cast must NOT stack");
        assert_eq!(
            active[0].remaining_pulses, 4,
            "remaining_pulses refreshed to 4 (5 total - 1 just fired)"
        );
    }

    #[tokio::test]
    async fn register_different_invoker_stacks_separate_instance() {
        // Phase E stacking semantics: different source = stack.
        let mut mgr = make_mgr();
        let effect = make_dot_effect(5, 1.0, 10);
        let (tx, _rx) = mpsc::channel(64);
        register_active_effect(&mut mgr, 2, 1, &effect, Instant::now(), &tx).await;
        // A different invoker (id 99) lands the same effect_id.
        register_active_effect(&mut mgr, 2, 99, &effect, Instant::now(), &tx).await;
        let active = &mgr.get_entity(2).unwrap().active_effects;
        assert_eq!(active.len(), 2, "different invokers must stack");
        let invokers: Vec<u32> = active.iter().map(|i| i.invoker_id).collect();
        assert!(invokers.contains(&1));
        assert!(invokers.contains(&99));
    }

    #[tokio::test]
    async fn pulse_tick_fires_due_pulse_and_decrements_remaining() {
        let mut mgr = make_mgr();
        let effect = make_dot_effect(5, 1.0, 10);
        mgr.effect_defs.insert(effect.effect_id, effect.clone());
        let past = Instant::now() - Duration::from_secs(2);
        let (tx, _rx) = mpsc::channel(64);
        register_active_effect(&mut mgr, 2, 1, &effect, past, &tx).await;
        if let Some(t) = mgr.get_entity_mut(2) {
            if let Some(inst) = t.active_effects.first_mut() {
                inst.next_pulse_at = past;
            }
        }
        let hp_before = mgr.get_entity(2).unwrap().stats.get(HEALTH).unwrap().cur;
        effect_pulse_tick(&tx, &mut mgr).await;
        let hp_after = mgr.get_entity(2).unwrap().stats.get(HEALTH).unwrap().cur;
        assert_eq!(hp_after, hp_before - 10, "DoT pulse should subtract 10");
        let active = &mgr.get_entity(2).unwrap().active_effects;
        assert_eq!(active[0].remaining_pulses, 3);
    }

    #[tokio::test]
    async fn pulse_tick_removes_instance_when_remaining_hits_zero() {
        let mut mgr = make_mgr();
        let mut effect = make_dot_effect(2, 1.0, 10);
        effect.effect_id = 8888;
        mgr.effect_defs.insert(effect.effect_id, effect.clone());
        let (tx, _rx) = mpsc::channel(64);
        register_active_effect(&mut mgr, 2, 1, &effect, Instant::now(), &tx).await;
        if let Some(t) = mgr.get_entity_mut(2) {
            if let Some(inst) = t.active_effects.first_mut() {
                inst.next_pulse_at = Instant::now() - Duration::from_secs(2);
                inst.remaining_pulses = 1;
            }
        }
        effect_pulse_tick(&tx, &mut mgr).await;
        let active = &mgr.get_entity(2).unwrap().active_effects;
        assert!(
            active.is_empty(),
            "instance removed after its last pulse fires"
        );
    }

    #[tokio::test]
    async fn pulse_tick_skips_pulse_on_dead_target() {
        let mut mgr = make_mgr();
        let effect = make_dot_effect(5, 1.0, 10);
        mgr.effect_defs.insert(effect.effect_id, effect.clone());
        let (tx, _rx) = mpsc::channel(64);
        register_active_effect(&mut mgr, 2, 1, &effect, Instant::now(), &tx).await;
        if let Some(t) = mgr.get_entity_mut(2) {
            if let Some(s) = t.stats.get_mut(HEALTH) {
                s.update(0, 0, 100);
            }
            if let Some(inst) = t.active_effects.first_mut() {
                inst.next_pulse_at = Instant::now() - Duration::from_secs(2);
            }
        }
        effect_pulse_tick(&tx, &mut mgr).await;
        let hp = mgr.get_entity(2).unwrap().stats.get(HEALTH).unwrap().cur;
        assert_eq!(hp, 0);
        let remaining = mgr.get_entity(2).unwrap().active_effects[0].remaining_pulses;
        assert_eq!(remaining, 3);
    }
}
