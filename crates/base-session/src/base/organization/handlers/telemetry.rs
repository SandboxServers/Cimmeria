//! The outcome row and counter every Team and Command action ends in.
//!
//! Work-packets § "Telemetry (owner rule, 2026-09-27)": each handled action
//! ends in exactly one INFO event on `org` with `event`, `outcome` (`ok` or
//! `rejected`) and, on a refusal, a closed snake_case `reason`, and counts
//! once on `org_actions_total{action, outcome, reason}` (`reason = none` on
//! `ok`). Identity fields are `Option`s from the session, never 0.

use cimmeria_entity::organization::TextReject;

/// Why a Team or Command action was refused: the closed `reason` set of the
/// actions in this module. Stable strings; they are metric labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrgReject {
    /// The actor is not a member of the organization the call names.
    NotMember,
    /// A Team or Command leader tried to leave while others remain
    /// (D-ORG12).
    LeaderCannotLeave,
    /// The disband would lose vault items or treasury cash (D-ORG20).
    VaultNotEmpty,
    /// No organization has that id (a GM disband of a stale id).
    NoSuchOrg,
    /// A GM command from a session below GameMaster (D-ORG13).
    NotGm,
    /// The forwarded actor is no longer that character's live session.
    ActorMismatch,
    /// The server has no database.
    NoDb,
    /// A database error; the transaction was rolled back.
    DbError,
    // ORG-07: invite, invite response, kick, rank change.
    /// The actor's rank lacks the bit the action needs (`Invite`, `Eject`,
    /// `Promote` or `Demote`; D-ORG09 (1)).
    MissingPermission,
    /// The actor's rank is not strictly above the target's current rank, or
    /// above the rank being assigned (D-ORG09 (2)).
    RankTooLow,
    /// The rank is not one the organization's type uses (D-ORG09 (5)).
    RankNotInType,
    /// A rank change named `Leader` (D-ORG09 (4)).
    LeaderNotAssignable,
    /// The actor named themself.
    SelfTarget,
    /// A rank change to the rank the member already holds.
    RankUnchanged,
    /// No online character has that name.
    TargetNotFound,
    /// More than one online character matches the name case-insensitively.
    TargetAmbiguous,
    /// The target is mid world entry (a login or gate travel).
    TargetInTransition,
    /// Nobody in that organization has that name.
    TargetNotMember,
    /// The target is already in an organization of that type (D-ORG18).
    AlreadyInOrgType,
    /// The target is already a member of that organization.
    AlreadyMember,
    /// The target ignores the inviter (SS-C1's Ignore list).
    Ignored,
    /// One pending invite per pair, or five held by the invitee.
    InviteLimit,
    /// The inviter sent five invites in the last 30 s.
    RateLimited,
    /// The base request-id range is spent.
    IdsExhausted,
    /// No invite under that id for this character (never issued, or
    /// already answered).
    InviteUnknown,
    /// The invite expired unanswered.
    InviteExpired,
    /// The id is another character's pending invite.
    InviteForeign,
    /// The organization the invite named no longer exists.
    OrgGone,
    /// The inviter is no longer a member of the organization.
    InviterLeft,
    /// The inviter's rank no longer holds `Invite`.
    InviterMissingPermission,
    /// `organizationInviteByType` named no Team or Command type.
    OrgTypeInvalid,
    /// `.org_rank` without an org id, for a character in a Team and a
    /// Command.
    OrgAmbiguous,
    // ORG-10: the GM suite.
    /// `.org_set_perms` on the `Leader` row, which always holds every bit
    /// (D-ORG08).
    LeaderRowPinned,
    /// `.org_set_perms` whose mask, after the D-ORG09 (6) clamp, changes
    /// nothing.
    PermissionsUnchanged,
    // ORG-08: MOTD, notes and the rank editor.
    /// A rank-permission or rank-name edit of the actor's own rank
    /// (D-ORG09 (3)).
    OwnRank,
    /// A rank-permission edit that changes a bit the editor does not hold
    /// (D-ORG09 (6), D-ORG22).
    ChangesUnheldBits,
    /// The text failed the D-ORG10 / D-ORG23 rules; `reason` is the text
    /// rule's own (`too_long`, `bidi_control`, `zero_width`, ...).
    InvalidText(TextReject),
}

impl OrgReject {
    pub fn reason(self) -> &'static str {
        match self {
            OrgReject::NotMember => "not_member",
            OrgReject::LeaderCannotLeave => "leader_cannot_leave",
            OrgReject::VaultNotEmpty => "vault_not_empty",
            OrgReject::NoSuchOrg => "no_such_org",
            OrgReject::NotGm => "not_gm",
            OrgReject::ActorMismatch => "actor_mismatch",
            OrgReject::NoDb => "no_db",
            OrgReject::DbError => "db_error",
            OrgReject::MissingPermission => "missing_permission",
            OrgReject::RankTooLow => "rank_too_low",
            OrgReject::RankNotInType => "rank_not_in_type",
            OrgReject::LeaderNotAssignable => "leader_not_assignable",
            OrgReject::SelfTarget => "self_target",
            OrgReject::RankUnchanged => "rank_unchanged",
            OrgReject::TargetNotFound => "target_not_found",
            OrgReject::TargetAmbiguous => "target_ambiguous",
            OrgReject::TargetInTransition => "target_in_transition",
            OrgReject::TargetNotMember => "target_not_member",
            OrgReject::AlreadyInOrgType => "already_in_org_type",
            OrgReject::AlreadyMember => "already_member",
            OrgReject::Ignored => "ignored",
            OrgReject::InviteLimit => "invite_limit",
            OrgReject::RateLimited => "rate_limited",
            OrgReject::IdsExhausted => "ids_exhausted",
            OrgReject::InviteUnknown => "invite_unknown",
            OrgReject::InviteExpired => "invite_expired",
            OrgReject::InviteForeign => "invite_foreign",
            OrgReject::OrgGone => "org_gone",
            OrgReject::InviterLeft => "inviter_left",
            OrgReject::InviterMissingPermission => "inviter_missing_permission",
            OrgReject::OrgTypeInvalid => "org_type_invalid",
            OrgReject::OrgAmbiguous => "org_ambiguous",
            OrgReject::LeaderRowPinned => "leader_row_pinned",
            OrgReject::PermissionsUnchanged => "permissions_unchanged",
            OrgReject::OwnRank => "own_rank",
            OrgReject::ChangesUnheldBits => "changes_unheld_bits",
            OrgReject::InvalidText(r) => r.reason(),
        }
    }
}

/// The correlators of one outcome row.
#[derive(Debug, Clone, Copy)]
pub(super) struct Row {
    /// `event`, e.g. `org.leave`.
    pub event: &'static str,
    /// The `action` label of `org_actions_total`, e.g. `leave`.
    pub action: &'static str,
    pub account_id: Option<u32>,
    pub player_id: Option<i32>,
    pub entity_id: Option<u32>,
    pub org_id: i32,
    /// `None` when the organization was never read (a refusal before the
    /// lock).
    pub org_type: Option<&'static str>,
}

impl Row {
    /// The `ok` row. `after` names what the action did (`left`,
    /// `disbanded`, ...).
    pub(super) fn ok(self, after: &'static str) {
        tracing::info!(
            target: "org",
            event = self.event,
            outcome = "ok",
            after,
            account_id = self.account_id,
            player_id = self.player_id,
            entity_id = self.entity_id,
            org_id = self.org_id,
            org_type = self.org_type,
            "organization action succeeded"
        );
        count(self.action, "ok", "none");
    }

    /// The `rejected` row.
    pub(super) fn rejected(self, why: OrgReject) {
        tracing::info!(
            target: "org",
            event = self.event,
            outcome = "rejected",
            reason = why.reason(),
            account_id = self.account_id,
            player_id = self.player_id,
            entity_id = self.entity_id,
            org_id = self.org_id,
            org_type = self.org_type,
            "organization action rejected"
        );
        count(self.action, "rejected", why.reason());
    }
}

/// One `org_actions_total` increment. Labels are enumerated only, never an
/// id.
pub(super) fn count(action: &'static str, outcome: &'static str, reason: &'static str) {
    cimmeria_observability::counter!(
        "org_actions_total",
        "action" => action,
        "outcome" => outcome,
        "reason" => reason,
    );
}

/// The outcome row of an action that involves a second player (ORG-07:
/// invite, invite response, kick, rank change, and the GM `.org_join` and
/// `.org_rank`): [`Row`]'s fields plus the target's identity, the request
/// id and the ranks D-ORG09 compared. A field the action never read stays
/// `None` and is logged empty; nothing is made up.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct ActionRow {
    pub event: &'static str,
    pub action: &'static str,
    pub account_id: Option<u32>,
    pub player_id: Option<i32>,
    pub entity_id: Option<u32>,
    pub org_id: Option<i32>,
    pub org_type: Option<&'static str>,
    pub target_account_id: Option<u32>,
    pub target_player_id: Option<i32>,
    pub request_id: Option<i32>,
    /// The actor's rank, read under the lock.
    pub actor_rank: Option<u8>,
    /// The target's rank before the action.
    pub target_rank: Option<u8>,
    /// The rank a rank change or a join assigns.
    pub to_rank: Option<u8>,
    /// The rank a permission edit names (`.org_set_perms`).
    pub rank: Option<u8>,
    /// How many rows a GM listing or re-push covered (`.org_info`,
    /// `.org_list`, `gmReloadOrganizations`).
    pub count: Option<usize>,
    /// A GM command whose own `event` is not `org.gm_action` (`.org_join`,
    /// `.org_rank`): the row is written twice, once under its event and
    /// once as the `org.gm_action` audit row (D-ORG13), so every GM
    /// organization command, refused or not, has exactly one
    /// `org.gm_action` row with the GM, the target and the result. Only the
    /// first counts on `org_actions_total`.
    pub gm_audit: bool,
}

impl ActionRow {
    /// The `ok` row. `after` names what the action did.
    pub(super) fn ok(&self, after: &'static str) {
        self.emit(self.event, "ok", None, Some(after));
        if self.gm_audit {
            self.emit(GM_ACTION_EVENT, "ok", None, Some(after));
        }
        count(self.action, "ok", "none");
    }

    /// The `rejected` row.
    pub(super) fn rejected(&self, why: OrgReject) {
        self.emit(self.event, "rejected", Some(why.reason()), None);
        if self.gm_audit {
            self.emit(GM_ACTION_EVENT, "rejected", Some(why.reason()), None);
        }
        count(self.action, "rejected", why.reason());
    }

    fn emit(
        &self,
        event: &'static str,
        outcome: &'static str,
        reason: Option<&'static str>,
        after: Option<&'static str>,
    ) {
        tracing::info!(
            target: "org",
            event,
            action = self.action,
            outcome,
            reason,
            after,
            account_id = self.account_id,
            player_id = self.player_id,
            entity_id = self.entity_id,
            org_id = self.org_id,
            org_type = self.org_type,
            target_account_id = self.target_account_id,
            target_player_id = self.target_player_id,
            request_id = self.request_id,
            actor_rank = self.actor_rank,
            target_rank = self.target_rank,
            to_rank = self.to_rank,
            rank = self.rank,
            count = self.count,
            "organization action {}",
            if outcome == "ok" { "succeeded" } else { "rejected" }
        );
    }
}

/// The `event` of the one audit row every GM organization command writes
/// (D-ORG13).
pub(super) const GM_ACTION_EVENT: &str = "org.gm_action";
