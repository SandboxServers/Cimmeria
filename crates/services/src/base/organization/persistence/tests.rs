//! Live-DB tests for org persistence.
//!
//! Bug shape: persistence mutations (create_org, add_member, remove_member,
//! set_member_rank, set_motd) must round-trip through the DB correctly.
//! These tests guard against silent failures where the SQL runs but the
//! data doesn't match expectations (wrong rank FK, wrong permission mask,
//! orphan rows after remove).
//!
//! Run with:
//!   DATABASE_URL=postgres://w-testing:w-testing@localhost:5433/sgw \
//!   cargo nextest run --profile=ci-live-db -p cimmeria-services --lib
//!
//! All tests use a sentinel player_id range (sentinel values < 0 or near
//! i32::MAX) to avoid collisions.  Cleanup deletes by exact sentinel.
//!
//! NOTE: These tests require the sgw_player row to exist for the
//! leader_player_id used in create_org (FK constraint on org members).
//! We use a well-known test player from the seed data (player_id = -99999)
//! inserted here if absent, then cleaned up after.

use super::{
    add_member, create_org, load_all_orgs, remove_member, set_member_rank, set_motd,
    DEFAULT_PERMISSION_LADDER,
};
use crate::test_support::require_db_or_skip;
use cimmeria_game::social::guilds::{OrgPermission, OrgRank, OrgType};
use sqlx::PgPool;

// ── Sentinel helpers ──────────────────────────────────────────────────────────

/// Sentinel player_id used by all org live-DB tests.
/// Must be < 0 (impossible in real play, fits i32).
const SENTINEL_PLAYER_ID: i32 = -97531;
const SENTINEL_PLAYER_ID_2: i32 = -97532;

/// Insert a sentinel account + sgw_player row if they don't exist.
/// Mirrors the contact-list `insert_minimal_player` helper — sgw_player has
/// many NOT NULL columns and no `character_name` (the name column is
/// `player_name`); the account FK must exist first. account_id reuses the
/// (negative, unique) sentinel player_id.
async fn ensure_sentinel_player(pool: &PgPool, player_id: i32) {
    sqlx::query(
        "INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '') \
         ON CONFLICT (account_id) DO NOTHING",
    )
    .bind(player_id)
    .bind(format!("org-persist-test-{player_id}"))
    .execute(pool)
    .await
    .expect("ensure_sentinel_player: account insert failed");

    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, alignment, archetype, gender, \
            player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id\
         ) VALUES ($1, $1, 1, 0, 1, 1, $2, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                   0.0, 0.0, 0.0, 0) \
         ON CONFLICT (player_id) DO NOTHING",
    )
    .bind(player_id)
    .bind(format!("TestOrg_{player_id}"))
    .execute(pool)
    .await
    .expect("ensure_sentinel_player: player insert failed");
}

/// Delete all org data created with the sentinel player_id(s).
/// Cascade from sgw_organizations deletes ranks and members automatically.
async fn cleanup_sentinel_orgs(pool: &PgPool) {
    // Remove member rows for our sentinel players (handles orgs we may not own).
    sqlx::query(
        "DELETE FROM sgw_organization_members \
         WHERE player_id IN ($1, $2)",
    )
    .bind(SENTINEL_PLAYER_ID)
    .bind(SENTINEL_PLAYER_ID_2)
    .execute(pool)
    .await
    .expect("cleanup sentinel member rows");

    // Delete orgs where our sentinel is the leader (cascade kills ranks + members).
    sqlx::query(
        "DELETE FROM sgw_organizations \
         WHERE leader_player_id IN ($1, $2)",
    )
    .bind(SENTINEL_PLAYER_ID)
    .bind(SENTINEL_PLAYER_ID_2)
    .execute(pool)
    .await
    .expect("cleanup sentinel orgs");

    // Remove sentinel player rows.
    sqlx::query("DELETE FROM sgw_player WHERE player_id IN ($1, $2)")
        .bind(SENTINEL_PLAYER_ID)
        .bind(SENTINEL_PLAYER_ID_2)
        .execute(pool)
        .await
        .expect("cleanup sentinel players");
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// Bug shape: `create_org` must insert exactly 9 rank rows (0–8) and the
/// leader member row. Missing ranks break the composite FK for future members.
#[tokio::test]
async fn create_org_seeds_nine_ranks_and_leader_member() {
    let pool = require_db_or_skip!();
    cleanup_sentinel_orgs(&pool).await;
    ensure_sentinel_player(&pool, SENTINEL_PLAYER_ID).await;

    let org_id = create_org(&pool, OrgType::Team, "Sentinel Team", SENTINEL_PLAYER_ID)
        .await
        .expect("create_org failed");

    // Assert exactly 9 rank rows.
    let rank_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sgw_organization_ranks WHERE org_id = $1")
            .bind(org_id)
            .fetch_one(&pool)
            .await
            .expect("rank count query");
    assert_eq!(rank_count, 9, "must seed exactly 9 rank rows (0–8)");

    // Assert leader member exists at rank 8.
    let (rank_value,): (i16,) = sqlx::query_as::<_, (i16,)>(
        "SELECT rank_value FROM sgw_organization_members \
         WHERE org_id = $1 AND player_id = $2",
    )
    .bind(org_id)
    .bind(SENTINEL_PLAYER_ID)
    .fetch_one(&pool)
    .await
    .expect("leader member row missing");
    assert_eq!(
        rank_value,
        OrgRank::Leader as i16,
        "leader must be at rank 8"
    );

    cleanup_sentinel_orgs(&pool).await;
}

/// Bug shape: `load_all_orgs` must round-trip the org just created —
/// returning it with correct header fields, rank configs, and the leader member.
#[tokio::test]
async fn load_all_orgs_round_trips_create() {
    let pool = require_db_or_skip!();
    cleanup_sentinel_orgs(&pool).await;
    ensure_sentinel_player(&pool, SENTINEL_PLAYER_ID).await;

    let org_id = create_org(
        &pool,
        OrgType::Command,
        "Sentinel Command",
        SENTINEL_PLAYER_ID,
    )
    .await
    .expect("create_org failed");

    let orgs = load_all_orgs(&pool).await.expect("load_all_orgs failed");
    let loaded = orgs
        .iter()
        .find(|o| o.org_id == org_id)
        .expect("created org not found in load_all_orgs");

    assert_eq!(loaded.org_type, OrgType::Command);
    assert_eq!(loaded.name, "Sentinel Command");
    assert_eq!(loaded.leader_player_id, SENTINEL_PLAYER_ID);
    assert!(loaded.motd.is_none());
    assert_eq!(loaded.cash, 0);
    assert_eq!(
        loaded.rank_configs.len(),
        9,
        "must have 9 rank config entries"
    );
    assert!(
        loaded.members.contains_key(&SENTINEL_PLAYER_ID),
        "leader must be in member map"
    );
    assert_eq!(loaded.members[&SENTINEL_PLAYER_ID].rank, OrgRank::Leader);

    // Verify the Leader rank has ALL permissions.
    let leader_perms = loaded.rank_configs[&(OrgRank::Leader as u8)].permissions;
    assert_eq!(
        leader_perms,
        OrgPermission::ALL,
        "Leader rank must have ALL permissions"
    );

    cleanup_sentinel_orgs(&pool).await;
}

/// Bug shape: `add_member` then `remove_member` must leave no orphan rows.
#[tokio::test]
async fn add_and_remove_member_leaves_no_orphan() {
    let pool = require_db_or_skip!();
    cleanup_sentinel_orgs(&pool).await;
    ensure_sentinel_player(&pool, SENTINEL_PLAYER_ID).await;
    ensure_sentinel_player(&pool, SENTINEL_PLAYER_ID_2).await;

    let org_id = create_org(&pool, OrgType::Team, "Sentinel Team 2", SENTINEL_PLAYER_ID)
        .await
        .expect("create_org failed");

    // Add a second member.
    let added = add_member(&pool, org_id, SENTINEL_PLAYER_ID_2)
        .await
        .expect("add_member failed");
    assert!(added, "add_member must return true on first insert");

    // Verify it's there at Initiate.
    let (rank_value,): (i16,) = sqlx::query_as::<_, (i16,)>(
        "SELECT rank_value FROM sgw_organization_members \
         WHERE org_id = $1 AND player_id = $2",
    )
    .bind(org_id)
    .bind(SENTINEL_PLAYER_ID_2)
    .fetch_one(&pool)
    .await
    .expect("added member row missing");
    assert_eq!(rank_value, OrgRank::Initiate as i16);

    // Remove the member.
    let removed = remove_member(&pool, org_id, SENTINEL_PLAYER_ID_2)
        .await
        .expect("remove_member failed");
    assert!(removed, "remove_member must return true when row found");

    // Assert zero orphan rows.
    let orphan_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sgw_organization_members \
         WHERE org_id = $1 AND player_id = $2",
    )
    .bind(org_id)
    .bind(SENTINEL_PLAYER_ID_2)
    .fetch_one(&pool)
    .await
    .expect("orphan count query");
    assert_eq!(orphan_count, 0, "remove_member must leave no orphan rows");

    cleanup_sentinel_orgs(&pool).await;
}

/// Bug shape: `set_member_rank` must persist the new rank to the DB.
#[tokio::test]
async fn set_member_rank_persists() {
    let pool = require_db_or_skip!();
    cleanup_sentinel_orgs(&pool).await;
    ensure_sentinel_player(&pool, SENTINEL_PLAYER_ID).await;
    ensure_sentinel_player(&pool, SENTINEL_PLAYER_ID_2).await;

    let org_id = create_org(&pool, OrgType::Team, "Sentinel Team 3", SENTINEL_PLAYER_ID)
        .await
        .expect("create_org");

    add_member(&pool, org_id, SENTINEL_PLAYER_ID_2)
        .await
        .expect("add_member");

    // Promote to Officer.
    let updated = set_member_rank(&pool, org_id, SENTINEL_PLAYER_ID_2, OrgRank::Officer)
        .await
        .expect("set_member_rank");
    assert!(updated);

    let (rank_value,): (i16,) = sqlx::query_as::<_, (i16,)>(
        "SELECT rank_value FROM sgw_organization_members \
         WHERE org_id = $1 AND player_id = $2",
    )
    .bind(org_id)
    .bind(SENTINEL_PLAYER_ID_2)
    .fetch_one(&pool)
    .await
    .expect("rank row missing");
    assert_eq!(
        rank_value,
        OrgRank::Officer as i16,
        "rank must be Officer after set"
    );

    cleanup_sentinel_orgs(&pool).await;
}

/// Bug shape: `set_motd` must persist to the DB; reading back via load_all_orgs
/// must return the new MOTD.
#[tokio::test]
async fn set_motd_persists() {
    let pool = require_db_or_skip!();
    cleanup_sentinel_orgs(&pool).await;
    ensure_sentinel_player(&pool, SENTINEL_PLAYER_ID).await;

    let org_id = create_org(
        &pool,
        OrgType::Command,
        "Sentinel Command 2",
        SENTINEL_PLAYER_ID,
    )
    .await
    .expect("create_org");

    set_motd(&pool, org_id, Some("Welcome to the resistance!"))
        .await
        .expect("set_motd");

    let (motd,): (Option<String>,) = sqlx::query_as::<_, (Option<String>,)>(
        "SELECT motd FROM sgw_organizations WHERE org_id = $1",
    )
    .bind(org_id)
    .fetch_one(&pool)
    .await
    .expect("motd query");

    assert_eq!(motd.as_deref(), Some("Welcome to the resistance!"));

    cleanup_sentinel_orgs(&pool).await;
}

/// Bug shape: the default permission ladder must be seeded correctly for all
/// 9 ranks. Specifically: Leader (8) = ALL, Officer (6) has INVITE+EJECT+MOTD
/// but NOT PROMOTE/DEMOTE.
#[tokio::test]
async fn default_permission_ladder_correctness() {
    // Unit-level check — no DB needed.
    let leader_perm = OrgPermission::from_bits(DEFAULT_PERMISSION_LADDER[8]);
    assert_eq!(
        leader_perm,
        OrgPermission::ALL,
        "Leader must have ALL perms"
    );

    let officer_perm = OrgPermission::from_bits(DEFAULT_PERMISSION_LADDER[6]);
    assert!(
        officer_perm.contains(OrgPermission::INVITE),
        "Officer must have INVITE"
    );
    assert!(
        officer_perm.contains(OrgPermission::EJECT),
        "Officer must have EJECT"
    );
    assert!(
        officer_perm.contains(OrgPermission::MOTD),
        "Officer must have MOTD"
    );
    assert!(
        !officer_perm.contains(OrgPermission::PROMOTE),
        "Officer must NOT have PROMOTE — reserved for SeniorOfficer+"
    );
    assert!(
        !officer_perm.contains(OrgPermission::DEMOTE),
        "Officer must NOT have DEMOTE — reserved for SeniorOfficer+"
    );

    let initiate_perm = OrgPermission::from_bits(DEFAULT_PERMISSION_LADDER[1]);
    assert_eq!(
        initiate_perm,
        OrgPermission::NONE,
        "Initiate must have no permissions"
    );
}
