//! Squad telemetry, on the `squad` target (organizations campaign telemetry
//! rule, `docs/architecture/observability.md` `org` / `squad` row).
//!
//! - Every action ends in exactly one INFO **outcome row**: `event` =
//!   `squad.<action>`, `outcome` = `ok` or `rejected`, a closed [`Reason`]
//!   on a refusal, the actor's `account_id` / `player_id`, the target's for
//!   the actions that have one (invite, invite response, kick), and
//!   `squad_id` / `request_id` where known. Each row also counts on
//!   `squad_actions_total{action, outcome, reason}`.
//! - Every state change is one DEBUG **transition** row (`squad_created`,
//!   `member_joined`, `member_left`, `leader_changed`, `loot_mode_changed`,
//!   `disbanded`, `invite_created`, `invite_consumed`, `invite_expired`).
//!
//! Identities are `Option`s from the cell's resolver: an unresolvable one
//! is omitted from the row, never written as 0.

use cimmeria_cell_world::cell::squad::SquadResources;
use cimmeria_entity::cell_entity::PlayerIdentity;
use cimmeria_entity::organization::{OrgLeaveReason, OrgRank, SquadLootType};

use crate::cell::space_manager::SpaceManager;
use crate::cell::squad::{
    count_action, Departure, ExpiredInvite, ForceJoinReject, InviteReject, KickReject, LootReject,
    PingReject, ResponseReject, TakeMiss,
};

/// A squad action: one span and one outcome row each.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Invite,
    InviteResponse,
    Leave,
    Kick,
    LootMode,
    /// CM 10 `BroadcastMinimapPing` (ORG-04).
    Ping,
}

impl Action {
    /// The `action` counter label.
    fn label(self) -> &'static str {
        match self {
            Action::Invite => "invite",
            Action::InviteResponse => "invite_response",
            Action::Leave => "leave",
            Action::Kick => "kick",
            Action::LootMode => "loot_mode",
            Action::Ping => "ping",
        }
    }

    /// The outcome row's `event`, the same as the span name.
    fn event(self) -> &'static str {
        match self {
            Action::Invite => "squad.invite",
            Action::InviteResponse => "squad.invite_response",
            Action::Leave => "squad.leave",
            Action::Kick => "squad.kick",
            Action::LootMode => "squad.loot_mode",
            Action::Ping => "squad.ping",
        }
    }
}

/// Why a squad action was refused. The closed set the catalog documents;
/// `ignored` (the target ignores the actor) is reserved: the cell has no
/// ignore list yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    TargetAmbiguous,
    TargetInTransition,
    TargetNotFound,
    SelfTarget,
    NotAPlayer,
    SquadFull,
    AlreadyInSquad,
    NotLeader,
    RateLimited,
    InviteLimit,
    InviteUnknown,
    InviteExpired,
    InviteForeign,
    LootModeInvalid,
    NotInSquad,
    WrongSquad,
    TargetNotInSquad,
    InviterLeft,
    InviterNotLeader,
    InviterOffline,
    SquadGone,
    IdsExhausted,
    NotReady,
    ActorMismatch,
}

impl Reason {
    pub fn as_str(self) -> &'static str {
        match self {
            Reason::TargetAmbiguous => "target_ambiguous",
            Reason::TargetInTransition => "target_in_transition",
            Reason::TargetNotFound => "target_not_found",
            Reason::SelfTarget => "self_target",
            Reason::NotAPlayer => "not_a_player",
            Reason::SquadFull => "squad_full",
            Reason::AlreadyInSquad => "already_in_squad",
            Reason::NotLeader => "not_leader",
            Reason::RateLimited => "rate_limited",
            Reason::InviteLimit => "invite_limit",
            Reason::InviteUnknown => "invite_unknown",
            Reason::InviteExpired => "invite_expired",
            Reason::InviteForeign => "invite_foreign",
            Reason::LootModeInvalid => "loot_mode_invalid",
            Reason::NotInSquad => "not_in_squad",
            Reason::WrongSquad => "wrong_squad",
            Reason::TargetNotInSquad => "target_not_in_squad",
            Reason::InviterLeft => "inviter_left",
            Reason::InviterNotLeader => "inviter_not_leader",
            Reason::InviterOffline => "inviter_offline",
            Reason::SquadGone => "squad_gone",
            Reason::IdsExhausted => "ids_exhausted",
            Reason::NotReady => "not_ready",
            Reason::ActorMismatch => "actor_mismatch",
        }
    }
}

impl From<InviteReject> for Reason {
    fn from(r: InviteReject) -> Self {
        match r {
            InviteReject::TargetInSquad => Reason::AlreadyInSquad,
            InviteReject::InviterNotLeader => Reason::NotLeader,
            InviteReject::SquadFull => Reason::SquadFull,
            InviteReject::DuplicatePending | InviteReject::InviteeInboxFull => Reason::InviteLimit,
            InviteReject::RateLimited => Reason::RateLimited,
            InviteReject::RequestIdsExhausted => Reason::IdsExhausted,
        }
    }
}

impl From<TakeMiss> for Reason {
    fn from(m: TakeMiss) -> Self {
        match m {
            TakeMiss::Unknown => Reason::InviteUnknown,
            TakeMiss::Expired => Reason::InviteExpired,
            TakeMiss::Foreign => Reason::InviteForeign,
        }
    }
}

impl From<ResponseReject> for Reason {
    fn from(r: ResponseReject) -> Self {
        match r {
            ResponseReject::InviteeInSquad => Reason::AlreadyInSquad,
            ResponseReject::SquadGone => Reason::SquadGone,
            ResponseReject::InviterLeft => Reason::InviterLeft,
            ResponseReject::InviterNotLeader => Reason::InviterNotLeader,
            ResponseReject::SquadFull => Reason::SquadFull,
            ResponseReject::InviterOffline => Reason::InviterOffline,
            ResponseReject::SquadIdsExhausted => Reason::IdsExhausted,
        }
    }
}

impl Reason {
    /// A kick refusal; `in_a_squad` splits "names a squad you are not in"
    /// into `wrong_squad` (you have another) and `not_in_squad` (none).
    pub fn from_kick(r: KickReject, in_a_squad: bool) -> Self {
        match r {
            KickReject::NotInThatSquad if in_a_squad => Reason::WrongSquad,
            KickReject::NotInThatSquad => Reason::NotInSquad,
            KickReject::NotLeader => Reason::NotLeader,
            KickReject::TargetNotInSquad => Reason::TargetNotInSquad,
            KickReject::SelfKick => Reason::SelfTarget,
        }
    }
}

impl From<PingReject> for Reason {
    fn from(r: PingReject) -> Self {
        match r {
            PingReject::NotInSquad => Reason::NotInSquad,
            PingReject::WrongSquad => Reason::WrongSquad,
            PingReject::RateLimited => Reason::RateLimited,
        }
    }
}

impl From<ForceJoinReject> for Reason {
    fn from(r: ForceJoinReject) -> Self {
        match r {
            ForceJoinReject::SelfTarget => Reason::SelfTarget,
            ForceJoinReject::AlreadyInSquad => Reason::AlreadyInSquad,
            ForceJoinReject::SquadFull => Reason::SquadFull,
            ForceJoinReject::SquadIdsExhausted => Reason::IdsExhausted,
        }
    }
}

impl From<LootReject> for Reason {
    fn from(r: LootReject) -> Self {
        match r {
            LootReject::NotInSquad => Reason::NotInSquad,
            LootReject::OutOfRange { .. } => Reason::LootModeInvalid,
            LootReject::NotLeader { .. } => Reason::NotLeader,
        }
    }
}

/// `EReasons` as a log label.
pub fn leave_reason(r: OrgLeaveReason) -> &'static str {
    match r {
        OrgLeaveReason::Requested => "requested",
        OrgLeaveReason::Kicked => "kicked",
        OrgLeaveReason::Disbanded => "disbanded",
        OrgLeaveReason::Logout => "logout",
    }
}

fn loot_label(l: SquadLootType) -> &'static str {
    match l {
        SquadLootType::RoundRobin => "round_robin",
        SquadLootType::FreeForAll => "free_for_all",
    }
}

/// The identity of the player behind `entity_id` (both halves `None` when
/// it does not resolve).
pub fn of_entity(space_mgr: &SpaceManager, entity_id: u32) -> PlayerIdentity {
    space_mgr.player_identity(entity_id)
}

/// The identity of character `player_id`: resolved through their live
/// entity for the account half, and the `player_id` itself even when they
/// are offline or in transit.
pub fn of_player(space_mgr: &SpaceManager, player_id: i32) -> PlayerIdentity {
    let live = space_mgr
        .player_entity_by_player_id(player_id)
        .map_or(PlayerIdentity::UNKNOWN, |eid| {
            space_mgr.player_identity(eid)
        });
    PlayerIdentity {
        player_id: Some(player_id),
        ..live
    }
}

/// The identity a base-forwarded call claims, before the cell confirms it:
/// the `player_id` from the base session, no account half.
pub fn claimed(player_id: i32) -> PlayerIdentity {
    PlayerIdentity::new(None, Some(player_id))
}

/// One outcome row, before it is emitted.
pub struct Outcome {
    pub action: Action,
    pub entity_id: u32,
    pub actor: PlayerIdentity,
    /// Only for actions with a second player: invite, response, kick.
    pub target: Option<PlayerIdentity>,
    pub squad_id: Option<i32>,
    pub request_id: Option<i32>,
    /// Clients the action reached, for the actions that fan out or
    /// deliberately do not (the ping always logs 0).
    pub recipients: Option<usize>,
}

impl Outcome {
    pub fn new(action: Action, entity_id: u32, actor: PlayerIdentity) -> Self {
        Self {
            action,
            entity_id,
            actor,
            target: None,
            squad_id: None,
            request_id: None,
            recipients: None,
        }
    }

    pub fn ok(self) {
        self.emit(None);
    }

    pub fn rejected(self, reason: Reason) {
        self.emit(Some(reason));
    }

    fn emit(self, reason: Option<Reason>) {
        let outcome = if reason.is_some() { "rejected" } else { "ok" };
        let reason_str = reason.map(Reason::as_str);
        let target = self.target.unwrap_or(PlayerIdentity::UNKNOWN);
        tracing::info!(
            target: "squad",
            event = self.action.event(),
            outcome,
            reason = reason_str,
            account_id = self.actor.account_id,
            account_name = self.actor.account_name,
            player_id = self.actor.player_id,
            player_name = self.actor.player_name,
            entity_id = self.entity_id,
            // The entity is the actor's own player entity, so its label is
            // the actor's character name.
            entity_name = self.actor.player_name,
            target_account_id = target.account_id,
            target_account_name = target.account_name,
            target_player_id = target.player_id,
            target_player_name = target.player_name,
            squad_id = self.squad_id, // nt:id-only squads have no name, only a runtime id
            request_id = self.request_id, // nt:id-only invite request token, nothing to name
            recipients = self.recipients,
            "squad action {}",
            outcome
        );
        count_action(self.action.label(), outcome, reason_str.unwrap_or("none"));
    }
}

pub fn squad_created(squad_id: i32, leader: PlayerIdentity) {
    tracing::debug!(
        target: "squad",
        event = "squad_created",
        squad_id, // nt:id-only squads have no name, only a runtime id
        account_id = leader.account_id,
        account_name = leader.account_name,
        player_id = leader.player_id,
        player_name = leader.player_name,
        "squad created"
    );
}

pub fn member_joined(squad_id: i32, who: PlayerIdentity, rank: OrgRank) {
    tracing::debug!(
        target: "squad",
        event = "member_joined",
        squad_id, // nt:id-only squads have no name, only a runtime id
        account_id = who.account_id,
        account_name = who.account_name,
        player_id = who.player_id,
        player_name = who.player_name,
        rank = rank.as_u8(),
        "squad member joined"
    );
}

/// `member_left`, `leader_changed` and `disbanded` for one departure.
/// `departed` is resolved by the caller before the teardown.
pub fn departure(space_mgr: &SpaceManager, d: &Departure, departed: PlayerIdentity) {
    let reason = leave_reason(d.reason);
    tracing::debug!(
        target: "squad",
        event = "member_left",
        squad_id = d.squad_id, // nt:id-only squads have no name, only a runtime id
        account_id = departed.account_id,
        account_name = departed.account_name,
        player_id = departed.player_id,
        player_name = departed.player_name,
        reason,
        remaining = d.remaining.len(),
        "squad member left"
    );
    if let Some(to) = d.new_leader {
        let target = of_player(space_mgr, to);
        tracing::debug!(
            target: "squad",
            event = "leader_changed",
            squad_id = d.squad_id, // nt:id-only squads have no name, only a runtime id
            account_id = departed.account_id,
            account_name = departed.account_name,
            player_id = departed.player_id,
            player_name = departed.player_name,
            target_account_id = target.account_id,
            target_account_name = target.account_name,
            target_player_id = target.player_id,
            target_player_name = target.player_name,
            from_player_id = d.departed.player_id,
            from_player_name = departed.player_name,
            to_player_id = to,
            // The roster carries the new leader's name even when they are
            // in transit with no live entity to resolve.
            to_player_name = target.player_name.or_else(|| {
                d.remaining
                    .iter()
                    .find(|m| m.player_id == to && !m.name.is_empty())
                    .map(|m| m.name.as_str())
            }),
            reason,
            "squad leader changed"
        );
    }
    if d.disbanded {
        tracing::debug!(
            target: "squad",
            event = "disbanded",
            squad_id = d.squad_id, // nt:id-only squads have no name, only a runtime id
            account_id = departed.account_id,
            account_name = departed.account_name,
            player_id = departed.player_id,
            player_name = departed.player_name,
            reason,
            "squad disbanded"
        );
    }
}

pub fn loot_mode_changed(
    squad_id: i32,
    actor: PlayerIdentity,
    from: SquadLootType,
    to: SquadLootType,
) {
    tracing::debug!(
        target: "squad",
        event = "loot_mode_changed",
        squad_id, // nt:id-only squads have no name, only a runtime id
        account_id = actor.account_id,
        account_name = actor.account_name,
        player_id = actor.player_id,
        player_name = actor.player_name,
        from = loot_label(from),
        to = loot_label(to),
        "squad loot mode changed"
    );
}

pub fn invite_created(
    request_id: i32,
    squad_id: Option<i32>,
    inviter: PlayerIdentity,
    invitee: PlayerIdentity,
) {
    tracing::debug!(
        target: "squad",
        event = "invite_created",
        request_id, // nt:id-only invite request token, nothing to name
        squad_id, // nt:id-only squads have no name, only a runtime id
        account_id = inviter.account_id,
        account_name = inviter.account_name,
        player_id = inviter.player_id,
        player_name = inviter.player_name,
        target_account_id = invitee.account_id,
        target_account_name = invitee.account_name,
        target_player_id = invitee.player_id,
        target_player_name = invitee.player_name,
        "squad invite created"
    );
}

pub fn invite_consumed(
    request_id: i32,
    squad_id: Option<i32>,
    accepted: bool,
    invitee: PlayerIdentity,
    inviter: PlayerIdentity,
) {
    tracing::debug!(
        target: "squad",
        event = "invite_consumed",
        request_id, // nt:id-only invite request token, nothing to name
        squad_id, // nt:id-only squads have no name, only a runtime id
        accepted,
        account_id = invitee.account_id,
        account_name = invitee.account_name,
        player_id = invitee.player_id,
        player_name = invitee.player_name,
        target_account_id = inviter.account_id,
        target_account_name = inviter.account_name,
        target_player_id = inviter.player_id,
        target_player_name = inviter.player_name,
        "squad invite consumed"
    );
}

/// One `invite_expired` row per invite that expired since the last drain.
pub fn invites_expired(space_mgr: &mut SpaceManager) {
    let expired: Vec<ExpiredInvite> = space_mgr.resources.squads_mut().drain_expired();
    for e in expired {
        let inviter = of_player(space_mgr, e.inviter_player_id);
        let invitee = of_player(space_mgr, e.invitee_player_id);
        tracing::debug!(
            target: "squad",
            event = "invite_expired",
            request_id = e.request_id, // nt:id-only invite request token, nothing to name
            squad_id = e.squad_id, // nt:id-only squads have no name, only a runtime id
            account_id = inviter.account_id,
            account_name = inviter.account_name,
            player_id = inviter.player_id,
            player_name = inviter.player_name,
            target_account_id = invitee.account_id,
            target_account_name = invitee.account_name,
            target_player_id = invitee.player_id,
            target_player_name = invitee.player_name,
            "squad invite expired unanswered"
        );
    }
}
