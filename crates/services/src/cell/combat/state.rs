//! Combatant state flags (dead/alive) packed into a `stateField` bitmask.
//!
//! From `python/Atrea/enums.py:176-184`. `BSF_*` enum values are bit *indices*;
//! `setStateFlag(flag)` does `stateField |= 1 << flag`. BSF_Dead = 0 → value 1.

/// Bitmask for dead state in combatantState (1 << BSF_Dead).
pub const PLAYER_STATE_DEAD: u32 = 1;

/// Bit position for dead flag in stateField (sent to client). Matches python
/// `Atrea.enums.BSF_Dead = 0`. The earlier value 13 was a wire-protocol bug
/// that left dead NPCs visible as "attackable" on the client.
pub const BSF_DEAD: u32 = 0;

/// Check if a state field indicates the entity is dead.
pub fn is_dead_state(state_field: u32) -> bool {
    state_field & (1 << BSF_DEAD) != 0
}

/// Set the dead flag in a state field.
pub fn set_dead_state(state_field: &mut u32) {
    *state_field |= 1 << BSF_DEAD;
}

/// Clear the dead flag in a state field (revive).
pub fn clear_dead_state(state_field: &mut u32) {
    *state_field &= !(1 << BSF_DEAD);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dead_state_flags() {
        let mut state = 0u32;
        assert!(!is_dead_state(state));

        set_dead_state(&mut state);
        assert!(is_dead_state(state));
        assert_eq!(state, 1);

        clear_dead_state(&mut state);
        assert!(!is_dead_state(state));
        assert_eq!(state, 0);
    }
}
