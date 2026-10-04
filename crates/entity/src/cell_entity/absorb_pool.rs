//! Absorb shields on the timed effect ledger (ability-mechanics AB-10).
//!
//! A shield ("Absorption: 500 Physical") is a ledger entry that also holds
//! one [`AbsorbPool`] per damage type it blocks. The pool is mutable: the
//! damage it soaks comes off `remaining`, and the entry comes off the ledger
//! when every pool is empty or when it expires, whichever is first.
//!
//! **The `absorb*` stat is the pool's public face.** `alias.xml` defines
//! `absorbPhysical` ("how much physical damage to absorb") and its siblings
//! as stats, so the client sees a shield through `onStatUpdate`, and the
//! damage pipeline (`calculate_damage_penetrating`) already drains them. So
//! granting a pool adds its amount to the stat, and the pipeline keeps
//! draining the stat; [`CellEntity::settle_absorb_pools`] then charges what
//! the stat lost to the ledger's pools, oldest entry first, and takes off
//! the entries it emptied. Removing an entry for any other reason (expiry,
//! death, a cleanse, a refresh) takes its pools' `remaining` back off the
//! stat. Before this the `AbsorbShield` script added to the stat and nothing
//! ever took a timed shield's capacity back off.
//!
//! The stat can hold capacity no ledger pool owns (an item, an older
//! pulsing shield). Settling charges a pool only for what the stat lost
//! below the ledger's total, so that capacity is spent first.

use super::stat_buff::TimedEffect;
use super::CellEntity;

/// One damage type's share of a shield entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbsorbPool {
    /// The `absorb*` stat it fills (`stats::ABSORB_PHYSICAL`, ...).
    pub stat_id: i32,
    /// What it added to the stat when it went on (its asked-for amount,
    /// less anything the stat's max refused).
    pub granted: i32,
    /// What is left; damage of the stat's type takes it down.
    pub remaining: i32,
}

impl TimedEffect {
    /// Whether it is a shield whose every pool is spent.
    pub fn is_drained(&self) -> bool {
        !self.absorb.is_empty() && self.absorb.iter().all(|p| p.remaining <= 0)
    }

    /// Capacity left across its pools.
    pub fn absorb_remaining(&self) -> i32 {
        self.absorb.iter().map(|p| p.remaining.max(0)).sum()
    }
}

/// What [`CellEntity::settle_absorb_pools`] did.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AbsorbSettlement {
    /// `(effect_id, invoker_id, stat_id, absorbed)` for every pool that
    /// paid for damage, in charge order.
    pub charged: Vec<(i32, u32, i32, i32)>,
    /// The entries it emptied and took off.
    pub drained: Vec<TimedEffect>,
}

impl CellEntity {
    /// Add each `(stat_id, amount)` to its `absorb*` stat for a new entry.
    /// Returns the pools, each holding what the stat really took; a stat the
    /// entity lacks, a non-positive amount or a stat already at its max adds
    /// no pool.
    pub(super) fn grant_absorb(&mut self, wanted: &[(i32, i32)]) -> Vec<AbsorbPool> {
        wanted
            .iter()
            .filter(|&&(_, amount)| amount > 0)
            .filter_map(|&(stat_id, amount)| {
                let stat = self.stats.get_mut(stat_id)?;
                let granted = stat.change(amount);
                (granted > 0).then_some(AbsorbPool {
                    stat_id,
                    granted,
                    remaining: granted,
                })
            })
            .collect()
    }

    /// Take a removed entry's unspent capacity back off its stats. The
    /// entry is already out of the ledger.
    pub(super) fn release_absorb(&mut self, entry: &TimedEffect) {
        for pool in &entry.absorb {
            if let Some(stat) = self.stats.get_mut(pool.stat_id) {
                let back = pool.remaining.clamp(0, stat.cur.max(0));
                if back > 0 {
                    stat.change(-back);
                }
            }
        }
    }

    /// Charge what each `absorb*` stat lost since the pools were last
    /// settled to the ledger's pools of that stat (oldest entry first), and
    /// take off every shield entry that is now empty. Call it after anything
    /// that drains the stats: a hit, a pulse, a damage script.
    pub fn settle_absorb_pools(&mut self) -> AbsorbSettlement {
        let mut out = AbsorbSettlement::default();
        let mut stat_ids: Vec<i32> = self
            .stat_buffs
            .entries
            .iter()
            .flat_map(|e| e.absorb.iter().map(|p| p.stat_id))
            .collect();
        stat_ids.sort_unstable();
        stat_ids.dedup();
        for stat_id in stat_ids {
            let owned: i32 = self
                .stat_buffs
                .entries
                .iter()
                .flat_map(|e| e.absorb.iter())
                .filter(|p| p.stat_id == stat_id)
                .map(|p| p.remaining.max(0))
                .sum();
            let cur = self.stats.get(stat_id).map_or(0, |s| s.cur.max(0));
            let mut spent = owned - cur;
            for entry in self.stat_buffs.entries.iter_mut() {
                if spent <= 0 {
                    break;
                }
                for pool in entry.absorb.iter_mut().filter(|p| p.stat_id == stat_id) {
                    let take = spent.min(pool.remaining.max(0));
                    if take > 0 {
                        pool.remaining -= take;
                        spent -= take;
                        out.charged
                            .push((entry.effect_id, entry.invoker_id, stat_id, take));
                    }
                }
            }
        }
        out.drained = self.remove_timed_effects_where(TimedEffect::is_drained);
        out
    }
}
