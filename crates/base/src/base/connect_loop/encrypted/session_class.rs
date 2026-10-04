//! The clientIndex of a session's entity, which decides what an inbound
//! message id means for logs: base method ids 0xC0+ are `Account` methods
//! at character select and the player's in world.

use cimmeria_wire::names;

use super::super::super::ConnectedClientState;

/// The clientIndex of `state`'s entity, for naming its messages: the
/// player's class once it has a player entity (the same test that routes
/// 0xC0/0xC1 in `dispatch_client_bundle`), `Account` at character select.
/// Read under the session lock the receive gate already holds, so naming
/// costs no extra lock.
pub(super) fn session_class_id(state: &ConnectedClientState) -> u8 {
    match state.player_entity_id {
        Some(_) => state.player_class_id.unwrap_or(names::SGWPLAYER_CLASS_ID),
        None => names::ACCOUNT_CLASS_ID,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_default_connected_client_state;

    /// 0xC0 is `versionInfoRequest` at character select and `chatJoin` in
    /// world, so the class must follow the phase, and a GM keeps its class.
    #[test]
    fn class_follows_the_session_phase() {
        let mut state = test_default_connected_client_state();
        assert_eq!(session_class_id(&state), names::ACCOUNT_CLASS_ID);

        state.player_entity_id = Some(9);
        assert_eq!(session_class_id(&state), names::SGWPLAYER_CLASS_ID);

        state.player_class_id = Some(names::SGWGMPLAYER_CLASS_ID);
        assert_eq!(session_class_id(&state), names::SGWGMPLAYER_CLASS_ID);
    }
}
