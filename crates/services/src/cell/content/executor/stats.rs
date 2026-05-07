//! `Action::ChangeStat` — apply min/max/set_to_max/amount adjustments to a
//! stat on the calling entity, broadcast the resulting onStatUpdate.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Apply the bounds-and-delta sequence (`min` / `max` / `set_to_max` / `amount`)
/// to the entity's stat, then broadcast the dirty payload as an
/// `onStatUpdate`. The legacy `stat_id = -1` sentinel and the explicit
/// `use_ammo_stat = Some(true)` form both skip cleanly — the active-ammo-slot
/// resolution path is not yet implemented.
#[allow(clippy::too_many_arguments)]
pub(super) async fn change_stat(
    stat_id: i32,
    min: Option<i32>,
    max: Option<i32>,
    set_to_max: Option<bool>,
    amount: Option<i32>,
    use_ammo_stat: Option<bool>,
    entity_id: u32,
    chain_id: i64,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // `use_ammo_stat=true` (or the legacy `stat_id=-1` sentinel
    // chain 2011 uses) means "resolve to the entity's active
    // ammo slot stat at apply time". That bandolier-aware
    // resolution is its own piece of work — implementing it
    // here would also need to track which slot the action
    // targets across grant / equip / reload cycles. Warn so
    // the silent no-op is visible in production logs (the
    // calling chain almost certainly expects the side
    // effect); flip back to info/debug when the resolution
    // path lands.
    if use_ammo_stat == Some(true) || stat_id < 0 {
        tracing::warn!(
            entity_id,
            chain_id,
            stat_id,
            ?use_ammo_stat,
            "Content: ChangeStat use_ammo_stat / negative stat_id \
             is not yet implemented; skipping (deliberate stub)"
        );
        return;
    }

    // Apply bounds first so `set_to_max` and `amount` see the
    // adjusted range, then `set_to_max`, then `amount` (the
    // delta path consumables use). All stat mutations bump the
    // dirty flag; `serialize_dirty` collects them into one
    // `onStatUpdate` payload.
    let payload = match space_mgr.get_entity_mut(entity_id) {
        Some(entity) => match entity.stats.get_mut(stat_id) {
            Some(stat) => {
                if let Some(new_min) = min {
                    stat.set_min(new_min);
                }
                if let Some(new_max) = max {
                    stat.set_max(new_max);
                }
                if set_to_max == Some(true) {
                    stat.set_current(stat.max);
                }
                if let Some(delta) = amount {
                    stat.change(delta);
                }
                tracing::info!(
                    entity_id,
                    stat_id,
                    ?min,
                    ?max,
                    ?set_to_max,
                    ?amount,
                    cur = stat.cur,
                    max = stat.max,
                    chain_id,
                    "Content: ChangeStat applied"
                );
                let p = entity.stats.serialize_dirty();
                entity.stats.clear_dirty();
                p
            }
            None => {
                tracing::warn!(
                    entity_id,
                    stat_id,
                    chain_id,
                    "Content: ChangeStat target stat not found"
                );
                Vec::new()
            }
        },
        None => {
            tracing::warn!(
                entity_id,
                chain_id,
                "Content: ChangeStat source entity not found"
            );
            Vec::new()
        }
    };

    if !payload.is_empty() {
        if let Err(e) = tx
            .send(CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index: crate::mercury::method_idx::ON_STAT_UPDATE,
                args: payload,
            })
            .await
        {
            tracing::error!(
                entity_id, chain_id, error = %e,
                "Content: ChangeStat onStatUpdate send failed"
            );
        }
    }
}
