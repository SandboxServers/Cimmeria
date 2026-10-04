//! Posted presses waiting for the router (the asynchronous half of the
//! press chain): the router hook claims one by method and ability id.

use std::collections::VecDeque;

use super::Source;

/// How long a posted press waits for the router before it is counted as
/// expired. The router runs on the next pump of the same thread, so a
/// healthy client claims it within a frame.
pub(crate) const SEND_TTL_MS: u64 = 5_000;

/// A ground reticle waits for the player to place it.
pub(crate) const GROUND_TTL_MS: u64 = 60_000;

/// Unclaimed presses kept at most.
pub(crate) const MAX_PENDING: usize = 32;

/// A press whose event was posted, waiting for the router.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PendingSend {
    /// The press.
    pub press_id: u32,
    /// Its source, for a router-side drop row.
    pub source: Source,
    /// The method the router will see.
    pub method: &'static str,
    /// Its ability.
    pub ability_id: Option<i32>,
    /// When it was posted.
    pub at_ms: u64,
    /// How long it may wait.
    pub ttl_ms: u64,
}

/// Posted presses, oldest first.
#[derive(Debug, Default)]
pub(crate) struct PendingTable {
    q: VecDeque<PendingSend>,
    expired: u64,
}

impl PendingTable {
    fn expire(&mut self, now_ms: u64) {
        let before = self.q.len();
        self.q
            .retain(|p| now_ms.saturating_sub(p.at_ms) <= p.ttl_ms);
        self.expired += (before - self.q.len()) as u64;
    }

    /// Add a posted press.
    pub(crate) fn push(&mut self, p: PendingSend) {
        self.expire(p.at_ms);
        if self.q.len() >= MAX_PENDING {
            self.q.pop_front();
            self.expired += 1;
        }
        self.q.push_back(p);
    }

    /// Claim the oldest press for `method` and `ability_id`.
    pub(crate) fn take(
        &mut self,
        method: &str,
        ability_id: Option<i32>,
        now_ms: u64,
    ) -> Option<PendingSend> {
        self.expire(now_ms);
        let i = self
            .q
            .iter()
            .position(|p| p.method == method && p.ability_id == ability_id)?;
        self.q.remove(i)
    }

    /// Presses that expired unclaimed since the last call.
    pub(crate) fn take_expired(&mut self, now_ms: u64) -> u64 {
        self.expire(now_ms);
        std::mem::take(&mut self.expired)
    }
}
