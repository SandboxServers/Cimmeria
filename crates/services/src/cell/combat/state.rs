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

/// `BSF_AutoCycling` mask. The client emits `Event_UI_AutoCycle` on every
/// transition of this bit (verified via the XOR-delta dispatcher at
/// `ghidra://SGW.exe@0x00e01c90` — `TEST BL, 0x2` → `EmitAutoCycleStateChanged`
/// at `0x00e05fb0`). `USGWTargetIndicator` listens to the resulting CME event
/// to highlight the gun-icon button.
///
/// Server-side, the flag is set the moment the player presses the
/// auto-cycle button (`setAutoCycle(1)`) so the client gets immediate
/// visual feedback, independent of whether the loop has had its first
/// ability commit yet. Cleared on stop: `setAutoCycle(0)`, target death,
/// manual fire of a different ability, an
/// `AF_DEACTIVATE_AUTO_CYCLE`-flagged ability firing, target deselect,
/// or dead/despawned target during the loop. See
/// [`crate::cell::service::ticks::auto_cycle_tick`] for the driver loop.
///
/// From python `Atrea.enums.BSF_AutoCycling = 1`.
pub const BSF_AUTO_CYCLING: u32 = 1 << 1;

/// `BSF_InCombat` mask. The client uses this bit to route right-click on
/// selected entities to `useAbility` (auto-attack) instead of `interact`.
/// From python `Atrea.enums.BSF_InCombat = 3`.
///
/// Per-player threat-set management (#92) is the long-term setter; today the
/// flag is set on weapon-fire/reload and cleared on the kill that drops the
/// last (single-target) aggro source.
pub const BSF_IN_COMBAT: u32 = 1 << 3;

/// `BSF_MovementLock` mask. Multi-source flag: death applies it,
/// future stun/fear effects will too. Going through the ref-counted entity
/// helpers means clearing one source doesn't drop the others.
/// From python `Atrea.enums.BSF_MovementLock = 6`.
pub const BSF_MOVEMENT_LOCK: u32 = 1 << 6;

/// Check if a state field indicates the entity is dead. Reads are fine
/// against the raw `state_field` bitmask — it's the writes that need to
/// route through the ref-counted helpers.
pub fn is_dead_state(state_field: u32) -> bool {
    state_field & BSF_DEAD != 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_common::{EntityId, SpaceId, Vector3};
    use cimmeria_entity::cell_entity::CellEntity;

    fn entity() -> CellEntity {
        CellEntity::new(EntityId(1), SpaceId(0), Vector3::new(0.0, 0.0, 0.0))
    }

    /// Stand-in test flag used to exercise the generic ref-counted helpers
    /// against a flag that's neither `BSF_DEAD` nor `BSF_MOVEMENT_LOCK`.
    /// Bit 12 sits in the wire-dead range (bits 8-31; client only
    /// dispatches on bits 0-7 per `docs/architecture/state-field-bits.md`),
    /// so these tests pin the counter map's bounded-growth invariants
    /// without affecting any client-visible behavior.
    const BSF_TEST_UNUSED_BIT_12: u32 = 1 << 12;

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
        // Multi-source semantics: two stuns both setting BSF_MovementLock
        // bump the counter to 2; clearing one MUST keep the bit set until
        // the second clear drains the counter.
        let mut e = entity();
        assert!(
            e.set_state_flag(BSF_MOVEMENT_LOCK),
            "first set should transition"
        );
        assert!(
            !e.set_state_flag(BSF_MOVEMENT_LOCK),
            "second set must not re-fire"
        );
        assert!(e.has_state_flag(BSF_MOVEMENT_LOCK));

        assert!(
            !e.unset_state_flag(BSF_MOVEMENT_LOCK),
            "first unset should not transition"
        );
        assert!(
            e.has_state_flag(BSF_MOVEMENT_LOCK),
            "flag must remain set with one source still active"
        );

        assert!(
            e.unset_state_flag(BSF_MOVEMENT_LOCK),
            "second unset should clear"
        );
        assert!(!e.has_state_flag(BSF_MOVEMENT_LOCK));
    }

    #[test]
    fn unset_at_zero_is_noop() {
        // Defensive: a stray clear without a prior set must not underflow
        // (would silently flip the bit on the next set if it did).
        let mut e = entity();
        assert!(
            !e.unset_state_flag(BSF_DEAD),
            "unset on empty counter is a no-op"
        );
        assert_eq!(e.state_field, 0);
    }

    #[test]
    fn unset_on_unowned_flag_does_not_grow_counter_map() {
        // Hot paths that defensively unset flags they may not own (e.g.
        // npc_ai cleanup loops) shouldn't leak map entries — best-effort
        // clears must stay zero-cost.
        let mut e = entity();
        for _ in 0..100 {
            e.unset_state_flag(BSF_DEAD);
            e.unset_state_flag(BSF_MOVEMENT_LOCK);
            e.unset_state_flag(BSF_TEST_UNUSED_BIT_12);
        }
        assert!(
            e.state_flag_counts.is_empty(),
            "no map entries from stray unsets"
        );
    }

    #[test]
    fn drained_counter_drops_map_entry() {
        // After a balanced set/unset pair the counter map should hold no
        // residual zero-count entry. Otherwise the map grows without bound
        // for any flag ever touched, even ones currently unheld.
        let mut e = entity();
        e.set_state_flag(BSF_DEAD);
        e.unset_state_flag(BSF_DEAD);
        assert!(
            e.state_flag_counts.is_empty(),
            "drained counter should be removed"
        );
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
        // Pin: counter map is keyed by mask, so each flag has its own
        // counter. Clearing one must not affect the other.
        let mut e = entity();
        e.set_state_flag(BSF_DEAD);
        e.set_state_flag(BSF_TEST_UNUSED_BIT_12);
        e.unset_state_flag(BSF_DEAD);
        assert!(!e.has_state_flag(BSF_DEAD));
        assert!(e.has_state_flag(BSF_TEST_UNUSED_BIT_12));
    }
}
