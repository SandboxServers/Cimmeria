//! The bank bits on the moves that need two of them, or the less obvious
//! one (bank-vault BV-07b; D-BV12), and a bound item entering through a
//! swap. One live-DB guard (TESTING.md type 3) per rule, each failing if
//! its check is removed (server-authority review S3).

use cimmeria_entity::cell_entity::VaultScope;
use cimmeria_entity::organization::OrgPermission;

use super::moves::{at_banker, inv_item, mv, refused};
use super::*;
use crate::test_support::{require_db_or_skip, LogCapture};

/// Give the Team's `Member` rank exactly `perms`.
async fn member_perms(fx: &Fx, org_id: i32, perms: OrgPermission) {
    sqlx::query(
        "UPDATE sgw_organization_ranks SET permissions = $3 WHERE org_id = $1 AND rank = $2",
    )
    .bind(org_id)
    .bind(2_i16)
    .bind(perms.to_wire())
    .execute(&fx.pool)
    .await
    .unwrap();
}

/// Set the charges of a row in either table, so two stacks of one type do
/// not merge and the move swaps.
async fn set_charges(fx: &Fx, item_id: i32, charges: i32) {
    for table in ["sgw_inventory", "sgw_organization_vault_items"] {
        let sql = match table {
            "sgw_inventory" => "UPDATE sgw_inventory SET charges = $2 WHERE item_id = $1",
            _ => "UPDATE sgw_organization_vault_items SET charges = $2 WHERE item_id = $1",
        };
        sqlx::query(sql)
            .bind(item_id)
            .bind(charges)
            .execute(&fx.pool)
            .await
            .unwrap();
    }
}

/// A withdraw-only rank may take items out, but a withdrawal that swaps a
/// carried item into the vault also deposits, and needs `DepositBank`; so
/// does rearranging the vault. Fails if either check is removed.
#[tokio::test]
async fn withdraw_only_ranks_cannot_swap_in_or_rearrange() {
    let pool = require_db_or_skip!();
    let fx = Fx::new(&pool, 13, 2).await;
    let team = fx.org(0, 1, &[0]).await;
    member_perms(&fx, team, OrgPermission::WITHDRAW_BANK).await;
    let vault = at_banker(VaultScope::Team, team);
    let client = Client::in_world(fx.entity(0), 40884);
    let (v, c) = (fx.item(0), fx.item(1));
    fx.put(team, v, 0, BANKABLE, 3).await;
    fx.carry(0, c, 1, 2, BANKABLE, 1, false).await;
    set_charges(&fx, c, 4).await;

    let capture = LogCapture::install();
    mv(&fx, &client, 0, v, 1, 2, -1, vault).await;
    refused(&capture, "missing_permission", &[("perm", "DepositBank")]);
    assert_eq!(fx.vault(team).await, vec![(v, 0, 3)], "no swap");
    assert_eq!(fx.bag(0).await, vec![(c, 1, 2, 1)]);

    let capture = LogCapture::install();
    mv(&fx, &client, 0, v, 19, 5, -1, vault).await;
    refused(&capture, "missing_permission", &[("perm", "DepositBank")]);
    assert_eq!(fx.vault(team).await, vec![(v, 0, 3)], "not rearranged");

    // The plain withdrawal it may make.
    mv(&fx, &client, 0, v, 1, 3, -1, vault).await;
    assert!(fx.vault(team).await.is_empty());
    fx.teardown().await;
}

/// A default Member may deposit, but a deposit that swaps with a vault
/// item also withdraws it, and needs `WithdrawBank`. Fails if the swap's
/// second bit is not checked.
#[tokio::test]
async fn a_deposit_swap_needs_withdraw_bank() {
    let pool = require_db_or_skip!();
    let fx = Fx::new(&pool, 14, 2).await;
    let team = fx.org(0, 1, &[0]).await;
    let vault = at_banker(VaultScope::Team, team);
    let client = Client::in_world(fx.entity(0), 40885);
    let (v, c) = (fx.item(0), fx.item(1));
    fx.put(team, v, 0, BANKABLE, 3).await;
    set_charges(&fx, v, 4).await;
    fx.carry(0, c, 1, 2, BANKABLE, 1, false).await;

    let capture = LogCapture::install();
    mv(&fx, &client, 0, c, 19, 0, -1, vault).await;
    refused(&capture, "missing_permission", &[("perm", "WithdrawBank")]);
    assert_eq!(fx.vault(team).await, vec![(v, 0, 3)]);
    assert_eq!(fx.bag(0).await, vec![(c, 1, 2, 1)]);
    assert!(
        client.saw_bytes(&inv_item(c, BANKABLE, 1, 2, 1)),
        "snap-back"
    );
    fx.teardown().await;
}

/// A withdrawal onto a carried bound item would swap the bound item into
/// the shared vault: refused `bound_item_not_org_storable` in Rust, before
/// the table's `CHECK (NOT bound)` could turn it into a rolled-back write.
/// Fails if the occupant's entry rules are skipped (the refusal is then
/// `move_failed`).
#[tokio::test]
async fn a_withdraw_swap_cannot_bring_a_bound_item_into_the_vault() {
    let pool = require_db_or_skip!();
    let fx = Fx::new(&pool, 15, 1).await;
    let team = fx.org(0, 0, &[]).await;
    let vault = at_banker(VaultScope::Team, team);
    let client = Client::in_world(fx.entity(0), 40886);
    let (v, c) = (fx.item(0), fx.item(1));
    fx.put(team, v, 0, BANKABLE, 3).await;
    fx.carry(0, c, 1, 2, BANKABLE, 1, true).await;

    let capture = LogCapture::install();
    mv(&fx, &client, 0, v, 1, 2, -1, vault).await;
    refused(&capture, "bound_item_not_org_storable", &[("rank", "8")]);
    assert_eq!(fx.vault(team).await, vec![(v, 0, 3)]);
    assert_eq!(fx.bag(0).await, vec![(c, 1, 2, 1)]);
    fx.teardown().await;
}
