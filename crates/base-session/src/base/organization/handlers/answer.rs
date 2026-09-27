//! What the player reads when an invite, response, kick or rank change is
//! refused (ORG-07), and the refusal itself: the outcome row, then one
//! feedback line, so no press is silent (work-packets § "Common
//! acceptance"). `onErrorCode` is not sent: the client has no organization
//! text for it (ORG-E1 Q4).
//!
//! The wording is project policy (audit A-14): the client ships no strings
//! for these refusals.

use cimmeria_entity::organization::OrgType;

use super::fanout::feedback;
use super::telemetry::{ActionRow, OrgReject};
use super::OrgCtx;

pub const NOT_MEMBER_TEXT: &str = "You are not a member of that organization.";
pub const NO_PERMISSION_TEXT: &str = "Your rank does not allow that.";
pub const RANK_TOO_LOW_TEXT: &str = "Your rank must be above theirs, and above the rank you give.";
pub const RANK_NOT_IN_TYPE_TEXT: &str = "That rank does not exist in this organization.";
pub const LEADER_NOT_ASSIGNABLE_TEXT: &str = "Leadership cannot be given by a rank change.";
pub const RANK_UNCHANGED_TEXT: &str = "They already hold that rank.";
pub const TARGET_NOT_FOUND_TEXT: &str = "No player by that name is online.";
pub const TARGET_AMBIGUOUS_TEXT: &str =
    "More than one player matches that name. Type the full name.";
pub const TARGET_IN_TRANSITION_TEXT: &str = "That player is travelling. Try again in a moment.";
pub const TARGET_NOT_MEMBER_TEXT: &str = "Nobody by that name is a member of that organization.";
pub const IGNORED_TEXT: &str = "That player is not accepting invitations from you.";
pub const INVITE_LIMIT_TEXT: &str =
    "That player already has an invitation from you, or too many invitations.";
pub const RATE_LIMITED_TEXT: &str = "You are sending invitations too quickly. Wait a moment.";
pub const INVITE_INVALID_TEXT: &str = "That invitation is no longer valid.";
pub const ORG_GONE_TEXT: &str = "That organization no longer exists.";
pub const ORG_TYPE_INVALID_TEXT: &str = "That is not an organization type.";
pub const ORG_UNAVAILABLE_TEXT: &str =
    "Organizations are unavailable right now. Please try again later.";

/// "Team", "Command" or "squad", as a player reads it; "organization"
/// when the type is not known.
pub fn type_title(org_type: Option<OrgType>) -> &'static str {
    org_type.map_or("organization", |t| match t {
        OrgType::Team => "Team",
        OrgType::Command => "Command",
        OrgType::Squad => "squad",
    })
}

/// The refusal line for `why`. `self_text` is the action's own wording for
/// naming yourself ("You cannot invite yourself.", ...); `org_type` names
/// the type in the one-per-type refusal.
pub fn refusal_text(why: OrgReject, self_text: &str, org_type: Option<OrgType>) -> String {
    let type_name = type_title(org_type);
    match why {
        OrgReject::NotMember => NOT_MEMBER_TEXT.into(),
        OrgReject::MissingPermission => NO_PERMISSION_TEXT.into(),
        OrgReject::RankTooLow => RANK_TOO_LOW_TEXT.into(),
        OrgReject::RankNotInType => RANK_NOT_IN_TYPE_TEXT.into(),
        OrgReject::LeaderNotAssignable => LEADER_NOT_ASSIGNABLE_TEXT.into(),
        OrgReject::RankUnchanged => RANK_UNCHANGED_TEXT.into(),
        OrgReject::SelfTarget => self_text.into(),
        OrgReject::TargetNotFound => TARGET_NOT_FOUND_TEXT.into(),
        OrgReject::TargetAmbiguous => TARGET_AMBIGUOUS_TEXT.into(),
        OrgReject::TargetInTransition => TARGET_IN_TRANSITION_TEXT.into(),
        OrgReject::TargetNotMember => TARGET_NOT_MEMBER_TEXT.into(),
        OrgReject::AlreadyInOrgType => format!("They are already in a {type_name}."),
        OrgReject::AlreadyMember => "They are already a member.".into(),
        OrgReject::Ignored => IGNORED_TEXT.into(),
        OrgReject::InviteLimit => INVITE_LIMIT_TEXT.into(),
        OrgReject::RateLimited => RATE_LIMITED_TEXT.into(),
        // Identical for all three, so a client cannot probe which request
        // ids are live in other players' inboxes.
        OrgReject::InviteUnknown | OrgReject::InviteExpired | OrgReject::InviteForeign => {
            INVITE_INVALID_TEXT.into()
        }
        OrgReject::InviterLeft | OrgReject::InviterMissingPermission => INVITE_INVALID_TEXT.into(),
        OrgReject::OrgGone | OrgReject::NoSuchOrg => ORG_GONE_TEXT.into(),
        OrgReject::OrgTypeInvalid => ORG_TYPE_INVALID_TEXT.into(),
        OrgReject::OrgAmbiguous => "They are in a Team and a Command; name the org id.".into(),
        OrgReject::LeaderCannotLeave
        | OrgReject::VaultNotEmpty
        | OrgReject::NotGm
        | OrgReject::ActorMismatch
        | OrgReject::IdsExhausted
        | OrgReject::NoDb
        | OrgReject::DbError => ORG_UNAVAILABLE_TEXT.into(),
    }
}

/// Log the `rejected` row for `why`, send `text` to the actor, and return
/// `Err(why)`.
pub(super) async fn refuse<T>(
    ctx: &OrgCtx<'_>,
    row: &ActionRow,
    entity_id: u32,
    why: OrgReject,
    text: &str,
) -> Result<T, OrgReject> {
    row.rejected(why);
    feedback(ctx, entity_id, text).await;
    Err(why)
}

/// A database failure in an ORG-07 action: WARN `org.action_failed` with
/// the `action` and the error, and the `db_error` refusal for the caller to
/// report.
pub(super) fn db_failed(row: &ActionRow, e: &dyn std::fmt::Display) -> OrgReject {
    tracing::warn!(
        target: "org",
        event = "org.action_failed",
        action = row.action,
        account_id = row.account_id,
        player_id = row.player_id,
        org_id = row.org_id,
        reason = "db_error",
        error = %e,
        "organization action failed in the database"
    );
    OrgReject::DbError
}

/// ORG-01's "not available yet" pair for a Team or Command call no packet
/// serves yet (CM 10 and 13-17 until ORG-08, CM 19 until the Bank's BV-08):
/// `onErrorCode(ERRORCODE_SYSTEM_Ability, instance_id,
/// CONDITION_FEEDBACK_InvalidEntity)`, then the feedback line the player
/// actually reads. The caller has resolved `entity_id` to the actor's live
/// session.
pub async fn not_available(ctx: &OrgCtx<'_>, entity_id: u32, instance_id: i32) {
    use cimmeria_wire::cell::client_methods::organization::ORG_NOT_AVAILABLE_TEXT;
    use cimmeria_wire::cell::client_methods::player::{
        build_on_error_code, CONDITION_FEEDBACK_INVALID_ENTITY, ERRORCODE_SYSTEM_ABILITY,
        ON_ERROR_CODE,
    };
    let args = build_on_error_code(
        ERRORCODE_SYSTEM_ABILITY,
        instance_id,
        CONDITION_FEEDBACK_INVALID_ENTITY,
    );
    if let Err(reason) =
        super::fanout::send_to_player(ctx, entity_id, &[(ON_ERROR_CODE, args)]).await
    {
        tracing::warn!(
            target: "org",
            event = "org.send_failed",
            what = "not_available",
            org_id = instance_id,
            entity_id,
            reason,
            "organization refusal could not be sent"
        );
    }
    feedback(ctx, entity_id, ORG_NOT_AVAILABLE_TEXT).await;
}
