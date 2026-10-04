//! `.cleareffects [target]` — strip every effect from the selected target
//! (else the caller): the teardown step between two UAT rows.
//!
//! The same removal a cleanse does (`cleanse` in
//! `cimmeria-cell-effect-scripts`), applied to everything:
//!
//! - every ledger entry comes off through
//!   `SpaceManager::remove_timed_effects` with reason `cleansed`, which
//!   restores exactly the stats it moved, takes a shield's unspent pool back
//!   off its stat, releases its state-flag references and queues its icon
//!   clear (one `stat_buff_removed` row each);
//! - every pulsing instance comes off `active_effects`, its script's
//!   `on_remove` runs (a stateful script undoes what it did), and its icon
//!   clear is queued;
//! - then the owed clears go out (`onTimerUpdate` with zero times, to the
//!   entity's own client), with the `onStateFieldUpdate` a released stun owes
//!   and the restored stats (`onStatUpdate`).
//!
//! Cooldowns and a warmup are left alone: `.cooldowns reset` is the tool
//! for the first, and a warmup ends on its own in seconds.

use std::time::Instant;

use cimmeria_cell_world::cell::effects::stat_buff::StatBuffRemoval;
use cimmeria_cell_world::cell::effects::{dispatch_on_remove, EffectContext};
use tokio::sync::mpsc;

use super::display_name;
use crate::cell::abilities::send_entity_method;
use crate::cell::client_methods::combatant::ON_STAT_UPDATE;
use crate::cell::console::send_gm_feedback;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

pub(super) async fn run(
    caller_id: u32,
    subject: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    if space_mgr.get_entity(subject).is_none() {
        let line = format!(".cleareffects: entity {subject} is gone");
        return send_gm_feedback(caller_id, &line, tx).await;
    }
    let ledger = space_mgr.remove_timed_effects(subject, StatBuffRemoval::Cleansed, |_| true);
    let pulses = strip_pulses(subject, space_mgr);

    cimmeria_cell_combat::cell::effects::flush_stat_buff_timers(
        subject,
        Instant::now(),
        tx,
        space_mgr,
    )
    .await;
    flush_stats(subject, tx, space_mgr).await;

    let ledger_ids: Vec<i32> = ledger.iter().map(|e| e.effect_id).collect();
    let pulse_ids: Vec<i32> = pulses.iter().map(|&(effect_id, _)| effect_id).collect();
    let gm = space_mgr.player_identity(caller_id);
    let target = space_mgr.player_identity(subject);
    tracing::info!(
        target: "abilities.gm",
        event = "effects_cleared",
        reason = StatBuffRemoval::Cleansed.reason(),
        entity_id = caller_id,
        account_id = gm.account_id,
        player_id = gm.player_id,
        target_id = subject,
        target_player_id = target.player_id,
        ledger_removed = ledger_ids.len(),
        ledger_effect_ids = ?ledger_ids,
        pulses_removed = pulse_ids.len(),
        pulse_effect_ids = ?pulse_ids,
        "GM stripped every timed and pulsing effect"
    );
    let line = if ledger_ids.is_empty() && pulse_ids.is_empty() {
        format!(
            "cleareffects [{subject}] {}: nothing to clear",
            display_name(space_mgr, subject)
        )
    } else {
        format!(
            "cleareffects [{subject}] {}: {} ledger entr(ies) {:?} and {} pulsing effect(s) {:?} removed; icon clears sent",
            display_name(space_mgr, subject),
            ledger_ids.len(),
            ledger_ids,
            pulse_ids.len(),
            pulse_ids
        )
    };
    send_gm_feedback(caller_id, &line, tx).await;
}

/// Take every pulsing instance off `subject`, run each script's `on_remove`
/// and queue each effect's icon clear once. Returns `(effect_id, invoker_id)`
/// of each instance removed.
fn strip_pulses(subject: u32, space_mgr: &mut SpaceManager) -> Vec<(i32, u32)> {
    let Some(entity) = space_mgr.get_entity_mut(subject) else {
        return Vec::new();
    };
    let removed: Vec<(i32, u32)> = std::mem::take(&mut entity.active_effects)
        .into_iter()
        .map(|i| (i.effect_id, i.invoker_id))
        .collect();
    for &(effect_id, invoker_id) in &removed {
        if let Some(def) = space_mgr.effect_defs.get(&effect_id).cloned() {
            if let Some(script) = def.script_name.clone() {
                let mut ctx = EffectContext {
                    source_id: invoker_id,
                    target_id: subject,
                    effect: &def,
                    space_mgr,
                };
                dispatch_on_remove(&script, &mut ctx);
            }
        }
        if let Some(entity) = space_mgr.get_entity_mut(subject) {
            let owed = &mut entity.stat_buffs.pending_timer_clears;
            if !owed.iter().any(|&(e, _)| e == effect_id) {
                owed.push((effect_id, invoker_id));
            }
        }
    }
    removed
}

/// Send `subject`'s dirty stats and clear the flags (the restored stats).
async fn flush_stats(subject: u32, tx: &mpsc::Sender<CellToBaseMsg>, space_mgr: &mut SpaceManager) {
    let Some(entity) = space_mgr.get_entity_mut(subject) else {
        return;
    };
    if !entity.stats.has_dirty() {
        return;
    }
    let dirty = entity.stats.serialize_dirty();
    entity.stats.clear_dirty();
    send_entity_method(subject, ON_STAT_UPDATE, dirty, tx, space_mgr).await;
}
