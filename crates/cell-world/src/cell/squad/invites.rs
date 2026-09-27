//! Pending squad invites and the invite rate limits (D-ORG06, CAT-M-18).
//!
//! An invite is keyed by the invitee's `player_id` **and** the request id
//! together, so guessing another player's request id finds nothing. It is
//! consumed by the first response, accept or decline, and answers for 60 s.
//! Request ids count up from 1 with [`BASE_INVITE_REQUEST_FLAG`] clear, so
//! `organizationInviteResponse` (CM 8) routes on that bit; they are never
//! reused within a server run.

use std::time::Instant;

use cimmeria_entity::organization::BASE_INVITE_REQUEST_FLAG;

use super::registry::SquadRegistry;
use super::{INVITE_RATE_MAX, INVITE_RATE_WINDOW, INVITE_TTL, MAX_PENDING_PER_INVITEE};

/// One pending squad invite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingInvite {
    pub inviter_player_id: i32,
    /// For the decline feedback.
    pub inviter_name: String,
    /// The inviter's squad when the invite was issued; `None` when the
    /// inviter had none and the accept will found one.
    pub squad_id: Option<i32>,
    pub expires_at: Instant,
}

/// An invite that reached its expiry unanswered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpiredInvite {
    pub invitee_player_id: i32,
    pub request_id: i32,
    pub inviter_player_id: i32,
    pub squad_id: Option<i32>,
}

impl ExpiredInvite {
    fn of(invitee_player_id: i32, request_id: i32, inv: &PendingInvite) -> Self {
        Self {
            invitee_player_id,
            request_id,
            inviter_player_id: inv.inviter_player_id,
            squad_id: inv.squad_id,
        }
    }
}

/// Why a response found no invite to consume.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TakeMiss {
    /// Never issued under this id, or already answered.
    Unknown,
    /// Issued to this player, but past its 60 s.
    Expired,
    /// The id is another player's pending invite.
    Foreign,
}

/// A successfully issued invite: what `onOrganizationInvite` [34] carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IssuedInvite {
    pub request_id: i32,
    pub squad_id: Option<i32>,
}

/// Why an invite was not issued.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InviteReject {
    TargetInSquad,
    /// The inviter is in a squad they do not lead.
    InviterNotLeader,
    SquadFull,
    /// This inviter already has a pending invite to this player.
    DuplicatePending,
    /// The invitee already holds [`MAX_PENDING_PER_INVITEE`] invites.
    InviteeInboxFull,
    /// The inviter sent [`INVITE_RATE_MAX`] invites in the last
    /// [`INVITE_RATE_WINDOW`].
    RateLimited,
    /// The cell's request-id range is spent (2^29 - 1 invites).
    RequestIdsExhausted,
}

impl InviteReject {
    pub fn reason(self) -> &'static str {
        match self {
            InviteReject::TargetInSquad => "target_in_squad",
            InviteReject::InviterNotLeader => "inviter_not_leader",
            InviteReject::SquadFull => "squad_full",
            InviteReject::DuplicatePending => "duplicate_pending",
            InviteReject::InviteeInboxFull => "invitee_inbox_full",
            InviteReject::RateLimited => "rate_limited",
            InviteReject::RequestIdsExhausted => "request_ids_exhausted",
        }
    }
}

/// Why a consumed, accepted invite could not be applied, or why a
/// response found no invite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseReject {
    InviteeInSquad,
    /// The squad the invite named no longer exists.
    SquadGone,
    /// The inviter is no longer in the squad the invite named.
    InviterLeft,
    /// The inviter is in the squad but no longer leads it.
    InviterNotLeader,
    SquadFull,
    /// The inviter has no squad and is not online to found one.
    InviterOffline,
    /// The squad-id range is spent.
    SquadIdsExhausted,
}

impl ResponseReject {
    pub fn reason(self) -> &'static str {
        match self {
            ResponseReject::InviteeInSquad => "invitee_in_squad",
            ResponseReject::SquadGone => "squad_gone",
            ResponseReject::InviterLeft => "inviter_left",
            ResponseReject::InviterNotLeader => "inviter_not_leader",
            ResponseReject::SquadFull => "squad_full",
            ResponseReject::InviterOffline => "inviter_offline",
            ResponseReject::SquadIdsExhausted => "squad_ids_exhausted",
        }
    }
}

impl SquadRegistry {
    /// Drop every invite that has expired by `now`. Every invite call runs
    /// this first, so an expired invite neither answers nor counts.
    pub fn purge_expired(&mut self, now: Instant) {
        let expired = &mut self.expired;
        self.invites.retain(|&(invitee, request_id), inv| {
            let live = now < inv.expires_at;
            if !live {
                expired.push(ExpiredInvite::of(invitee, request_id, inv));
            }
            live
        });
        self.sent.retain(|_, times| {
            while times
                .front()
                .is_some_and(|&t| now.saturating_duration_since(t) >= INVITE_RATE_WINDOW)
            {
                times.pop_front();
            }
            !times.is_empty()
        });
    }

    /// Issue an invite from `inviter_player_id` to `invitee_player_id`.
    ///
    /// The caller has already resolved the invitee to an online player
    /// other than the inviter (D-ORG06 names the checks the cell makes
    /// here: both sides' squads, room, and the three limits).
    pub fn invite(
        &mut self,
        inviter_player_id: i32,
        inviter_name: &str,
        invitee_player_id: i32,
        now: Instant,
    ) -> Result<IssuedInvite, InviteReject> {
        self.purge_expired(now);
        if self.member_of.contains_key(&invitee_player_id) {
            return Err(InviteReject::TargetInSquad);
        }
        let squad_id = self.squad_of(inviter_player_id);
        if let Some(squad) = squad_id.and_then(|id| self.squads.get(&id)) {
            if squad.leader_player_id() != inviter_player_id {
                return Err(InviteReject::InviterNotLeader);
            }
            if squad.is_full() {
                return Err(InviteReject::SquadFull);
            }
        }
        let mut held = 0;
        for (&(invitee, _), inv) in &self.invites {
            if invitee != invitee_player_id {
                continue;
            }
            if inv.inviter_player_id == inviter_player_id {
                return Err(InviteReject::DuplicatePending);
            }
            held += 1;
        }
        if held >= MAX_PENDING_PER_INVITEE {
            return Err(InviteReject::InviteeInboxFull);
        }
        if self
            .sent
            .get(&inviter_player_id)
            .is_some_and(|t| t.len() >= INVITE_RATE_MAX)
        {
            return Err(InviteReject::RateLimited);
        }
        let Some(request_id) = self.next_request_id else {
            return Err(InviteReject::RequestIdsExhausted);
        };
        self.next_request_id = request_id
            .checked_add(1)
            .filter(|&n| n < BASE_INVITE_REQUEST_FLAG);
        self.invites.insert(
            (invitee_player_id, request_id),
            PendingInvite {
                inviter_player_id,
                inviter_name: inviter_name.to_owned(),
                squad_id,
                expires_at: now + INVITE_TTL,
            },
        );
        self.sent
            .entry(inviter_player_id)
            .or_default()
            .push_back(now);
        Ok(IssuedInvite {
            request_id,
            squad_id,
        })
    }

    /// Consume the invite `invitee_player_id` holds under `request_id`.
    /// The first response, accept or decline, removes it; a second finds
    /// nothing. A request id issued to another player finds nothing either,
    /// and leaves that player's invite in place.
    ///
    /// The miss says why, for the outcome log: the entry expired (it is
    /// dropped and recorded as expired), the id belongs to another invitee,
    /// or nothing is known under it (never issued, or already answered).
    pub fn take_invite(
        &mut self,
        invitee_player_id: i32,
        request_id: i32,
        now: Instant,
    ) -> Result<PendingInvite, TakeMiss> {
        let key = (invitee_player_id, request_id);
        let miss = match self.invites.get(&key) {
            Some(inv) if now < inv.expires_at => None,
            Some(_) => Some(TakeMiss::Expired),
            None if self.invites.keys().any(|&(_, id)| id == request_id) => Some(TakeMiss::Foreign),
            None => Some(TakeMiss::Unknown),
        };
        // Purge after the lookup, so an expired entry reports as expired
        // rather than unknown.
        self.purge_expired(now);
        match miss {
            None => Ok(self.invites.remove(&key).expect("checked live above")),
            Some(m) => Err(m),
        }
    }

    /// The invites that expired since the last call, for the
    /// `invite_expired` transition log.
    pub fn drain_expired(&mut self) -> Vec<ExpiredInvite> {
        std::mem::take(&mut self.expired)
    }

    /// The request ids of the unexpired invites `invitee_player_id` holds
    /// at `now`, ascending.
    pub fn pending_requests(&self, invitee_player_id: i32, now: Instant) -> Vec<i32> {
        let mut ids: Vec<i32> = self
            .invites
            .iter()
            .filter(|&(&(invitee, _), inv)| invitee == invitee_player_id && now < inv.expires_at)
            .map(|(&(_, request_id), _)| request_id)
            .collect();
        ids.sort_unstable();
        ids
    }

    /// Unexpired invites `invitee_player_id` holds at `now`.
    pub fn pending_for(&self, invitee_player_id: i32, now: Instant) -> usize {
        self.invites
            .iter()
            .filter(|&(&(invitee, _), inv)| invitee == invitee_player_id && now < inv.expires_at)
            .count()
    }
}
