//! Display reads: a player's memberships, a roster, the rank table, and
//! the name-availability check.
//!
//! None of these is an authorization read. They take any executor (the pool
//! for the login push, or a transaction) and never lock; anything that
//! decides whether an action is allowed uses
//! `api::member_access_locked` inside the mutation's transaction.

use cimmeria_entity::organization::{org_text, OrgPermission, OrgRank, OrgType, TextField};
use sqlx::PgExecutor;

use super::super::api::{org_type_from_db, permissions_from_db, rank_from_db, OrgHeader};
use super::observe::observed;
use super::OrgStoreError;

/// One organization a player belongs to, with the player's standing in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrgMembership {
    pub header: OrgHeader,
    pub rank: OrgRank,
    /// The rank's mask, for the client's display (the login push).
    /// **Display only:** read without the organization lock, so it may
    /// already be stale. Never authorize from it; use
    /// `api::member_access_locked` inside the mutation's transaction.
    pub display_permissions: OrgPermission,
}

/// One roster row: what `RosterInfo` (`onOrganizationRosterInfo` [38])
/// needs, plus the `player_id` the base keys online sessions by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RosterMember {
    pub player_id: i32,
    pub name: String,
    pub level: i32,
    pub archetype: i32,
    pub rank: OrgRank,
    pub note: String,
    pub officer_note: String,
}

/// One rank row: its custom name (`None` shows the client default) and
/// its permission mask.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankRow {
    pub rank: OrgRank,
    pub name: Option<String>,
    pub permissions: OrgPermission,
}

/// Every Team and Command `player_id` belongs to, Team first. A memberless
/// organization has no member row, so it never appears here, and the login
/// push skips it for free.
pub async fn load_memberships<'e>(
    executor: impl PgExecutor<'e>,
    player_id: i32,
) -> Result<Vec<OrgMembership>, OrgStoreError> {
    observed("load_memberships", None, Some(player_id), async {
        type Row = (i32, i16, String, String, i64, i64, i16, i32);
        let rows: Vec<Row> = sqlx::query_as(
        "SELECT o.org_id, o.org_type, o.name, o.motd, o.cash, o.experience, m.rank, r.permissions \
         FROM sgw_organization_members m \
         JOIN sgw_organizations o ON o.org_id = m.org_id \
         JOIN sgw_organization_ranks r ON r.org_id = m.org_id AND r.rank = m.rank \
         WHERE m.player_id = $1 \
         ORDER BY o.org_type",
    )
    .bind(player_id)
    .fetch_all(executor)
    .await?;
        rows.into_iter()
            .map(
                |(org_id, org_type, name, motd, cash, experience, rank, perms)| {
                    Ok(OrgMembership {
                        header: OrgHeader {
                            org_id,
                            org_type: org_type_from_db(org_type)?,
                            name,
                            motd,
                            cash,
                            experience,
                        },
                        rank: rank_from_db(rank)?,
                        display_permissions: permissions_from_db(perms),
                    })
                },
            )
            .collect::<Result<Vec<_>, OrgStoreError>>()
            .inspect(|ms| {
                tracing::debug!(
                    target: "org",
                    event = "load_memberships",
                    player_id,
                    rows_affected = ms.len(),
                    "Organization memberships loaded"
                )
            })
    })
    .await
}

/// The organization's members, highest rank first, then by name.
pub async fn load_roster<'e>(
    executor: impl PgExecutor<'e>,
    org_id: i32,
) -> Result<Vec<RosterMember>, OrgStoreError> {
    observed("load_roster", Some(org_id), None, async {
        type Row = (i32, String, i32, i32, i16, String, String);
        let rows: Vec<Row> = sqlx::query_as(
        "SELECT m.player_id, p.player_name, p.level, p.archetype, m.rank, m.note, m.officer_note \
         FROM sgw_organization_members m \
         JOIN sgw_player p ON p.player_id = m.player_id \
         WHERE m.org_id = $1 \
         ORDER BY m.rank DESC, p.player_name",
    )
    .bind(org_id)
    .fetch_all(executor)
    .await?;
        rows.into_iter()
            .map(
                |(player_id, name, level, archetype, rank, note, officer_note)| {
                    Ok(RosterMember {
                        player_id,
                        name,
                        level,
                        archetype,
                        rank: rank_from_db(rank)?,
                        note,
                        officer_note,
                    })
                },
            )
            .collect::<Result<Vec<_>, OrgStoreError>>()
            .inspect(|r| {
                tracing::debug!(
                    target: "org",
                    event = "load_roster",
                    org_id,
                    rows_affected = r.len(),
                    "Organization roster loaded"
                )
            })
    })
    .await
}

/// The organization's rank rows, lowest rank first.
pub async fn load_ranks<'e>(
    executor: impl PgExecutor<'e>,
    org_id: i32,
) -> Result<Vec<RankRow>, OrgStoreError> {
    observed("load_ranks", Some(org_id), None, async {
        let rows: Vec<(i16, Option<String>, i32)> = sqlx::query_as(
            "SELECT rank, name, permissions FROM sgw_organization_ranks \
         WHERE org_id = $1 ORDER BY rank",
        )
        .bind(org_id)
        .fetch_all(executor)
        .await?;
        rows.into_iter()
            .map(|(rank, name, perms)| {
                Ok(RankRow {
                    rank: rank_from_db(rank)?,
                    name,
                    permissions: permissions_from_db(perms),
                })
            })
            .collect::<Result<Vec<_>, OrgStoreError>>()
            .inspect(|r| {
                tracing::debug!(
                    target: "org",
                    event = "load_ranks",
                    org_id,
                    rows_affected = r.len(),
                    "Organization ranks loaded"
                )
            })
    })
    .await
}

/// `true` if no organization of `org_type` has `name`'s key (D-ORG10), so
/// a creation dialog can refuse early. Advisory only: the unique key on
/// `(org_type, name_key)` is what `create_org` actually relies on, since a
/// name can be taken between this read and the insert.
///
/// A name that fails the text rules is [`OrgStoreError::InvalidText`].
pub async fn name_available<'e>(
    executor: impl PgExecutor<'e>,
    org_type: OrgType,
    name: &str,
) -> Result<bool, OrgStoreError> {
    observed("name_available", None, None, async {
        if !org_type.is_persistent() {
            return Err(OrgStoreError::NotPersistent);
        }
        let name = org_text::validate(TextField::Name, name)?;
        let taken: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM sgw_organizations WHERE org_type = $1 AND name_key = $2)",
        )
        .bind(i16::from(org_type.as_u8()))
        .bind(org_text::name_key(&name))
        .fetch_one(executor)
        .await?;
        tracing::debug!(
            target: "org",
            event = "name_available",
            org_type = org_type.name(),
            name_units = name.encode_utf16().count(),
            available = !taken,
            "Organization name checked"
        );
        Ok(!taken)
    })
    .await
}
