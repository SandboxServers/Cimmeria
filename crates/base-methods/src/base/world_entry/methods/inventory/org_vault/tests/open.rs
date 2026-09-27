//! The base half of the Team and Command vault open (bank-vault BV-07):
//! membership under the organization lock, the `onBagInfo` and contents
//! sent, the grant to the cell, and every `org_vault_open_rejected` reason.

use cimmeria_entity::cell_entity::VaultScope;
use tokio::sync::mpsc;
use tracing::Level;

use super::super::access::{lock_actor, OrgLockMiss};
use super::super::open::{
    handle_org_vault_open, org_vault_bag_info, OrgVaultIo, OrgVaultOpenRequest,
};
use super::*;
use crate::cell::messages::{BankBaseToCell, BaseToCellMsg};
use crate::test_support::{require_db_or_skip, Captured, LogCapture, LogCaptureGuard};

const BANKER: u32 = 0x7000_B9D8;

fn request(fx: &Fx, scope: VaultScope) -> OrgVaultOpenRequest {
    OrgVaultOpenRequest {
        entity_id: fx.entity(0),
        account_id: Some(fx.account_id as u32),
        player_id: Some(fx.player(0)),
        scope,
        banker_id: BANKER,
        distance: Some(2.5),
        space_id: 3,
    }
}

/// Run one open; returns the bank messages it sent the cell (any other
/// message fails the test).
async fn open(
    pool: Option<Arc<PgPool>>,
    req: OrgVaultOpenRequest,
    client: &Client,
) -> Vec<BankBaseToCell> {
    let (cell_tx, mut cell_rx) = mpsc::channel(8);
    let cell_tx = Some(cell_tx);
    handle_org_vault_open(
        req,
        OrgVaultIo {
            db_pool: &pool,
            cell_tx: &cell_tx,
            transport: &client.dyn_transport,
            connected: &client.conn,
            entity_to_addr: &client.e2a,
        },
    )
    .await;
    let mut out = Vec::new();
    while let Ok(m) = cell_rx.try_recv() {
        match m {
            BaseToCellMsg::Bank(bank) => out.push(bank),
            _ => panic!("the open sent the cell a non-bank message"),
        }
    }
    out
}

fn bank_rows(capture: &LogCaptureGuard, event: &str) -> Vec<Captured> {
    capture
        .all()
        .into_iter()
        .filter(|c| c.target == "bank" && c.has_field("event", event))
        .collect()
}

/// Byte-exact `onBagInfo` for an org vault open: all 20 containers in id
/// order, 17 at `bank_slots`, 19 at the Team's size and 20 at 100. Fails if
/// the Team's size is not declared, or the container set shrinks.
#[test]
fn org_vault_bag_info_declares_the_real_team_size() {
    let args = org_vault_bag_info(50, 60);
    let mut want = 20u32.to_le_bytes().to_vec();
    for (id, slots) in [
        (1, 40),
        (2, 100),
        (3, 4),
        (4, 1),
        (5, 1),
        (6, 1),
        (7, 1),
        (8, 1),
        (9, 1),
        (10, 1),
        (11, 1),
        (12, 1),
        (13, 1),
        (14, 1),
        (15, 100),
        (16, 12),
        (17, 50),
        (18, 100),
        (19, 60),
        (20, 100),
    ] {
        want.extend_from_slice(&i32::to_le_bytes(id));
        want.extend_from_slice(&i32::to_le_bytes(slots));
    }
    assert_eq!(args, want);
}

/// A member opens the Team vault: the base sends `onBagInfo` with the
/// Team's size and the vault's rows, logs `org_vault_opened` with the rank
/// and bits, and grants the cell the vault of that org. Fails if the open
/// is refused, sends no size, or grants another org.
#[tokio::test]
async fn a_member_gets_size_contents_and_a_grant() {
    let pool = require_db_or_skip!();
    let fx = Fx::new(&pool, 0, 2).await;
    let team = fx.org(0, 1, &[0]).await;
    sqlx::query("UPDATE sgw_organizations SET vault_slots = 60 WHERE org_id = $1")
        .bind(team)
        .execute(&pool)
        .await
        .unwrap();
    fx.put(team, fx.item(0), 3, BANKABLE, 7).await;
    let client = Client::in_world(fx.entity(0), 40870);
    let capture = LogCapture::install();

    let sent = open(
        Some(Arc::new(pool.clone())),
        request(&fx, VaultScope::Team),
        &client,
    )
    .await;

    assert!(
        matches!(
            sent.as_slice(),
            [BankBaseToCell::OrgVaultGranted {
                org_id, scope: VaultScope::Team, banker_id: BANKER, ..
            }] if *org_id == team
        ),
        "one grant for the Team: {sent:?}"
    );
    assert!(
        client.saw_bytes(&org_vault_bag_info(40, 60)),
        "onBagInfo with 19 at 60"
    );
    // The vault row, as an InvItem: id, type, stack, 1-indexed slot, 19.
    let mut item = fx.item(0).to_le_bytes().to_vec();
    for v in [BANKABLE, 7, 4, 19] {
        item.extend_from_slice(&v.to_le_bytes());
    }
    assert!(
        client.saw_bytes(&item),
        "the vault's row reached the client"
    );

    let rows = bank_rows(&capture, "org_vault_opened");
    assert_eq!(rows.len(), 1, "{:#?}", capture.all());
    let row = &rows[0];
    assert_eq!(row.level, Level::DEBUG);
    for (k, v) in [
        ("account_id", &fx.account_id.to_string()[..]),
        ("player_id", &fx.player(0).to_string()[..]),
        ("entity_id", &fx.entity(0).to_string()[..]),
        ("org_id", &team.to_string()[..]),
        ("org_type", "team"),
        ("rank", "2"),
        ("can_deposit", "true"),
        ("can_withdraw", "false"),
        ("vault_slots", "60"),
        ("item_count", "1"),
        ("banker_id", &BANKER.to_string()[..]),
        ("space_id", "3"),
        ("distance", "2.5"),
    ] {
        assert!(row.has_field(k, v), "{k}={v}: {row:#?}");
    }
    fx.teardown().await;
}

/// One refusal: exactly one WARN `org_vault_open_rejected` with `reason`,
/// no grant, and the chat line.
fn assert_refused(
    capture: &LogCaptureGuard,
    sent: &[BankBaseToCell],
    client: &Client,
    reason: &str,
    line: &str,
) {
    let rows = bank_rows(capture, "org_vault_open_rejected");
    assert_eq!(rows.len(), 1, "{reason}: {:#?}", capture.all());
    assert_eq!(rows[0].level, Level::WARN);
    assert!(rows[0].has_field("reason", reason), "{:#?}", rows[0]);
    assert!(
        rows[0].has_field("banker_id", &BANKER.to_string()),
        "{:#?}",
        rows[0]
    );
    assert!(sent.is_empty(), "{reason}: no grant: {sent:?}");
    assert!(client.saw_text(line), "{reason}: the player is told");
}

/// A player in no Team is refused `not_in_org`, told, and not granted.
/// Fails if the open skips the membership lookup.
#[tokio::test]
async fn a_player_in_no_team_is_refused_not_in_org() {
    let pool = require_db_or_skip!();
    let fx = Fx::new(&pool, 1, 2).await;
    // Only the other character is in a Team.
    fx.org(0, 1, &[]).await;
    let client = Client::in_world(fx.entity(0), 40871);
    let capture = LogCapture::install();

    let sent = open(
        Some(Arc::new(pool.clone())),
        request(&fx, VaultScope::Team),
        &client,
    )
    .await;

    assert_refused(
        &capture,
        &sent,
        &client,
        "not_in_org",
        "You are not in a Team, so there is no Team vault to open.",
    );
    let row = &bank_rows(&capture, "org_vault_open_rejected")[0];
    assert!(
        row.has_field("account_id", &fx.account_id.to_string()),
        "{row:#?}"
    );
    fx.teardown().await;
}

/// The cell had no `player_id`: refused `player_unknown` before any read.
#[tokio::test]
async fn a_request_without_a_player_is_refused_player_unknown() {
    let client = Client::in_world(0x7000_B9DF, 40872);
    let mut req = OrgVaultOpenRequest {
        entity_id: 0x7000_B9DF,
        account_id: None,
        player_id: None,
        scope: VaultScope::Command,
        banker_id: BANKER,
        distance: None,
        space_id: 1,
    };
    let capture = LogCapture::install();
    let sent = open(Some(Arc::new(unreachable_pool())), req, &client).await;
    assert_refused(
        &capture,
        &sent,
        &client,
        "player_unknown",
        "The Command vault could not be opened. Please try again.",
    );

    // Database down: `open_query_failed`, still with a line.
    req.player_id = Some(0x7000_B9DE);
    let capture = LogCapture::install();
    let sent = open(Some(Arc::new(unreachable_pool())), req, &client).await;
    let rows = bank_rows(&capture, "org_vault_open_rejected");
    assert_eq!(rows.len(), 1, "{:#?}", capture.all());
    assert!(
        rows[0].has_field("reason", "open_query_failed"),
        "{:#?}",
        rows[0]
    );
    assert!(sent.is_empty());
}

/// The lock-time refusals, straight from [`lock_actor`]: another org's
/// member (`not_a_member`), a gone org (`no_such_org`), a Team asked for as
/// a Command (`wrong_org_type`), and a missing character
/// (`player_missing`). Fails if membership is not re-read under the lock.
#[tokio::test]
async fn lock_actor_refuses_non_members_gone_orgs_and_the_wrong_type() {
    let pool = require_db_or_skip!();
    let fx = Fx::new(&pool, 2, 2).await;
    let team = fx.org(0, 0, &[]).await;
    let other = fx.org(2, 1, &[]).await;
    let mut tx = pool.begin().await.unwrap();

    let miss = |r: Result<Result<_, OrgLockMiss>, sqlx::Error>| r.unwrap().err();
    assert_eq!(
        miss(lock_actor(&mut tx, fx.player(0), other, VaultScope::Team).await),
        Some(OrgLockMiss::NotAMember)
    );
    assert_eq!(
        miss(lock_actor(&mut tx, fx.player(0), 0x3FFF_FF00, VaultScope::Team).await),
        Some(OrgLockMiss::NoSuchOrg)
    );
    assert_eq!(
        miss(lock_actor(&mut tx, fx.player(0), team, VaultScope::Command).await),
        Some(OrgLockMiss::WrongOrgType)
    );
    assert_eq!(
        miss(lock_actor(&mut tx, 0x7000_B9DE, team, VaultScope::Team).await),
        Some(OrgLockMiss::PlayerMissing)
    );
    let ok = lock_actor(&mut tx, fx.player(0), team, VaultScope::Team)
        .await
        .unwrap()
        .expect("the leader is authorized");
    assert_eq!(ok.vault_slots, 40, "a new Team's vault");
    tx.rollback().await.unwrap();
    fx.teardown().await;
}
