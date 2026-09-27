//! The lines a squad refusal or confirmation shows the player.
//!
//! Every refusal is `onErrorCode` plus one of these lines on the feedback
//! channel; the line is what the player reads, because the client has no
//! text for an organization error code (ORG-E1 Q4). The wording is project
//! policy: the client ships no squad error strings (audit A-14).

use crate::cell::squad::{InviteReject, KickReject, LootReject, ResponseReject};

pub(super) fn invite_rejected(reject: InviteReject, target: &str) -> String {
    match reject {
        InviteReject::TargetInSquad => format!("{target} is already in a squad."),
        InviteReject::InviterNotLeader => "Only the squad leader can invite players.".into(),
        InviteReject::SquadFull => "Your squad is full.".into(),
        InviteReject::DuplicatePending => {
            format!("{target} already has an invitation from you.")
        }
        InviteReject::InviteeInboxFull => {
            format!("{target} has too many invitations pending. Try again later.")
        }
        InviteReject::RateLimited => {
            "You are sending invitations too quickly. Wait a moment.".into()
        }
        InviteReject::RequestIdsExhausted => "Squad invitations are unavailable.".into(),
    }
}

pub(super) fn response_rejected(reject: ResponseReject, inviter: &str) -> String {
    match reject {
        ResponseReject::InviteeInSquad => "You are already in a squad.".into(),
        ResponseReject::SquadGone => "That squad no longer exists.".into(),
        ResponseReject::InviterLeft | ResponseReject::InviterNotLeader => {
            format!("{inviter} can no longer invite you to that squad.")
        }
        ResponseReject::SquadFull => "That squad is full.".into(),
        ResponseReject::InviterOffline => format!("{inviter} is no longer online."),
        ResponseReject::SquadIdsExhausted => "Squads are unavailable.".into(),
    }
}

pub(super) fn kick_rejected(reject: KickReject, target: &str) -> String {
    match reject {
        KickReject::NotInThatSquad => "You are not in that squad.".into(),
        KickReject::NotLeader => "Only the squad leader can remove members.".into(),
        KickReject::TargetNotInSquad => format!("{target} is not in your squad."),
        KickReject::SelfKick => "Leave the squad instead of removing yourself.".into(),
    }
}

pub(super) fn loot_rejected(reject: LootReject) -> &'static str {
    match reject {
        LootReject::NotInSquad => "You are not in a squad.",
        LootReject::OutOfRange { .. } => "That loot mode does not exist.",
        LootReject::NotLeader { .. } => "Only the squad leader can change the loot mode.",
    }
}

pub(super) const INVITE_INVALID: &str = "That invitation is no longer valid.";
pub(super) const INVITE_EXPIRED: &str = "That invitation has expired.";
pub(super) const NOT_IN_THAT_SQUAD: &str = "You are not in that squad.";
pub(super) const INVITE_SELF: &str = "You cannot invite yourself.";
pub(super) const NOT_READY: &str = "Squads are not available until you have entered the world.";

pub(super) fn target_not_found(target: &str) -> String {
    format!("No player named {target} is online.")
}

pub(super) fn target_travelling(target: &str) -> String {
    format!("{target} is travelling. Try again in a moment.")
}

pub(super) fn target_ambiguous(target: &str) -> String {
    format!("More than one player is called {target}; the invitation was not sent.")
}

pub(super) fn invite_sent(target: &str) -> String {
    format!("You invited {target} to your squad.")
}

pub(super) fn invite_declined(invitee: &str) -> String {
    format!("{invitee} declined your squad invitation.")
}
