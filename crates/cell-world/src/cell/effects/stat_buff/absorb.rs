//! The logged face of the shields' absorb pools (ability-mechanics AB-10).
//! The arithmetic is `CellEntity::settle_absorb_pools` in
//! `cimmeria-entity` (`cell_entity/absorb_pool.rs`); this adds the rows.
//!
//! Every damage seam that can drain an `absorb*` stat calls
//! [`SpaceManager::settle_absorb_shields`] once it has written the target's
//! stats: the hit in `damage_apply` (NVP and script damage) and each DoT
//! pulse. A shield emptied there comes off with `reason = drained`, and its
//! icon clear is queued for the caller's `flush_stat_buff_timers`.
//!
//! Identity follows the ledger's rows: `entity_id`, `account_id` and
//! `player_id` name the shield's invoker, `target_id` and
//! `target_player_id` the entity that holds it.

use crate::cell::space_manager::SpaceManager;

use super::ledger::log_removed;
use super::StatBuffRemoval;

impl SpaceManager {
    /// Charge what `target`'s `absorb*` stats lost to its shield entries'
    /// pools, oldest first, and take off every shield now empty. Returns
    /// how many came off. A target with no shield costs one lookup.
    pub fn settle_absorb_shields(&mut self, target: u32) -> usize {
        let has_shield = self
            .get_entity(target)
            .is_some_and(|e| e.stat_buffs.entries.iter().any(|b| !b.absorb.is_empty()));
        if !has_shield {
            return 0;
        }
        let target_who = self.player_identity(target);
        let world = crate::cell::effects::ability_metrics::world_of(self, target);
        let Some(entity) = self.get_entity_mut(target) else {
            return 0;
        };
        let settled = entity.settle_absorb_pools();
        for &(effect_id, invoker_id, stat_id, absorbed) in &settled.charged {
            let left = entity
                .stat_buffs
                .entries
                .iter()
                .find(|b| b.key() == (effect_id, invoker_id))
                .map_or(0, |b| b.absorb_remaining());
            // The shield's entry names who put it up and with which cast
            // (AB-T1): the pools live on the entry, so its `cast_id` is theirs.
            let (who, cast_id) = settled
                .drained
                .iter()
                .chain(entity.stat_buffs.entries.iter())
                .find(|b| b.key() == (effect_id, invoker_id))
                .map(|b| (b.invoker_identity, b.cast_id))
                .unwrap_or_default();
            tracing::debug!(
                target: "abilities",
                event = "shield_absorbed",
                account_id = who.account_id,
                player_id = who.player_id,
                entity_id = invoker_id,
                target_id = target,
                target_player_id = target_who.player_id,
                effect_id,
                cast_id,
                stat_id,
                absorbed,
                absorb_left = left,
                "shield pool absorbed damage"
            );
        }
        for entry in &settled.drained {
            // Nothing is left to restore: the pools are empty, and the
            // entry's own stats (none on a seeded shield) went back with it.
            let restored: Vec<Option<i32>> = entry
                .stats
                .iter()
                .map(|s| entity.stats.get(s.stat_id).map(|st| st.cur))
                .collect();
            log_removed(
                target,
                target_who,
                entry,
                StatBuffRemoval::Drained,
                (restored.clone(), restored),
                world,
            );
        }
        settled.drained.len()
    }
}
