//! Live-DB guards for `takeCashFromMailMessage` and `takeItemFromMailMessage`
//! (SS-M3, audit § 6 CAT-G-02 and CAT-G-03). Sentinels: accounts, players
//! and entities `0x7300_18xx`, items `0x7300_19xx` and `0x7300_1Axx`.

use std::time::Instant;

use cimmeria_entity::inventory::INV_MAIN;

use super::packets::{Client, Received};
use super::*;
use crate::mercury::method_idx;
use crate::test_support::LogCapture;

const BASE: i32 = 0x7300_1800;
const ITEMS: i32 = 0x7300_1900;

/// Owner (`base + 1`) and sender (`base + 2`) on account `base`.
async fn two_players(pool: &PgPool, base: i32, tag: &str) -> (i32, i32) {
    cleanup(pool, base).await;
    let (owner, sender) = (base + 1, base + 2);
    insert_players(
        pool,
        base,
        &[
            (owner, &format!("SsmThreeO{tag}")),
            (sender, &format!("SsmThreeS{tag}")),
        ],
    )
    .await;
    (owner, sender)
}

/// CAT-G-02: taking a mail's gift cash twice credits it once. The first
/// take credits 500 and zeroes the mail, and the client gets
/// `onCashChanged` with the committed balance and the refreshed header
/// (remove, then the row with `cash` 0). The second take is refused
/// `no_cash` with feedback and changes nothing. Revert that proves it: drop
/// the `SET cash = 0` zeroing (the second take credits 500 again, 2,000).
/// Dropping both `cash > 0` gates (the Rust check and the SQL predicate)
/// also fails it: the second take credits 0 but answers `onCashChanged`
/// and logs a second `mail.cash_taken`.
#[tokio::test]
async fn live_db_take_cash_twice_credits_once() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (owner, sender) = two_players(&pool, BASE, "Cash").await;
    set_naquadah(&pool, owner, 1_000).await;
    let mail_id = AttachedMail::from(owner, sender, "SsmThreeSCash")
        .cash(500)
        .insert(&pool)
        .await;

    let c = Client::new(BASE as u32 + 0x80, owner, 55_100, "SsmThreeOCash");
    let now = Instant::now();
    c.op(MailOp::TakeCash { mail_id }, Some(&pool), now).await;
    let first = c.take();
    assert_eq!(first[0], Received::CashChanged(1_500), "{first:?}");
    assert_eq!(
        first[1],
        Received::Other(method_idx::ON_MAIL_HEADER_REMOVE),
        "{first:?}"
    );
    match &first[2] {
        Received::HeaderInfo { headers, cash, .. } => {
            assert_eq!(headers.len(), 1);
            assert_eq!(headers[0].0, mail_id);
            assert_eq!(cash, &vec![0]);
        }
        other => panic!("expected the refreshed header, got {other:?}"),
    }
    assert_eq!(naquadah(&pool, owner).await, 1_500);
    assert_eq!(mail_state(&pool, mail_id).await, Some((owner, 0, 0, false)));
    let taken = capture
        .all()
        .into_iter()
        .find(|e| e.has_field("event", "mail.cash_taken"))
        .expect("mail.cash_taken");
    for (k, v) in [
        ("naquadah_before", "1000"),
        ("naquadah_after", "1500"),
        ("cash", "500"),
        ("target_player_id", &sender.to_string()),
        ("mail_id", &mail_id.to_string()),
    ] {
        assert!(taken.has_field(k, v), "{k}={v}: {taken:?}");
    }

    c.op(MailOp::TakeCash { mail_id }, Some(&pool), now).await;
    assert_eq!(
        c.take(),
        vec![Received::Feedback(
            "That gate-mail message holds no naquadah.".to_string()
        )]
    );
    assert_eq!(naquadah(&pool, owner).await, 1_500, "credited once");
    assert_refused(&capture, "take_cash", "no_cash", mail_id);
    assert_eq!(
        capture
            .all()
            .iter()
            .filter(|e| e.has_field("event", "mail.cash_taken")
                && e.has_field("mail_id", &mail_id.to_string()))
            .count(),
        1
    );

    cleanup(&pool, BASE).await;
}

/// CAT-G-02 / D-SS09: the cash on an unpaid COD is its price, never gift
/// cash. Take-cash is refused `cod_unpaid`, and the price stays on the mail.
/// Fails when both COD gates are removed, the Rust `mail.cod()` check and
/// the SQL `(flags & MAIL_COD) = 0` predicate (the recipient is paid the
/// price they owe); either alone refuses.
#[tokio::test]
async fn live_db_take_cash_rejects_cod_mail() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (owner, sender) = two_players(&pool, BASE + 0x10, "Cod").await;
    let type_id = any_type_id(&pool).await;
    let mail_id = AttachedMail::from(owner, sender, "SsmThreeSCod")
        .cod(300)
        .item(ITEMS + 0x10, type_id, 1)
        .insert(&pool)
        .await;

    let c = Client::new(BASE as u32 + 0x90, owner, 55_101, "SsmThreeOCod");
    c.op(MailOp::TakeCash { mail_id }, Some(&pool), Instant::now())
        .await;
    assert_eq!(
        c.take(),
        vec![Received::Feedback(
            "The naquadah on a COD message is its price, not a gift. Pay the COD to receive \
             the item, or return the message."
                .to_string()
        )]
    );
    assert_eq!(naquadah(&pool, owner).await, 0);
    assert_eq!(
        mail_state(&pool, mail_id).await,
        Some((owner, 300, crate::cell::mail::codes::flags::MAIL_COD, false))
    );
    assert_refused(&capture, "take_cash", "cod_unpaid", mail_id);
    // The refusal names the mail's sender (the review's telemetry nit).
    assert!(capture.all().iter().any(|e| {
        e.has_field("event", "mail.op_refused")
            && e.has_field("mail_id", &mail_id.to_string())
            && e.has_field("target_player_id", &sender.to_string())
    }));

    cleanup(&pool, BASE + 0x10).await;
}

/// The credit cannot overflow `naquadah` (`integer`): a take that would
/// pass `i32::MAX` is refused `balance_overflow`, and the cash stays in the
/// mail (the zeroing rolls back with the refused credit). Fails when the
/// overflow check is removed (the `UPDATE` errors and the take answers
/// `db_error` instead of the refusal).
#[tokio::test]
async fn live_db_take_cash_refuses_balance_overflow() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (owner, sender) = two_players(&pool, BASE + 0x20, "Ovf").await;
    set_naquadah(&pool, owner, i32::MAX - 10).await;
    let mail_id = AttachedMail::from(owner, sender, "SsmThreeSOvf")
        .cash(500)
        .insert(&pool)
        .await;

    let c = Client::new(BASE as u32 + 0xA0, owner, 55_102, "SsmThreeOOvf");
    c.op(MailOp::TakeCash { mail_id }, Some(&pool), Instant::now())
        .await;
    assert!(matches!(&c.take()[..], [Received::Feedback(_)]));
    assert_eq!(naquadah(&pool, owner).await, i32::MAX - 10);
    assert_eq!(
        mail_state(&pool, mail_id).await,
        Some((owner, 500, 0, false))
    );
    assert_refused(&capture, "take_cash", "balance_overflow", mail_id);

    cleanup(&pool, BASE + 0x20).await;
}

/// CAT-G-03: an item is taken once. The first take restores the escrowed
/// instance, every instance column as it was in escrow, into the owner's
/// first free main slot and deletes the escrow row; the second is refused
/// `no_item`.
/// Fails when the escrow `DELETE` is removed (the second take inserts a
/// duplicate or errors).
#[tokio::test]
async fn live_db_take_item_twice_moves_once() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (owner, sender) = two_players(&pool, BASE + 0x30, "Twice").await;
    let type_id = any_type_id(&pool).await;
    let item_id = ITEMS + 0x30;
    let mail_id = AttachedMail::from(owner, sender, "SsmThreeSTwice")
        .item(item_id, type_id, 4)
        .insert(&pool)
        .await;

    let c = Client::new(BASE as u32 + 0xB0, owner, 55_103, "SsmThreeOTwice");
    let now = Instant::now();
    c.op(
        MailOp::TakeItem {
            mail_id,
            container_id: -1,
            slot_id: -1,
        },
        Some(&pool),
        now,
    )
    .await;
    let first = c.take();
    assert!(
        first
            .iter()
            .any(|r| matches!(r, Received::UpdateItem(items) if items.contains(&(item_id, 4)))),
        "the owner's client is sent the item: {first:?}"
    );
    assert_eq!(
        inventory_rows(&pool, item_id).await,
        vec![(owner, INV_MAIN, 0, 4)]
    );
    // Every instance column survives, including the two that live on
    // `sgw_inventory_base` (the fixture sets each off its default).
    type Restored = (i32, i32, i32, i32, bool, i32, i32, String, String);
    let restored: Restored = sqlx::query_as(
        "SELECT type_id, durability, charges, flags, bound, ammo, cur_ammo_type, \
                ammo_type::text, ammo_types::text \
         FROM sgw_inventory WHERE item_id = $1",
    )
    .bind(item_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        restored,
        (
            type_id,
            TEST_DURABILITY,
            TEST_CHARGES,
            5,
            true,
            9,
            2,
            "Bullet_EMP".to_string(),
            "{Bullet_Default,Bullet_EMP}".to_string()
        )
    );
    assert!(!has_escrow(&pool, mail_id).await);
    assert!(capture
        .all()
        .iter()
        .any(|e| e.has_field("event", "mail.item_taken")
            && e.has_field("item_id", &item_id.to_string())
            && e.has_field("slot_id", "0")));

    c.op(
        MailOp::TakeItem {
            mail_id,
            container_id: -1,
            slot_id: -1,
        },
        Some(&pool),
        now,
    )
    .await;
    assert_eq!(
        c.take(),
        vec![Received::Feedback(
            "That gate-mail message holds no item.".to_string()
        )]
    );
    assert_eq!(inventory_rows(&pool, item_id).await.len(), 1);
    assert_refused(&capture, "take_item", "no_item", mail_id);

    cleanup(&pool, BASE + 0x30).await;
}

/// CAT-G-03 / SS-E1 M-Q5: the client's `ContainerId` and `SlotId` are never
/// read. A request naming a vault container, an occupied slot, or stack
/// garbage still lands in the caller's own first free main-bag slot (here
/// slot 2, after the caller's items in 0 and 1). Fails if the handler honours
/// the client's container or slot.
#[tokio::test]
async fn live_db_take_item_ignores_client_container_and_slot() {
    let pool = require_db_or_skip!();
    let (owner, sender) = two_players(&pool, BASE + 0x40, "Slot").await;
    let type_id = any_type_id(&pool).await;
    insert_item(&pool, TestItem::main(ITEMS + 0x40, owner, 0, 1), type_id).await;
    insert_item(&pool, TestItem::main(ITEMS + 0x41, owner, 1, 1), type_id).await;
    let c = Client::new(BASE as u32 + 0xC0, owner, 55_104, "SsmThreeOSlot");
    // A vault container and an occupied slot, then uninitialised stack.
    for (n, (container_id, slot_id)) in [(17, 0), (0x5A5A_5A5A, -0x2121_2122)]
        .into_iter()
        .enumerate()
    {
        let item_id = ITEMS + 0x48 + n as i32;
        let mail_id = AttachedMail::from(owner, sender, "SsmThreeSSlot")
            .item(item_id, type_id, 1)
            .insert(&pool)
            .await;
        c.op(
            MailOp::TakeItem {
                mail_id,
                container_id,
                slot_id,
            },
            Some(&pool),
            Instant::now(),
        )
        .await;
        c.take();
        assert_eq!(
            inventory_rows(&pool, item_id).await,
            vec![(owner, INV_MAIN, 2 + n as i32, 1)],
            "request ({container_id}, {slot_id})"
        );
    }

    cleanup(&pool, BASE + 0x40).await;
}

/// CAT-G-03: only the mail's owner can take its item, and a take writes
/// nowhere but the owner's own inventory. Another character naming the
/// mail id is refused `not_found_for_owner` (feedback plus a header remove
/// for its stale list), the escrow row is untouched, and that character's
/// inventory gains nothing. Fails when `lock_mail` drops the
/// `character_id` scope.
#[tokio::test]
async fn live_db_take_item_never_writes_outside_callers_inventory() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (owner, sender) = two_players(&pool, BASE + 0x50, "Own").await;
    let type_id = any_type_id(&pool).await;
    let item_id = ITEMS + 0x50;
    let mail_id = AttachedMail::from(owner, sender, "SsmThreeSOwn")
        .cash(40)
        .item(item_id, type_id, 1)
        .insert(&pool)
        .await;

    let intruder = Client::new(BASE as u32 + 0xD0, sender, 55_105, "SsmThreeSOwn");
    let now = Instant::now();
    intruder
        .op(
            MailOp::TakeItem {
                mail_id,
                container_id: INV_MAIN,
                slot_id: 0,
            },
            Some(&pool),
            now,
        )
        .await;
    intruder
        .op(MailOp::TakeCash { mail_id }, Some(&pool), now)
        .await;
    let seen = intruder.take();
    assert_eq!(seen.len(), 4, "{seen:?}");
    assert!(seen.contains(&Received::Other(method_idx::ON_MAIL_HEADER_REMOVE)));
    assert_eq!(inventory_rows(&pool, item_id).await, vec![]);
    assert!(has_escrow(&pool, mail_id).await);
    assert_eq!(naquadah(&pool, sender).await, 0);
    assert_eq!(
        mail_state(&pool, mail_id).await,
        Some((owner, 40, 0, false))
    );
    assert_refused(&capture, "take_item", "not_found_for_owner", mail_id);
    assert_refused(&capture, "take_cash", "not_found_for_owner", mail_id);

    let c = Client::new(BASE as u32 + 0xD1, owner, 55_106, "SsmThreeOOwn");
    c.op(
        MailOp::TakeItem {
            mail_id,
            container_id: -1,
            slot_id: -1,
        },
        Some(&pool),
        now,
    )
    .await;
    assert_eq!(
        inventory_rows(&pool, item_id).await,
        vec![(owner, INV_MAIN, 0, 1)]
    );

    cleanup(&pool, BASE + 0x50).await;
}

/// CAT-G-03 / owner constraint (Bank campaign): a full backpack leaves the
/// item in escrow and says so; it never spills into another container
/// (a vault or the bandolier). Fails when the free-slot reservation is
/// skipped (the insert lands in an occupied slot and the unique slot index
/// errors, answered as `db_error`).
#[tokio::test]
async fn live_db_take_item_full_bags_keeps_escrow() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (owner, sender) = two_players(&pool, BASE + 0x60, "Full").await;
    let type_id = any_type_id(&pool).await;
    sqlx::query(
        "INSERT INTO sgw_inventory \
            (item_id, character_id, type_id, stack_size, container_id, slot_id) \
         SELECT $1 + s, $2, $3, 1, $4, s FROM generate_series(0, 39) AS s",
    )
    .bind(0x7300_1A00)
    .bind(owner)
    .bind(type_id)
    .bind(INV_MAIN)
    .execute(&pool)
    .await
    .unwrap();
    let item_id = ITEMS + 0x60;
    let mail_id = AttachedMail::from(owner, sender, "SsmThreeSFull")
        .item(item_id, type_id, 1)
        .insert(&pool)
        .await;

    let c = Client::new(BASE as u32 + 0xE0, owner, 55_107, "SsmThreeOFull");
    c.op(
        MailOp::TakeItem {
            mail_id,
            container_id: -1,
            slot_id: -1,
        },
        Some(&pool),
        Instant::now(),
    )
    .await;
    assert_eq!(
        c.take(),
        vec![Received::Feedback(
            "Your backpack is full. Make room and take the item again; it stays in the \
             message until then."
                .to_string()
        )]
    );
    assert_eq!(inventory_rows(&pool, item_id).await, vec![]);
    assert!(has_escrow(&pool, mail_id).await);
    assert_refused(&capture, "take_item", "bags_full", mail_id);

    cleanup(&pool, BASE + 0x60).await;
}

/// Type 12: a take that fails in the database answers with feedback on the
/// first press, logs ERROR `mail.op_failed` with `op`, `mail_id` and
/// `reason = db_error` (plus the rule-5 identity), and rolls back whole. The
/// failure is real, not injected: the escrowed instance id already exists in
/// `sgw_inventory` under a third character, so the restore hits the
/// inventory key. Fails when `answer_failure` stops logging the event.
#[tokio::test]
async fn live_db_take_item_db_failure_logs_op_failed_and_keeps_escrow() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (owner, sender) = two_players(&pool, BASE + 0x70, "Fail").await;
    let type_id = any_type_id(&pool).await;
    let item_id = ITEMS + 0x70;
    // The clashing row belongs to the sender, in a slot the owner's take
    // would not use.
    insert_item(&pool, TestItem::main(item_id, sender, 5, 1), type_id).await;
    let mail_id = AttachedMail::from(owner, sender, "SsmThreeSFail")
        .item(item_id, type_id, 1)
        .insert(&pool)
        .await;

    let c = Client::new(BASE as u32 + 0xF0, owner, 55_108, "SsmThreeOFail");
    c.op(
        MailOp::TakeItem {
            mail_id,
            container_id: -1,
            slot_id: -1,
        },
        Some(&pool),
        Instant::now(),
    )
    .await;
    assert_eq!(
        c.take(),
        vec![Received::Feedback(
            "The gate-mail request could not be completed. Nothing was changed.".to_string()
        )]
    );
    let ev = capture
        .all()
        .into_iter()
        .find(|e| {
            e.level == tracing::Level::ERROR
                && e.has_field("event", "mail.op_failed")
                && e.has_field("op", "take_item")
                && e.has_field("reason", "db_error")
                && e.has_field("mail_id", &mail_id.to_string())
        })
        .expect("mail.op_failed op=take_item reason=db_error");
    for key in ["account_id", "player_id", "entity_id", "error"] {
        assert!(ev.fields.contains_key(key), "{key} missing: {ev:?}");
    }
    assert!(has_escrow(&pool, mail_id).await, "rolled back: escrow kept");
    assert_eq!(
        inventory_rows(&pool, item_id).await,
        vec![(sender, INV_MAIN, 5, 1)],
        "nothing written for the owner"
    );

    cleanup(&pool, BASE + 0x70).await;
}
