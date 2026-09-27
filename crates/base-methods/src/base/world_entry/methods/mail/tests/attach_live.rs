//! Live-DB guards for gate mail with cash, an item or COD attached (SS-M2):
//! the D-SS06 transaction, the D-SS02 postage, the D-SS08 escrow and the
//! CAT-G-01 item checks. The rollback guards are in `attach_rollback`.
//! Sentinels `0x7300_14xx` (accounts, players,
//! entities) and `0x7300_16xx` (item instances).

use std::time::{Duration, Instant};

use super::packets::{plain_send, Client, Received};
use super::*;
use crate::cell::mail::codes::{flags, MailResult};
use crate::cell::messages::MailSend;
use crate::test_support::LogCapture;
use cimmeria_entity::inventory::{INV_BANDOLIER, INV_CHEST};

const BASE: i32 = 0x7300_1400;
const ITEMS: i32 = 0x7300_1600;

/// What one send got back.
#[derive(Debug, Default)]
pub(super) struct Reply {
    pub(super) code: Option<u8>,
    pub(super) lines: Vec<String>,
    pub(super) cash: Vec<i32>,
    pub(super) removed: Vec<i32>,
    pub(super) inventory_updates: usize,
    /// The `(id, stackSize)` list of the last `onUpdateItem`.
    pub(super) listed: Vec<(i32, i32)>,
}

pub(super) fn reply(received: Vec<Received>) -> Reply {
    let mut r = Reply::default();
    for p in received {
        match p {
            Received::SendMailResult { result, .. } => {
                assert!(r.code.is_none(), "exactly one sendMailResult per send");
                r.code = Some(result);
            }
            Received::Feedback(text) => r.lines.push(text),
            Received::CashChanged(total) => r.cash.push(total),
            Received::RemoveItem(ids) => r.removed.extend(ids),
            Received::UpdateItem(items) => {
                r.inventory_updates += 1;
                r.listed = items;
            }
            _ => {}
        }
    }
    r
}

pub(super) fn attached(to: &str, cash: i32, cod: bool, item_id: i32, quantity: i32) -> MailSend {
    let mut send = plain_send(&[to]);
    send.cash = cash;
    send.cod = cod;
    send.item_id = item_id;
    send.item_quantity = quantity;
    send
}

#[derive(Debug, PartialEq, Eq, sqlx::FromRow)]
struct MailCashRow {
    mail_id: i32,
    cash: i64,
    flags: i32,
    item_id: Option<i32>,
}

async fn mails(pool: &PgPool, character_id: i32) -> Vec<MailCashRow> {
    sqlx::query_as(
        "SELECT mail_id, cash, flags, item_id FROM sgw_gate_mail \
         WHERE character_id = $1 ORDER BY mail_id",
    )
    .bind(character_id)
    .fetch_all(pool)
    .await
    .unwrap()
}

/// Two players, `sender` holding `balance`. Returns the item type used.
pub(super) async fn setup(
    pool: &PgPool,
    acct: i32,
    sender: (i32, &str),
    rcpt: (i32, &str),
    balance: i32,
) -> i32 {
    cleanup(pool, acct).await;
    insert_players(pool, acct, &[sender, rcpt]).await;
    set_naquadah(pool, sender.0, balance).await;
    any_type_id(pool).await
}

/// CAT-G-01 / D-SS02 / D-SS06 / D-SS08, end to end:
///
/// 1. gift 300 plus a whole one-item stack: 325 debited, the mail holds 300,
///    the row leaves the bag with every column into escrow, and the client
///    gets the balance, `onRemoveItem` and an inventory list;
/// 2. COD 500 on two of a five-stack: only postage is debited, the mail is
///    flagged COD with the price, the stack drops to three and a new
///    instance id holds two in escrow; no `onRemoveItem`;
/// 3. one naquadah short of cash plus postage: `NotEnoughCash`, nothing
///    changes; exactly enough: sent, balance 0.
#[tokio::test]
async fn send_debits_cash_and_postage_atomically() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (acct, sender, rcpt) = (BASE, BASE + 1, BASE + 2);
    let type_id = setup(
        &pool,
        acct,
        (sender, "SsmTwoPaySend"),
        (rcpt, "SsmTwoPayRcpt"),
        1_000,
    )
    .await;
    let whole = TestItem::main(ITEMS, sender, 0, 1);
    let stack = TestItem::main(ITEMS + 1, sender, 1, 5);
    insert_item(&pool, whole, type_id).await;
    insert_item(&pool, stack, type_id).await;

    let c = Client::new(0x7300_1481, sender, 54_741, "SsmTwoPaySend");
    let t0 = Instant::now();

    // 1. Gift cash plus the whole row.
    c.op(
        MailOp::Send(attached("SsmTwoPayRcpt", 300, false, whole.item_id, 1)),
        Some(&pool),
        t0,
    )
    .await;
    let r = reply(c.take());
    assert_eq!(r.code, Some(MailResult::Sent.code()), "{r:?}");
    assert_eq!(
        r.cash,
        vec![675],
        "onCashChanged carries the committed balance"
    );
    assert_eq!(
        r.removed,
        vec![whole.item_id],
        "the whole row leaves the bag"
    );
    assert_eq!(r.inventory_updates, 1, "{r:?}");
    assert_eq!(
        r.listed,
        vec![(stack.item_id, 5)],
        "the bag list no longer carries the mailed item"
    );
    // Owner telemetry rule: every cash and item change logs before and
    // after, with the correlating ids and the recipient.
    let mail_id = mails(&pool, rcpt).await[0].mail_id.to_string();
    let cash_ev = capture
        .find_message(
            tracing::Level::INFO,
            "postage and attached naquadah debited",
        )
        .expect("mail.cash_debited");
    let item_ev = capture
        .find_message(
            tracing::Level::INFO,
            "moved from the sender's bag into escrow",
        )
        .expect("mail.item_escrowed");
    for (ev, want) in [
        (
            &cash_ev,
            vec![
                ("naquadah_before", "1000".to_string()),
                ("naquadah_after", "675".to_string()),
                ("mail_id", mail_id.clone()),
                ("target_player_id", rcpt.to_string()),
            ],
        ),
        (
            &item_ev,
            vec![
                ("item_id", whole.item_id.to_string()),
                ("stack_before", "1".to_string()),
                ("stack_after", "0".to_string()),
                ("mail_id", mail_id.clone()),
                ("target_player_id", rcpt.to_string()),
            ],
        ),
    ] {
        for (key, value) in want {
            assert!(ev.has_field(key, &value), "{key}={value}: {ev:?}");
        }
        for key in ["account_id", "player_id", "entity_id"] {
            assert!(ev.fields.contains_key(key), "{key} missing: {ev:?}");
        }
    }
    assert_eq!(naquadah(&pool, sender).await, 675, "300 gift + 25 postage");
    assert_eq!(
        naquadah(&pool, rcpt).await,
        0,
        "the recipient is paid on take (SS-M3)"
    );
    assert_eq!(inventory_row(&pool, whole.item_id).await, None);
    let m = mails(&pool, rcpt).await;
    assert_eq!(m.len(), 1);
    assert_eq!((m[0].cash, m[0].flags, m[0].item_id), (300, 0, None));
    let escrow = escrow_for(&pool, rcpt).await;
    assert_eq!(
        escrow,
        vec![EscrowRow {
            mail_id: m[0].mail_id,
            item_id: whole.item_id,
            type_id,
            stack_size: 1,
            durability: TEST_DURABILITY,
            charges: TEST_CHARGES,
            bound: false,
            source_character_id: sender,
        }],
        "a whole-row move keeps the instance id and every column"
    );

    // 2. COD on part of a stack.
    c.op(
        MailOp::Send(attached("SsmTwoPayRcpt", 500, true, stack.item_id, 2)),
        Some(&pool),
        t0 + Duration::from_secs(11),
    )
    .await;
    let r = reply(c.take());
    assert_eq!(r.code, Some(MailResult::Sent.code()), "{r:?}");
    assert_eq!(r.cash, vec![650], "COD pays postage only");
    assert!(r.removed.is_empty(), "a split leaves the row in the bag");
    assert_eq!(r.inventory_updates, 1);
    assert_eq!(
        r.listed,
        vec![(stack.item_id, 3)],
        "the client is told the committed stack, and never sees the escrowed id"
    );
    assert_eq!(naquadah(&pool, sender).await, 650);
    assert_eq!(inventory_row(&pool, stack.item_id).await, Some((sender, 3)));
    let m = mails(&pool, rcpt).await;
    assert_eq!((m[1].cash, m[1].flags), (500, flags::MAIL_COD));
    let escrow = escrow_for(&pool, rcpt).await;
    assert_eq!(escrow.len(), 2);
    assert_eq!(escrow[1].mail_id, m[1].mail_id);
    assert_eq!(escrow[1].stack_size, 2);
    assert_ne!(
        escrow[1].item_id, stack.item_id,
        "a split gets a fresh instance id"
    );
    assert_eq!(
        (escrow[1].durability, escrow[1].charges),
        (TEST_DURABILITY, TEST_CHARGES)
    );

    // 3. The postage boundary: 626 + 25 > 650, 625 + 25 = 650.
    c.op(
        MailOp::Send(attached("SsmTwoPayRcpt", 626, false, 0, 0)),
        Some(&pool),
        t0 + Duration::from_secs(22),
    )
    .await;
    let r = reply(c.take());
    assert_eq!(r.code, Some(MailResult::NotEnoughCash.code()), "{r:?}");
    assert_eq!(r.lines.len(), 1);
    assert!(r.cash.is_empty());
    assert_eq!(naquadah(&pool, sender).await, 650);
    assert_eq!(mail_count(&pool, rcpt).await, 2);

    c.op(
        MailOp::Send(attached("SsmTwoPayRcpt", 625, false, 0, 0)),
        Some(&pool),
        t0 + Duration::from_secs(33),
    )
    .await;
    let r = reply(c.take());
    assert_eq!(r.code, Some(MailResult::Sent.code()), "{r:?}");
    assert_eq!(r.cash, vec![0]);
    assert_eq!(naquadah(&pool, sender).await, 0);
    assert_eq!(r.inventory_updates, 0, "cash only: the bag is untouched");

    cleanup(&pool, acct).await;
}

/// A refused attached send changes nothing anywhere: the sender's balance,
/// both inventories, both mailboxes and the escrow table.
pub(super) async fn assert_untouched(
    pool: &PgPool,
    sender: i32,
    rcpt: i32,
    balance: i32,
    items: &[TestItem],
) {
    assert_eq!(naquadah(pool, sender).await, balance, "nothing debited");
    for item in items {
        assert_eq!(
            inventory_row(pool, item.item_id).await,
            Some((item.owner, item.stack_size)),
            "item {} untouched",
            item.item_id
        );
    }
    assert_eq!(mail_count(pool, rcpt).await, 0, "no mail");
    assert_eq!(mail_count(pool, sender).await, 0, "no mail");
    assert_eq!(escrow_from(pool, sender).await, 0, "no escrow");
    assert_eq!(escrow_from(pool, rcpt).await, 0, "no escrow");
}

/// CAT-G-01: an `ItemId` the sender does not hold (here the recipient's
/// own item) is `ItemNotAvailable`, and neither player's anything moves.
/// The item is looked up with the sender's `character_id`; keyed on the
/// id alone, this would mail the recipient's item to themselves at the
/// sender's cost.
#[tokio::test]
async fn send_rejects_item_not_owned() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (acct, sender, rcpt) = (BASE + 10, BASE + 11, BASE + 12);
    let type_id = setup(
        &pool,
        acct,
        (sender, "SsmTwoOwnSend"),
        (rcpt, "SsmTwoOwnRcpt"),
        500,
    )
    .await;
    let theirs = TestItem::main(ITEMS + 10, rcpt, 0, 1);
    insert_item(&pool, theirs, type_id).await;

    let c = Client::new(0x7300_1482, sender, 54_742, "SsmTwoOwnSend");
    c.op(
        MailOp::Send(attached("SsmTwoOwnRcpt", 0, false, theirs.item_id, 1)),
        Some(&pool),
        Instant::now(),
    )
    .await;
    let r = reply(c.take());
    assert_eq!(r.code, Some(MailResult::ItemNotAvailable.code()), "{r:?}");
    assert_eq!(
        r.lines,
        vec!["The attached item is no longer in your bags. The message was not sent.".to_string()]
    );
    assert!(r.cash.is_empty() && r.removed.is_empty());
    assert!(capture
        .find_event(
            tracing::Level::WARN,
            "sendMailMessage refused",
            "item_not_owned"
        )
        .is_some());
    assert_untouched(&pool, sender, rcpt, 500, &[theirs]).await;

    cleanup(&pool, acct).await;
}

/// CAT-G-01 / D-SS08: a bound item is `ItemNotAvailable`, nothing moves.
#[tokio::test]
async fn send_rejects_bound_item() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (acct, sender, rcpt) = (BASE + 20, BASE + 21, BASE + 22);
    let type_id = setup(
        &pool,
        acct,
        (sender, "SsmTwoBndSend"),
        (rcpt, "SsmTwoBndRcpt"),
        500,
    )
    .await;
    let bound = TestItem {
        bound: true,
        ..TestItem::main(ITEMS + 20, sender, 0, 1)
    };
    insert_item(&pool, bound, type_id).await;

    let c = Client::new(0x7300_1483, sender, 54_743, "SsmTwoBndSend");
    c.op(
        MailOp::Send(attached("SsmTwoBndRcpt", 10, false, bound.item_id, 1)),
        Some(&pool),
        Instant::now(),
    )
    .await;
    let r = reply(c.take());
    assert_eq!(r.code, Some(MailResult::ItemNotAvailable.code()), "{r:?}");
    assert_eq!(r.lines.len(), 1);
    // Both refusal rows name the resolved recipient (rule 5), not only the
    // sender.
    let refused = capture
        .find_event(
            tracing::Level::WARN,
            "sendMailMessage refused",
            "item_bound",
        )
        .expect("mail.send_refused reason=item_bound");
    assert!(
        refused.has_field("target_player_id", &rcpt.to_string()),
        "{refused:?}"
    );
    let detail = capture
        .all()
        .into_iter()
        .find(|e| e.has_field("event", "mail.attachment_refused"))
        .expect("mail.attachment_refused");
    assert!(
        detail.has_field("target_player_id", &rcpt.to_string()),
        "{detail:?}"
    );
    assert_untouched(&pool, sender, rcpt, 500, &[bound]).await;

    cleanup(&pool, acct).await;
}

/// D-SS08 plus the trade allowlist: only the main bag can be mailed. An
/// item on the bandolier or equipped is refused, and so is asking for more
/// than the stack holds. Nothing moves in any case.
#[tokio::test]
async fn send_rejects_item_outside_main_bag_or_over_stack() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (acct, sender, rcpt) = (BASE + 30, BASE + 31, BASE + 32);
    let type_id = setup(
        &pool,
        acct,
        (sender, "SsmTwoBagSend"),
        (rcpt, "SsmTwoBagRcpt"),
        500,
    )
    .await;
    let bandolier = TestItem {
        container_id: INV_BANDOLIER,
        ..TestItem::main(ITEMS + 30, sender, 0, 1)
    };
    let equipped = TestItem {
        container_id: INV_CHEST,
        ..TestItem::main(ITEMS + 31, sender, 0, 1)
    };
    let stack = TestItem::main(ITEMS + 32, sender, 0, 2);
    for item in [bandolier, equipped, stack] {
        insert_item(&pool, item, type_id).await;
    }

    let c = Client::new(0x7300_1484, sender, 54_744, "SsmTwoBagSend");
    let t0 = Instant::now();
    let cases = [
        (bandolier.item_id, 1, "item_not_in_main_bag"),
        (equipped.item_id, 1, "item_not_in_main_bag"),
        (stack.item_id, 3, "item_quantity_exceeds_stack"),
    ];
    for (i, (item_id, quantity, reason)) in cases.into_iter().enumerate() {
        c.op(
            MailOp::Send(attached("SsmTwoBagRcpt", 0, false, item_id, quantity)),
            Some(&pool),
            t0 + Duration::from_secs(11 * i as u64),
        )
        .await;
        let r = reply(c.take());
        assert_eq!(
            r.code,
            Some(MailResult::ItemNotAvailable.code()),
            "{reason}"
        );
        assert_eq!(r.lines.len(), 1, "{reason}");
        assert!(
            capture
                .find_event(tracing::Level::WARN, "sendMailMessage refused", reason)
                .is_some(),
            "{reason}"
        );
    }
    assert_untouched(&pool, sender, rcpt, 500, &[bandolier, equipped, stack]).await;

    cleanup(&pool, acct).await;
}

/// SS-E1 M-Q4: the recipient's header list carries one `MessageAttachment`
/// per mail holding an item (`id` = mail id, `itemId` = the type id, the
/// escrowed stack, durability and charges), the cash on every header, and
/// `MAIL_COD` on the COD mail. A mail with cash only has no attachment.
#[tokio::test]
async fn headers_carry_attachment_and_cod_flag() {
    let pool = require_db_or_skip!();
    let (acct, sender, rcpt) = (BASE + 50, BASE + 51, BASE + 52);
    let type_id = setup(
        &pool,
        acct,
        (sender, "SsmTwoHdrSend"),
        (rcpt, "SsmTwoHdrRcpt"),
        1_000,
    )
    .await;
    let stack = TestItem::main(ITEMS + 50, sender, 0, 6);
    insert_item(&pool, stack, type_id).await;

    let s = Client::new(0x7300_1486, sender, 54_746, "SsmTwoHdrSend");
    let t0 = Instant::now();
    s.op(
        MailOp::Send(attached("SsmTwoHdrRcpt", 40, false, 0, 0)),
        Some(&pool),
        t0,
    )
    .await;
    s.op(
        MailOp::Send(attached("SsmTwoHdrRcpt", 90, true, stack.item_id, 4)),
        Some(&pool),
        t0 + Duration::from_secs(1),
    )
    .await;
    s.take();
    let m = mails(&pool, rcpt).await;
    let (gift_id, cod_id) = (m[0].mail_id, m[1].mail_id);

    let r = Client::new(0x7300_1487, rcpt, 54_747, "SsmTwoHdrRcpt");
    r.op(MailOp::RequestHeaders { b_archive: 0 }, Some(&pool), t0)
        .await;
    match r.take().as_slice() {
        [Received::HeaderInfo {
            headers,
            cash,
            attachments,
            ..
        }] => {
            // Newest first.
            assert_eq!(headers, &vec![(cod_id, flags::MAIL_COD), (gift_id, 0)]);
            assert_eq!(cash, &vec![90, 40]);
            assert_eq!(
                attachments,
                &vec![[cod_id, type_id, 4, TEST_DURABILITY, TEST_CHARGES]],
                "one attachment, joined by mail id, carrying the type id"
            );
        }
        other => panic!("expected one onMailHeaderInfo, got {other:?}"),
    }

    cleanup(&pool, acct).await;
}

/// D-SS08 / audit A-14: once escrowed, an item is in neither player's
/// inventory as `INVENTORY_ITEM_SELECT` (the world-entry load and every
/// resync) reads it: not the sender's (the row moved), not the
/// recipient's (it is theirs only on take). A split leaves only the
/// remainder in the sender's list.
#[tokio::test]
async fn escrowed_item_absent_from_inventory_select() {
    let pool = require_db_or_skip!();
    let (acct, sender, rcpt) = (BASE + 60, BASE + 61, BASE + 62);
    let type_id = setup(
        &pool,
        acct,
        (sender, "SsmTwoSelSend"),
        (rcpt, "SsmTwoSelRcpt"),
        500,
    )
    .await;
    let whole = TestItem::main(ITEMS + 60, sender, 0, 1);
    let stack = TestItem::main(ITEMS + 61, sender, 1, 3);
    insert_item(&pool, whole, type_id).await;
    insert_item(&pool, stack, type_id).await;

    let c = Client::new(0x7300_1488, sender, 54_748, "SsmTwoSelSend");
    let t0 = Instant::now();
    c.op(
        MailOp::Send(attached("SsmTwoSelRcpt", 0, false, whole.item_id, 1)),
        Some(&pool),
        t0,
    )
    .await;
    c.op(
        MailOp::Send(attached("SsmTwoSelRcpt", 0, false, stack.item_id, 1)),
        Some(&pool),
        t0 + Duration::from_secs(1),
    )
    .await;
    c.take();
    let escrowed: Vec<i32> = escrow_for(&pool, rcpt)
        .await
        .iter()
        .map(|e| e.item_id)
        .collect();
    assert_eq!(escrowed.len(), 2, "both sends escrowed an item");

    let select = super::super::super::inventory::core::INVENTORY_ITEM_SELECT;
    for (player, want) in [(sender, vec![(stack.item_id, 2)]), (rcpt, vec![])] {
        let listed: Vec<(i32, i32)> = sqlx::query(select)
            .bind(player)
            .fetch_all(&pool)
            .await
            .unwrap()
            .iter()
            .map(|row| {
                use sqlx::Row;
                (row.get("item_id"), row.get("stack_size"))
            })
            .collect();
        for id in &escrowed {
            assert!(
                listed.iter().all(|(listed_id, _)| listed_id != id),
                "escrowed item {id} listed for player {player}: {listed:?}"
            );
        }
        assert_eq!(listed, want, "player {player}");
    }

    cleanup(&pool, acct).await;
}
