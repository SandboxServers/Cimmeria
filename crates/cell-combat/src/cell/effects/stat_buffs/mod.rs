//! The async half of the stat-buff ledger (`CellEntity::stat_buffs`, the
//! consumable stimpacks): expiry, the client's duration timers, and the
//! death strip.
//!
//! The ledger itself and the `StatBuff` script are synchronous and live in
//! `cimmeria-cell-world` (`effects::stat_buff`); a script cannot send, so
//! the ledger records what the client still needs to hear (a start timer
//! for each new buff, a clear for each effect whose last buff came off) and
//! this module sends it:
//!
//! - [`flush_stat_buff_timers`] sends the owed timers. Content-applied
//!   effects (`cell::content`'s `apply_effect`) call it right after the
//!   script runs, so the buff icon lands with the stat change; the tick
//!   calls it too, as the safety net for any other path that runs the
//!   script.
//! - [`stat_buff_tick_at`] runs every AoI tick (100 ms) from the cell loop,
//!   beside the owner-pet tick. It takes expired buffs off, flushes the
//!   timers and the changed stats, and returns at once when no entity has
//!   ledger work.
//! - [`clear_stat_buffs_on_death`] takes off the buffs whose effect carries
//!   `EF_ClearOnDeath`. `resolve_death` calls it for every death. No
//!   stimpack sets that bit (they carry `EF_Offline_Time_Counts`), so today
//!   a stim outlasts a death, as its row says.
//!
//! The wire is the one every duration effect already uses:
//! `onTimerUpdate(effect_id, TIMER_DURATION_EFFECT, invoker, effect_id,
//! TotalTime, BigWorldTimeComplete)`, the expiry absolute on the server's
//! game clock (decision 22 of the abilities ADR), and `0.0, 0.0` to clear.
//! The stat moves reach the client as an ordinary `onStatUpdate`.

use std::time::Instant;

use tokio::sync::mpsc;

use cimmeria_entity::abilities::{
    serialize_timer_update, EF_CLEAR_ON_DEATH, TIMER_DURATION_EFFECT,
};

use crate::cell::abilities::{send_entity_method, send_timer_update};
use crate::cell::effects::stat_buff::StatBuffRemoval;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

#[cfg(test)]
mod tests;

/// [`stat_buff_tick_at`] on the wall clock.
pub async fn stat_buff_tick(tx: &mpsc::Sender<CellToBaseMsg>, space_mgr: &mut SpaceManager) {
    let _ = stat_buff_tick_at(Instant::now(), tx, space_mgr).await;
}

/// Take off every stat buff due by `now`, then send each entity's owed
/// timers and changed stats. Returns how many buffs expired. `now` is a
/// parameter so tests can step past an hour without sleeping.
pub async fn stat_buff_tick_at(
    now: Instant,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> usize {
    let work = space_mgr.entities_with_stat_buff_work();
    if work.is_empty() {
        return 0;
    }
    let mut expired = 0;
    let due = space_mgr.entities_with_expired_stat_buffs(now);
    for &entity_id in &due {
        expired += space_mgr
            .remove_stat_buffs(entity_id, StatBuffRemoval::Expired, |b| b.expires_at <= now)
            .len();
    }
    for entity_id in work {
        flush_stat_buff_timers(entity_id, now, tx, space_mgr).await;
    }
    // Only the entities whose stats this tick moved: flushing a buffed
    // entity every tick would send (and clear) dirty bits other systems
    // set and flush themselves.
    for entity_id in due {
        flush_stats(entity_id, tx, space_mgr).await;
    }
    expired
}

/// Send `entity_id` the duration timers its ledger owes: first the clears
/// (an effect whose last buff came off), then one start per effect with a
/// buff the client has not been told about. Nothing is sent for an entity
/// with nothing owed.
pub async fn flush_stat_buff_timers(
    entity_id: u32,
    now: Instant,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(entity) = space_mgr.get_entity_mut(entity_id) else {
        return;
    };
    let clears = std::mem::take(&mut entity.stat_buffs.pending_timer_clears);
    // One start per effect: a multi-stat effect shares one icon.
    let mut starts: Vec<(i32, u32, f32, f32)> = Vec::new();
    for buff in entity.stat_buffs.buffs.iter_mut().filter(|b| !b.timer_sent) {
        buff.timer_sent = true;
        if starts
            .iter()
            .any(|&(effect_id, ..)| effect_id == buff.effect_id)
        {
            continue;
        }
        let remaining = buff.expires_at.saturating_duration_since(now).as_secs_f32();
        starts.push((
            buff.effect_id,
            buff.invoker_id,
            buff.duration_secs,
            remaining,
        ));
    }
    for (effect_id, invoker_id) in clears {
        let args = serialize_timer_update(
            effect_id,
            TIMER_DURATION_EFFECT,
            invoker_id as i32,
            effect_id,
            0.0,
            0.0,
        );
        send_timer_update(entity_id, args, tx, space_mgr).await;
    }
    for (effect_id, invoker_id, total, remaining) in starts {
        let args = serialize_timer_update(
            effect_id,
            TIMER_DURATION_EFFECT,
            invoker_id as i32,
            effect_id,
            total,
            crate::mercury::game_clock::game_time_secs() + remaining,
        );
        send_timer_update(entity_id, args, tx, space_mgr).await;
    }
}

/// Take off `entity_id`'s buffs whose effect carries `EF_ClearOnDeath`,
/// and tell the client. Called from `resolve_death` for every death.
pub async fn clear_stat_buffs_on_death(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> usize {
    let has_any = space_mgr
        .get_entity(entity_id)
        .is_some_and(|e| !e.stat_buffs.buffs.is_empty());
    if !has_any {
        return 0;
    }
    let removed = space_mgr
        .remove_stat_buffs(entity_id, StatBuffRemoval::Died, |b| {
            b.effect_flags & EF_CLEAR_ON_DEATH != 0
        })
        .len();
    if removed > 0 {
        flush_stat_buff_timers(entity_id, Instant::now(), tx, space_mgr).await;
        flush_stats(entity_id, tx, space_mgr).await;
    }
    removed
}

/// Send `entity_id`'s dirty stats (`onStatUpdate`) and clear the flags.
async fn flush_stats(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(entity) = space_mgr.get_entity_mut(entity_id) else {
        return;
    };
    if !entity.stats.has_dirty() {
        return;
    }
    let dirty = entity.stats.serialize_dirty();
    entity.stats.clear_dirty();
    send_entity_method(
        entity_id,
        crate::mercury::method_idx::ON_STAT_UPDATE,
        dirty,
        tx,
        space_mgr,
    )
    .await;
}
