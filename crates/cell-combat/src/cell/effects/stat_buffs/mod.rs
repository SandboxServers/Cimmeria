//! The async half of the timed effect ledger (`CellEntity::stat_buffs`:
//! the consumable stimpacks and, since ability mechanics AB-04, every
//! ability buff and debuff the `TimedStat` script lands): expiry, the
//! client's duration timers, and the clear hooks.
//!
//! The ledger itself and its scripts are synchronous and live in
//! `cimmeria-cell-world` (`effects::stat_buff`) and
//! `cimmeria-cell-effect-scripts`; a script cannot send, so the ledger
//! records what the client still needs to hear (a start for each effect
//! whose icon changed, a clear for each effect whose last entry came off;
//! one icon per `effect_id`, see `StatBuffLedger`) and this module sends it:
//!
//! - [`flush_stat_buff_timers`] sends the owed timers. Every caller that
//!   runs an effect script calls it right after (`fire_beneficial`,
//!   `damage_apply`, content's `apply_effect`), so the icon lands with the
//!   stat change; the tick calls it too, as the safety net for any other
//!   path that runs the script.
//! - [`stat_buff_tick_at`] runs every AoI tick (100 ms) from the cell loop,
//!   beside the owner-pet tick. It takes expired entries off, flushes the
//!   timers and the changed stats, and returns at once when no entity has
//!   ledger work.
//! - [`strip_timed_effects`] takes off the entries a predicate picks, with a
//!   [`StatBuffRemoval`] reason, and tells the client: the one async seam
//!   for every clear hook. [`clear_stat_buffs_on_death`] is its
//!   `EF_ClearOnDeath` caller (`resolve_death`, every death); AB-11's
//!   ClearOnDamage / ClearOnRez / bandolier hooks use it the same way. A
//!   toggle-off and a stance switch remove inside the script
//!   (`stat_buff::held`), and the caster's `fire_beneficial` flush sends
//!   their clears.
//!
//! The wire is the one every duration effect already uses:
//! `onTimerUpdate(effect_id, TIMER_DURATION_EFFECT, invoker, effect_id,
//! TotalTime, BigWorldTimeComplete)`, the expiry absolute on the server's
//! game clock (decision 22 of the abilities ADR), and `0.0, 0.0` to clear.
//! A held toggle entry (no expiry, a stance) gets a start timer with a long
//! horizon, [`HELD_ICON_SECS`], as both its `TotalTime` and its time left.
//! The client keeps an effect only while `clock < BigWorldTimeComplete`
//! (`EffectSet_HandleOnTimerUpdate`, effect-execution-model.md), the stock
//! effect bar (`Effect.lua`) draws every non-hidden entry with
//! `TimeRemaining / TotalTime` as its sweep, so a zero `TotalTime` would
//! divide by zero, and the action bar has no toggled state to show instead
//! (`ActionButtons.lua` reads only cooldowns from `getActionInfo`). So the
//! effect bar is the only stock surface, and a far expiry is how it shows an
//! effect with none. A passive (`EF_AlwaysPersist`) gets no icon: it is a
//! trait, not a state the player switches, and the bar has ten slots a side
//! (B-73). Either removal still sends the clear. The stat moves reach the
//! client as an ordinary `onStatUpdate`.

use std::time::Instant;

use tokio::sync::mpsc;

use cimmeria_entity::abilities::{
    serialize_timer_update, EF_ALWAYS_PERSIST, EF_CLEAR_ON_DEATH, TIMER_DURATION_EFFECT,
};

use cimmeria_entity::cell_entity::TimedEffect;

use crate::cell::abilities::{send_entity_method, send_timer_update};
use crate::cell::effects::stat_buff::StatBuffRemoval;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

#[cfg(test)]
mod tests;

/// The horizon a held toggle's icon counts down from: a day, longer than
/// any play session. The client shows it as the time remaining in the
/// icon's tooltip; nothing on the server expires at the end of it.
pub const HELD_ICON_SECS: f32 = 86_400.0;

/// [`stat_buff_tick_at`] on the wall clock.
pub async fn stat_buff_tick(tx: &mpsc::Sender<CellToBaseMsg>, space_mgr: &mut SpaceManager) {
    let _ = stat_buff_tick_at(Instant::now(), tx, space_mgr).await;
}

/// Take off every entry due by `now`, then send each entity's owed timers
/// and changed stats. Returns how many entries expired. `now` is a
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
            .remove_timed_effects(entity_id, StatBuffRemoval::Expired, |b| b.is_expired(now))
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
/// (an effect whose last entry came off), then one start per effect with an
/// entry the client has not been told about, carrying the latest expiry of
/// that effect's entries. Nothing is sent for an entity with nothing owed.
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
    // One icon per effect_id: the client keys it by SecondaryId alone, so
    // stacked casters share it. Its start carries the entry that lapses last
    // (its source and length); a held toggle counts down from the horizon.
    let mut stale: Vec<i32> = entity
        .stat_buffs
        .entries
        .iter()
        .filter(|b| !b.timer_sent)
        .map(|b| b.effect_id)
        .collect();
    stale.sort_unstable();
    stale.dedup();
    let mut starts: Vec<(i32, u32, f32, f32)> = Vec::new();
    for effect_id in stale {
        let latest = entity
            .stat_buffs
            .entries
            .iter()
            .filter(|b| b.effect_id == effect_id)
            .filter_map(|b| b.expires_at.map(|t| (t, b)))
            .max_by_key(|(t, _)| *t);
        if let Some((expires_at, entry)) = latest {
            let remaining = expires_at.saturating_duration_since(now).as_secs_f32();
            starts.push((effect_id, entry.invoker_id, entry.duration_secs, remaining));
        } else if let Some(held) = entity.stat_buffs.entries.iter().find(|b| {
            b.effect_id == effect_id
                && b.expires_at.is_none()
                && b.effect_flags & EF_ALWAYS_PERSIST == 0
        }) {
            starts.push((effect_id, held.invoker_id, HELD_ICON_SECS, HELD_ICON_SECS));
        }
        for b in entity
            .stat_buffs
            .entries
            .iter_mut()
            .filter(|b| b.effect_id == effect_id)
        {
            b.timer_sent = true;
        }
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
        let complete = crate::mercury::game_clock::game_time_secs() + remaining;
        let args = serialize_timer_update(
            effect_id,
            TIMER_DURATION_EFFECT,
            invoker_id as i32,
            effect_id,
            total,
            complete,
        );
        send_timer_update(entity_id, args, tx, space_mgr).await;
    }
}

/// Take off `entity_id`'s entries for which `pred` holds, logging each with
/// `why`, and send the client the timer clears and the restored stats.
/// Returns how many came off. The async seam for every clear hook.
pub async fn strip_timed_effects(
    entity_id: u32,
    why: StatBuffRemoval,
    pred: impl Fn(&TimedEffect) -> bool,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> usize {
    let has_any = space_mgr
        .get_entity(entity_id)
        .is_some_and(|e| !e.stat_buffs.entries.is_empty());
    if !has_any {
        return 0;
    }
    let removed = space_mgr.remove_timed_effects(entity_id, why, pred).len();
    if removed > 0 {
        flush_stat_buff_timers(entity_id, Instant::now(), tx, space_mgr).await;
        flush_stats(entity_id, tx, space_mgr).await;
    }
    removed
}

/// Take off `entity_id`'s entries whose effect carries `EF_ClearOnDeath`,
/// and tell the client. Called from `resolve_death` for every death.
pub async fn clear_stat_buffs_on_death(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> usize {
    strip_timed_effects(
        entity_id,
        StatBuffRemoval::Death,
        |b| b.effect_flags & EF_CLEAR_ON_DEATH != 0,
        tx,
        space_mgr,
    )
    .await
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
