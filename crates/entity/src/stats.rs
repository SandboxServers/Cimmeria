//! Combat stat system for beings (players and NPCs).
//!
//! Each stat has two sets of values:
//! - **Base** (`base_min`/`base_cur`/`base_max`): Change on level-up, archetype, etc.
//! - **Dynamic** (`min`/`cur`/`max`): Change due to buffs, debuffs, equipment, effects.
//!
//! The wire format for stat updates is `StatUpdateList` from `custom_alias.xml`:
//! `count:u32`, then per entry: `stat_id:i32, min:i32, current:i32, max:i32`.
//!
//! This corresponds to the Python `Stat` class in `python/cell/SGWBeing.py:40`
//! and stat IDs from `python/Atrea/enums.py:295`.

use std::collections::HashMap;

// ── Stat IDs ────────────────────────────────────────────────────────────────
// From `python/Atrea/enums.py:295-376`.

/// Primary attributes
pub const COORDINATION: i32 = 0;
pub const ENGAGEMENT: i32 = 1;
pub const FORTITUDE: i32 = 2;
pub const MORALE: i32 = 3;
pub const PERCEPTION: i32 = 4;
pub const INTELLIGENCE: i32 = 5;

/// Movement
pub const MOVEMENT_SPEED_MOD: i32 = 6;

/// Pools
pub const HEALTH: i32 = 7;
pub const FOCUS: i32 = 8;
pub const HEALTH_REGEN: i32 = 9;
pub const FOCUS_REGEN: i32 = 10;

/// Combat modifiers
pub const ACCURACY: i32 = 11;
pub const DEFENSE: i32 = 12;
pub const QR_MOD: i32 = 13;

/// Armor factors
pub const PHYSICAL_AF: i32 = 18;
pub const ENERGY_AF: i32 = 23;
pub const HAZMAT_AF: i32 = 24;
pub const PSIONIC_AF: i32 = 28;

/// Resistances
pub const KINETIC_RES: i32 = 29;
pub const MENTAL_RES: i32 = 34;
pub const HEALTH_RES: i32 = 40;

/// Stealth
pub const STEALTH_RATING: i32 = 46;
pub const RANGE_MODIFIER: i32 = 47;
pub const COVER_QR_MODIFIER: i32 = 48;

/// Ammo slots
pub const AMMO_SLOT_1: i32 = 49;
pub const AMMO_SLOT_2: i32 = 50;
pub const AMMO_SLOT_3: i32 = 51;
pub const AMMO_SLOT_4: i32 = 52;
pub const AMMO_SLOT_5: i32 = 53;
pub const DEPLOYMENT_BAR_AMMO: i32 = 54;

/// Combat
pub const RESPONSE: i32 = 55;
pub const DAMAGE: i32 = 56;
pub const PENETRATION: i32 = 57;

/// Density
pub const PHYSICAL_DENSITY: i32 = 58;
pub const ENERGY_DENSITY: i32 = 59;
pub const HAZMAT_DENSITY: i32 = 60;
pub const PSIONIC_DENSITY: i32 = 61;

/// Awareness
pub const TRACKING: i32 = 62;
pub const STABILIZATION: i32 = 63;
pub const AWARENESS: i32 = 64;
pub const INTERRUPT_RES: i32 = 65;

/// Cover/crouch
pub const COVER_ACCURACY: i32 = 66;
pub const COVER_DEFENSE: i32 = 67;
pub const CROUCHING_ACCURACY: i32 = 68;
pub const CROUCHING_DEFENSE: i32 = 69;
pub const STEALTH_MOVEMENT: i32 = 70;

/// Reveal/disguise
pub const REVEAL_RATING: i32 = 71;
pub const NEGATION: i32 = 72;

/// Damage type percentages
pub const PHYSICAL_DAMAGE_PERCENT: i32 = 73;
pub const ENERGY_DAMAGE_PERCENT: i32 = 74;
pub const HAZMAT_DAMAGE_PERCENT: i32 = 75;
pub const PSIONIC_DAMAGE_PERCENT: i32 = 76;
pub const UNTYPED_DAMAGE_PERCENT: i32 = 77;

/// Disguise
pub const DISGUISE_RATING: i32 = 78;
pub const DISGUISE_DETECTION: i32 = 79;

/// Mitigation and movement
pub const MITIGATION: i32 = 80;
pub const ROTATION_SPEED_MOD: i32 = 81;
pub const ENERGY_POOL: i32 = 82;
pub const ENERGY_REGEN: i32 = 83;

/// Absorb stats
pub const ABSORB_PHYSICAL: i32 = 89;
pub const ABSORB_ENERGY: i32 = 90;
pub const ABSORB_HAZMAT: i32 = 91;
pub const ABSORB_PSIONIC: i32 = 92;
pub const ABSORB_UNTYPED: i32 = 93;
pub const ABSORB_PHYSICAL_ITEM: i32 = 94;
pub const ABSORB_ENERGY_ITEM: i32 = 95;
pub const ABSORB_HAZMAT_ITEM: i32 = 96;
pub const ABSORB_PSIONIC_ITEM: i32 = 97;
pub const ABSORB_UNTYPED_ITEM: i32 = 98;
pub const ABSORB_PHYSICAL_ENERGY: i32 = 99;
pub const ABSORB_ENERGY_ENERGY: i32 = 100;
pub const ABSORB_HAZMAT_ENERGY: i32 = 101;
pub const ABSORB_PSIONIC_ENERGY: i32 = 102;
pub const ABSORB_UNTYPED_ENERGY: i32 = 103;

/// Speed modifiers
pub const SPEED_RELOAD: i32 = 104;
pub const SPEED_GRENADE: i32 = 105;
pub const SPEED_DEPLOY: i32 = 106;
pub const SPEED_ATTACK: i32 = 107;
pub const RECOVERY: i32 = 108;
pub const RESTORATION: i32 = 109;
pub const SUBTLETY: i32 = 110;
pub const SPEED_PET: i32 = 111;

// ── Public stats (sent to witnesses, not just owner) ────────────────────────
// From `SGWBeing.publicStats` — python/cell/SGWBeing.py:235-247

/// Stats visible to all nearby clients (not just the owner).
pub const PUBLIC_STATS: &[i32] = &[
    MOVEMENT_SPEED_MOD,
    HEALTH,
    FOCUS,
    AMMO_SLOT_1,
    AMMO_SLOT_2,
    AMMO_SLOT_3,
    AMMO_SLOT_4,
    AMMO_SLOT_5,
    ROTATION_SPEED_MOD,
    ENERGY_POOL,
    ENERGY_REGEN,
];

// ── Stat struct ─────────────────────────────────────────────────────────────

/// A single combat stat with dynamic and base value sets.
///
/// Mirrors `python/cell/SGWBeing.py:40 class Stat`.
#[derive(Debug, Clone)]
pub struct Stat {
    /// Dynamic minimum value (can change from effects).
    pub min: i32,
    /// Dynamic current value (clamped between min and max).
    pub cur: i32,
    /// Dynamic maximum value (can change from buffs/equipment).
    pub max: i32,
    /// Base minimum (changes only on level-up / archetype change).
    pub base_min: i32,
    /// Base current.
    pub base_cur: i32,
    /// Base maximum.
    pub base_max: i32,
    /// True if dynamic values changed since last sync.
    pub dirty: bool,
    /// True if base values changed since last sync.
    pub base_dirty: bool,
}

impl Stat {
    /// Create a new stat with explicit values for all 6 fields.
    pub fn new(min: i32, cur: i32, max: i32, base_min: i32, base_cur: i32, base_max: i32) -> Self {
        Self {
            min, cur, max,
            base_min, base_cur, base_max,
            dirty: false,
            base_dirty: false,
        }
    }

    /// Update dynamic min/current/max and mark dirty.
    pub fn update(&mut self, min: i32, current: i32, max: i32) {
        self.min = min;
        self.cur = current;
        self.max = max;
        self.dirty = true;
    }

    /// Set the dynamic current value, clamping to [min, max].
    /// Returns the new current value.
    pub fn set_current(&mut self, value: i32) -> i32 {
        self.cur = value.clamp(self.min, self.max);
        self.dirty = true;
        self.cur
    }

    /// Change the current value by a relative amount, clamping to [min, max].
    /// Returns the actual change applied.
    pub fn change(&mut self, delta: i32) -> i32 {
        let old = self.cur;
        self.cur = (self.cur + delta).clamp(self.min, self.max);
        self.dirty = true;
        self.cur - old
    }

    /// Change current by a percentage of current value.
    pub fn change_by_percent(&mut self, multiplier: f32) -> i32 {
        let delta = (self.cur as f32 * multiplier).round() as i32;
        self.change(delta)
    }

    /// Change current by a percentage of max value.
    pub fn change_by_max_percent(&mut self, multiplier: f32) -> i32 {
        let delta = (self.max as f32 * multiplier).round() as i32;
        self.change(delta)
    }

    /// Set dynamic maximum, clamping current if needed.
    pub fn set_max(&mut self, max: i32) {
        self.max = max;
        if self.cur > self.max {
            self.cur = self.max;
        }
        self.dirty = true;
    }

    /// Set dynamic minimum, clamping current if needed.
    pub fn set_min(&mut self, min: i32) {
        self.min = min;
        if self.cur < self.min {
            self.cur = self.min;
        }
        self.dirty = true;
    }

    /// Update base values and mark base_dirty.
    pub fn set_base(&mut self, base_min: i32, base_cur: i32, base_max: i32) {
        self.base_min = base_min;
        self.base_cur = base_cur;
        self.base_max = base_max;
        self.base_dirty = true;
    }

    /// Clear dirty flags after syncing to clients.
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
        self.base_dirty = false;
    }
}

// ── StatList ────────────────────────────────────────────────────────────────

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
            (ENGAGEMENT,   0, arch.engagement,   arch.engagement),
            (FORTITUDE,    0, arch.fortitude,     arch.fortitude),
            (MORALE,       0, arch.morale,        arch.morale),
            (PERCEPTION,   0, arch.perception,    arch.perception),
            (INTELLIGENCE, 0, arch.intelligence,  arch.intelligence),
            (HEALTH,       0, arch.health,        arch.health),
            (FOCUS,        0, arch.focus,          arch.focus),
            (KINETIC_RES,  0, 40,                  2000),
            (MENTAL_RES,   0, 20,                  2000),
            (HEALTH_RES,   0, 30,                  2000),
            (DEPLOYMENT_BAR_AMMO, 0, 0,            1),
        ] {
            if let Some(stat) = self.stats.get_mut(&id) {
                stat.update(min, cur, max);
                stat.set_base(min, cur, max);
                stat.dirty = false;
                stat.base_dirty = false;
            }
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
        self.serialize_entries(self.stats.iter().map(|(&id, s)| (id, s.base_min, s.base_cur, s.base_max)))
    }

    /// Serialize only dirty stats for `onStatUpdate`.
    pub fn serialize_dirty(&self) -> Vec<u8> {
        self.serialize_entries(
            self.stats.iter()
                .filter(|(_, s)| s.dirty)
                .map(|(&id, s)| (id, s.min, s.cur, s.max))
        )
    }

    /// Serialize only base-dirty stats for `onStatBaseUpdate`.
    pub fn serialize_dirty_base(&self) -> Vec<u8> {
        self.serialize_entries(
            self.stats.iter()
                .filter(|(_, s)| s.base_dirty)
                .map(|(&id, s)| (id, s.base_min, s.base_cur, s.base_max))
        )
    }

    /// Serialize only public stats for witness updates (`onStatUpdate`).
    pub fn serialize_public(&self) -> Vec<u8> {
        self.serialize_entries(
            PUBLIC_STATS.iter()
                .filter_map(|&id| self.stats.get(&id).map(|s| (id, s.min, s.cur, s.max)))
        )
    }

    /// Serialize only public base stats for witness updates (`onStatBaseUpdate`).
    pub fn serialize_public_base(&self) -> Vec<u8> {
        self.serialize_entries(
            PUBLIC_STATS.iter()
                .filter_map(|&id| self.stats.get(&id).map(|s| (id, s.base_min, s.base_cur, s.base_max)))
        )
    }

    /// Serialize only dirty public stats for witness updates.
    pub fn serialize_dirty_public(&self) -> Vec<u8> {
        self.serialize_entries(
            PUBLIC_STATS.iter()
                .filter_map(|&id| {
                    self.stats.get(&id)
                        .filter(|s| s.dirty)
                        .map(|s| (id, s.min, s.cur, s.max))
                })
        )
    }

    /// Serialize only dirty public base stats for witness updates.
    pub fn serialize_dirty_public_base(&self) -> Vec<u8> {
        self.serialize_entries(
            PUBLIC_STATS.iter()
                .filter_map(|&id| {
                    self.stats.get(&id)
                        .filter(|s| s.base_dirty)
                        .map(|s| (id, s.base_min, s.base_cur, s.base_max))
                })
        )
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
    /// Reference: Missing `setLevel()` in Python — this is the implementation
    /// that `python/cell/SGWPlayer.py:794` called but never defined.
    pub fn scale_for_level(&mut self, level: u32, arch: &ArchetypeStatValues) {
        let bonus_health = arch.health_per_level * (level as i32 - 1);
        let new_health_max = arch.health + bonus_health;
        if let Some(stat) = self.stats.get_mut(&HEALTH) {
            stat.update(0, new_health_max, new_health_max);
        }

        let bonus_focus = arch.focus_per_level * (level as i32 - 1);
        let new_focus_max = arch.focus + bonus_focus;
        if let Some(stat) = self.stats.get_mut(&FOCUS) {
            stat.update(0, new_focus_max, new_focus_max);
        }
    }

    /// Returns true if any stat has dirty or base_dirty set.
    pub fn has_dirty(&self) -> bool {
        self.stats.values().any(|s| s.dirty || s.base_dirty)
    }

    // ── Internal ──

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

// ── Archetype stat input ────────────────────────────────────────────────────

/// Archetype-specific base stat values passed to [`StatList::apply_archetype`].
///
/// Same fields as `ArchetypeStats` in mercury_ext.rs but decoupled from
/// wire format concerns.
#[derive(Debug, Clone)]
pub struct ArchetypeStatValues {
    pub coordination: i32,
    pub engagement: i32,
    pub fortitude: i32,
    pub morale: i32,
    pub perception: i32,
    pub intelligence: i32,
    pub health: i32,
    pub focus: i32,
    pub health_per_level: i32,
    pub focus_per_level: i32,
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stat_new_and_fields() {
        let s = Stat::new(0, 50, 100, 0, 50, 100);
        assert_eq!(s.min, 0);
        assert_eq!(s.cur, 50);
        assert_eq!(s.max, 100);
        assert_eq!(s.base_min, 0);
        assert_eq!(s.base_cur, 50);
        assert_eq!(s.base_max, 100);
        assert!(!s.dirty);
        assert!(!s.base_dirty);
    }

    #[test]
    fn stat_update_marks_dirty() {
        let mut s = Stat::new(0, 50, 100, 0, 50, 100);
        s.update(10, 60, 90);
        assert!(s.dirty);
        assert_eq!(s.min, 10);
        assert_eq!(s.cur, 60);
        assert_eq!(s.max, 90);
    }

    #[test]
    fn stat_set_current_clamps() {
        let mut s = Stat::new(0, 50, 100, 0, 50, 100);
        assert_eq!(s.set_current(200), 100);
        assert_eq!(s.set_current(-10), 0);
        assert_eq!(s.set_current(75), 75);
        assert!(s.dirty);
    }

    #[test]
    fn stat_change_clamps() {
        let mut s = Stat::new(0, 50, 100, 0, 50, 100);
        assert_eq!(s.change(30), 30);
        assert_eq!(s.cur, 80);
        assert_eq!(s.change(30), 20); // clamped at 100
        assert_eq!(s.cur, 100);
        assert_eq!(s.change(-150), -100); // clamped at 0
        assert_eq!(s.cur, 0);
    }

    #[test]
    fn stat_set_max_clamps_current() {
        let mut s = Stat::new(0, 80, 100, 0, 80, 100);
        s.set_max(50);
        assert_eq!(s.max, 50);
        assert_eq!(s.cur, 50);
    }

    #[test]
    fn stat_set_min_clamps_current() {
        let mut s = Stat::new(0, 10, 100, 0, 10, 100);
        s.set_min(20);
        assert_eq!(s.min, 20);
        assert_eq!(s.cur, 20);
    }

    #[test]
    fn stat_change_by_percent() {
        let mut s = Stat::new(0, 200, 1000, 0, 200, 1000);
        let delta = s.change_by_percent(0.5); // 50% of 200 = 100
        assert_eq!(delta, 100);
        assert_eq!(s.cur, 300);
    }

    #[test]
    fn stat_change_by_max_percent() {
        let mut s = Stat::new(0, 200, 1000, 0, 200, 1000);
        let delta = s.change_by_max_percent(0.1); // 10% of 1000 = 100
        assert_eq!(delta, 100);
        assert_eq!(s.cur, 300);
    }

    #[test]
    fn stat_set_base_marks_base_dirty() {
        let mut s = Stat::new(0, 50, 100, 0, 50, 100);
        s.set_base(10, 60, 200);
        assert!(s.base_dirty);
        assert_eq!(s.base_min, 10);
        assert_eq!(s.base_cur, 60);
        assert_eq!(s.base_max, 200);
    }

    #[test]
    fn stat_clear_dirty() {
        let mut s = Stat::new(0, 50, 100, 0, 50, 100);
        s.dirty = true;
        s.base_dirty = true;
        s.clear_dirty();
        assert!(!s.dirty);
        assert!(!s.base_dirty);
    }

    #[test]
    fn statlist_default_has_all_stats() {
        let list = StatList::new();
        // Check a sample of stats exist with expected defaults
        let health = list.get(HEALTH).unwrap();
        assert_eq!(health.cur, 100);
        assert_eq!(health.max, 100);

        let coord = list.get(COORDINATION).unwrap();
        assert_eq!(coord.cur, 1);
        assert_eq!(coord.max, 1);

        let speed = list.get(MOVEMENT_SPEED_MOD).unwrap();
        assert_eq!(speed.cur, 100);
        assert_eq!(speed.max, 500);

        let accuracy = list.get(ACCURACY).unwrap();
        assert_eq!(accuracy.min, -1000);
        assert_eq!(accuracy.max, 1000);

        // Should have 70+ stats
        assert!(list.len() >= 70);
    }

    #[test]
    fn statlist_apply_archetype() {
        let mut list = StatList::new();
        let arch = ArchetypeStatValues {
            coordination: 15,
            engagement: 10,
            fortitude: 12,
            morale: 13,
            perception: 14,
            intelligence: 11,
            health: 500,
            focus: 200,
            health_per_level: 10,
            focus_per_level: 70,
        };
        list.apply_archetype(&arch);

        let health = list.get(HEALTH).unwrap();
        assert_eq!(health.cur, 500);
        assert_eq!(health.max, 500);
        assert_eq!(health.base_cur, 500);
        assert_eq!(health.base_max, 500);

        let coord = list.get(COORDINATION).unwrap();
        assert_eq!(coord.cur, 15);

        let kres = list.get(KINETIC_RES).unwrap();
        assert_eq!(kres.cur, 40);
        assert_eq!(kres.max, 2000);

        let deploy = list.get(DEPLOYMENT_BAR_AMMO).unwrap();
        assert_eq!(deploy.max, 1);
    }

    #[test]
    fn statlist_serialize_all_wire_format() {
        let mut list = StatList::new();
        // Apply archetype so we have known values
        let arch = ArchetypeStatValues {
            coordination: 15, engagement: 10, fortitude: 12,
            morale: 13, perception: 14, intelligence: 11,
            health: 500, focus: 200,
            health_per_level: 10, focus_per_level: 70,
        };
        list.apply_archetype(&arch);

        let data = list.serialize_all();
        // First 4 bytes = count (u32 LE)
        let count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        assert_eq!(count as usize, list.len());
        // Total size = 4 + count * 16
        assert_eq!(data.len(), 4 + (count as usize) * 16);

        // Verify one specific entry exists in the serialized data
        // Each entry is (stat_id:i32, min:i32, cur:i32, max:i32) = 16 bytes
        let mut found_health = false;
        for i in 0..count as usize {
            let offset = 4 + i * 16;
            let id = i32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]);
            if id == HEALTH {
                let cur = i32::from_le_bytes([data[offset+8], data[offset+9], data[offset+10], data[offset+11]]);
                let max = i32::from_le_bytes([data[offset+12], data[offset+13], data[offset+14], data[offset+15]]);
                assert_eq!(cur, 500);
                assert_eq!(max, 500);
                found_health = true;
            }
        }
        assert!(found_health, "Health stat should be in serialized data");
    }

    #[test]
    fn statlist_serialize_dirty_only() {
        let mut list = StatList::new();
        // Nothing is dirty initially
        let data = list.serialize_dirty();
        let count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        assert_eq!(count, 0);

        // Dirty one stat
        list.get_mut(HEALTH).unwrap().set_current(50);
        let data = list.serialize_dirty();
        let count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        assert_eq!(count, 1);
        // Verify it's health
        let id = i32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        assert_eq!(id, HEALTH);
    }

    #[test]
    fn statlist_serialize_public() {
        let list = StatList::new();
        let data = list.serialize_public();
        let count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        assert_eq!(count as usize, PUBLIC_STATS.len());
    }

    #[test]
    fn statlist_clear_dirty() {
        let mut list = StatList::new();
        list.get_mut(HEALTH).unwrap().set_current(50);
        list.get_mut(FOCUS).unwrap().set_base(0, 100, 200);
        assert!(list.has_dirty());
        list.clear_dirty();
        assert!(!list.has_dirty());
    }

    #[test]
    fn statlist_serialize_base_values() {
        let mut list = StatList::new();
        let arch = ArchetypeStatValues {
            coordination: 15, engagement: 10, fortitude: 12,
            morale: 13, perception: 14, intelligence: 11,
            health: 500, focus: 200,
            health_per_level: 10, focus_per_level: 70,
        };
        list.apply_archetype(&arch);

        let data = list.serialize_all_base();
        let count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        assert_eq!(count as usize, list.len());

        // Find health and verify base values
        let mut found = false;
        for i in 0..count as usize {
            let offset = 4 + i * 16;
            let id = i32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]);
            if id == HEALTH {
                let cur = i32::from_le_bytes([data[offset+8], data[offset+9], data[offset+10], data[offset+11]]);
                let max = i32::from_le_bytes([data[offset+12], data[offset+13], data[offset+14], data[offset+15]]);
                assert_eq!(cur, 500);
                assert_eq!(max, 500);
                found = true;
            }
        }
        assert!(found);
    }

    #[test]
    fn stat_ids_match_python_enums() {
        // Verify a selection of stat IDs match python/Atrea/enums.py
        assert_eq!(COORDINATION, 0);
        assert_eq!(HEALTH, 7);
        assert_eq!(FOCUS, 8);
        assert_eq!(KINETIC_RES, 29);
        assert_eq!(MENTAL_RES, 34);
        assert_eq!(HEALTH_RES, 40);
        assert_eq!(AMMO_SLOT_1, 49);
        assert_eq!(DEPLOYMENT_BAR_AMMO, 54);
        assert_eq!(STEALTH_MOVEMENT, 70);
        assert_eq!(ROTATION_SPEED_MOD, 81);
        assert_eq!(ABSORB_PHYSICAL, 89);
        assert_eq!(SPEED_RELOAD, 104);
    }

    #[test]
    fn scale_for_level_increases_health_and_focus() {
        let mut list = StatList::new();
        let arch = ArchetypeStatValues {
            coordination: 5, engagement: 4, fortitude: 3, morale: 4,
            perception: 3, intelligence: 2, health: 760, focus: 1570,
            health_per_level: 10, focus_per_level: 70,
        };
        list.apply_archetype(&arch);

        // Level 1: base values
        assert_eq!(list.get(HEALTH).unwrap().max, 760);
        assert_eq!(list.get(FOCUS).unwrap().max, 1570);

        // Scale to level 5: health = 760 + 10*(5-1) = 800, focus = 1570 + 70*(5-1) = 1850
        list.scale_for_level(5, &arch);
        assert_eq!(list.get(HEALTH).unwrap().max, 800);
        assert_eq!(list.get(HEALTH).unwrap().cur, 800);
        assert_eq!(list.get(FOCUS).unwrap().max, 1850);
        assert_eq!(list.get(FOCUS).unwrap().cur, 1850);
        assert!(list.has_dirty());
    }

    #[test]
    fn scale_for_level_1_is_base() {
        let mut list = StatList::new();
        let arch = ArchetypeStatValues {
            coordination: 5, engagement: 4, fortitude: 3, morale: 4,
            perception: 3, intelligence: 2, health: 760, focus: 1570,
            health_per_level: 10, focus_per_level: 70,
        };
        list.apply_archetype(&arch);
        list.clear_dirty();
        list.scale_for_level(1, &arch);
        // Level 1: no bonus
        assert_eq!(list.get(HEALTH).unwrap().max, 760);
        assert_eq!(list.get(FOCUS).unwrap().max, 1570);
    }
}
