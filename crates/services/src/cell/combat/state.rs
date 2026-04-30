//! Combatant state flags (dead/alive) packed into a `stateField` bitmask.
//!
//! From `python/Atrea/enums.py`. The client reads `stateField` and treats
//! bit 13 as "dead" — value 8192 = `PLAYER_STATE_DEAD`.

/// Bitmask for dead state in combatantState.
pub const PLAYER_STATE_DEAD: u32 = 8192;

/// Bit position for dead flag in stateField (sent to client).
pub const BSF_DEAD: u32 = 13; // bit 13 → value 8192

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
        assert_eq!(state, 8192);

        clear_dead_state(&mut state);
        assert!(!is_dead_state(state));
        assert_eq!(state, 0);
    }
}
