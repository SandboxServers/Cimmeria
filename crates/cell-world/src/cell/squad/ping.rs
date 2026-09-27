//! Minimap pings (CM 10 `BroadcastMinimapPing`, ORG-04): the membership
//! check and the one-per-second limit.
//!
//! ORG-E1 Q3 found no receive-side message in the client
//! (`receivedMinimapPing` is server-internal and never reaches a client), so
//! a ping is validated and logged, never fanned out.

use std::time::{Duration, Instant};

use super::SquadRegistry;

/// The shortest gap between two accepted pings from one member.
pub const PING_MIN_INTERVAL: Duration = Duration::from_secs(1);

/// Why a ping was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PingReject {
    /// The caller is in no squad.
    NotInSquad,
    /// The caller is in a squad, but not the one the id names.
    WrongSquad,
    /// Less than [`PING_MIN_INTERVAL`] since the caller's last accepted ping.
    RateLimited,
}

impl PingReject {
    pub fn reason(self) -> &'static str {
        match self {
            PingReject::NotInSquad => "not_in_squad",
            PingReject::WrongSquad => "wrong_squad",
            PingReject::RateLimited => "rate_limited",
        }
    }
}

impl SquadRegistry {
    /// Accept or refuse a ping from `player_id` naming `squad_id`. Only an
    /// accepted ping starts a new interval, so a refused one never pushes
    /// the next allowed ping further out.
    pub fn check_ping(
        &mut self,
        player_id: i32,
        squad_id: i32,
        now: Instant,
    ) -> Result<(), PingReject> {
        match self.squad_of(player_id) {
            None => return Err(PingReject::NotInSquad),
            Some(sid) if sid != squad_id => return Err(PingReject::WrongSquad),
            Some(_) => {}
        }
        if let Some(&last) = self.last_ping.get(&player_id) {
            if now.saturating_duration_since(last) < PING_MIN_INTERVAL {
                return Err(PingReject::RateLimited);
            }
        }
        self.last_ping.insert(player_id, now);
        Ok(())
    }
}
