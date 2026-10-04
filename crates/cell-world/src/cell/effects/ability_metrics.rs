//! The ability metrics that live below the combat crate (ability-mechanics
//! AB-T6): the damage and heal histograms and the ledger-removal counter,
//! plus the `world` label every ability metric carries.
//!
//! The rest of the AB-T6 set (casts, refusals, effect plans, QR results,
//! failed wire sends, press to fire) is in `cimmeria-cell-combat`'s
//! `abilities::metrics`, which re-exports this module so one pinning test
//! covers every ability label.
//!
//! **Labels are enumerated** (Rule 4 of `instrumentation-discipline.md`):
//! [`Pool`] and [`StatBuffRemoval`] here, and `world`, which is bounded by
//! the world table (about 70 rows). Correlators (`cast_id`, `player_id`)
//! stay on the log rows.

use std::collections::HashSet;
use std::sync::{OnceLock, RwLock};

use cimmeria_entity::stats::{FOCUS, HEALTH};

use super::stat_buff::StatBuffRemoval;
use crate::cell::space_manager::SpaceManager;

/// `abilities_damage_dealt{pool, world}`: what one effect took from one
/// target's pool (before a GM god-mode restore).
pub const DAMAGE_DEALT: &str = "abilities_damage_dealt";
/// `abilities_heal_done{pool, world}`: what one effect put back.
pub const HEAL_DONE: &str = "abilities_heal_done";
/// `abilities_ledger_removed_total{reason, world}`: timed effects taken off
/// the ledger, by [`StatBuffRemoval`].
pub const LEDGER_REMOVED_TOTAL: &str = "abilities_ledger_removed_total";

/// `world` when the entity is in no loaded space.
pub const UNKNOWN_WORLD: &str = "unknown";

cimmeria_observability::metric_label! {
    /// The pool a damage or heal sample moved.
    pub enum Pool {
        Health => "health",
        Focus => "focus",
        /// What a shield's absorb pools took instead of Health or Focus.
        Absorb => "absorb",
    }
}

/// Whether the metrics facade emits anything. Callers skip work that only
/// feeds a metric (a pool sample, a world lookup) when it does not.
pub fn enabled() -> bool {
    cimmeria_observability::meter().is_some()
}

/// The `world` label for `entity_id`: its space's world name, interned so
/// labels are `&'static` and cheap to copy into row structs. The set is
/// bounded by the world table. [`UNKNOWN_WORLD`] when the entity is in no
/// loaded space, or when metrics are off (nothing reads it then).
pub fn world_of(space_mgr: &SpaceManager, entity_id: u32) -> &'static str {
    if !enabled() {
        return UNKNOWN_WORLD;
    }
    space_mgr
        .get_entity_space_id(entity_id)
        .and_then(|sid| space_mgr.world_name_for_space(sid))
        .map_or(UNKNOWN_WORLD, intern)
}

fn intern(name: &str) -> &'static str {
    static NAMES: OnceLock<RwLock<HashSet<&'static str>>> = OnceLock::new();
    let names = NAMES.get_or_init(|| RwLock::new(HashSet::new()));
    if let Some(&n) = names.read().unwrap_or_else(|e| e.into_inner()).get(name) {
        return n;
    }
    let mut w = names.write().unwrap_or_else(|e| e.into_inner());
    if let Some(&n) = w.get(name) {
        return n;
    }
    // Leaked once per world name for the process's life: the world table
    // bounds it.
    let leaked: &'static str = Box::leak(name.to_owned().into_boxed_str());
    w.insert(leaked);
    leaked
}

/// Record `amount` of damage taken from `pool`. Zero or less records
/// nothing.
pub fn damage_dealt(pool: Pool, amount: i32, world: &'static str) {
    if amount > 0 {
        cimmeria_observability::histogram!(
            DAMAGE_DEALT,
            f64::from(amount),
            "pool" => pool.label(),
            "world" => world,
        );
    }
}

/// Record `amount` healed into `pool`. Zero or less records nothing.
pub fn heal_done(pool: Pool, amount: i32, world: &'static str) {
    if amount > 0 {
        cimmeria_observability::histogram!(
            HEAL_DONE,
            f64::from(amount),
            "pool" => pool.label(),
            "world" => world,
        );
    }
}

/// A target's Health and Focus at one moment, to turn an effect's change
/// into damage and heal samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoolSample {
    pub health: i32,
    pub focus: i32,
}

impl PoolSample {
    /// `entity_id`'s pools; `None` when it is gone or metrics are off.
    pub fn take(space_mgr: &SpaceManager, entity_id: u32) -> Option<Self> {
        if !enabled() {
            return None;
        }
        let e = space_mgr.get_entity(entity_id)?;
        let cur = |id| e.stats.get(id).map_or(0, |s| s.cur);
        Some(Self {
            health: cur(HEALTH),
            focus: cur(FOCUS),
        })
    }

    /// Record the change from `self` to `after`: a fall is damage, a rise
    /// is a heal, per pool.
    pub fn record_change(self, after: Self, world: &'static str) {
        for (pool, before, now) in [
            (Pool::Health, self.health, after.health),
            (Pool::Focus, self.focus, after.focus),
        ] {
            damage_dealt(pool, before - now, world);
            heal_done(pool, now - before, world);
        }
    }
}

/// Count one timed effect taken off the ledger.
pub fn ledger_removed(why: StatBuffRemoval, world: &'static str) {
    cimmeria_observability::counter!(
        LEDGER_REMOVED_TOTAL,
        "reason" => why.reason(),
        "world" => world,
    );
}
