//! Ability system for entity combat.
//!
//! Each being has an `AbilityManager` that tracks known abilities, cooldowns,
//! and the currently active ability. Abilities are defined in the database
//! (`resources.abilities`) and organized into archetype-specific ability trees.
//!
//! Reference: `python/cell/AbilityManager.py`, `python/common/defs/Ability.py`

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

// ── Ability flags (from python/Atrea/enums.py) ────────────────────────────

pub const AF_USE_WEAPON_RANGE: u32 = 4;
pub const AF_RESPONSE: u32 = 16;
pub const AF_DEACTIVATE_AUTO_CYCLE: u32 = 1024;
pub const AF_SPEED_GRENADE: u32 = 2048;
pub const AF_SPEED_DEPLOY: u32 = 4096;
pub const AF_SPEED_ATTACK: u32 = 8192;

// ── Target types ──────────────────────────────────────────────────────────

pub const TARGET_NONE: i32 = 0;
pub const TARGET_SELF: i32 = 1;
pub const TARGET_TARGET: i32 = 2;
pub const TARGET_GROUND: i32 = 3;

// ── Timer types (sent via onTimerUpdate) ──────────────────────────────────

pub const TIMER_ABILITY_WARMUP: i8 = 1;
pub const TIMER_ABILITY_COOLDOWN: i8 = 2;
pub const TIMER_DURATION_EFFECT: i8 = 5;
pub const TIMER_CATEGORY_COOLDOWN: i8 = 8;

// ── Result codes ──────────────────────────────────────────────────────────

pub const RC_NONE: u8 = 0;
pub const RC_HIT: u8 = 1;
pub const RC_MISS: u8 = 2;
pub const RC_CRITICAL: u8 = 3;
pub const RC_DOUBLE_CRITICAL: u8 = 4;
pub const RC_GLANCING: u8 = 5;

// ── Damage types ──────────────────────────────────────────────────────────

pub const DT_UNTYPED: i8 = 0;
pub const DT_ENERGY: i8 = 1;
pub const DT_HAZMAT: i8 = 2;
pub const DT_PHYSICAL: i8 = 3;
pub const DT_PSIONIC: i8 = 4;

// ── Stat result codes ─────────────────────────────────────────────────────

pub const SRC_NONE: i8 = 0;
pub const SRC_ABSORB: i8 = 1;
pub const SRC_IMMUNE: i8 = 2;
pub const SRC_MORTAL: i8 = 3;

// ── AbilityDef ────────────────────────────────────────────────────────────

/// An ability definition loaded from the database.
///
/// Mirrors `python/common/defs/Ability.py` fields needed for validation and
/// invocation. Full damage/effect resolution will use the effect system.
#[derive(Debug, Clone)]
pub struct AbilityDef {
    pub ability_id: i32,
    pub name: String,
    pub cooldown: f32,
    pub warmup: f32,
    pub flags: u32,
    pub is_ranged: bool,
    pub min_range: i32,
    pub max_range: i32,
    pub target_type_id: i32,
    pub effect_ids: Vec<i32>,
    pub moniker_ids: Vec<i64>,
    pub required_ammo: i32,
}

// ── AbilityTreeData ───────────────────────────────────────────────────────

/// Archetype ability tree: 3 branches of ability IDs.
///
/// Sent to the client via `onAbilityTreeInfo(ARRAY<ARRAY<INT32>>)`.
/// Source: `resources.archetype_ability_tree` table, grouped by `tree_index` (0-2).
#[derive(Debug, Clone, Default)]
pub struct AbilityTreeData {
    pub trees: [Vec<i32>; 3],
}

impl AbilityTreeData {
    /// Serialize for `onAbilityTreeInfo(ARRAY<ARRAY<INT32>>)`.
    ///
    /// Wire: outer ARRAY has no count prefix (the .def says it's
    /// `ARRAY <of> ARRAY <of> INT32 </of> </of>` — but the outer array
    /// count is implicitly 3 based on the Python code which always sends
    /// exactly 3 arrays). Wire: `count1:u32 [ids...] count2:u32 [ids...] count3:u32 [ids...]`.
    pub fn serialize(&self) -> Vec<u8> {
        let total_ids: usize = self.trees.iter().map(|t| t.len()).sum();
        let mut buf = Vec::with_capacity(12 + total_ids * 4);
        for tree in &self.trees {
            buf.extend_from_slice(&(tree.len() as u32).to_le_bytes());
            for &id in tree {
                buf.extend_from_slice(&id.to_le_bytes());
            }
        }
        buf
    }
}

// ── Cooldown tracking ─────────────────────────────────────────────────────

/// A cooldown entry for an ability or moniker group.
#[derive(Debug, Clone)]
pub struct CooldownEntry {
    /// When the cooldown expires.
    pub expires_at: Instant,
    /// Total cooldown duration (for client timer display).
    pub total_duration: Duration,
}

// ── AbilityManager ────────────────────────────────────────────────────────

/// Per-entity ability manager.
///
/// Tracks known abilities, cooldowns, and auto-cycle state.
/// Mirrors `python/cell/AbilityManager.py`.
#[derive(Debug)]
pub struct AbilityManager {
    /// Set of ability IDs this entity knows.
    known_abilities: HashSet<i32>,

    /// Active ability cooldowns keyed by ability_id.
    ability_cooldowns: HashMap<i32, CooldownEntry>,

    /// Active moniker cooldowns keyed by moniker_id.
    moniker_cooldowns: HashMap<i64, CooldownEntry>,

    /// Whether auto-cycle is enabled.
    pub auto_cycle: bool,

    /// The ability ID being auto-cycled (if any).
    pub auto_cycle_ability_id: Option<i32>,

    /// Next unique effect sequence ID.
    pub effect_sequence_id: i32,
}

impl AbilityManager {
    /// Create a new empty ability manager.
    pub fn new() -> Self {
        Self {
            known_abilities: HashSet::new(),
            ability_cooldowns: HashMap::new(),
            moniker_cooldowns: HashMap::new(),
            auto_cycle: false,
            auto_cycle_ability_id: None,
            effect_sequence_id: 1,
        }
    }

    /// Create a new ability manager with the given known ability IDs.
    pub fn with_abilities(ability_ids: &[i32]) -> Self {
        let mut mgr = Self::new();
        for &id in ability_ids {
            mgr.known_abilities.insert(id);
        }
        mgr
    }

    /// Check if the entity knows a given ability.
    pub fn has_ability(&self, ability_id: i32) -> bool {
        self.known_abilities.contains(&ability_id)
    }

    /// Add a known ability.
    pub fn add_ability(&mut self, ability_id: i32) {
        self.known_abilities.insert(ability_id);
    }

    /// Remove a known ability.
    pub fn remove_ability(&mut self, ability_id: i32) {
        self.known_abilities.remove(&ability_id);
    }

    /// Get all known ability IDs (unsorted).
    pub fn known_ability_ids(&self) -> Vec<i32> {
        self.known_abilities.iter().copied().collect()
    }

    /// Number of known abilities.
    pub fn known_count(&self) -> usize {
        self.known_abilities.len()
    }

    /// Check if an ability is on cooldown.
    pub fn is_on_cooldown(&self, ability_id: i32) -> bool {
        if let Some(entry) = self.ability_cooldowns.get(&ability_id) {
            Instant::now() < entry.expires_at
        } else {
            false
        }
    }

    /// Check if a moniker group is on cooldown.
    pub fn is_moniker_on_cooldown(&self, moniker_id: i64) -> bool {
        if let Some(entry) = self.moniker_cooldowns.get(&moniker_id) {
            Instant::now() < entry.expires_at
        } else {
            false
        }
    }

    /// Start a cooldown for an ability.
    pub fn start_ability_cooldown(&mut self, ability_id: i32, duration: Duration) {
        self.ability_cooldowns.insert(ability_id, CooldownEntry {
            expires_at: Instant::now() + duration,
            total_duration: duration,
        });
    }

    /// Start cooldowns for an ability and its moniker groups.
    pub fn start_cooldowns(&mut self, ability: &AbilityDef, cooldown_secs: f32) {
        let duration = Duration::from_secs_f32(cooldown_secs);
        self.start_ability_cooldown(ability.ability_id, duration);

        // Moniker cooldowns expire slightly before ability cooldown
        // to prevent race conditions (Python: completeTime - 0.001)
        let moniker_duration = if cooldown_secs > 0.002 {
            Duration::from_secs_f32(cooldown_secs - 0.001)
        } else {
            duration
        };
        for &moniker_id in &ability.moniker_ids {
            self.moniker_cooldowns.insert(moniker_id, CooldownEntry {
                expires_at: Instant::now() + moniker_duration,
                total_duration: moniker_duration,
            });
        }
    }

    /// Clear all ability and moniker cooldowns (e.g., on respawn).
    pub fn clear_all_cooldowns(&mut self) {
        self.ability_cooldowns.clear();
        self.moniker_cooldowns.clear();
    }

    /// Remove expired cooldowns. Call periodically (e.g., each tick).
    pub fn cleanup_expired(&mut self) {
        let now = Instant::now();
        self.ability_cooldowns.retain(|_, e| now < e.expires_at);
        self.moniker_cooldowns.retain(|_, e| now < e.expires_at);
    }

    /// Check if an ability can be used (basic validation).
    ///
    /// Returns `Ok(())` or an error string describing why not.
    pub fn can_use_ability(&self, ability: &AbilityDef) -> Result<(), &'static str> {
        if !self.has_ability(ability.ability_id) {
            return Err("entity does not have ability");
        }
        if self.is_on_cooldown(ability.ability_id) {
            return Err("ability on cooldown");
        }
        for &moniker_id in &ability.moniker_ids {
            if self.is_moniker_on_cooldown(moniker_id) {
                return Err("moniker on cooldown");
            }
        }
        Ok(())
    }

    /// Get next unique effect sequence ID and increment.
    pub fn next_effect_id(&mut self) -> i32 {
        let id = self.effect_sequence_id;
        self.effect_sequence_id += 1;
        id
    }

    /// Serialize known abilities for `onKnownAbilitiesUpdate(ARRAY<INT32>)`.
    ///
    /// Wire: `count:u32 [ability_id:i32 ...]`.
    pub fn serialize_known_abilities(&self) -> Vec<u8> {
        let ids: Vec<i32> = self.known_abilities.iter().copied().collect();
        let mut buf = Vec::with_capacity(4 + ids.len() * 4);
        buf.extend_from_slice(&(ids.len() as u32).to_le_bytes());
        for id in &ids {
            buf.extend_from_slice(&id.to_le_bytes());
        }
        buf
    }
}

impl Default for AbilityManager {
    fn default() -> Self {
        Self::new()
    }
}

// ── Wire format helpers ───────────────────────────────────────────────────

/// A single stat change result sent to the client in `onEffectResults`.
///
/// Wire per entry: `stat_id:i8, delta:i32, damage_code:i8, stat_result_code:i8` (7 bytes).
#[derive(Debug, Clone)]
pub struct ClientEffectResult {
    pub stat_id: i8,
    pub delta: i32,
    pub damage_code: i8,
    pub stat_result_code: i8,
}

impl ClientEffectResult {
    pub fn serialize(&self, buf: &mut Vec<u8>) {
        buf.push(self.stat_id as u8);
        buf.extend_from_slice(&self.delta.to_le_bytes());
        buf.push(self.damage_code as u8);
        buf.push(self.stat_result_code as u8);
    }
}

/// Serialize `onTimerUpdate` arguments.
///
/// Wire: `id:i32, type:i8, sourceId:i32, secondaryId:i32, totalTime:f32, expireTime:f32`.
pub fn serialize_timer_update(
    id: i32,
    timer_type: i8,
    source_id: i32,
    total_time: f32,
    expire_time: f32,
) -> Vec<u8> {
    let mut buf = Vec::with_capacity(21);
    buf.extend_from_slice(&id.to_le_bytes());
    buf.push(timer_type as u8);
    buf.extend_from_slice(&source_id.to_le_bytes());
    buf.extend_from_slice(&0i32.to_le_bytes()); // secondaryId always 0
    buf.extend_from_slice(&total_time.to_le_bytes());
    buf.extend_from_slice(&expire_time.to_le_bytes());
    buf
}

/// Serialize `onEffectResults` arguments.
///
/// Wire: `sourceId:i32, abilityId:i32, effectId:i32, targetId:i32,
///        resultCode:u8, count:u32, [ClientEffectResult...]`.
pub fn serialize_effect_results(
    source_id: i32,
    ability_id: i32,
    effect_id: i32,
    target_id: i32,
    result_code: u8,
    stat_results: &[ClientEffectResult],
) -> Vec<u8> {
    let mut buf = Vec::with_capacity(21 + stat_results.len() * 7);
    buf.extend_from_slice(&source_id.to_le_bytes());
    buf.extend_from_slice(&ability_id.to_le_bytes());
    buf.extend_from_slice(&effect_id.to_le_bytes());
    buf.extend_from_slice(&target_id.to_le_bytes());
    buf.push(result_code);
    buf.extend_from_slice(&(stat_results.len() as u32).to_le_bytes());
    for result in stat_results {
        result.serialize(&mut buf);
    }
    buf
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ability() -> AbilityDef {
        AbilityDef {
            ability_id: 597,
            name: "Rapid Fire".to_string(),
            cooldown: 5.0,
            warmup: 1.5,
            flags: AF_SPEED_ATTACK,
            is_ranged: true,
            min_range: 0,
            max_range: 30,
            target_type_id: TARGET_TARGET,
            effect_ids: vec![100, 101],
            moniker_ids: vec![42],
            required_ammo: 1,
        }
    }

    #[test]
    fn new_manager_is_empty() {
        let mgr = AbilityManager::new();
        assert_eq!(mgr.known_count(), 0);
        assert!(!mgr.auto_cycle);
        assert_eq!(mgr.effect_sequence_id, 1);
    }

    #[test]
    fn add_and_check_abilities() {
        let mut mgr = AbilityManager::new();
        mgr.add_ability(597);
        mgr.add_ability(592);
        assert!(mgr.has_ability(597));
        assert!(mgr.has_ability(592));
        assert!(!mgr.has_ability(999));
        assert_eq!(mgr.known_count(), 2);
    }

    #[test]
    fn remove_ability() {
        let mut mgr = AbilityManager::with_abilities(&[597, 592, 594]);
        assert_eq!(mgr.known_count(), 3);
        mgr.remove_ability(592);
        assert!(!mgr.has_ability(592));
        assert_eq!(mgr.known_count(), 2);
    }

    #[test]
    fn with_abilities_constructor() {
        let mgr = AbilityManager::with_abilities(&[1, 2, 3]);
        assert_eq!(mgr.known_count(), 3);
        assert!(mgr.has_ability(1));
        assert!(mgr.has_ability(2));
        assert!(mgr.has_ability(3));
    }

    #[test]
    fn cooldown_tracking() {
        let mut mgr = AbilityManager::with_abilities(&[597]);
        let ability = test_ability();

        assert!(!mgr.is_on_cooldown(597));
        mgr.start_ability_cooldown(597, Duration::from_secs(5));
        assert!(mgr.is_on_cooldown(597));

        // Moniker cooldown
        assert!(!mgr.is_moniker_on_cooldown(42));
        mgr.start_cooldowns(&ability, 5.0);
        assert!(mgr.is_moniker_on_cooldown(42));
    }

    #[test]
    fn can_use_ability_checks() {
        let mgr = AbilityManager::with_abilities(&[597]);
        let ability = test_ability();
        assert!(mgr.can_use_ability(&ability).is_ok());

        // Unknown ability
        let unknown = AbilityDef { ability_id: 999, ..test_ability() };
        assert_eq!(mgr.can_use_ability(&unknown).unwrap_err(), "entity does not have ability");
    }

    #[test]
    fn can_use_ability_cooldown_check() {
        let mut mgr = AbilityManager::with_abilities(&[597]);
        let ability = test_ability();
        mgr.start_ability_cooldown(597, Duration::from_secs(5));
        assert_eq!(mgr.can_use_ability(&ability).unwrap_err(), "ability on cooldown");
    }

    #[test]
    fn next_effect_id_increments() {
        let mut mgr = AbilityManager::new();
        assert_eq!(mgr.next_effect_id(), 1);
        assert_eq!(mgr.next_effect_id(), 2);
        assert_eq!(mgr.next_effect_id(), 3);
    }

    #[test]
    fn serialize_known_abilities() {
        let mgr = AbilityManager::with_abilities(&[597, 592]);
        let data = mgr.serialize_known_abilities();
        let count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        assert_eq!(count, 2);
        assert_eq!(data.len(), 4 + 2 * 4); // count + 2 ids
    }

    #[test]
    fn ability_tree_empty() {
        let tree = AbilityTreeData::default();
        let data = tree.serialize();
        // 3 arrays, each with count 0
        assert_eq!(data.len(), 12);
        for i in 0..3 {
            let count = u32::from_le_bytes([
                data[i * 4], data[i * 4 + 1], data[i * 4 + 2], data[i * 4 + 3],
            ]);
            assert_eq!(count, 0);
        }
    }

    #[test]
    fn ability_tree_with_data() {
        let tree = AbilityTreeData {
            trees: [
                vec![592, 1005, 642],
                vec![597, 646, 641],
                vec![],
            ],
        };
        let data = tree.serialize();
        // tree0: count(4) + 3*4=12 = 16
        // tree1: count(4) + 3*4=12 = 16
        // tree2: count(4) = 4
        assert_eq!(data.len(), 36);

        // First tree count
        let c0 = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        assert_eq!(c0, 3);
        // First ability in tree 0
        let a0 = i32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        assert_eq!(a0, 592);
    }

    #[test]
    fn serialize_timer_update_format() {
        let data = serialize_timer_update(597, TIMER_ABILITY_COOLDOWN, 100, 5.0, 12345.0);
        assert_eq!(data.len(), 21);
        let id = i32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        assert_eq!(id, 597);
        assert_eq!(data[4], TIMER_ABILITY_COOLDOWN as u8);
    }

    #[test]
    fn serialize_effect_results_empty() {
        let data = serialize_effect_results(100, 597, 101, 200, RC_HIT, &[]);
        assert_eq!(data.len(), 21); // 4*4 + 1 + 4 = 21
        assert_eq!(data[16], RC_HIT);
        let count = u32::from_le_bytes([data[17], data[18], data[19], data[20]]);
        assert_eq!(count, 0);
    }

    #[test]
    fn serialize_effect_results_with_stats() {
        let results = vec![
            ClientEffectResult {
                stat_id: 10, // health
                delta: -50,
                damage_code: DT_PHYSICAL,
                stat_result_code: SRC_NONE,
            },
        ];
        let data = serialize_effect_results(100, 597, 101, 200, RC_CRITICAL, &results);
        assert_eq!(data.len(), 21 + 7); // 21 header + 7 per result
        assert_eq!(data[16], RC_CRITICAL);
        let count = u32::from_le_bytes([data[17], data[18], data[19], data[20]]);
        assert_eq!(count, 1);
        // stat_id
        assert_eq!(data[21], 10);
    }

    #[test]
    fn client_effect_result_serialize() {
        let r = ClientEffectResult {
            stat_id: 10,
            delta: -100,
            damage_code: DT_ENERGY,
            stat_result_code: SRC_ABSORB,
        };
        let mut buf = Vec::new();
        r.serialize(&mut buf);
        assert_eq!(buf.len(), 7);
        assert_eq!(buf[0], 10); // stat_id
        let delta = i32::from_le_bytes([buf[1], buf[2], buf[3], buf[4]]);
        assert_eq!(delta, -100);
        assert_eq!(buf[5], DT_ENERGY as u8);
        assert_eq!(buf[6], SRC_ABSORB as u8);
    }

    #[test]
    fn cleanup_expired_removes_old() {
        let mut mgr = AbilityManager::with_abilities(&[597]);
        // Add a cooldown that already expired
        mgr.ability_cooldowns.insert(597, CooldownEntry {
            expires_at: Instant::now() - Duration::from_secs(1),
            total_duration: Duration::from_secs(5),
        });
        assert_eq!(mgr.ability_cooldowns.len(), 1);
        mgr.cleanup_expired();
        assert_eq!(mgr.ability_cooldowns.len(), 0);
    }
}
