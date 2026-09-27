//! Squads and their membership: join, leave, kick, disconnect, loot mode.

use std::collections::{HashMap, VecDeque};
use std::time::Instant;

use cimmeria_entity::organization::{
    OrgLeaveReason, OrgRank, SquadLootType, MAX_SQUAD_SIZE, SQUAD_ORG_ID_MAX, SQUAD_ORG_ID_MIN,
};

use super::invites::{PendingInvite, ResponseReject};
use super::SquadMember;

/// One squad.
#[derive(Debug, Clone)]
pub struct Squad {
    id: i32,
    /// In join order. The first member is the longest-standing one, who
    /// inherits the lead when the leader goes (D-ORG12).
    members: Vec<SquadMember>,
    leader: i32,
    loot: SquadLootType,
}

impl Squad {
    pub fn id(&self) -> i32 {
        self.id
    }

    /// Members in join order.
    pub fn members(&self) -> &[SquadMember] {
        &self.members
    }

    pub fn leader_player_id(&self) -> i32 {
        self.leader
    }

    pub fn loot(&self) -> SquadLootType {
        self.loot
    }

    /// `Leader` for the leader, `Member` for everyone else (D-ORG07).
    pub fn rank_of(&self, player_id: i32) -> OrgRank {
        if player_id == self.leader {
            OrgRank::LEADER
        } else {
            OrgRank::MEMBER
        }
    }

    pub fn is_full(&self) -> bool {
        self.members.len() >= MAX_SQUAD_SIZE
    }

    fn position(&self, player_id: i32) -> Option<usize> {
        self.members.iter().position(|m| m.player_id == player_id)
    }
}

/// The result of an accepted invite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JoinOutcome {
    pub squad_id: i32,
    /// `true` when this accept created the squad: the inviter is a new
    /// member too and gets the join sequence.
    pub created: bool,
}

/// A member leaving by any path (leave, kick, disconnect).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Departure {
    pub squad_id: i32,
    pub departed: SquadMember,
    /// The departed member's last known entity id (see
    /// [`SquadRegistry::note_entity`]), for the [39] the others get when
    /// the member has no live entity (in gate transit).
    pub departed_entity: Option<u32>,
    pub reason: OrgLeaveReason,
    /// The members left behind, in join order. When the squad disbanded
    /// this is the one member who was left alone.
    pub remaining: Vec<SquadMember>,
    /// Set when the leader left and a squad of two or more remains: the
    /// longest-standing remaining member (D-ORG12).
    pub new_leader: Option<i32>,
    /// A squad of one dissolves. Evaluated before promotion, so a leader
    /// leaving a squad of two promotes nobody.
    pub disbanded: bool,
}

/// Why a kick was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KickReject {
    /// The actor is not in the squad the id names.
    NotInThatSquad,
    NotLeader,
    /// Nobody in the squad has that name.
    TargetNotInSquad,
    /// The leader named themselves; leaving is CM 9.
    SelfKick,
}

impl KickReject {
    pub fn reason(self) -> &'static str {
        match self {
            KickReject::NotInThatSquad => "not_in_that_squad",
            KickReject::NotLeader => "not_leader",
            KickReject::TargetNotInSquad => "target_not_in_squad",
            KickReject::SelfKick => "self_kick",
        }
    }
}

/// Why a loot-mode change was refused (D-ORG16).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LootReject {
    NotInSquad,
    /// Not `RoundRobin` (0) or `FreeForAll` (1). Carries the squad and its
    /// current mode so the caller can re-send it.
    OutOfRange {
        squad_id: i32,
        current: SquadLootType,
    },
    NotLeader {
        squad_id: i32,
        current: SquadLootType,
    },
}

impl LootReject {
    pub fn reason(self) -> &'static str {
        match self {
            LootReject::NotInSquad => "not_in_squad",
            LootReject::OutOfRange { .. } => "out_of_range",
            LootReject::NotLeader { .. } => "not_leader",
        }
    }
}

/// Every squad on this cell, and the pending squad invites.
#[derive(Debug)]
pub struct SquadRegistry {
    pub(super) squads: HashMap<i32, Squad>,
    pub(super) member_of: HashMap<i32, i32>,
    /// Next squad id; `None` once the range is spent. Never reused.
    pub(super) next_squad_id: Option<i32>,
    /// Pending invites keyed by `(invitee player_id, request_id)`.
    pub(super) invites: HashMap<(i32, i32), PendingInvite>,
    /// Next request id; `None` once the cell's range is spent. Never
    /// reused.
    pub(super) next_request_id: Option<i32>,
    /// When each inviter issued their recent invites, for the rate limit.
    pub(super) sent: HashMap<i32, VecDeque<Instant>>,
    /// Players removed from a squad while no client could be told (in gate
    /// transit): `player_id -> (squad_id, reason)`. Delivered on their next
    /// world entry.
    pub(super) owed_left: HashMap<i32, (i32, OrgLeaveReason)>,
    /// Each member's last live entity id: `player_id -> entity_id`. Gate
    /// travel removes the cell entity until the arrival re-creates it with
    /// the same id, so a `DisconnectEntity` for a member in transit (an
    /// aborted transfer, a crash or timeout mid-transfer) finds no entity
    /// to read the `player_id` from. This map is how it still finds them.
    pub(super) entity_of: HashMap<i32, u32>,
    /// Invites that expired since the last `drain_expired`.
    pub(super) expired: Vec<super::invites::ExpiredInvite>,
}

impl Default for SquadRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SquadRegistry {
    pub fn new() -> Self {
        Self {
            squads: HashMap::new(),
            member_of: HashMap::new(),
            next_squad_id: Some(SQUAD_ORG_ID_MIN),
            invites: HashMap::new(),
            next_request_id: Some(1),
            sent: HashMap::new(),
            owed_left: HashMap::new(),
            entity_of: HashMap::new(),
            expired: Vec::new(),
        }
    }

    /// The squad `player_id` is in.
    pub fn squad_of(&self, player_id: i32) -> Option<i32> {
        self.member_of.get(&player_id).copied()
    }

    pub fn squad(&self, squad_id: i32) -> Option<&Squad> {
        self.squads.get(&squad_id)
    }

    /// The squad `player_id` is in.
    pub fn squad_for(&self, player_id: i32) -> Option<&Squad> {
        self.squad_of(player_id).and_then(|id| self.squads.get(&id))
    }

    pub fn squad_count(&self) -> usize {
        self.squads.len()
    }

    /// Record `entity_id` as member `player_id`'s live entity (on join and
    /// on every world entry). Ignored for a player in no squad.
    pub fn note_entity(&mut self, player_id: i32, entity_id: u32) {
        if self.member_of.contains_key(&player_id) {
            self.entity_of.insert(player_id, entity_id);
        }
    }

    /// The member whose last live entity was `entity_id`, if any.
    pub fn member_by_entity(&self, entity_id: u32) -> Option<i32> {
        self.entity_of
            .iter()
            .find_map(|(&pid, &eid)| (eid == entity_id).then_some(pid))
    }

    /// Apply a consumed, accepted invite after re-validating it (D-ORG06).
    ///
    /// `inviter` is the inviter's fresh snapshot, or `None` when the
    /// inviter is not online as a player right now. A squad is created on
    /// the first accept: an inviter with no squad at invite time who still
    /// has none founds one; one who has since come to lead a squad (another
    /// of their invites was accepted first) brings the invitee into it.
    pub fn accept(
        &mut self,
        invite: &PendingInvite,
        invitee: SquadMember,
        inviter: Option<SquadMember>,
    ) -> Result<JoinOutcome, ResponseReject> {
        if self.member_of.contains_key(&invitee.player_id) {
            return Err(ResponseReject::InviteeInSquad);
        }
        let inviter_pid = invite.inviter_player_id;
        let inviter_squad = self.squad_of(inviter_pid);
        let target = match (invite.squad_id, inviter_squad) {
            // Invited into an existing squad: it must still exist, and the
            // inviter must still be in it.
            (Some(sid), _) if !self.squads.contains_key(&sid) => {
                return Err(ResponseReject::SquadGone)
            }
            (Some(sid), Some(now_sid)) if sid == now_sid => Some(sid),
            (Some(_), _) => return Err(ResponseReject::InviterLeft),
            (None, Some(now_sid)) => Some(now_sid),
            (None, None) => None,
        };
        match target {
            Some(sid) => {
                let squad = self.squads.get_mut(&sid).expect("checked above");
                if squad.leader != inviter_pid {
                    return Err(ResponseReject::InviterNotLeader);
                }
                if squad.is_full() {
                    return Err(ResponseReject::SquadFull);
                }
                squad.members.push(invitee.clone());
                self.member_of.insert(invitee.player_id, sid);
                Ok(JoinOutcome {
                    squad_id: sid,
                    created: false,
                })
            }
            None => {
                let Some(inviter) = inviter.filter(|m| m.player_id == inviter_pid) else {
                    return Err(ResponseReject::InviterOffline);
                };
                let Some(sid) = self.next_squad_id else {
                    return Err(ResponseReject::SquadIdsExhausted);
                };
                // The range ends where `checked_add` overflows.
                const _: () = assert!(SQUAD_ORG_ID_MAX == i32::MAX);
                self.next_squad_id = sid.checked_add(1);
                self.member_of.insert(inviter.player_id, sid);
                self.member_of.insert(invitee.player_id, sid);
                self.squads.insert(
                    sid,
                    Squad {
                        id: sid,
                        members: vec![inviter, invitee],
                        leader: inviter_pid,
                        loot: SquadLootType::default(),
                    },
                );
                Ok(JoinOutcome {
                    squad_id: sid,
                    created: true,
                })
            }
        }
    }

    /// `player_id` leaves their squad of their own accord (CM 9).
    pub fn leave(&mut self, player_id: i32) -> Option<Departure> {
        self.remove_member(player_id, OrgLeaveReason::Requested)
    }

    /// The leader of `squad_id` kicks the member called `target_name`.
    pub fn kick(
        &mut self,
        actor_player_id: i32,
        squad_id: i32,
        target_name: &str,
    ) -> Result<Departure, KickReject> {
        if self.squad_of(actor_player_id) != Some(squad_id) {
            return Err(KickReject::NotInThatSquad);
        }
        let squad = self.squads.get(&squad_id).expect("member_of is in step");
        if squad.leader != actor_player_id {
            return Err(KickReject::NotLeader);
        }
        let target = squad
            .members
            .iter()
            .find(|m| m.name == target_name)
            .map(|m| m.player_id)
            .ok_or(KickReject::TargetNotInSquad)?;
        if target == actor_player_id {
            return Err(KickReject::SelfKick);
        }
        Ok(self
            .remove_member(target, OrgLeaveReason::Kicked)
            .expect("target is a member"))
    }

    /// `player_id` went offline: remove them with `Logout`, and drop every
    /// invite they sent or hold, their rate window and anything owed to
    /// them.
    pub fn remove_player(&mut self, player_id: i32) -> Option<Departure> {
        self.invites.retain(|&(invitee, _), inv| {
            invitee != player_id && inv.inviter_player_id != player_id
        });
        self.sent.remove(&player_id);
        self.owed_left.remove(&player_id);
        self.remove_member(player_id, OrgLeaveReason::Logout)
    }

    fn remove_member(&mut self, player_id: i32, reason: OrgLeaveReason) -> Option<Departure> {
        let squad_id = self.member_of.remove(&player_id)?;
        let departed_entity = self.entity_of.remove(&player_id);
        let squad = self
            .squads
            .get_mut(&squad_id)
            .expect("member_of is in step");
        let pos = squad.position(player_id).expect("member_of is in step");
        let departed = squad.members.remove(pos);
        let remaining = squad.members.clone();
        // Disband first: a leader leaving a squad of two promotes nobody.
        if remaining.len() <= 1 {
            self.squads.remove(&squad_id);
            for m in &remaining {
                self.member_of.remove(&m.player_id);
                self.entity_of.remove(&m.player_id);
            }
            // Invites into a squad that no longer exists can never be
            // accepted; drop them now rather than at their expiry.
            self.invites.retain(|_, inv| inv.squad_id != Some(squad_id));
            return Some(Departure {
                squad_id,
                departed,
                departed_entity,
                reason,
                remaining,
                new_leader: None,
                disbanded: true,
            });
        }
        let new_leader = (squad.leader == player_id).then(|| {
            squad.leader = remaining[0].player_id;
            squad.leader
        });
        Some(Departure {
            squad_id,
            departed,
            departed_entity,
            reason,
            remaining,
            new_leader,
            disbanded: false,
        })
    }

    /// Set the loot mode of `player_id`'s squad (CM 18, D-ORG16): the
    /// leader only, and only `RoundRobin` (0) or `FreeForAll` (1). Returns
    /// the squad id.
    pub fn set_loot(&mut self, player_id: i32, raw: i32) -> Result<i32, LootReject> {
        let squad_id = self.squad_of(player_id).ok_or(LootReject::NotInSquad)?;
        let squad = self
            .squads
            .get_mut(&squad_id)
            .expect("member_of is in step");
        let current = squad.loot;
        let Ok(mode) = SquadLootType::try_from(raw) else {
            return Err(LootReject::OutOfRange { squad_id, current });
        };
        if squad.leader != player_id {
            return Err(LootReject::NotLeader { squad_id, current });
        }
        squad.loot = mode;
        Ok(squad_id)
    }

    /// Record that `player_id` left `squad_id` while their client could
    /// not be told (in gate transit), so their next world entry delivers
    /// `onOrganizationLeft`.
    pub fn owe_left(&mut self, player_id: i32, squad_id: i32, reason: OrgLeaveReason) {
        self.owed_left.insert(player_id, (squad_id, reason));
    }

    /// Take the `onOrganizationLeft` owed to `player_id`, if any.
    pub fn take_owed_left(&mut self, player_id: i32) -> Option<(i32, OrgLeaveReason)> {
        self.owed_left.remove(&player_id)
    }
}

#[cfg(test)]
impl SquadRegistry {
    /// Start both counters near their limits, to test exhaustion.
    pub(super) fn with_next_ids(next_squad_id: i32, next_request_id: i32) -> Self {
        Self {
            next_squad_id: Some(next_squad_id),
            next_request_id: Some(next_request_id),
            ..Self::new()
        }
    }
}
