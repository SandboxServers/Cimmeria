//! The Team and Command vault's storage rules (bank-vault BV-07; D-BV13,
//! D-BV18), on real `sgw_organization_vault_items` rows: the vault
//! predicate the disbands and the member-delete trigger decide on, the
//! non-cascading foreign key, and the table's CHECKs.
//!
//! Vault item ids: `0x7000_B9C0..=0x7000_B9CF` (the BV-07 block). Each
//! test deletes its own rows by exact id before the fixture teardown, which
//! deletes the organizations (the RESTRICT key would refuse it otherwise).

use cimmeria_entity::organization::OrgType;

use super::super::super::api::org_vault_is_empty;
use super::super::super::character_delete::delete_character;
use super::super::{disband, OrgStoreError};
use super::*;
use crate::test_support::require_db_or_skip;

const ITEM_BASE: i32 = 0x7000_B9C0;

/// Any seeded item type, for the vault row's `type_id` key.
async fn some_type(pool: &PgPool) -> i32 {
    sqlx::query_scalar("SELECT min(item_id) FROM resources.items WHERE 17 = ANY(container_sets)")
        .fetch_one(pool)
        .await
        .unwrap()
}

/// Insert one vault row. `org_type` and `container_id` are taken as given,
/// so the CHECK tests can pass a mismatched pair.
async fn put(
    pool: &PgPool,
    item_id: i32,
    org_id: i32,
    org_type: i16,
    container_id: i32,
    bound: bool,
) -> Result<(), sqlx::Error> {
    let type_id = some_type(pool).await;
    sqlx::query(
        "INSERT INTO sgw_organization_vault_items \
         (item_id, org_id, org_type, container_id, slot_id, type_id, stack_size, charges, \
          durability, flags, bound, ammo, cur_ammo_type, ammo_type, ammo_types, \
          deposited_by_player_id) \
         VALUES ($1, $2, $3, $4, 0, $5, 1, 0, -1, 0, $6, 0, 0, 'AMMO_NONE', '{}', 0)",
    )
    .bind(item_id)
    .bind(org_id)
    .bind(org_type)
    .bind(container_id)
    .bind(type_id)
    .bind(bound)
    .execute(pool)
    .await
    .map(|_| ())
}

async fn clear(pool: &PgPool, ids: &[i32]) {
    sqlx::query("DELETE FROM sgw_organization_vault_items WHERE item_id = ANY($1)")
        .bind(ids)
        .execute(pool)
        .await
        .unwrap();
}

/// The Rust predicate and the SQL function, read in one transaction.
async fn empty_both(pool: &PgPool, org_id: i32) -> (bool, bool) {
    let mut tx = pool.begin().await.unwrap();
    let rust = org_vault_is_empty(&mut tx, org_id).await.unwrap();
    let sql: bool = sqlx::query_scalar("SELECT org_vault_is_empty_sql($1)")
        .bind(org_id)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    tx.rollback().await.unwrap();
    (rust, sql)
}

/// The predicate follows real vault rows and the treasury, in Rust and in
/// SQL alike. Fails if either is the old always-`true` stub.
#[tokio::test]
async fn vault_predicate_follows_items_and_cash() {
    let pool = require_db_or_skip!();
    let item = ITEM_BASE;
    clear(&pool, &[item]).await;
    let fx = setup(&pool, 40, 1, &["Bv07 Vault Predicate"]).await;
    let team = create(&pool, OrgType::Team, "Bv07 Vault Predicate", fx.player(0))
        .await
        .org_id;

    assert_eq!(empty_both(&pool, team).await, (true, true), "a new vault");
    put(&pool, item, team, 1, 19, false).await.unwrap();
    assert_eq!(empty_both(&pool, team).await, (false, false), "one item");
    clear(&pool, &[item]).await;
    sqlx::query("UPDATE sgw_organizations SET cash = 5 WHERE org_id = $1")
        .bind(team)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(empty_both(&pool, team).await, (false, false), "cash only");
    sqlx::query("UPDATE sgw_organizations SET cash = 0 WHERE org_id = $1")
        .bind(team)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(empty_both(&pool, team).await, (true, true), "emptied again");

    teardown(&pool, &fx).await;
}

/// D-BV13: a disband is refused while the vault holds an item, and nothing
/// is deleted. Fails with the stub predicate (the disband then hits the
/// RESTRICT key, a database error, not `VaultNotEmpty`).
#[tokio::test]
async fn disband_is_refused_while_the_vault_holds_an_item() {
    let pool = require_db_or_skip!();
    let item = ITEM_BASE + 1;
    clear(&pool, &[item]).await;
    let fx = setup(&pool, 41, 1, &["Bv07 Vault Disband"]).await;
    let cmd = create(&pool, OrgType::Command, "Bv07 Vault Disband", fx.player(0))
        .await
        .org_id;
    put(&pool, item, cmd, 2, 20, false).await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let r = as_sys!(disband, tx, cmd);
    tx.rollback().await.unwrap();
    assert!(matches!(r, Err(OrgStoreError::VaultNotEmpty)), "{r:?}");
    assert!(org_exists(&pool, cmd).await);

    clear(&pool, &[item]).await;
    let mut tx = pool.begin().await.unwrap();
    as_sys!(disband, tx, cmd).expect("an empty vault disbands");
    tx.commit().await.unwrap();
    assert!(!org_exists(&pool, cmd).await);

    teardown(&pool, &fx).await;
}

/// D-BV18: the vault's key does not cascade. Deleting an organization that
/// still holds items fails, and the items survive. Fails if the key is
/// `ON DELETE CASCADE`.
#[tokio::test]
async fn deleting_an_org_never_deletes_its_vault() {
    let pool = require_db_or_skip!();
    let item = ITEM_BASE + 2;
    clear(&pool, &[item]).await;
    let fx = setup(&pool, 42, 1, &["Bv07 Vault Restrict"]).await;
    let team = create(&pool, OrgType::Team, "Bv07 Vault Restrict", fx.player(0))
        .await
        .org_id;
    put(&pool, item, team, 1, 19, false).await.unwrap();

    let err = sqlx::query("DELETE FROM sgw_organizations WHERE org_id = $1")
        .bind(team)
        .execute(&pool)
        .await
        .expect_err("a non-empty vault must block the delete");
    assert_eq!(
        violated(&err).as_deref(),
        Some("sgw_organization_vault_items_org_fkey")
    );
    let left: i64 =
        sqlx::query_scalar("SELECT count(*) FROM sgw_organization_vault_items WHERE item_id = $1")
            .bind(item)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(left, 1, "the item survives");

    clear(&pool, &[item]).await;
    teardown(&pool, &fx).await;
}

/// D-BV18 with the member-delete trigger: when the last member's character
/// is deleted and the vault holds an item, the organization stays,
/// memberless, for GM recovery. Fails with the stub SQL predicate (the
/// trigger then deletes the organization and the RESTRICT key fails the
/// character delete).
#[tokio::test]
async fn last_member_delete_keeps_a_memberless_org_holding_its_vault() {
    let pool = require_db_or_skip!();
    let item = ITEM_BASE + 3;
    clear(&pool, &[item]).await;
    let fx = setup(&pool, 43, 1, &["Bv07 Vault Memberless"]).await;
    let team = create(&pool, OrgType::Team, "Bv07 Vault Memberless", fx.player(0))
        .await
        .org_id;
    put(&pool, item, team, 1, 19, false).await.unwrap();

    let deleted = delete_character(&pool, fx.player(0), fx.account_id)
        .await
        .expect("character delete");
    assert!(deleted.deleted);
    assert!(org_exists(&pool, team).await, "the organization is kept");
    assert!(member_ranks(&pool, team).await.is_empty(), "memberless");
    let event: Option<String> = sqlx::query_scalar(
        "SELECT event FROM sgw_organization_events WHERE org_id = $1 ORDER BY org_event_id DESC",
    )
    .bind(team)
    .fetch_optional(&pool)
    .await
    .unwrap();
    assert_eq!(event.as_deref(), Some("left_memberless"));

    clear(&pool, &[item]).await;
    sqlx::query("DELETE FROM sgw_organization_events WHERE org_id = $1")
        .bind(team)
        .execute(&pool)
        .await
        .unwrap();
    teardown(&pool, &fx).await;
}

/// The table's own guards: a Team holds only container 19 (a Command only
/// 20, pinned through the composite key to the organization's type), and a
/// bound item never enters a shared vault.
#[tokio::test]
async fn vault_rows_match_their_org_type_and_are_never_bound() {
    let pool = require_db_or_skip!();
    let ids = [ITEM_BASE + 4, ITEM_BASE + 5, ITEM_BASE + 6];
    clear(&pool, &ids).await;
    let fx = setup(&pool, 44, 1, &["Bv07 Vault Checks"]).await;
    let team = create(&pool, OrgType::Team, "Bv07 Vault Checks", fx.player(0))
        .await
        .org_id;

    let wrong_container = put(&pool, ids[0], team, 1, 20, false).await.unwrap_err();
    assert_eq!(
        violated(&wrong_container).as_deref(),
        Some("sgw_organization_vault_items_container_check")
    );
    let wrong_type = put(&pool, ids[1], team, 2, 20, false).await.unwrap_err();
    assert_eq!(
        violated(&wrong_type).as_deref(),
        Some("sgw_organization_vault_items_org_fkey")
    );
    let bound = put(&pool, ids[2], team, 1, 19, true).await.unwrap_err();
    assert_eq!(
        violated(&bound).as_deref(),
        Some("sgw_organization_vault_items_not_bound_chk")
    );

    clear(&pool, &ids).await;
    teardown(&pool, &fx).await;
}
