//! Text and rank-permission writes: MOTD, notes, rank names and masks.

use cimmeria_entity::organization::{org_text, OrgPermission, OrgRank, TextField};
use sqlx::{Postgres, Transaction};

use super::super::api::permissions_from_db;
use super::super::api::OrgAccess;
use super::observe::{observed, units};
use super::{authorize, OrgStoreError};

/// Which organization text [`set_text`] writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrgTextTarget {
    /// The message of the day (CM 13).
    Motd,
    /// A member's own roster note (CM 14).
    Note { player_id: i32 },
    /// An officer note on a member (CM 15).
    OfficerNote { player_id: i32 },
    /// A rank's display name (CM 17).
    RankName { rank: OrgRank },
}

impl OrgTextTarget {
    fn field(self) -> TextField {
        match self {
            OrgTextTarget::Motd => TextField::Motd,
            OrgTextTarget::Note { .. } => TextField::Note,
            OrgTextTarget::OfficerNote { .. } => TextField::OfficerNote,
            OrgTextTarget::RankName { .. } => TextField::RankName,
        }
    }

    /// The member a note is on, for the log.
    fn player_id(self) -> Option<i32> {
        match self {
            OrgTextTarget::Note { player_id } | OrgTextTarget::OfficerNote { player_id } => {
                Some(player_id)
            }
            OrgTextTarget::Motd | OrgTextTarget::RankName { .. } => None,
        }
    }

    fn miss(self) -> OrgStoreError {
        match self {
            OrgTextTarget::Motd => OrgStoreError::NoSuchOrg,
            OrgTextTarget::Note { .. } | OrgTextTarget::OfficerNote { .. } => {
                OrgStoreError::NotAMember
            }
            OrgTextTarget::RankName { rank } => OrgStoreError::RankNotInType(rank),
        }
    }
}

/// Validate `text` (D-ORG10) and store it, returning the stored form (a
/// rank name is trimmed and its whitespace collapsed).
///
/// Refusals: [`OrgStoreError::InvalidText`], [`OrgStoreError::NoSuchOrg`],
/// [`OrgStoreError::NotAMember`] for a note on a non-member, and
/// [`OrgStoreError::RankNotInType`] for a rank the type does not use. The
/// permission checks (MOTD, RosterNotes, OfficerNotes, RankNames, and
/// D-ORG09 (3)) are the caller's, under the same lock.
pub async fn set_text(
    tx: &mut Transaction<'_, Postgres>,
    actor: &OrgAccess,
    org_id: i32,
    target: OrgTextTarget,
    text: &str,
) -> Result<String, OrgStoreError> {
    observed("set_text", Some(org_id), target.player_id(), async {
        let text = org_text::validate(target.field(), text)?;
        let header = authorize(tx, actor, org_id).await?;
        // Each UPDATE returns the text it replaced through a self-join on
        // the same row (RETURNING alone sees only the new values), so the
        // log can carry both lengths. `None` means no row matched.
        let old: Option<String> = match target {
            OrgTextTarget::Motd => {
                sqlx::query_scalar(
                    "UPDATE sgw_organizations n SET motd = $2 FROM sgw_organizations o \
                     WHERE n.org_id = $1 AND o.org_id = n.org_id RETURNING o.motd",
                )
                .bind(org_id)
                .bind(&text)
                .fetch_optional(&mut **tx)
                .await?
            }
            OrgTextTarget::Note { player_id } => {
                sqlx::query_scalar(
                    "UPDATE sgw_organization_members n SET note = $3 \
                     FROM sgw_organization_members o \
                     WHERE n.org_id = $1 AND n.player_id = $2 \
                       AND o.org_id = n.org_id AND o.player_id = n.player_id \
                     RETURNING o.note",
                )
                .bind(org_id)
                .bind(player_id)
                .bind(&text)
                .fetch_optional(&mut **tx)
                .await?
            }
            OrgTextTarget::OfficerNote { player_id } => {
                sqlx::query_scalar(
                    "UPDATE sgw_organization_members n SET officer_note = $3 \
                     FROM sgw_organization_members o \
                     WHERE n.org_id = $1 AND n.player_id = $2 \
                       AND o.org_id = n.org_id AND o.player_id = n.player_id \
                     RETURNING o.officer_note",
                )
                .bind(org_id)
                .bind(player_id)
                .bind(&text)
                .fetch_optional(&mut **tx)
                .await?
            }
            OrgTextTarget::RankName { rank } => {
                if !rank.is_valid_for(header.org_type) {
                    return Err(OrgStoreError::RankNotInType(rank));
                }
                sqlx::query_scalar(
                    "UPDATE sgw_organization_ranks n SET name = $3 \
                     FROM sgw_organization_ranks o \
                     WHERE n.org_id = $1 AND n.rank = $2 \
                       AND o.org_id = n.org_id AND o.rank = n.rank \
                     RETURNING COALESCE(o.name, '')",
                )
                .bind(org_id)
                .bind(i16::from(rank.as_u8()))
                .bind(&text)
                .fetch_optional(&mut **tx)
                .await?
            }
        };
        let old = old.ok_or_else(|| target.miss())?;
        let rank = match target {
            OrgTextTarget::RankName { rank } => Some(rank.as_u8()),
            _ => None,
        };
        tracing::debug!(
            target: "org",
            event = "set_text",
            org_id,
            player_id = target.player_id(),
            field = target.field().name(),
            rank,
            from_units = units(&old),
            to_units = units(&text),
            rows_affected = 1u64,
            "Organization text changed"
        );
        Ok(text)
    })
    .await
}

/// Store `permissions` as `rank`'s mask, returning the mask it replaced.
///
/// The `Leader` row always holds every bit and is refused
/// ([`OrgStoreError::LeaderPinned`], D-ORG08; the database `CHECK` pins it
/// too). A rank the type does not use is [`OrgStoreError::RankNotInType`].
/// Working out the new mask from the client's (D-ORG22,
/// `OrgPermission::apply_edit`) and whether the editor may make the change
/// (D-ORG09 (3), (6)) is the caller's, under the same lock.
pub async fn set_rank_permissions(
    tx: &mut Transaction<'_, Postgres>,
    actor: &OrgAccess,
    org_id: i32,
    rank: OrgRank,
    permissions: OrgPermission,
) -> Result<OrgPermission, OrgStoreError> {
    observed("set_rank_permissions", Some(org_id), None, async {
        let header = authorize(tx, actor, org_id).await?;
        if !rank.is_valid_for(header.org_type) {
            return Err(OrgStoreError::RankNotInType(rank));
        }
        if rank == OrgRank::LEADER {
            return Err(OrgStoreError::LeaderPinned);
        }
        // The old mask comes back through a self-join on the same row: an
        // UPDATE's RETURNING sees only the new values.
        let old: Option<i32> = sqlx::query_scalar(
            "UPDATE sgw_organization_ranks r SET permissions = $3 \
             FROM sgw_organization_ranks o \
             WHERE r.org_id = $1 AND r.rank = $2 AND o.org_id = r.org_id AND o.rank = r.rank \
             RETURNING o.permissions",
        )
        .bind(org_id)
        .bind(i16::from(rank.as_u8()))
        .bind(permissions.to_wire())
        .fetch_optional(&mut **tx)
        .await?;
        let old = old
            .map(permissions_from_db)
            .ok_or(OrgStoreError::RankNotInType(rank))?;
        tracing::debug!(
            target: "org",
            event = "set_rank_permissions",
            org_id,
            rank = rank.as_u8(),
            from_mask = old.bits(),
            to_mask = permissions.bits(),
            rows_affected = 1u64,
            "Organization rank permissions changed"
        );
        Ok(old)
    })
    .await
}
