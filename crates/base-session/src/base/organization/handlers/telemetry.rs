//! The outcome row and counter every Team and Command action ends in.
//!
//! Work-packets § "Telemetry (owner rule, 2026-09-27)": each handled action
//! ends in exactly one INFO event on `org` with `event`, `outcome` (`ok` or
//! `rejected`) and, on a refusal, a closed snake_case `reason`, and counts
//! once on `org_actions_total{action, outcome, reason}` (`reason = none` on
//! `ok`). Identity fields are `Option`s from the session, never 0.

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
