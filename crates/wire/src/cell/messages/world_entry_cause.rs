//! Why the base sent an `InitPlayerState` (TG-NET-15).

/// What brought the player into this world: a fresh entry from character
/// select, or a cross-world hop inside an existing session (stargate dial,
/// space transfer, cross-world ring transport, GM travel, cross-world
/// respawn — every one of them replays world entry through
/// `gate_travel::handle_gate_travel`).
///
/// The base decides it from `ConnectedClientState::listed_online` at
/// `onClientReady`: the flag is set by the first world entry of a character
/// session and cleared only by `logOff` or teardown, so a travel finds it
/// already set. The cell stamps it on `session.start` as `start_cause`, so
/// SigNoz can count logins without the travel re-entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldEntryCause {
    /// The character's first world entry of this session.
    Login,
    /// A re-entry within the same character session, almost always after a
    /// cross-world transfer.
    Travel,
}

impl WorldEntryCause {
    /// The `start_cause` value on `session.start`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Login => "login",
            Self::Travel => "travel",
        }
    }
}
