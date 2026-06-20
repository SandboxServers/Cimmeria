//! Per-entity ability manager: known abilities, cooldowns, auto-cycle
//! state, and the weapon-grant accounting that lets weapon swaps revoke
//! exactly the abilities the previous weapon added.
//!
//! Mirrors `python/cell/AbilityManager.py`.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use super::AbilityDef;

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

    /// Subset of [`Self::known_abilities`] that were added on behalf of the
    /// **currently-equipped weapon** via the `items_event_sets` binding
    /// (e.g., equipping a P90 grants the SMG Auto Attack ability 559).
    ///
    /// Maintained separately from `known_abilities` so weapon-equip
    /// swaps can revoke exactly the abilities the *previous* weapon
    /// added, without touching player-trained or archetype-starter
    /// abilities. An ability is tracked here ONLY when the weapon
    /// binding added it — if the player already knew the same id
    /// (trained it, archetype-granted), we leave it untracked and
    /// never revoke it on weapon swap.
    ///
    /// See [`Self::swap_weapon_granted_abilities`] for the diff math.
    ///
    /// **Runtime-only, not persisted.** The set is rebuilt on
    /// session start: when a player re-enters the world, the
    /// bandolier's `active_bandolier_slot` is loaded from the
    /// `bandolier_slot` column, and the cell's
    /// `swap_weapon_granted_abilities_for_slot` re-runs against an
    /// empty tracked set + the loaded weapon — producing the same
    /// granted set the previous session ended with. The DB doesn't
    /// need a column for this; the items_event_sets table + the
    /// active slot are the canonical source.
    weapon_granted_abilities: HashSet<i32>,

    /// Active ability cooldowns keyed by ability_id.
    ability_cooldowns: HashMap<i32, CooldownEntry>,

    /// Active moniker cooldowns keyed by moniker_id.
    moniker_cooldowns: HashMap<i64, CooldownEntry>,

    /// Whether auto-cycle is enabled.
    ///
    /// Set by the `setAutoCycle(enabled=1)` cell method (gun-icon button) and
    /// cleared by `setAutoCycle(0)`, by `stoppedAutoCycling`-equivalent events
    /// (target death, `AF_DEACTIVATE_AUTO_CYCLE` ability fire), or by a manual
    /// `useAbility` against a different ability ID. The actual re-fire loop
    /// lives in `cimmeria-services` (`cell::service::ticks::auto_cycle_tick`) —
    /// this flag just arms it.
    pub auto_cycle: bool,

    /// The ability ID being auto-cycled (if any).
    ///
    /// Stashed at the first commit of `useAbility` after `auto_cycle` flipped
    /// to `true`. The driver tick re-invokes `handle_use_ability` with this
    /// id every time the cooldown clears.
    pub auto_cycle_ability_id: Option<i32>,

    /// The last ability the player successfully fired. Stashed at every
    /// `handle_use_ability` commit regardless of `auto_cycle` state, so
    /// `setAutoCycle(1)` can fire immediately against the player's
    /// previously-fired ability + current target without requiring the
    /// player to right-click an enemy first.
    ///
    /// `None` only at session start before any fire. Once set, kept
    /// for the rest of the session — never cleared on auto-cycle
    /// stop, death, or respawn. Stale values are harmless: the
    /// immediate-fire path runs the value through the normal
    /// `handle_use_ability` validation, which rejects abilities the
    /// player no longer has (e.g. after a weapon unequip) and leaves
    /// the loop BSF-armed for the next manual fire to refresh.
    ///
    /// Distinct from `auto_cycle_ability_id`: that field is the ability
    /// the LOOP is currently committed to and clears on stop; this
    /// field is the player's most-recent fire, used as the seed for
    /// the immediate-fire-on-enable path.
    pub last_fired_ability_id: Option<i32>,

    /// Next unique effect sequence ID.
    pub effect_sequence_id: i32,
}

impl AbilityManager {
    /// Create a new empty ability manager.
    pub fn new() -> Self {
        Self {
            known_abilities: HashSet::new(),
            weapon_granted_abilities: HashSet::new(),
            ability_cooldowns: HashMap::new(),
            moniker_cooldowns: HashMap::new(),
            auto_cycle: false,
            auto_cycle_ability_id: None,
            last_fired_ability_id: None,
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

    /// Get the first known ability (for NPCs using their default attack).
    pub fn first_known_ability(&self) -> Option<i32> {
        self.known_abilities.iter().next().copied()
    }

    /// Remove a known ability.
    pub fn remove_ability(&mut self, ability_id: i32) {
        self.known_abilities.remove(&ability_id);
        // If the ability was tracked as weapon-granted, drop the tracking
        // too — caller is removing it unconditionally, so the
        // weapon-grant accounting shouldn't outlive the underlying
        // known-set entry.
        self.weapon_granted_abilities.remove(&ability_id);
    }

    /// Swap the set of abilities granted by the currently-equipped
    /// weapon for a new set (i.e., player equipped a different weapon).
    ///
    /// Returns `(removed, added)` — the ability IDs that left and
    /// joined `known_abilities` as a result of this call. An empty
    /// `(removed, added)` means nothing changed and the caller should
    /// skip the `onKnownAbilitiesUpdate` broadcast to avoid wire noise.
    ///
    /// Semantics:
    ///
    /// 1. For each id in the **previous** `weapon_granted_abilities`
    ///    that is NOT in the new set: remove it from `known_abilities`
    ///    AND drop the weapon-grant tag. This is the revoke path —
    ///    the old weapon no longer grants it and we added it on the
    ///    weapon's behalf, so it leaves with the weapon.
    /// 2. For each id in the **new** set that the player does NOT
    ///    already know: add it to `known_abilities` AND tag it as
    ///    weapon-granted. The tag is what makes step 1 work on the
    ///    NEXT swap.
    /// 3. For each id in the **new** set that the player already
    ///    knows (trained, archetype-granted, etc.): leave
    ///    `known_abilities` untouched AND do NOT tag it as weapon-
    ///    granted. The player owns this ability independently of the
    ///    weapon and we won't revoke it on unequip.
    ///
    /// Passing an empty `new_set` is the "unequip — no weapon" call:
    /// every previously-tracked weapon ability is revoked, and nothing
    /// is added.
    ///
    /// **Why a separate tracked set rather than computing the diff
    /// from items_event_sets at unequip time?** The previous weapon
    /// may have been removed from the items table mid-session
    /// (content reload, hot-fix), or its bindings may have changed.
    /// Tracking what we actually added means we can always undo
    /// exactly that, independent of the current state of the data.
    pub fn swap_weapon_granted_abilities(&mut self, new_set: HashSet<i32>) -> (Vec<i32>, Vec<i32>) {
        // Removed: previously weapon-granted but not in the new set.
        let removed: Vec<i32> = self
            .weapon_granted_abilities
            .difference(&new_set)
            .copied()
            .collect();
        for id in &removed {
            self.known_abilities.remove(id);
            self.weapon_granted_abilities.remove(id);
        }

        // Added: new entries the player didn't already know. If they
        // already knew it (trained/archetype), the weapon-grant tag
        // isn't taken — that ability stays player-owned and won't be
        // revoked on the next swap.
        let mut added: Vec<i32> = Vec::new();
        for id in new_set {
            if !self.known_abilities.contains(&id) {
                self.known_abilities.insert(id);
                self.weapon_granted_abilities.insert(id);
                added.push(id);
            }
        }

        (removed, added)
    }

    /// Inspect which abilities are currently tagged as weapon-granted.
    /// Test helper; exposed for verification, not for runtime mutation.
    #[doc(hidden)]
    pub fn weapon_granted_ability_ids(&self) -> Vec<i32> {
        self.weapon_granted_abilities.iter().copied().collect()
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
        self.ability_cooldowns.insert(
            ability_id,
            CooldownEntry {
                expires_at: Instant::now() + duration,
                total_duration: duration,
            },
        );
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
            self.moniker_cooldowns.insert(
                moniker_id,
                CooldownEntry {
                    expires_at: Instant::now() + moniker_duration,
                    total_duration: moniker_duration,
                },
            );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::{AF_SPEED_ATTACK, TARGET_TARGET};

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
            event_set_id: None,
            velocity: 0.0,
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
        let unknown = AbilityDef {
            ability_id: 999,
            ..test_ability()
        };
        assert_eq!(
            mgr.can_use_ability(&unknown).unwrap_err(),
            "entity does not have ability"
        );
    }

    #[test]
    fn can_use_ability_cooldown_check() {
        let mut mgr = AbilityManager::with_abilities(&[597]);
        let ability = test_ability();
        mgr.start_ability_cooldown(597, Duration::from_secs(5));
        assert_eq!(
            mgr.can_use_ability(&ability).unwrap_err(),
            "ability on cooldown"
        );
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

    // ── swap_weapon_granted_abilities ────────────────────────────────

    /// First weapon equip — empty tracked set, non-empty new set:
    /// every new id lands in known_abilities AND in
    /// weapon_granted_abilities. `added` returns the full new set;
    /// `removed` is empty.
    #[test]
    fn swap_weapon_granted_first_equip_adds_all_new() {
        let mut mgr = AbilityManager::with_abilities(&[594, 597]);
        // Pre: archetype starter has Strike (594) + Heal Focus (597).
        // Equip P90 — items_event_sets binds 559 (SMG Auto) + 595 (SMG Melee).
        let new_set: HashSet<i32> = [559, 595].into_iter().collect();
        let (removed, added) = mgr.swap_weapon_granted_abilities(new_set);

        assert!(removed.is_empty(), "first equip removes nothing");
        let mut added_sorted = added.clone();
        added_sorted.sort();
        assert_eq!(added_sorted, vec![559, 595]);
        assert!(mgr.has_ability(559));
        assert!(mgr.has_ability(595));
        // Archetype abilities untouched.
        assert!(mgr.has_ability(594));
        assert!(mgr.has_ability(597));
        // Weapon-grant tags hold exactly the added ids.
        let mut weapon_granted = mgr.weapon_granted_ability_ids();
        weapon_granted.sort();
        assert_eq!(weapon_granted, vec![559, 595]);
    }

    /// Swap weapon — old tracked abilities revoke, new ones grant.
    /// Player's archetype abilities are NOT in either diff.
    #[test]
    fn swap_weapon_granted_weapon_swap_revokes_old_grants_new() {
        let mut mgr = AbilityManager::with_abilities(&[594, 597]);
        // Equip pistol first — adds 579 + 708 to known + tracked.
        let pistol_set: HashSet<i32> = [579, 708].into_iter().collect();
        mgr.swap_weapon_granted_abilities(pistol_set);

        // Then swap to P90 — adds 559 + 595, revokes 579 + 708.
        let p90_set: HashSet<i32> = [559, 595].into_iter().collect();
        let (removed, added) = mgr.swap_weapon_granted_abilities(p90_set);

        let mut removed_sorted = removed.clone();
        removed_sorted.sort();
        assert_eq!(removed_sorted, vec![579, 708]);

        let mut added_sorted = added.clone();
        added_sorted.sort();
        assert_eq!(added_sorted, vec![559, 595]);

        // Pistol abilities gone, P90 abilities present, archetype intact.
        assert!(!mgr.has_ability(579));
        assert!(!mgr.has_ability(708));
        assert!(mgr.has_ability(559));
        assert!(mgr.has_ability(595));
        assert!(mgr.has_ability(594));
        assert!(mgr.has_ability(597));
    }

    /// Unequip — empty new set revokes every tracked weapon ability
    /// and leaves the player with just their non-weapon known set.
    #[test]
    fn swap_weapon_granted_unequip_revokes_all_tracked() {
        let mut mgr = AbilityManager::with_abilities(&[594, 597]);
        mgr.swap_weapon_granted_abilities([579, 708].into_iter().collect());

        let (removed, added) = mgr.swap_weapon_granted_abilities(HashSet::new());

        let mut removed_sorted = removed.clone();
        removed_sorted.sort();
        assert_eq!(removed_sorted, vec![579, 708]);
        assert!(added.is_empty());
        assert!(!mgr.has_ability(579));
        assert!(!mgr.has_ability(708));
        assert!(mgr.has_ability(594));
        assert!(mgr.has_ability(597));
        assert!(mgr.weapon_granted_ability_ids().is_empty());
    }

    /// **The player-already-knew-it carve-out.** If the new weapon's
    /// binding overlaps an ability the player ALREADY had (trained,
    /// archetype-granted, or already weapon-granted), the swap MUST
    /// NOT tag it as weapon-granted — otherwise the next unequip
    /// would revoke a player-owned ability.
    ///
    /// Reverting the `!self.known_abilities.contains(&id)` gate around
    /// the weapon-grant insert trips this guard by re-tagging an
    /// already-known ability, which then gets revoked on the next
    /// swap — the assertion `has_ability(594)` fails after the second
    /// swap.
    #[test]
    fn swap_weapon_granted_skips_tagging_already_known_ability() {
        // Player has Strike (594) trained as part of archetype.
        let mut mgr = AbilityManager::with_abilities(&[594]);
        // Equip a hypothetical weapon that ALSO binds 594 + 559.
        let weapon_set: HashSet<i32> = [594, 559].into_iter().collect();
        let (_removed, added) = mgr.swap_weapon_granted_abilities(weapon_set);

        // 594 was already known — must NOT appear in `added` (it
        // didn't transition into the known set) and must NOT be
        // tagged.
        assert_eq!(added, vec![559], "only 559 transitioned into known");
        let weapon_granted = mgr.weapon_granted_ability_ids();
        assert_eq!(
            weapon_granted,
            vec![559],
            "594 must NOT be tagged as weapon-granted — player already knew it"
        );

        // Now swap to a weapon that grants neither 594 nor 559.
        // 559 (was tracked) gets revoked; 594 STAYS because it was
        // never tagged.
        let other_set: HashSet<i32> = [777].into_iter().collect();
        let (removed2, _added2) = mgr.swap_weapon_granted_abilities(other_set);
        assert_eq!(removed2, vec![559]);
        assert!(
            mgr.has_ability(594),
            "player-owned 594 must survive the second swap — \
             reverting the already-known carve-out makes this fail"
        );
    }

    /// No-op swap — old and new sets are identical → empty diff,
    /// no broadcast warranted. Pins the empty-tuple short-circuit
    /// the bandolier handler keys on.
    #[test]
    fn swap_weapon_granted_identical_set_returns_empty_diff() {
        let mut mgr = AbilityManager::with_abilities(&[594, 597]);
        mgr.swap_weapon_granted_abilities([559, 595].into_iter().collect());

        // Re-apply the same set.
        let (removed, added) = mgr.swap_weapon_granted_abilities([559, 595].into_iter().collect());
        assert!(removed.is_empty());
        assert!(added.is_empty());
    }

    #[test]
    fn cleanup_expired_removes_old() {
        let mut mgr = AbilityManager::with_abilities(&[597]);
        // Add a cooldown that already expired
        mgr.ability_cooldowns.insert(
            597,
            CooldownEntry {
                expires_at: Instant::now() - Duration::from_secs(1),
                total_duration: Duration::from_secs(5),
            },
        );
        assert_eq!(mgr.ability_cooldowns.len(), 1);
        mgr.cleanup_expired();
        assert_eq!(mgr.ability_cooldowns.len(), 0);
    }

    // ── first_known_ability ──────────────────────────────────────────────

    #[test]
    fn first_known_ability_returns_some() {
        let mgr = AbilityManager::with_abilities(&[597, 600, 605]);
        let first = mgr.first_known_ability();
        assert!(first.is_some());
        // HashSet iteration order is non-deterministic, but it must be one of the three
        assert!([597, 600, 605].contains(&first.unwrap()));
    }

    #[test]
    fn first_known_ability_returns_none_when_empty() {
        let mgr = AbilityManager::new();
        assert!(mgr.first_known_ability().is_none());
    }

    #[test]
    fn first_known_ability_single_element() {
        let mut mgr = AbilityManager::new();
        mgr.add_ability(597);
        assert_eq!(mgr.first_known_ability(), Some(597));
    }

    // ── cooldown edge branches ───────────────────────────────────────────

    /// `is_on_cooldown` must return `false` for an entry that is *present*
    /// but already expired (now >= expires_at), not just for a missing
    /// entry. Covers the `Some(entry)` arm's false-comparison branch that
    /// the happy-path `cooldown_tracking` test never reaches.
    #[test]
    fn is_on_cooldown_false_for_expired_present_entry() {
        let mut mgr = AbilityManager::with_abilities(&[597]);
        mgr.ability_cooldowns.insert(
            597,
            CooldownEntry {
                expires_at: Instant::now() - Duration::from_secs(1),
                total_duration: Duration::from_secs(5),
            },
        );
        assert!(
            !mgr.is_on_cooldown(597),
            "an expired-but-present entry must read as not-on-cooldown"
        );
    }

    /// Companion to the above for the moniker map's `Some(entry)`
    /// expired branch.
    #[test]
    fn is_moniker_on_cooldown_false_for_expired_present_entry() {
        let mut mgr = AbilityManager::new();
        mgr.moniker_cooldowns.insert(
            42,
            CooldownEntry {
                expires_at: Instant::now() - Duration::from_secs(1),
                total_duration: Duration::from_secs(5),
            },
        );
        assert!(!mgr.is_moniker_on_cooldown(42));
    }

    /// `start_cooldowns` with a sub-0.002s cooldown takes the `else`
    /// branch: moniker_duration == the full ability duration (no
    /// 0.001s shave). The happy-path test only exercises the
    /// `> 0.002` arm.
    #[test]
    fn start_cooldowns_tiny_cooldown_uses_full_moniker_duration() {
        let mut mgr = AbilityManager::with_abilities(&[597]);
        let ability = test_ability(); // moniker_ids = [42]
        mgr.start_cooldowns(&ability, 0.001);
        let entry = mgr
            .moniker_cooldowns
            .get(&42)
            .expect("moniker cooldown must be set");
        // else branch: total_duration is the full (un-shaved) duration.
        assert_eq!(entry.total_duration, Duration::from_secs_f32(0.001));
    }

    /// `can_use_ability` must reject when a moniker group is on cooldown
    /// even though the ability itself is known and off cooldown. Covers
    /// the moniker-on-cooldown error arm.
    #[test]
    fn can_use_ability_rejects_on_moniker_cooldown() {
        let mut mgr = AbilityManager::with_abilities(&[597]);
        let ability = test_ability(); // moniker_ids = [42]
                                      // Put only the moniker on cooldown, not the ability.
        mgr.moniker_cooldowns.insert(
            42,
            CooldownEntry {
                expires_at: Instant::now() + Duration::from_secs(5),
                total_duration: Duration::from_secs(5),
            },
        );
        assert!(!mgr.is_on_cooldown(597), "ability itself is off cooldown");
        assert_eq!(
            mgr.can_use_ability(&ability).unwrap_err(),
            "moniker on cooldown"
        );
    }

    /// `clear_all_cooldowns` empties both the ability and moniker maps.
    #[test]
    fn clear_all_cooldowns_empties_both_maps() {
        let mut mgr = AbilityManager::with_abilities(&[597]);
        let ability = test_ability();
        mgr.start_cooldowns(&ability, 5.0);
        assert!(!mgr.ability_cooldowns.is_empty());
        assert!(!mgr.moniker_cooldowns.is_empty());
        mgr.clear_all_cooldowns();
        assert!(mgr.ability_cooldowns.is_empty());
        assert!(mgr.moniker_cooldowns.is_empty());
    }

    /// `cleanup_expired` must also prune the moniker map (not just the
    /// ability map) and must keep entries that are still live.
    #[test]
    fn cleanup_expired_prunes_monikers_keeps_live() {
        let mut mgr = AbilityManager::new();
        // Expired moniker.
        mgr.moniker_cooldowns.insert(
            42,
            CooldownEntry {
                expires_at: Instant::now() - Duration::from_secs(1),
                total_duration: Duration::from_secs(5),
            },
        );
        // Still-live moniker.
        mgr.moniker_cooldowns.insert(
            43,
            CooldownEntry {
                expires_at: Instant::now() + Duration::from_secs(60),
                total_duration: Duration::from_secs(60),
            },
        );
        mgr.cleanup_expired();
        assert!(
            !mgr.moniker_cooldowns.contains_key(&42),
            "expired moniker pruned"
        );
        assert!(
            mgr.moniker_cooldowns.contains_key(&43),
            "live moniker retained"
        );
    }
}
