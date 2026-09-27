//! Who may read officer notes, and keeping clients in step when that
//! changes (ORG-08, CAT-M-10).
//!
//! Officer notes reach only members whose rank holds `OfficerNotes`:
//!
//! - an officer-note edit (CM 15) sends [47] to the holders, a set read
//!   **inside** the edit's locked transaction ([`holders_locked`]), never
//!   from a post-commit read that a concurrent revoke could make stale;
//! - the login push blanks every officer note for a recipient whose rank
//!   lacks the bit ([`rank_reads_officer_notes`], fail closed);
//! - a change of who holds the bit, a rank-permission edit (CM 16) or a
//!   member moved between ranks (0xD2, `.org_rank`), sends the moved
//!   members one [47] per non-empty note: the text on a grant, an empty
//!   note on a revoke ([`NoteSync`]). It is computed under the lock and sent
//!   after the commit, under the organization's order guard
//!   ([`super::order`]), so it cannot overtake or be overtaken by another
//!   edit's fanout.

use cimmeria_entity::organization::{OrgPermission, OrgRank};
use cimmeria_wire::cell::client_methods::organization::{
    build_on_organization_officer_note_update, ON_ORGANIZATION_OFFICER_NOTE_UPDATE,
};
use sqlx::{Postgres, Transaction};

use super::fanout::{online_members, send_to_members};
use super::OrgCtx;
use crate::base::organization::persistence::{
    load_ranks, load_roster, OrgStoreError, RankRow, RosterMember,
};

/// Whether `rank`'s row in `ranks` holds `OfficerNotes`. A rank with no row
/// reads nothing (fail closed).
pub fn rank_reads_officer_notes(ranks: &[RankRow], rank: OrgRank) -> bool {
    ranks
        .iter()
        .find(|r| r.rank == rank)
        .is_some_and(|r| r.permissions.contains(OrgPermission::OFFICER_NOTES))
}

/// The members of `roster` whose rank holds `OfficerNotes`.
pub fn holders(ranks: &[RankRow], roster: &[RosterMember]) -> Vec<i32> {
    roster
        .iter()
        .filter(|m| rank_reads_officer_notes(ranks, m.rank))
        .map(|m| m.player_id)
        .collect()
}

/// [`holders`], read inside the caller's ORG-LOCK transaction.
pub(super) async fn holders_locked(
    tx: &mut Transaction<'_, Postgres>,
    org_id: i32,
) -> Result<Vec<i32>, OrgStoreError> {
    let ranks = load_ranks(&mut **tx, org_id).await?;
    let roster = load_roster(&mut **tx, org_id).await?;
    Ok(holders(&ranks, &roster))
}

/// Officer notes to show to, or hide from, members whose read access just
/// changed. Built under the lock, sent after the commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteSync {
    pub org_id: i32,
    /// The members whose access changed, by `player_id`.
    pub recipients: Vec<i32>,
    /// `true` on a grant (send the text), `false` on a revoke (send "").
    pub show: bool,
    /// Every non-empty officer note: `(member name, note)`.
    pub notes: Vec<(String, String)>,
}

impl NoteSync {
    /// A sync for `recipients`, or `None` when there is nothing to send.
    fn build(
        org_id: i32,
        recipients: Vec<i32>,
        show: bool,
        roster: &[RosterMember],
    ) -> Option<NoteSync> {
        let notes: Vec<(String, String)> = roster
            .iter()
            .filter(|m| !m.officer_note.is_empty())
            .map(|m| (m.name.clone(), m.officer_note.clone()))
            .collect();
        (!recipients.is_empty() && !notes.is_empty()).then_some(NoteSync {
            org_id,
            recipients,
            show,
            notes,
        })
    }

    /// The [47] calls one recipient gets.
    pub fn messages(&self) -> Vec<(u16, Vec<u8>)> {
        self.notes
            .iter()
            .map(|(name, note)| {
                let text = if self.show { note.as_str() } else { "" };
                (
                    ON_ORGANIZATION_OFFICER_NOTE_UPDATE,
                    build_on_organization_officer_note_update(self.org_id, name, text),
                )
            })
            .collect()
    }

    /// Send to the recipients online now (resolved by `player_id`). Each
    /// failed send is WARN `org.send_failed` (`what = officer_note_sync`).
    /// Logs DEBUG `org.officer_note_sync`. Returns how many were reached.
    pub async fn send(&self, ctx: &OrgCtx<'_>) -> usize {
        let online = online_members(ctx, &self.recipients);
        let sent = send_to_members(
            ctx,
            self.org_id,
            &online,
            &self.messages(),
            "officer_note_sync",
        )
        .await;
        tracing::debug!(
            target: "org",
            event = "org.officer_note_sync",
            org_id = self.org_id,
            show = self.show,
            notes = self.notes.len(),
            members = self.recipients.len(),
            online_members = online.len(),
            recipients = sent,
            "officer-note visibility synced"
        );
        sent
    }
}

/// After `rank`'s mask changed from `old` to `new` (under the caller's
/// lock): the members at that rank gain or lose officer notes.
pub(super) async fn sync_for_rank_locked(
    tx: &mut Transaction<'_, Postgres>,
    org_id: i32,
    rank: OrgRank,
    old: OrgPermission,
    new: OrgPermission,
) -> Result<Option<NoteSync>, OrgStoreError> {
    let bit = OrgPermission::OFFICER_NOTES;
    if old.contains(bit) == new.contains(bit) {
        return Ok(None);
    }
    let roster = load_roster(&mut **tx, org_id).await?;
    let recipients = roster
        .iter()
        .filter(|m| m.rank == rank)
        .map(|m| m.player_id)
        .collect();
    Ok(NoteSync::build(
        org_id,
        recipients,
        new.contains(bit),
        &roster,
    ))
}

/// After `player_id` moved from rank `from` to `to` (under the caller's
/// lock): they gain or lose officer notes if the two ranks differ in the
/// bit.
pub(super) async fn sync_for_member_locked(
    tx: &mut Transaction<'_, Postgres>,
    org_id: i32,
    player_id: i32,
    from: OrgRank,
    to: OrgRank,
) -> Result<Option<NoteSync>, OrgStoreError> {
    let ranks = load_ranks(&mut **tx, org_id).await?;
    let (was, now) = (
        rank_reads_officer_notes(&ranks, from),
        rank_reads_officer_notes(&ranks, to),
    );
    if was == now {
        return Ok(None);
    }
    let roster = load_roster(&mut **tx, org_id).await?;
    Ok(NoteSync::build(org_id, vec![player_id], now, &roster))
}
