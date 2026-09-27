//! `organizationMOTD` (CM 13), `organizationNote` (CM 14) and
//! `organizationOfficerNote` (CM 15) for a Team or Command id (ORG-08,
//! CAT-M-10).
//!
//! The text is checked against D-ORG10 / D-ORG23 first (rejected, never
//! truncated). Then, under ORG-LOCK (D-ORG04) and the organization's order
//! guard ([`super::order`]):
//!
//! 1. the actor is a member (`not_member`) whose rank holds the method's
//!    bit: `MOTD`, `RosterNotes` or `OfficerNotes` (`missing_permission`).
//!    `RosterNotes` is not in any editor's set, so in practice every rank
//!    but `Initiate` may write its own note (D-ORG08 defaults);
//! 2. an officer note's target is found by name among **that
//!    organization's** member rows only (`target_not_member`,
//!    `target_ambiguous`), is not the actor (`self_target`), and ranks
//!    strictly below the actor (`rank_too_low`, D-ORG09 (2));
//! 3. text equal to what is stored is `ok` / `unchanged`: no write and no
//!    fanout, but the actor still gets the line.
//!
//! After the commit: [45] to every online member, [46] (the actor's
//! name and note) to every online member, or [47] only to the members
//! whose rank held `OfficerNotes` under the lock
//! ([`super::officer_notes::holders_locked`]). Every outcome ends in one
//! `org.set_text` row ([`EditRow`]) and a feedback line.

use cimmeria_entity::organization::{org_text, OrgPermission, TextField};
use cimmeria_wire::cell::client_methods::organization::{
    build_on_organization_motd_update, build_on_organization_note_update,
    build_on_organization_officer_note_update, ON_ORGANIZATION_MOTD_UPDATE,
    ON_ORGANIZATION_NOTE_UPDATE, ON_ORGANIZATION_OFFICER_NOTE_UPDATE,
};
use sqlx::{Postgres, Transaction};

use super::broadcast::broadcast_to_org;
use super::edit_row::{units, EditRow};
use super::fanout::{feedback, online_members, send_to_members};
use super::officer_notes::holders_locked;
use super::order::org_order_guard;
use super::targets::member_by_name;
use super::telemetry::OrgReject;
use super::{OrgCtx, OrgPlayer};
use crate::base::organization::api::{lock_org, member_access_locked};
use crate::base::organization::persistence::{set_text, OrgStoreError, OrgTextTarget};

pub const MOTD_SAVED_TEXT: &str = "Message of the day updated.";
pub const NOTE_SAVED_TEXT: &str = "Your note is saved.";
const OFFICER_NOTE_SELF_TEXT: &str = "You cannot write an officer note on yourself.";

/// Which text a call edits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextEdit<'a> {
    /// CM 13.
    Motd,
    /// CM 14: the actor's own roster note.
    Note,
    /// CM 15: `target_name` as the client typed it.
    OfficerNote { target_name: &'a str },
}

impl TextEdit<'_> {
    fn field(self) -> TextField {
        match self {
            TextEdit::Motd => TextField::Motd,
            TextEdit::Note => TextField::Note,
            TextEdit::OfficerNote { .. } => TextField::OfficerNote,
        }
    }

    fn required(self) -> OrgPermission {
        match self {
            TextEdit::Motd => OrgPermission::MOTD,
            TextEdit::Note => OrgPermission::ROSTER_NOTES,
            TextEdit::OfficerNote { .. } => OrgPermission::OFFICER_NOTES,
        }
    }
}

/// What the locked part decided.
struct Decided {
    changed: bool,
    /// The member the text is about (the actor for a note), as stored.
    member_name: String,
    /// For an officer note: the members allowed to read it.
    holders: Vec<i32>,
}

/// Handle CM 13, 14 or 15 for a Team or Command id. `Ok(true)` when the
/// stored text changed, `Ok(false)` when it already held that text.
#[tracing::instrument(
    name = "org.set_text",
    level = "info",
    skip_all,
    fields(player_id = player.player_id, entity_id = player.entity_id, org_id, field = edit.field().name())
)]
pub async fn handle_set_text(
    ctx: &OrgCtx<'_>,
    player: &OrgPlayer,
    org_id: i32,
    edit: TextEdit<'_>,
    text: &str,
) -> Result<bool, OrgReject> {
    let mut row = EditRow {
        event: "org.set_text",
        action: "set_text",
        account_id: player.account_id,
        player_id: Some(player.player_id),
        entity_id: Some(player.entity_id),
        org_id: Some(org_id),
        field: Some(edit.field().name()),
        to_units: Some(units(text)),
        ..EditRow::default()
    };
    let text = match org_text::validate(edit.field(), text) {
        Ok(t) => t,
        Err(r) => {
            return row
                .refuse(ctx, OrgReject::InvalidText(r), OFFICER_NOTE_SELF_TEXT)
                .await
        }
    };
    row.to_units = Some(units(&text));
    let _order = org_order_guard(org_id).await;
    let decided = match ctx.db_pool.as_deref() {
        None => Err(OrgReject::NoDb),
        Some(pool) => match pool.begin().await {
            Ok(mut tx) => match text_locked(&mut tx, player, org_id, edit, &text, &mut row).await {
                Ok(d) => tx.commit().await.map(|()| d).map_err(|e| row.db_failed(&e)),
                Err(why) => Err(why),
            },
            Err(e) => Err(row.db_failed(&e)),
        },
    };
    let d = match decided {
        Ok(d) => d,
        Err(why) => return row.refuse(ctx, why, OFFICER_NOTE_SELF_TEXT).await,
    };
    if d.changed {
        match edit {
            TextEdit::Motd => {
                let args = build_on_organization_motd_update(org_id, &text);
                broadcast_to_org(ctx, org_id, ON_ORGANIZATION_MOTD_UPDATE, &args, None).await;
            }
            TextEdit::Note => {
                let args = build_on_organization_note_update(org_id, &d.member_name, &text);
                broadcast_to_org(ctx, org_id, ON_ORGANIZATION_NOTE_UPDATE, &args, None).await;
            }
            TextEdit::OfficerNote { .. } => {
                let args = build_on_organization_officer_note_update(org_id, &d.member_name, &text);
                let online = online_members(ctx, &d.holders);
                let sent = send_to_members(
                    ctx,
                    org_id,
                    &online,
                    &[(ON_ORGANIZATION_OFFICER_NOTE_UPDATE, args)],
                    "officer_note",
                )
                .await;
                tracing::debug!(
                    target: "org",
                    event = "org.broadcast",
                    what = "officer_note",
                    org_id,
                    method_index = ON_ORGANIZATION_OFFICER_NOTE_UPDATE,
                    required = OrgPermission::OFFICER_NOTES.bits(),
                    holders = d.holders.len(),
                    online_members = online.len(),
                    recipients = sent,
                    "officer note sent to the members who may read it"
                );
            }
        }
    }
    let line = match edit {
        TextEdit::Motd => MOTD_SAVED_TEXT.to_string(),
        TextEdit::Note => NOTE_SAVED_TEXT.to_string(),
        TextEdit::OfficerNote { .. } => format!("Officer note on {} saved.", d.member_name),
    };
    feedback(ctx, player.entity_id, &line).await;
    row.ok(if d.changed { "changed" } else { "unchanged" });
    Ok(d.changed)
}

/// The locked part: membership, the bit, the target, then the write.
async fn text_locked(
    tx: &mut Transaction<'_, Postgres>,
    player: &OrgPlayer,
    org_id: i32,
    edit: TextEdit<'_>,
    text: &str,
    row: &mut EditRow,
) -> Result<Decided, OrgReject> {
    let header = lock_org(tx, org_id)
        .await
        .map_err(|e| row.db_failed(&e))?
        .ok_or(OrgReject::NotMember)?;
    row.org_type = Some(header.org_type.name());
    let access = member_access_locked(tx, org_id, player.player_id)
        .await
        .map_err(|e| row.db_failed(&e))?
        .ok_or(OrgReject::NotMember)?;
    row.actor_rank = Some(access.rank().as_u8());
    if !access.permissions().contains(edit.required()) {
        return Err(OrgReject::MissingPermission);
    }
    let (target, member_player, old) = match edit {
        TextEdit::Motd => (OrgTextTarget::Motd, None, header.motd.clone()),
        TextEdit::Note => {
            let (name, old) = member_texts(tx, org_id, player.player_id, row).await?;
            (
                OrgTextTarget::Note {
                    player_id: player.player_id,
                },
                Some(name),
                old.0,
            )
        }
        TextEdit::OfficerNote { target_name } => {
            let t = member_by_name(tx, org_id, target_name)
                .await
                .map_err(|e| row.db_failed(&e))??;
            row.target_player_id = Some(t.player_id);
            row.target_account_id = u32::try_from(t.account_id).ok();
            row.target_rank = Some(t.rank.as_u8());
            if t.player_id == player.player_id {
                return Err(OrgReject::SelfTarget);
            }
            if access.rank() <= t.rank {
                return Err(OrgReject::RankTooLow);
            }
            let (_, old) = member_texts(tx, org_id, t.player_id, row).await?;
            (
                OrgTextTarget::OfficerNote {
                    player_id: t.player_id,
                },
                Some(t.name),
                old.1,
            )
        }
    };
    row.from_units = Some(units(&old));
    let changed = old != text;
    if changed {
        match set_text(tx, &access, org_id, target, text).await {
            Ok(_) => {}
            Err(OrgStoreError::InvalidText(r)) => return Err(OrgReject::InvalidText(r)),
            Err(OrgStoreError::NotAMember) => return Err(OrgReject::TargetNotMember),
            Err(e) => return Err(row.db_failed(&e)),
        }
    }
    let holders = match edit {
        TextEdit::OfficerNote { .. } => holders_locked(tx, org_id)
            .await
            .map_err(|e| row.db_failed(&e))?,
        _ => Vec::new(),
    };
    Ok(Decided {
        changed,
        member_name: member_player.unwrap_or_default(),
        holders,
    })
}

/// A member's stored name and `(note, officer_note)`, under the lock.
async fn member_texts(
    tx: &mut Transaction<'_, Postgres>,
    org_id: i32,
    player_id: i32,
    row: &EditRow,
) -> Result<(String, (String, String)), OrgReject> {
    let found: Option<(String, String, String)> = sqlx::query_as(
        "SELECT p.player_name, m.note, m.officer_note \
         FROM sgw_organization_members m \
         JOIN sgw_player p ON p.player_id = m.player_id \
         WHERE m.org_id = $1 AND m.player_id = $2",
    )
    .bind(org_id)
    .bind(player_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| row.db_failed(&e))?;
    let (name, note, officer_note) = found.ok_or(OrgReject::TargetNotMember)?;
    Ok((name, (note, officer_note)))
}
