//! Why a login could not claim its session.
//!
//! Split from `session.rs`, which is over the file-size cap. Re-exported as
//! `session::ClaimRejection`.

/// Why [`super::session::SessionRegistry::authenticate_and_claim`] refused a
/// login.
///
/// The registry does not log these; the connection task does, with the
/// peer address. Most come from the public port: the entity id is a small
/// guessable integer, so anyone can send a login naming a registered entity
/// with a made-up ticket. The ticket is 256 bits from a CSPRNG, so a
/// mismatch is never a real player and must not reach the Discord warn
/// harvest. Only [`ClaimRejection::AlreadyClaimed`] needs the real ticket,
/// which is why it alone stays WARN.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimRejection {
    /// No session is registered for that entity id.
    NoSession,
    /// The ticket does not match the registered session's.
    TicketMismatch { player_name: Option<String> },
    /// Right ticket, wrong game name in the login's zone.
    GameMismatch {
        player_name: Option<String>,
        expected: String,
    },
    /// The ticket is valid but a live connection already holds it.
    AlreadyClaimed { player_name: Option<String> },
}

impl ClaimRejection {
    /// Stable `reason` field value for the log row.
    pub fn reason(&self) -> &'static str {
        match self {
            ClaimRejection::NoSession => "no_session",
            ClaimRejection::TicketMismatch { .. } => "ticket_mismatch",
            ClaimRejection::GameMismatch { .. } => "game_name_mismatch",
            ClaimRejection::AlreadyClaimed { .. } => "ticket_already_claimed",
        }
    }

    /// The registered character's name, when a session exists.
    pub fn player_name(&self) -> Option<&str> {
        match self {
            ClaimRejection::NoSession => None,
            ClaimRejection::TicketMismatch { player_name }
            | ClaimRejection::GameMismatch { player_name, .. }
            | ClaimRejection::AlreadyClaimed { player_name } => player_name.as_deref(),
        }
    }
}
