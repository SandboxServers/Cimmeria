//! Combatant state flags (dead/alive) packed into a `stateField` bitmask.
//!
//! From `python/Atrea/enums.py:176-184`. `BSF_*` enum values are bit *indices*;
//! `setStateFlag(flag)` does `stateField |= 1 << flag`. BSF_Dead = 0 → value 1.

/// Bitmask for dead state in combatantState (1 << BSF_Dead).
pub const PLAYER_STATE_DEAD: u32 = 1;

/// Bit position for dead flag in stateField (sent to client). Matches python
/// `Atrea.enums.BSF_Dead = 0`. The earlier value 13 was a wire-protocol bug
/// that left dead NPCs visible as "attackable" on the client.
pub const BSF_DEAD_BIT: u32 = 0;

/// `BSF_Dead` mask. Single-source today — death always comes from one
/// authoritative kill site, and respawn does a hard `clear_all_state_flags`.
/// Kept as a mask constant (not just the bit index) so callers route through
/// the ref-counted entity helpers consistently with the other BSF_* flags.
pub const BSF_DEAD: u32 = 1 << BSF_DEAD_BIT;

/// `BSF_InCombat` mask. The client uses this bit to route right-click on
/// selected entities to `useAbility` (auto-attack) instead of `interact`.
/// From python `Atrea.enums.BSF_InCombat = 3`.
///
/// Per-player threat-set management (#92) is the long-term setter; today the
/// flag is set on weapon-fire/reload and cleared on the kill that drops the
/// last (single-target) aggro source.
pub const BSF_IN_COMBAT: u32 = 1 << 3;

/// `BSF_MovementLock` mask. Multi-source flag (#117): death applies it,
/// future stun/fear effects will too. Going through the ref-counted entity
/// helpers means clearing one source doesn't drop the others.
/// From python `Atrea.enums.BSF_MovementLock = 6`.
pub const BSF_MOVEMENT_LOCK: u32 = 1 << 6;

/// `BSF_Holster` mask. Set while the weapon is holstered. Cleared when the
/// player enters combat-ready (e.g., on reload or first attack).
/// From python `Atrea.enums.BSF_Holster = 8`.
pub const BSF_HOLSTER: u32 = 1 << 8;

/// Check if a state field indicates the entity is dead. Reads are fine
/// against the raw `state_field` bitmask — it's the writes that need to
/// route through the ref-counted helpers.
pub fn is_dead_state(state_field: u32) -> bool {
    state_field & BSF_DEAD != 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_entity::cell_entity::CellEntity;
    use cimmeria_common::{EntityId, SpaceId, Vector3};

    fn entity() -> CellEntity {
        CellEntity::new(EntityId(1), SpaceId(0), Vector3::new(0.0, 0.0, 0.0))
    }

    #[test]
    fn dead_state_via_entity_helpers() {
        let mut e = entity();
        assert!(!is_dead_state(e.state_field));

        e.set_state_flag(BSF_DEAD);
        assert!(is_dead_state(e.state_field));
        assert_eq!(e.state_field, BSF_DEAD);

        e.unset_state_flag(BSF_DEAD);
        assert!(!is_dead_state(e.state_field));
        assert_eq!(e.state_field, 0);
    }

    #[test]
    fn refcount_keeps_flag_set_after_partial_unset() {
        // Issue #117 regression: two sources both set a flag (e.g. two
        // stuns applying BSF_MovementLock); clearing one MUST keep the bit
        // set until the second clear drains the counter.
        let mut e = entity();
        assert!(e.set_state_flag(BSF_MOVEMENT_LOCK), "first set should transition");
        assert!(!e.set_state_flag(BSF_MOVEMENT_LOCK), "second set must not re-fire");
        assert!(e.has_state_flag(BSF_MOVEMENT_LOCK));

        assert!(!e.unset_state_flag(BSF_MOVEMENT_LOCK), "first unset should not transition");
        assert!(e.has_state_flag(BSF_MOVEMENT_LOCK), "flag must remain set with one source still active");

        assert!(e.unset_state_flag(BSF_MOVEMENT_LOCK), "second unset should clear");
        assert!(!e.has_state_flag(BSF_MOVEMENT_LOCK));
    }

    #[test]
    fn unset_at_zero_is_noop() {
        // Defensive: a stray clear without a prior set must not underflow
        // (would silently flip the bit on the next set if it did).
        let mut e = entity();
        assert!(!e.unset_state_flag(BSF_DEAD), "unset on empty counter is a no-op");
        assert_eq!(e.state_field, 0);
    }

    #[test]
    fn clear_all_resets_counters_too() {
        // Respawn path: hard reset must drop counters, otherwise the next
        // `unset_state_flag` for an unrelated source would still see leftover
        // refs and skip the bit transition.
        let mut e = entity();
        e.set_state_flag(BSF_DEAD);
        e.set_state_flag(BSF_MOVEMENT_LOCK);
        e.set_state_flag(BSF_MOVEMENT_LOCK); // counter = 2
        e.clear_all_state_flags();
        assert_eq!(e.state_field, 0);
        // Subsequent set should transition cleanly (counter starts at 0).
        assert!(e.set_state_flag(BSF_MOVEMENT_LOCK));
    }

    #[test]
    fn independent_flags_dont_share_counters() {
        // Pin: counter map is keyed by mask, so BSF_DEAD and BSF_HOLSTER
        // each have their own counter. Clearing one must not affect the
        // other.
        let mut e = entity();
        e.set_state_flag(BSF_DEAD);
        e.set_state_flag(BSF_HOLSTER);
        e.unset_state_flag(BSF_DEAD);
        assert!(!e.has_state_flag(BSF_DEAD));
        assert!(e.has_state_flag(BSF_HOLSTER));
    }
}
