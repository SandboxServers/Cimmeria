//! The outcome row of a text or rank edit (ORG-08): `org.set_text`,
//! `org.set_rank_permissions` and `org.set_rank_name`.
//!
//! Same contract as [`super::telemetry::ActionRow`] (one INFO row per
//! action with `outcome` and, on a refusal, `reason`; one
//! `org_actions_total` increment), plus the edit's before and after values:
//! text lengths in UTF-16 units (never the text), and the rank masks. A
//! field the action never read stays `None` and is logged empty.

use super::answer::refusal_text;
use super::fanout::feedback;
use super::telemetry::{count, OrgReject};
use super::OrgCtx;

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct EditRow {
    pub event: &'static str,
    pub action: &'static str,
    pub account_id: Option<u32>,
    pub player_id: Option<i32>,
    pub entity_id: Option<u32>,
    pub org_id: Option<i32>,
    pub org_type: Option<&'static str>,
    /// The actor's rank, read under the lock.
    pub actor_rank: Option<u8>,
    /// `motd`, `note`, `officer_note` or `rank_name` (`TextField::name`).
    pub field: Option<&'static str>,
    pub from_units: Option<usize>,
    pub to_units: Option<usize>,
    /// The member an officer note is on.
    pub target_account_id: Option<u32>,
    pub target_player_id: Option<i32>,
    pub target_rank: Option<u8>,
    /// The rank a rank edit names, as the wire sent it.
    pub rank: Option<i32>,
    /// The rank's stored mask before and after, and the client's mask.
    pub from_mask: Option<u32>,
    pub to_mask: Option<u32>,
    pub wire_mask: Option<i32>,
    /// The bits a refused permission edit would have changed without
    /// holding them (D-ORG09 (6)).
    pub unheld_mask: Option<u32>,
}

impl EditRow {
    /// The `ok` row. `after` is `changed` or `unchanged`.
    pub(super) fn ok(&self, after: &'static str) {
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
            actor_rank = self.actor_rank,
            field = self.field,
            from_units = self.from_units,
            to_units = self.to_units,
            target_account_id = self.target_account_id,
            target_player_id = self.target_player_id,
            target_rank = self.target_rank,
            rank = self.rank,
            from_mask = self.from_mask,
            to_mask = self.to_mask,
            wire_mask = self.wire_mask,
            "organization edit succeeded"
        );
        count(self.action, "ok", "none");
    }

    /// The `rejected` row.
    pub(super) fn rejected(&self, why: OrgReject) {
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
            actor_rank = self.actor_rank,
            field = self.field,
            from_units = self.from_units,
            to_units = self.to_units,
            target_account_id = self.target_account_id,
            target_player_id = self.target_player_id,
            target_rank = self.target_rank,
            rank = self.rank,
            from_mask = self.from_mask,
            to_mask = self.to_mask,
            wire_mask = self.wire_mask,
            unheld_mask = self.unheld_mask,
            "organization edit rejected"
        );
        count(self.action, "rejected", why.reason());
    }

    /// The `rejected` row, then the refusal line to the actor (`self_text`
    /// is the action's wording for naming yourself).
    pub(super) async fn refuse<T>(
        &self,
        ctx: &OrgCtx<'_>,
        why: OrgReject,
        self_text: &str,
    ) -> Result<T, OrgReject> {
        self.rejected(why);
        if let Some(entity_id) = self.entity_id {
            feedback(ctx, entity_id, &refusal_text(why, self_text, None)).await;
        }
        Err(why)
    }

    /// A database failure: WARN `org.action_failed` with the `action`, and
    /// the `db_error` refusal for the caller to report.
    pub(super) fn db_failed(&self, e: &dyn std::fmt::Display) -> OrgReject {
        tracing::warn!(
            target: "org",
            event = "org.action_failed",
            action = self.action,
            account_id = self.account_id,
            player_id = self.player_id,
            org_id = self.org_id,
            reason = "db_error",
            error = %e,
            "organization edit failed in the database"
        );
        OrgReject::DbError
    }
}

/// A text's length as the client counts it.
pub(super) fn units(text: &str) -> usize {
    text.encode_utf16().count()
}
