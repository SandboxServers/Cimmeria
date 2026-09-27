//! Live-DB guards for the system-mail writer (SS-U1, `mail/system/`): each
//! item variant, the server-held rule for existing instances, rollback with
//! the caller's transaction, and the cap exemption. Sentinels
//! `0x7300_51xx` (accounts and players) and `0x7300_52xx` (items).

use super::*;
use crate::base::world_entry::methods::mail::system::{
    send_system_mail, send_system_mail_tx, SystemItem, SystemMail, SystemMailError,
    SYSTEM_SOURCE_CHARACTER_ID,
};
use crate::test_support::LogCapture;
use cimmeria_entity::inventory::{
    INV_AUCTION, INV_BANDOLIER, INV_BANK, INV_BUYBACK, INV_CHEST, INV_COMMAND_BANK, INV_MAIN,
    INV_TEAM_BANK,
};

/// `(sender_id, sender_name, cash, flags)` of every mail `character_id`
/// holds, by mail id.
async fn mails(pool: &PgPool, character_id: i32) -> Vec<(Option<i32>, String, i64, i32)> {
    sqlx::query_as(
        "SELECT sender_id, sender_name, cash, flags FROM sgw_gate_mail \
         WHERE character_id = $1 ORDER BY mail_id",
    )
    .bind(character_id)
    .fetch_all(pool)
    .await
    .unwrap()
}

/// An item type whose `max_stack_size` is `1` (`stackable = false`) or
/// above 1.
async fn type_with_stack(pool: &PgPool, stackable: bool) -> (i32, i32) {
    let sql = if stackable {
        "SELECT item_id, max_stack_size FROM resources.items \
         WHERE max_stack_size > 1 ORDER BY item_id LIMIT 1"
    } else {
        "SELECT item_id, max_stack_size FROM resources.items \
         WHERE max_stack_size = 1 ORDER BY item_id LIMIT 1"
    };
    sqlx::query_as(sql).fetch_one(pool).await.unwrap()
}

fn mail_to(recipient: i32, cash: i64, item: SystemItem) -> SystemMail {
    SystemMail {
        sender_name: "Black Market".into(),
        recipient_player_id: recipient,
        subject: "Auction sold".into(),
        body: "Your item sold.".into(),
        cash,
        item,
    }
}

async fn setup(pool: &PgPool, acct: i32, players: &[(i32, &str)]) {
    cleanup(pool, acct).await;
    insert_players(pool, acct, players).await;
}

/// Cash only and minted item, the two `send_system_mail` shapes a payout
/// uses: one mail each, no `sender_id` (so it cannot be returned), the
/// label as `sender_name`, no COD flag; nobody is debited. The minted item
/// lands in escrow with `grant_item`'s defaults and the system source id.
#[tokio::test]
async fn system_mail_cash_and_minted_item() {
    let pool = require_db_or_skip!();
    let (acct, rcpt) = (0x7300_5100, 0x7300_5101);
    setup(&pool, acct, &[(rcpt, "SsuOneCashRcpt")]).await;
    set_naquadah(&pool, rcpt, 40).await;
    let (type_id, max) = type_with_stack(&pool, true).await;

    let cash = send_system_mail(&pool, &mail_to(rcpt, 1_500, SystemItem::None))
        .await
        .expect("cash-only system mail");
    assert!(cash.item.is_none());
    let item = send_system_mail(
        &pool,
        &mail_to(rcpt, 0, SystemItem::Minted { type_id, qty: max }),
    )
    .await
    .expect("minted-item system mail");

    assert_eq!(
        mails(&pool, rcpt).await,
        vec![
            (None, "Black Market".into(), 1_500, 0),
            (None, "Black Market".into(), 0, 0)
        ]
    );
    assert_eq!(naquadah(&pool, rcpt).await, 40, "nothing is credited yet");
    let escrow = escrow_for(&pool, rcpt).await;
    assert_eq!(escrow.len(), 1);
    let row = &escrow[0];
    assert_eq!(row.mail_id, item.mail_id);
    assert_eq!(Some(row.item_id), item.item.map(|i| i.item_id));
    assert_eq!((row.type_id, row.stack_size), (type_id, max));
    assert_eq!(row.durability, 100, "grant_item's durability");
    assert!(!row.bound);
    assert_eq!(row.source_character_id, SYSTEM_SOURCE_CHARACTER_ID);
    let charges: i32 = sqlx::query_scalar("SELECT charges FROM resources.items WHERE item_id = $1")
        .bind(type_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.charges, charges, "the template's charges");

    cleanup(&pool, acct).await;
}

/// A minted quantity above the type's stack size, an unknown type, a
/// missing recipient and out-of-range cash are refused, and nothing is
/// written.
#[tokio::test]
async fn system_mail_refusals_write_nothing() {
    let pool = require_db_or_skip!();
    let (acct, rcpt) = (0x7300_5110, 0x7300_5111);
    setup(&pool, acct, &[(rcpt, "SsuOneRefuseRcpt")]).await;
    let (single, _) = type_with_stack(&pool, false).await;

    for (mail, reason) in [
        (
            mail_to(
                rcpt,
                0,
                SystemItem::Minted {
                    type_id: single,
                    qty: 2,
                },
            ),
            "item_quantity_exceeds_stack",
        ),
        (
            mail_to(
                rcpt,
                0,
                SystemItem::Minted {
                    type_id: i32::MAX,
                    qty: 1,
                },
            ),
            "unknown_item_type",
        ),
        (
            mail_to(
                rcpt,
                0,
                SystemItem::Minted {
                    type_id: single,
                    qty: 0,
                },
            ),
            "invalid_item_quantity",
        ),
        (
            mail_to(0x7300_51FF, 10, SystemItem::None),
            "recipient_not_found",
        ),
        (mail_to(rcpt, -1, SystemItem::None), "negative_cash"),
        (
            mail_to(rcpt, i64::from(i32::MAX) + 1, SystemItem::None),
            "cash_too_large",
        ),
        (
            SystemMail {
                subject: String::new(),
                ..mail_to(rcpt, 10, SystemItem::None)
            },
            "too_short",
        ),
    ] {
        let err = send_system_mail(&pool, &mail).await.expect_err(reason);
        assert_eq!(err.reason(), reason, "{err}");
    }
    assert_eq!(mail_count(&pool, rcpt).await, 0);
    assert!(escrow_for(&pool, rcpt).await.is_empty());

    cleanup(&pool, acct).await;
}

/// An existing server-held row (the auction container) moves whole into
/// escrow: same instance id, every per-instance column kept, the seller as
/// `source_character_id`, gone from `sgw_inventory`.
#[tokio::test]
async fn system_mail_moves_server_held_instance() {
    let pool = require_db_or_skip!();
    let (acct, seller, buyer) = (0x7300_5120, 0x7300_5121, 0x7300_5122);
    setup(
        &pool,
        acct,
        &[(seller, "SsuOneHeldSeller"), (buyer, "SsuOneHeldBuyer")],
    )
    .await;
    let (type_id, _) = type_with_stack(&pool, true).await;
    let item = TestItem {
        container_id: INV_AUCTION,
        ..TestItem::main(0x7300_5200, seller, 4, 2)
    };
    insert_item(&pool, item, type_id).await;

    let sent = send_system_mail(
        &pool,
        &mail_to(
            buyer,
            0,
            SystemItem::ExistingInstance {
                item_id: item.item_id,
                owner_player_id: seller,
            },
        ),
    )
    .await
    .expect("server-held instance");

    assert_eq!(inventory_row(&pool, item.item_id).await, None);
    assert_eq!(
        escrow_for(&pool, buyer).await,
        vec![EscrowRow {
            mail_id: sent.mail_id,
            item_id: item.item_id,
            type_id,
            stack_size: 2,
            durability: TEST_DURABILITY,
            charges: TEST_CHARGES,
            bound: false,
            source_character_id: seller,
        }]
    );

    cleanup(&pool, acct).await;
}

/// CAT-G: a caller bug that names a live item is refused. Every container a
/// player holds (the bag, the bandolier, equipment, buyback and the three
/// player vaults) and an id that does not exist: `item_not_server_held` or
/// `item_not_found`, a WARN `mail.system_refused` naming the container and
/// owner, and nothing moves. Fails if `require_server_held` is removed.
#[tokio::test]
async fn system_mail_refuses_instance_in_player_inventory() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (acct, owner, rcpt) = (0x7300_5130, 0x7300_5131, 0x7300_5132);
    setup(
        &pool,
        acct,
        &[(owner, "SsuOneLiveOwner"), (rcpt, "SsuOneLiveRcpt")],
    )
    .await;
    let (type_id, _) = type_with_stack(&pool, false).await;
    let containers = [
        INV_MAIN,
        INV_BANDOLIER,
        INV_CHEST,
        INV_BUYBACK,
        INV_BANK,
        INV_TEAM_BANK,
        INV_COMMAND_BANK,
    ];
    let items: Vec<TestItem> = containers
        .iter()
        .enumerate()
        .map(|(i, c)| TestItem {
            container_id: *c,
            ..TestItem::main(0x7300_5210 + i as i32, owner, 0, 1)
        })
        .collect();
    for item in &items {
        insert_item(&pool, *item, type_id).await;
    }

    for item in &items {
        let err = send_system_mail(
            &pool,
            &mail_to(
                rcpt,
                100,
                SystemItem::ExistingInstance {
                    item_id: item.item_id,
                    owner_player_id: owner,
                },
            ),
        )
        .await
        .expect_err("a player's live item");
        assert!(
            matches!(
                err,
                SystemMailError::ItemNotServerHeld { container_id, owner: o }
                    if container_id == item.container_id && o == owner
            ),
            "container {}: {err}",
            item.container_id
        );
        assert_eq!(inventory_row(&pool, item.item_id).await, Some((owner, 1)));
    }
    let err = send_system_mail(
        &pool,
        &mail_to(
            rcpt,
            0,
            SystemItem::ExistingInstance {
                item_id: 0x7300_52FF,
                owner_player_id: owner,
            },
        ),
    )
    .await
    .expect_err("no such item");
    assert_eq!(err.reason(), "item_not_found");

    assert_eq!(mail_count(&pool, rcpt).await, 0);
    assert!(escrow_for(&pool, rcpt).await.is_empty());
    let row = capture
        .find_event(
            tracing::Level::WARN,
            "system gate-mail refused",
            "item_not_server_held",
        )
        .expect("mail.system_refused reason=item_not_server_held");
    assert!(row.has_field("event", "mail.system_refused"));
    assert!(row.has_field("owner_player_id", &owner.to_string()));

    cleanup(&pool, acct).await;
}

/// Atomicity: a payout written with `send_system_mail_tx` and then rolled
/// back by its caller leaves no mail and no escrow row, and the server-held
/// item is back in its container.
#[tokio::test]
async fn system_mail_rolls_back_with_callers_transaction() {
    let pool = require_db_or_skip!();
    let (acct, seller, buyer) = (0x7300_5140, 0x7300_5141, 0x7300_5142);
    setup(
        &pool,
        acct,
        &[(seller, "SsuOneTxSeller"), (buyer, "SsuOneTxBuyer")],
    )
    .await;
    let (type_id, _) = type_with_stack(&pool, false).await;
    let item = TestItem {
        container_id: INV_AUCTION,
        ..TestItem::main(0x7300_5220, seller, 0, 1)
    };
    insert_item(&pool, item, type_id).await;

    let mut tx = pool.begin().await.unwrap();
    let staged = send_system_mail_tx(
        &mut tx,
        &mail_to(
            buyer,
            250,
            SystemItem::ExistingInstance {
                item_id: item.item_id,
                owner_player_id: seller,
            },
        ),
    )
    .await
    .expect("staged");
    // Inside the transaction the move happened.
    let inside: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sgw_gate_mail_item WHERE mail_id = $1")
            .bind(staged.mail_id)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert_eq!(inside, 1);
    tx.rollback().await.unwrap();

    assert_eq!(mail_count(&pool, buyer).await, 0);
    assert!(escrow_for(&pool, buyer).await.is_empty());
    let container: i32 =
        sqlx::query_scalar("SELECT container_id FROM sgw_inventory WHERE item_id = $1")
            .bind(item.item_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(container, INV_AUCTION);

    cleanup(&pool, acct).await;
}

/// D-SS03: a full mailbox still gets a system mail (a payout is never lost
/// to the cap), and the send logs `mail.system_sent` with the open count
/// and `over_cap`.
#[tokio::test]
async fn system_mail_ignores_mailbox_cap() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (acct, rcpt) = (0x7300_5150, 0x7300_5151);
    setup(&pool, acct, &[(rcpt, "SsuOneFullRcpt")]).await;
    fill_mailbox(&pool, rcpt, 100, 0).await;

    let sent = send_system_mail(&pool, &mail_to(rcpt, 5, SystemItem::None))
        .await
        .expect("a full box still takes a system mail");
    assert_eq!(sent.recipient_open_mail, 101);
    assert_eq!(mail_count(&pool, rcpt).await, 101);
    let row = capture
        .all()
        .into_iter()
        .find(|c| c.has_field("event", "mail.system_sent"))
        .expect("mail.system_sent");
    assert!(row.has_field("mail_id", &sent.mail_id.to_string()));
    assert!(row.has_field("target_player_id", &rcpt.to_string()));
    assert!(row.has_field("over_cap", "true"));

    cleanup(&pool, acct).await;
}

/// A payout that names the wrong seller (an off-by-one listing id, a stale
/// cache) cannot take another seller's listed row, and a bound row only
/// ever goes back to its owner: to a third party it is refused, to the
/// owner it moves. Nothing is written by either refusal. Fails if the owner
/// check or the bound check is removed.
#[tokio::test]
async fn system_mail_existing_instance_owner_and_bound_rules() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (acct, seller, other, buyer) = (0x7300_5158, 0x7300_5159, 0x7300_515A, 0x7300_515B);
    setup(
        &pool,
        acct,
        &[
            (seller, "SsuOneOwnSeller"),
            (other, "SsuOneOwnOther"),
            (buyer, "SsuOneOwnBuyer"),
        ],
    )
    .await;
    let (type_id, _) = type_with_stack(&pool, false).await;
    let listed = TestItem {
        container_id: INV_AUCTION,
        ..TestItem::main(0x7300_5230, seller, 0, 1)
    };
    let bound = TestItem {
        container_id: INV_AUCTION,
        bound: true,
        ..TestItem::main(0x7300_5231, seller, 1, 1)
    };
    insert_item(&pool, listed, type_id).await;
    insert_item(&pool, bound, type_id).await;

    // The caller thinks `other` listed it.
    let err = send_system_mail(
        &pool,
        &mail_to(
            buyer,
            0,
            SystemItem::ExistingInstance {
                item_id: listed.item_id,
                owner_player_id: other,
            },
        ),
    )
    .await
    .expect_err("wrong owner");
    assert_eq!(err.reason(), "item_owner_mismatch", "{err}");
    // A bound row to a buyer.
    let err = send_system_mail(
        &pool,
        &mail_to(
            buyer,
            0,
            SystemItem::ExistingInstance {
                item_id: bound.item_id,
                owner_player_id: seller,
            },
        ),
    )
    .await
    .expect_err("bound to a third party");
    assert_eq!(err.reason(), "item_bound", "{err}");
    assert_eq!(mail_count(&pool, buyer).await, 0);
    assert_eq!(
        inventory_row(&pool, listed.item_id).await,
        Some((seller, 1))
    );
    assert_eq!(inventory_row(&pool, bound.item_id).await, Some((seller, 1)));
    let row = capture
        .find_event(
            tracing::Level::WARN,
            "system gate-mail refused",
            "item_owner_mismatch",
        )
        .expect("mail.system_refused reason=item_owner_mismatch");
    assert!(row.has_field("owner_player_id", &seller.to_string()));
    assert!(row.has_field("expected_owner_player_id", &other.to_string()));

    // The same bound row back to its owner (a cancelled listing) moves.
    send_system_mail(
        &pool,
        &mail_to(
            seller,
            0,
            SystemItem::ExistingInstance {
                item_id: bound.item_id,
                owner_player_id: seller,
            },
        ),
    )
    .await
    .expect("a bound row goes back to its owner");
    assert_eq!(inventory_row(&pool, bound.item_id).await, None);
    let escrow = escrow_for(&pool, seller).await;
    assert_eq!(escrow.len(), 1);
    assert!(escrow[0].bound);

    cleanup(&pool, acct).await;
}
