//! DB persistence for persistent organizations (Team / Command).
//!
//! Tables:
//! - `sgw_organizations`        — org header (id, type, name, leader, motd, cash)
//! - `sgw_organization_ranks`   — 9 rank rows per org (rank_value 0–8, perms)
//! - `sgw_organization_members` — one row per member (org_id, player_id, rank_value)
//!
//! **All functions** take a `&PgPool` and return `Result<_, sqlx::Error>`.
//! Uses non-macro `sqlx::query_as::<_, T>` throughout — the `query_as!` bang macro
//! requires a live DB at compile time, which the CI offline build doesn't have.
//!
//! ## Default permission ladder (design decision, confirmed choice)
//!
//! When a new org is created all 9 rank rows are seeded with the following masks:
//!
//! | Rank | Value | Permission mask |
//! |------|-------|-----------------|
//! | None | 0 | 0 (no perms — sentinel / unranked placeholder) |
//! | Initiate | 1 | 0 (can read, no actions) |
//! | Member | 2 | ROSTER_NOTES |
//! | SeniorMember | 3 | ROSTER_NOTES |
//! | Veteran | 4 | ROSTER_NOTES + OFFICER_CHAT |
//! | SeniorVeteran | 5 | ROSTER_NOTES + OFFICER_CHAT |
//! | Officer | 6 | INVITE + EJECT + ROSTER_NOTES + OFFICER_NOTES + OFFICER_CHAT + MOTD |
//! | SeniorOfficer | 7 | all Officer bits + PROMOTE + DEMOTE + RANK_NAMES |
//! | Leader | 8 | ALL (0x3FFFFFF) |
//!
//! Rationale: mirrors a canonical guild ladder from live MMO precedent and the
//! EORG_PERM_* documentation. Officers get operational authority (invite/kick/motd)
//! but cannot alter rank structure without the PROMOTE/DEMOTE/RANK_NAMES bits,
//! which are reserved for SeniorOfficer and Leader. The full ALTER_PERMS /
//! TRANSFER_LEADER bits remain Leader-only in the ALL mask.

use cimmeria_game::social::guilds::{
    Org, OrgMember, OrgPermission, OrgRank, OrgRankConfig, OrgType,
};
use sqlx::PgPool;
use std::collections::HashMap;

// ── Default permission ladder ────────────────────────────────────────────────

/// Default permission mask for each rank (index = rank_value 0–8).
///
/// Canonical definition lives in `cimmeria_game::social::guilds::DEFAULT_RANK_PERMISSIONS`.
/// This alias is kept as `pub` so existing callers of `persistence::DEFAULT_PERMISSION_LADDER`
/// continue to compile without changes.
pub use cimmeria_game::social::guilds::DEFAULT_RANK_PERMISSIONS as DEFAULT_PERMISSION_LADDER;

// ── Startup load ─────────────────────────────────────────────────────────────

/// Load all persistent organizations (Team/Command) at startup.
///
/// Returns a `Vec<Org>` with fully-populated `rank_configs` and `members`.
/// Called once from `OrgAuthority::load_all`.
pub async fn load_all_orgs(pool: &PgPool) -> Result<Vec<Org>, sqlx::Error> {
    // 1. Load org headers.
    // (org_id, org_type, name, leader_player_id, motd, cash)
    let headers: Vec<(i32, i16, String, i32, Option<String>, i64)> =
        sqlx::query_as::<_, (i32, i16, String, i32, Option<String>, i64)>(
            "SELECT org_id, org_type, name, leader_player_id, motd, cash \
             FROM sgw_organizations ORDER BY org_id",
        )
        .fetch_all(pool)
        .await?;

    if headers.is_empty() {
        return Ok(Vec::new());
    }

    let org_ids: Vec<i32> = headers.iter().map(|h| h.0).collect();

    // 2. Load all rank rows for these orgs.
    // (org_id, rank_value, custom_name, permission_mask)
    let rank_rows: Vec<(i32, i16, Option<String>, i32)> =
        sqlx::query_as::<_, (i32, i16, Option<String>, i32)>(
            "SELECT org_id, rank_value, custom_name, permission_mask \
             FROM sgw_organization_ranks \
             WHERE org_id = ANY($1) ORDER BY org_id, rank_value",
        )
        .bind(&org_ids)
        .fetch_all(pool)
        .await?;

    // 3. Load all members for these orgs.
    // (org_id, player_id, rank_value, notes, character_name)
    // Join to sgw_player for the character name (display name for roster).
    let member_rows: Vec<(i32, i32, i16, Option<String>, String)> =
        sqlx::query_as::<_, (i32, i32, i16, Option<String>, String)>(
            "SELECT m.org_id, m.player_id, m.rank_value, m.notes, p.player_name \
             FROM sgw_organization_members m \
             JOIN sgw_player p USING (player_id) \
             WHERE m.org_id = ANY($1) ORDER BY m.org_id, m.player_id",
        )
        .bind(&org_ids)
        .fetch_all(pool)
        .await?;

    // 4. Assemble.
    let mut orgs = Vec::with_capacity(headers.len());
    let mut rank_iter = rank_rows.into_iter().peekable();
    let mut member_iter = member_rows.into_iter().peekable();

    for (org_id, org_type_raw, name, leader_player_id, motd, cash) in headers {
        let org_type = match org_type_raw {
            1 => OrgType::Team,
            2 => OrgType::Command,
            _ => {
                tracing::warn!(
                    org_id,
                    org_type_raw,
                    "load_all_orgs: unknown org_type, skipping"
                );
                continue;
            }
        };

        // Collect rank configs for this org.
        let mut rank_configs: HashMap<u8, OrgRankConfig> = HashMap::new();
        // Pre-seed defaults so missing rows don't leave gaps.
        for r in 0u8..=8u8 {
            rank_configs.insert(
                r,
                OrgRankConfig {
                    custom_name: None,
                    permissions: OrgPermission::from_bits(DEFAULT_PERMISSION_LADDER[r as usize]),
                },
            );
        }
        while rank_iter.peek().is_some_and(|row| row.0 == org_id) {
            let (_, rank_value, custom_name, permission_mask) = rank_iter.next().unwrap();
            let r = rank_value as u8;
            rank_configs.insert(
                r,
                OrgRankConfig {
                    custom_name,
                    permissions: OrgPermission::from_bits(permission_mask as u32),
                },
            );
        }

        // Collect members for this org.
        let mut members: HashMap<i32, OrgMember> = HashMap::new();
        while member_iter.peek().is_some_and(|row| row.0 == org_id) {
            let (_, player_id, rank_value, notes, character_name) = member_iter.next().unwrap();
            let rank = OrgRank::from_u8(rank_value as u8).unwrap_or(OrgRank::Initiate);
            members.insert(
                player_id,
                OrgMember {
                    player_id,
                    character_name,
                    rank,
                    notes,
                },
            );
        }

        orgs.push(Org {
            org_id,
            org_type,
            name,
            leader_player_id,
            motd,
            cash,
            members,
            rank_configs,
        });
    }

    Ok(orgs)
}

// ── Create org ───────────────────────────────────────────────────────────────

/// Create a new persistent organization with its 9 rank rows and the leader member.
///
/// Runs inside a single transaction to satisfy the composite FK constraint:
/// `organization_members(org_id, rank_value) → organization_ranks(org_id, rank_value)`.
/// The rank rows MUST be inserted before the leader member row.
///
/// Returns the server-assigned `org_id`.
pub async fn create_org(
    pool: &PgPool,
    org_type: OrgType,
    name: &str,
    leader_player_id: i32,
) -> Result<i32, sqlx::Error> {
    let org_type_db = org_type as i16;

    let mut tx = pool.begin().await?;

    // Insert the org header.
    let org_id: i32 = sqlx::query_scalar(
        "INSERT INTO sgw_organizations (org_type, name, leader_player_id) \
         VALUES ($1, $2, $3) RETURNING org_id",
    )
    .bind(org_type_db)
    .bind(name)
    .bind(leader_player_id)
    .fetch_one(&mut *tx)
    .await?;

    // Insert all 9 rank rows with the default permission ladder.
    for rank_value in 0i16..=8i16 {
        let perm = DEFAULT_PERMISSION_LADDER[rank_value as usize] as i32;
        sqlx::query(
            "INSERT INTO sgw_organization_ranks (org_id, rank_value, permission_mask) \
             VALUES ($1, $2, $3)",
        )
        .bind(org_id)
        .bind(rank_value)
        .bind(perm)
        .execute(&mut *tx)
        .await?;
    }

    // Insert the leader member row at rank 8 (Leader).
    sqlx::query(
        "INSERT INTO sgw_organization_members (org_id, player_id, rank_value) \
         VALUES ($1, $2, $3)",
    )
    .bind(org_id)
    .bind(leader_player_id)
    .bind(OrgRank::Leader as i16)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(org_id)
}

// ── Member mutations ─────────────────────────────────────────────────────────

/// Add a member to an org at Initiate rank.
///
/// Returns `Ok(true)` on insert, `Ok(false)` if the member already exists
/// (idempotent guard for retry safety).
pub async fn add_member(pool: &PgPool, org_id: i32, player_id: i32) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "INSERT INTO sgw_organization_members (org_id, player_id, rank_value) \
         VALUES ($1, $2, $3) ON CONFLICT (org_id, player_id) DO NOTHING",
    )
    .bind(org_id)
    .bind(player_id)
    .bind(OrgRank::Initiate as i16)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

/// Remove a member from an org. Returns `Ok(true)` if the row was found and deleted.
pub async fn remove_member(
    pool: &PgPool,
    org_id: i32,
    player_id: i32,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM sgw_organization_members \
         WHERE org_id = $1 AND player_id = $2",
    )
    .bind(org_id)
    .bind(player_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

/// Change a member's rank. Returns `Ok(true)` on success.
///
/// The `rank_value` must already exist in `sgw_organization_ranks` for this org
/// (the composite FK enforces this). Caller must validate the target rank exists
/// before calling (it will always for ranks 0–8 after `create_org`).
pub async fn set_member_rank(
    pool: &PgPool,
    org_id: i32,
    player_id: i32,
    rank: OrgRank,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE sgw_organization_members \
         SET rank_value = $1 \
         WHERE org_id = $2 AND player_id = $3",
    )
    .bind(rank as i16)
    .bind(org_id)
    .bind(player_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

// ── Rank config mutations ────────────────────────────────────────────────────

/// Update a rank's custom name and/or permission mask.
#[allow(dead_code)] // Phase 4+: rank-name/permission editing
pub async fn set_rank_config(
    pool: &PgPool,
    org_id: i32,
    rank: OrgRank,
    custom_name: Option<&str>,
    permissions: OrgPermission,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE sgw_organization_ranks \
         SET custom_name = $1, permission_mask = $2 \
         WHERE org_id = $3 AND rank_value = $4",
    )
    .bind(custom_name)
    .bind(permissions.bits() as i32)
    .bind(org_id)
    .bind(rank as i16)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

// ── Org header mutations ─────────────────────────────────────────────────────

/// Update the org MOTD. Returns `Ok(true)` if a row was updated.
pub async fn set_motd(pool: &PgPool, org_id: i32, motd: Option<&str>) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("UPDATE sgw_organizations SET motd = $1 WHERE org_id = $2")
        .bind(motd)
        .bind(org_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// Add `delta` to the org treasury (can be negative for withdrawals, but the
/// DB CHECK prevents the total going below 0). Returns the new cash total.
#[allow(dead_code)] // Phase 4+: treasury deposit/withdrawal
pub async fn add_cash(pool: &PgPool, org_id: i32, delta: i64) -> Result<i64, sqlx::Error> {
    let new_cash: i64 = sqlx::query_scalar(
        "UPDATE sgw_organizations \
         SET cash = cash + $1 \
         WHERE org_id = $2 \
         RETURNING cash",
    )
    .bind(delta)
    .bind(org_id)
    .fetch_one(pool)
    .await?;
    Ok(new_cash)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
