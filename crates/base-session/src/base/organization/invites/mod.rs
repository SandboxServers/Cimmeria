//! Pending Team and Command invites (D-ORG06), held on the base session.
//!
//! Each character session carries one [`OrgInviteState`]: the invites that
//! character holds as the **invitee**, and the times it recently sent one
//! as the **inviter** (for the rate limit). Keeping the entries on the
//! invitee's session gives D-ORG06's "cleared on every disconnect path" for
//! free: `destroy_client_entities` removes the session, and `logOff`
//! (either variant) calls [`OrgInviteState::clear_for_logoff`] because the
//! session outlives a return to character select.
//!
//! An entry is found only by the invitee's `player_id` **and** the request
//! id together ([`OrgInviteState::take`]), and the first response removes
//! it, accept or decline. Request ids carry [`BASE_INVITE_REQUEST_FLAG`]
//! (bit 29), so the cell routes `organizationInviteResponse` (CM 8) here,
//! and count up from one process-wide counter that is never rewound, so an
//! id is never reused within a server run.
//!
//! The limits are the squad ones (ORG-03, `cimmeria_cell_world::cell::squad`):
//! 60 s to answer, one pending invite per inviter and invitee, five pending
//! per invitee, five sent per inviter in any 30 s. Project policy, not
//! recovered data.
//!
//! An entry stores the organization id. That is safe only because the
//! organization id sequence is `NO CYCLE` (ORG-02): a disbanded
//! organization's id is never handed to a new one, so a stale invite can
//! only miss (`org_gone`), never land the invitee somewhere else.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::{Duration, Instant};

use cimmeria_entity::organization::{OrgType, BASE_INVITE_REQUEST_FLAG};

/// How long an invite answers.
pub const INVITE_TTL: Duration = Duration::from_secs(60);
/// The window of the inviter's rate limit.
pub const INVITE_RATE_WINDOW: Duration = Duration::from_secs(30);
/// Invites one character may send inside [`INVITE_RATE_WINDOW`].
pub const INVITE_RATE_MAX: usize = 5;
/// Unanswered invites one character may hold at once.
pub const MAX_PENDING_PER_INVITEE: usize = 5;

/// The low 29 bits of the next request id. Starts at 1; a value that would
/// reach bit 29 means the range is spent, and nothing is issued again until
/// a restart.
static NEXT_REQUEST: AtomicI32 = AtomicI32::new(1);

/// The next base invite request id: `BASE_INVITE_REQUEST_FLAG | n`, with
/// `n` in `1..2^29`. `None` once the range is spent, so bit 30 (the squad
/// org-id bit) and the sign bit are never set.
pub fn next_request_id() -> Option<i32> {
    NEXT_REQUEST
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
            (n < BASE_INVITE_REQUEST_FLAG).then_some(n + 1)
        })
        .ok()
        .map(|n| BASE_INVITE_REQUEST_FLAG | n)
}

/// One pending Team or Command invite, on the invitee's session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingOrgInvite {
    pub request_id: i32,
    /// The character invited. A session keeps its state across a return to
    /// character select, so [`OrgInviteState::take`] matches it too.
    pub invitee_player_id: i32,
    pub inviter_player_id: i32,
    /// The inviter's character name, for the decline line.
    pub inviter_name: String,
    pub org_id: i32,
    /// The organization's name when the issuer knew it, for the expiry log.
    pub org_name: Option<String>,
    pub org_type: OrgType,
    pub expires_at: Instant,
}

/// Why a response found no invite to consume. Telemetry only: the client
/// gets the same line for all three, so it cannot probe which ids are live.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TakeMiss {
    /// Never issued to this character under this id, or already answered.
    Unknown,
    /// Issued to this character, but past its 60 s.
    Expired,
    /// The id is another character's pending invite. Decided by the caller,
    /// which can see the other sessions.
    Foreign,
}

impl TakeMiss {
    pub fn reason(self) -> &'static str {
        match self {
            TakeMiss::Unknown => "invite_unknown",
            TakeMiss::Expired => "invite_expired",
            TakeMiss::Foreign => "invite_foreign",
        }
    }
}

/// Why an invite could not be recorded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueReject {
    /// This inviter already has an unanswered invite to this character, or
    /// the character already holds [`MAX_PENDING_PER_INVITEE`].
    InviteLimit,
    /// The request-id range is spent.
    IdsExhausted,
}

/// A character session's invite state: held invites and recent sends.
#[derive(Debug, Default)]
pub struct OrgInviteState {
    pending: Vec<PendingOrgInvite>,
    sent: VecDeque<Instant>,
    /// Entries that expired unanswered since the last
    /// [`OrgInviteState::drain_expired`], for the `invite_expired` log.
    expired: Vec<PendingOrgInvite>,
}

impl OrgInviteState {
    /// Drop expired entries and sends older than the rate window.
    pub fn purge(&mut self, now: Instant) {
        let expired = &mut self.expired;
        self.pending.retain(|inv| {
            let live = now < inv.expires_at;
            if !live {
                expired.push(inv.clone());
            }
            live
        });
        while self
            .sent
            .front()
            .is_some_and(|&t| now.saturating_duration_since(t) >= INVITE_RATE_WINDOW)
        {
            self.sent.pop_front();
        }
    }

    /// Whether this character, as inviter, may send another invite now.
    /// Checked before any database work; [`OrgInviteState::record_sent`]
    /// counts the send once it happens.
    pub fn may_send(&mut self, now: Instant) -> bool {
        self.purge(now);
        self.sent.len() < INVITE_RATE_MAX
    }

    /// Count one sent invite against the rate limit.
    pub fn record_sent(&mut self, now: Instant) {
        self.sent.push_back(now);
    }

    /// Record an invite on this (the invitee's) session. The caller has
    /// authorized the inviter under ORG-LOCK and checked the rate limit on
    /// the inviter's session. Only entries for `invitee_player_id` count
    /// toward the per-invitee caps, so a character switched away from
    /// cannot crowd the next one out.
    pub fn issue(
        &mut self,
        invitee_player_id: i32,
        inviter_player_id: i32,
        inviter_name: &str,
        org_id: i32,
        org_type: OrgType,
        now: Instant,
    ) -> Result<PendingOrgInvite, IssueReject> {
        self.issue_named(
            invitee_player_id,
            inviter_player_id,
            inviter_name,
            org_id,
            None,
            org_type,
            now,
        )
    }

    /// [`OrgInviteState::issue`] that also records the organization's name,
    /// so a later `invite_expired` line can name it.
    #[allow(clippy::too_many_arguments)]
    pub fn issue_named(
        &mut self,
        invitee_player_id: i32,
        inviter_player_id: i32,
        inviter_name: &str,
        org_id: i32,
        org_name: Option<&str>,
        org_type: OrgType,
        now: Instant,
    ) -> Result<PendingOrgInvite, IssueReject> {
        self.purge(now);
        let mut held = 0;
        for inv in self
            .pending
            .iter()
            .filter(|i| i.invitee_player_id == invitee_player_id)
        {
            if inv.inviter_player_id == inviter_player_id {
                return Err(IssueReject::InviteLimit);
            }
            held += 1;
        }
        if held >= MAX_PENDING_PER_INVITEE {
            return Err(IssueReject::InviteLimit);
        }
        let request_id = next_request_id().ok_or(IssueReject::IdsExhausted)?;
        let entry = PendingOrgInvite {
            request_id,
            invitee_player_id,
            inviter_player_id,
            inviter_name: inviter_name.to_owned(),
            org_id,
            org_name: org_name.map(str::to_owned),
            org_type,
            expires_at: now + INVITE_TTL,
        };
        self.pending.push(entry.clone());
        Ok(entry)
    }

    /// Consume the invite `invitee_player_id` holds under `request_id`, in
    /// one call so a doubled response cannot accept twice. The first
    /// response removes it; a second finds nothing. Returns
    /// [`TakeMiss::Unknown`] for an id this session does not hold for that
    /// character; the caller upgrades it to [`TakeMiss::Foreign`] if another
    /// session holds it.
    pub fn take(
        &mut self,
        invitee_player_id: i32,
        request_id: i32,
        now: Instant,
    ) -> Result<PendingOrgInvite, TakeMiss> {
        let at = self
            .pending
            .iter()
            .position(|i| i.request_id == request_id && i.invitee_player_id == invitee_player_id);
        // Look up before the purge, so an expired entry reports as expired
        // rather than unknown.
        let expired = at.is_some_and(|i| now >= self.pending[i].expires_at);
        let taken = at.map(|i| self.pending.remove(i));
        self.purge(now);
        match taken {
            Some(inv) if !expired => Ok(inv),
            Some(inv) => {
                self.expired.push(inv);
                Err(TakeMiss::Expired)
            }
            None => Err(TakeMiss::Unknown),
        }
    }

    /// Whether this session holds `request_id` for any character (the
    /// foreign-id check the caller runs over the other sessions).
    pub fn holds(&self, request_id: i32) -> bool {
        self.pending.iter().any(|i| i.request_id == request_id)
    }

    /// Unexpired invites held for `invitee_player_id` at `now`.
    pub fn pending_for(&self, invitee_player_id: i32, now: Instant) -> usize {
        self.pending
            .iter()
            .filter(|i| i.invitee_player_id == invitee_player_id && now < i.expires_at)
            .count()
    }

    /// The entries that expired since the last call.
    pub fn drain_expired(&mut self) -> Vec<PendingOrgInvite> {
        std::mem::take(&mut self.expired)
    }

    /// A `logOff` (either variant): the character leaves the world, so its
    /// held invites go (D-ORG06). Returns how many were dropped. The send
    /// history stays: it belongs to the session, and a character switch
    /// must not reset a spammer's limit.
    pub fn clear_for_logoff(&mut self) -> usize {
        let n = self.pending.len();
        self.pending.clear();
        self.expired.clear();
        n
    }
}

#[cfg(test)]
mod tests;
