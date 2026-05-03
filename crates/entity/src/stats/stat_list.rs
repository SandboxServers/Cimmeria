//! Per-being collection of stats with batch serialization helpers.

use std::collections::HashMap;

use super::archetype::ArchetypeStatValues;
use super::stat::Stat;
use super::stat_ids::*;

/// Collection of all stats for a being entity.
///
/// Initialized from `SGWBeing.statsTemplate` defaults, then overwritten
/// by archetype-specific values in `setupPlayer()`.
#[derive(Clone)]
pub struct StatList {
    stats: HashMap<i32, Stat>,
}

impl StatList {
    /// Create a new StatList from the SGWBeing template defaults.
    ///
    /// All 70+ stats initialized to their default min/cur/max values
    /// matching `python/cell/SGWBeing.py:272-354`.
    pub fn new() -> Self {
        let mut stats = HashMap::with_capacity(80);
        // Helper to insert a template stat
        macro_rules! s {
            ($id:expr, $min:expr, $cur:expr, $max:expr) => {
                stats.insert($id, Stat::new($min, $cur, $max, $min, $cur, $max));
            };
        }

        // Primary attributes
        s!(COORDINATION, 0, 1, 1);
        s!(ENGAGEMENT, 0, 1, 1);
        s!(FORTITUDE, 0, 1, 1);
        s!(MORALE, 0, 1, 1);
        s!(PERCEPTION, 0, 1, 1);
        s!(INTELLIGENCE, 0, 1, 1);

        // Movement
        s!(MOVEMENT_SPEED_MOD, 0, 100, 500);

        // Pools
        s!(HEALTH, 0, 100, 100);
        s!(FOCUS, 0, 0, 0);
        s!(HEALTH_REGEN, 0, 0, 0);
        s!(FOCUS_REGEN, 0, 0, 0);

        // Combat modifiers
        s!(ACCURACY, -1000, 0, 1000);
        s!(DEFENSE, 0, 0, 0);
        s!(QR_MOD, 0, 0, 0);

        // Armor factors
        s!(PHYSICAL_AF, 0, 0, 50000);
        s!(ENERGY_AF, 0, 0, 50000);
        s!(HAZMAT_AF, 0, 0, 50000);
        s!(PSIONIC_AF, 0, 0, 50000);

        // Resistances
        s!(KINETIC_RES, 0, 0, 2000);
        s!(MENTAL_RES, 0, 0, 2000);
        s!(HEALTH_RES, 0, 0, 2000);

        // Stealth/cover
        s!(STEALTH_RATING, 0, 0, 100);
        s!(RANGE_MODIFIER, 0, 0, 0);
        s!(COVER_QR_MODIFIER, 0, 0, 0);

        // Ammo
        s!(AMMO_SLOT_1, 0, 0, 0);
        s!(AMMO_SLOT_2, 0, 0, 0);
        s!(AMMO_SLOT_3, 0, 0, 0);
        s!(AMMO_SLOT_4, 0, 0, 0);
        s!(AMMO_SLOT_5, 0, 0, 0);
        s!(DEPLOYMENT_BAR_AMMO, 0, 0, 0);

        // Combat
        s!(RESPONSE, 0, 0, 0);
        s!(DAMAGE, -100, 0, 100);
        s!(PENETRATION, -100, 0, 100);

        // Density
        s!(PHYSICAL_DENSITY, 0, 0, 0);
        s!(ENERGY_DENSITY, 0, 0, 0);
        s!(HAZMAT_DENSITY, 0, 0, 0);
        s!(PSIONIC_DENSITY, 0, 0, 0);

        // Awareness
        s!(TRACKING, 0, 0, 0);
        s!(STABILIZATION, 0, 0, 0);
        s!(AWARENESS, 0, 0, 0);
        s!(INTERRUPT_RES, 0, 0, 0);

        // Cover/crouch
        s!(COVER_ACCURACY, 0, 0, 0);
        s!(COVER_DEFENSE, 0, 0, 0);
        s!(CROUCHING_ACCURACY, 0, 0, 0);
        s!(CROUCHING_DEFENSE, 0, 0, 0);
        s!(STEALTH_MOVEMENT, 0, 0, 0);

        // Reveal/disguise
        s!(REVEAL_RATING, 0, 0, 100);
        s!(NEGATION, 0, 0, 0);

        // Damage percentages
        s!(PHYSICAL_DAMAGE_PERCENT, 0, 0, 0);
        s!(ENERGY_DAMAGE_PERCENT, 0, 0, 0);
        s!(HAZMAT_DAMAGE_PERCENT, 0, 0, 0);
        s!(PSIONIC_DAMAGE_PERCENT, 0, 0, 0);
        s!(UNTYPED_DAMAGE_PERCENT, 0, 0, 0);

        // Disguise
        s!(DISGUISE_RATING, 0, 0, 500);
        s!(DISGUISE_DETECTION, 0, 0, 0);

        // Mitigation/movement
        s!(MITIGATION, 0, 0, 0);
        s!(ROTATION_SPEED_MOD, 0, 100, 500);
        s!(ENERGY_POOL, 0, 0, 0);
        s!(ENERGY_REGEN, 0, 0, 0);

        // Speed modifiers
        s!(SPEED_RELOAD, 0, 0, 0);
        s!(SPEED_GRENADE, 0, 0, 0);
        s!(SPEED_DEPLOY, 0, 0, 0);
        s!(SPEED_ATTACK, 0, 0, 0);
        s!(RECOVERY, 0, 0, 0);
        s!(RESTORATION, 0, 0, 0);
        s!(SUBTLETY, 0, 0, 0);
        s!(SPEED_PET, 0, 0, 0);

        // Absorb stats
        s!(ABSORB_PHYSICAL, 0, 0, 1000);
        s!(ABSORB_ENERGY, 0, 0, 1000);
        s!(ABSORB_HAZMAT, 0, 0, 1000);
        s!(ABSORB_PSIONIC, 0, 0, 1000);
        s!(ABSORB_UNTYPED, 0, 0, 1000);
        s!(ABSORB_PHYSICAL_ITEM, 0, 0, 1000);
        s!(ABSORB_ENERGY_ITEM, 0, 0, 1000);
        s!(ABSORB_HAZMAT_ITEM, 0, 0, 1000);
        s!(ABSORB_PSIONIC_ITEM, 0, 0, 1000);
        s!(ABSORB_UNTYPED_ITEM, 0, 0, 1000);
        s!(ABSORB_PHYSICAL_ENERGY, 0, 0, 1000);
        s!(ABSORB_ENERGY_ENERGY, 0, 0, 1000);
        s!(ABSORB_HAZMAT_ENERGY, 0, 0, 1000);
        s!(ABSORB_PSIONIC_ENERGY, 0, 0, 1000);
        s!(ABSORB_UNTYPED_ENERGY, 0, 0, 1000);

        Self { stats }
    }

    /// Get a stat reference by ID.
    pub fn get(&self, stat_id: i32) -> Option<&Stat> {
        self.stats.get(&stat_id)
    }

    /// Get a mutable stat reference by ID.
    pub fn get_mut(&mut self, stat_id: i32) -> Option<&mut Stat> {
        self.stats.get_mut(&stat_id)
    }

    /// Apply archetype base stats for a fresh player.
    ///
    /// Mirrors `python/cell/SGWPlayer.py:setupPlayer()` lines 424-437.
    pub fn apply_archetype(&mut self, arch: &ArchetypeStatValues) {
        for &(id, min, cur, max) in &[
            (COORDINATION, 0, arch.coordination, arch.coordination),
            (ENGAGEMENT, 0, arch.engagement, arch.engagement),
            (FORTITUDE, 0, arch.fortitude, arch.fortitude),
            (MORALE, 0, arch.morale, arch.morale),
            (PERCEPTION, 0, arch.perception, arch.perception),
            (INTELLIGENCE, 0, arch.intelligence, arch.intelligence),
            (HEALTH, 0, arch.health, arch.health),
            (FOCUS, 0, arch.focus, arch.focus),
            (KINETIC_RES, 0, 40, 2000),
            (MENTAL_RES, 0, 20, 2000),
            (HEALTH_RES, 0, 30, 2000),
            (DEPLOYMENT_BAR_AMMO, 0, 0, 1),
        ] {
            let stat = self
                .stats
                .get_mut(&id)
                .expect("apply_archetype: core stat missing from StatList::new()");
            stat.update(min, cur, max);
            stat.set_base(min, cur, max);
            stat.dirty = false;
            stat.base_dirty = false;
        }
    }

    /// Serialize all stats as a `StatUpdateList` for `onStatUpdate`.
    ///
    /// Wire format: `count:u32`, per stat: `stat_id:i32, min:i32, cur:i32, max:i32`.
    pub fn serialize_all(&self) -> Vec<u8> {
        self.serialize_entries(self.stats.iter().map(|(&id, s)| (id, s.min, s.cur, s.max)))
    }

    /// Serialize all base stats as a `StatUpdateList` for `onStatBaseUpdate`.
    pub fn serialize_all_base(&self) -> Vec<u8> {
        self.serialize_entries(
            self.stats
                .iter()
                .map(|(&id, s)| (id, s.base_min, s.base_cur, s.base_max)),
        )
    }

    /// Serialize only dirty stats for `onStatUpdate`.
    pub fn serialize_dirty(&self) -> Vec<u8> {
        self.serialize_entries(
            self.stats
                .iter()
                .filter(|(_, s)| s.dirty)
                .map(|(&id, s)| (id, s.min, s.cur, s.max)),
        )
    }

    /// Serialize only base-dirty stats for `onStatBaseUpdate`.
    pub fn serialize_dirty_base(&self) -> Vec<u8> {
        self.serialize_entries(
            self.stats
                .iter()
                .filter(|(_, s)| s.base_dirty)
                .map(|(&id, s)| (id, s.base_min, s.base_cur, s.base_max)),
        )
    }

    /// Serialize only public stats for witness updates (`onStatUpdate`).
    pub fn serialize_public(&self) -> Vec<u8> {
        self.serialize_entries(
            PUBLIC_STATS
                .iter()
                .filter_map(|&id| self.stats.get(&id).map(|s| (id, s.min, s.cur, s.max))),
        )
    }

    /// Serialize only public base stats for witness updates (`onStatBaseUpdate`).
    pub fn serialize_public_base(&self) -> Vec<u8> {
        self.serialize_entries(PUBLIC_STATS.iter().filter_map(|&id| {
            self.stats
                .get(&id)
                .map(|s| (id, s.base_min, s.base_cur, s.base_max))
        }))
    }

    /// Serialize only dirty public stats for witness updates.
    pub fn serialize_dirty_public(&self) -> Vec<u8> {
        self.serialize_entries(PUBLIC_STATS.iter().filter_map(|&id| {
            self.stats
                .get(&id)
                .filter(|s| s.dirty)
                .map(|s| (id, s.min, s.cur, s.max))
        }))
    }

    /// Serialize only dirty public base stats for witness updates.
    pub fn serialize_dirty_public_base(&self) -> Vec<u8> {
        self.serialize_entries(PUBLIC_STATS.iter().filter_map(|&id| {
            self.stats
                .get(&id)
                .filter(|s| s.base_dirty)
                .map(|s| (id, s.base_min, s.base_cur, s.base_max))
        }))
    }

    /// Clear all dirty flags after sending updates.
    pub fn clear_dirty(&mut self) {
        for stat in self.stats.values_mut() {
            stat.clear_dirty();
        }
    }

    /// Returns the number of stats.
    pub fn len(&self) -> usize {
        self.stats.len()
    }

    /// Returns true if no stats exist.
    pub fn is_empty(&self) -> bool {
        self.stats.is_empty()
    }

    /// Iterate over all stat entries.
    pub fn iter(&self) -> impl Iterator<Item = (&i32, &Stat)> {
        self.stats.iter()
    }

    /// Iterate mutably over all stat entries.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&i32, &mut Stat)> {
        self.stats.iter_mut()
    }

    /// Scale health and focus stats for a given character level.
    ///
    /// Formula: `max = base + per_level * (level - 1)`
    /// Current value is restored to the new max (full heal on level-up).
    ///
    /// Levels are 1-based; `python/cell/SGWBeing.setLevel` asserts `>= 1`.
    /// We clamp to 1 here to keep the bonus non-negative — the DB schema
    /// permits `level = 0` but feeding that through the formula would
    /// compute `(level - 1) = -1` and shrink max below the archetype base.
    ///
    /// Reference: Missing `setLevel()` in Python — this is the implementation
    /// that `python/cell/SGWPlayer.py:794` called but never defined.
    pub fn scale_for_level(&mut self, level: u32, arch: &ArchetypeStatValues) {
        let level = level.max(1);

        let bonus_health = arch.health_per_level * (level as i32 - 1);
        let new_health_max = arch.health + bonus_health;
        let stat = self
            .stats
            .get_mut(&HEALTH)
            .expect("scale_for_level: HEALTH missing from StatList::new()");
        stat.update(0, new_health_max, new_health_max);
        stat.set_base(0, new_health_max, new_health_max);

        let bonus_focus = arch.focus_per_level * (level as i32 - 1);
        let new_focus_max = arch.focus + bonus_focus;
        let stat = self
            .stats
            .get_mut(&FOCUS)
            .expect("scale_for_level: FOCUS missing from StatList::new()");
        stat.update(0, new_focus_max, new_focus_max);
        stat.set_base(0, new_focus_max, new_focus_max);
    }

    /// Returns true if any stat has dirty or base_dirty set.
    pub fn has_dirty(&self) -> bool {
        self.stats.values().any(|s| s.dirty || s.base_dirty)
    }

    fn serialize_entries(&self, entries: impl Iterator<Item = (i32, i32, i32, i32)>) -> Vec<u8> {
        let entries: Vec<_> = entries.collect();
        let mut buf = Vec::with_capacity(4 + entries.len() * 16);
        buf.extend_from_slice(&(entries.len() as u32).to_le_bytes());
        for (id, min, cur, max) in entries {
            buf.extend_from_slice(&id.to_le_bytes());
            buf.extend_from_slice(&min.to_le_bytes());
            buf.extend_from_slice(&cur.to_le_bytes());
            buf.extend_from_slice(&max.to_le_bytes());
        }
        buf
    }
}

impl Default for StatList {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for StatList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StatList")
            .field("count", &self.stats.len())
            .field("health", &self.get(HEALTH).map(|s| s.cur))
            .field("focus", &self.get(FOCUS).map(|s| s.cur))
            .finish()
    }
}
