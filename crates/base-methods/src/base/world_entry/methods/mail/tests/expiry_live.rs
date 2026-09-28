//! Live-DB guards for mail expiry (SS-M4, D-SS04): the TTL every writer
//! sets, and the three terminal paths the sweep takes, on an injected
//! clock. Sentinels: accounts, players and entities `0x7300_2000` up, items
//! `0x7300_2080` up.
//!
//! The sweeps run at `now = NOW`, and the tests set `expires_at` at or
//! below it, so only these rows are due: every real writer stamps a
//! present-day `expires_at`, far above `NOW`.

use std::time::Instant;

use super::super::expiry::{sweep_due, sweep_mailbox, MAIL_TTL_SECS};
use super::super::system::{send_system_mail, SystemItem, SystemMail};
use super::packets::{plain_send, Client};
use super::*;
use crate::cell::mail::codes::flags::{MAIL_ARCHIVE, MAIL_COD};
use crate::cell::messages::{MailGmActor, MailGmCellToBase, MailOp};
use crate::test_support::LogCapture;

const BASE: i32 = 0x7300_2000;
const ITEMS: i32 = 0x7300_2080;

/// The injected clock: well after every `expires_at` a test sets, well
/// before any a real writer stamps.
const NOW: i32 = 1_000_000;

/// Owner (`base + 1`) and sender (`base + 2`) on account `base`.
async fn two_players(pool: &PgPool, base: i32, tag: &str) -> (i32, i32) {
    cleanup(pool, base).await;
    let (owner, sender) = (base + 1, base + 2);
    insert_players(
        pool,
        base,
        &[
            (owner, &format!("SsmFourExpO{tag}")),
            (sender, &format!("SsmFourExpS{tag}")),
        ],
    )
    .await;
    (owner, sender)
}

/// The newest mail in `character_id`'s mailbox.
async fn newest(pool: &PgPool, character_id: i32) -> i32 {
    sqlx::query_scalar::<_, Option<i32>>(
        "SELECT MAX(mail_id) FROM sgw_gate_mail WHERE character_id = $1",
    )
    .bind(character_id)
    .fetch_one(pool)
    .await
    .unwrap()
    .expect("a mail")
}

fn assert_ttl(row: &ExpiryRow, what: &str) {
    assert_eq!(
        row.expires_at,
        Some(row.sent_time + MAIL_TTL_SECS),
        "{what}: expires_at is sent_time + 720 h: {row:?}"
    );
}

/// D-SS04: every writer stamps `expires_at = sent_time + 720 h` at insert,
/// and a return restamps it with the fresh `sent_time`: a text send, a
/// send with cash, the COD payment mail, a player return, server mail, and
/// the GM's COD mail. Fails if any one writer leaves it NULL (that mail
/// would never expire and the client's Expires column would lie).
#[tokio::test]
async fn live_db_every_writer_sets_expires_at() {
    let pool = require_db_or_skip!();
    let (owner, sender) = two_players(&pool, BASE, "Ttl").await;
    set_naquadah(&pool, owner, 1_000).await;
    set_naquadah(&pool, sender, 1_000).await;
    let type_id = any_type_id(&pool).await;
    let s = Client::new(BASE as u32 + 0x40, sender, 55_200, "SsmFourExpSTtl");
    let o = Client::new(BASE as u32 + 0x41, owner, 55_201, "SsmFourExpOTtl");
    let now = Instant::now();

    s.op(
        MailOp::Send(plain_send(&["SsmFourExpOTtl"])),
        Some(&pool),
        now,
    )
    .await;
    let text = newest(&pool, owner).await;
    assert_ttl(&expiry_row(&pool, text).await.unwrap(), "text send");

    let mut with_cash = plain_send(&["SsmFourExpOTtl"]);
    with_cash.cash = 10;
    s.op(MailOp::Send(with_cash), Some(&pool), now).await;
    let cash = newest(&pool, owner).await;
    assert!(cash > text, "the cash send delivered");
    assert_ttl(&expiry_row(&pool, cash).await.unwrap(), "send with cash");

    let cod = AttachedMail::from(owner, sender, "SsmFourExpSTtl")
        .cod(50)
        .item(ITEMS, type_id, 1)
        .insert(&pool)
        .await;
    o.op(MailOp::PayCod { mail_id: cod }, Some(&pool), now)
        .await;
    let payment = newest(&pool, sender).await;
    let row = expiry_row(&pool, payment).await.unwrap();
    assert_eq!(row.sender_id, None, "the payment mail: {row:?}");
    assert_ttl(&row, "COD payment mail");

    let gift = AttachedMail::from(owner, sender, "SsmFourExpSTtl")
        .cash(5)
        .insert(&pool)
        .await;
    o.op(MailOp::Return { mail_id: gift }, Some(&pool), now)
        .await;
    let row = expiry_row(&pool, gift).await.unwrap();
    assert_eq!(row.character_id, sender, "returned: {row:?}");
    assert!(row.sent_time > 1, "the return restamped sent_time: {row:?}");
    assert_ttl(&row, "return");

    let system = send_system_mail(
        &pool,
        &SystemMail {
            sender_name: "Black Market".into(),
            recipient_player_id: owner,
            subject: "Auction sold".into(),
            body: "Your item sold.".into(),
            cash: 3,
            item: SystemItem::None,
        },
    )
    .await
    .unwrap();
    assert_ttl(
        &expiry_row(&pool, system.mail_id).await.unwrap(),
        "system mail",
    );

    s.gm(
        MailGmCellToBase::Send {
            actor: MailGmActor {
                entity_id: s.entity_id,
                player_id: sender,
                account_id: Some(0x7300_0001),
            },
            to: Some("SsmFourExpOTtl".into()),
            cash: 0,
            item: Some((type_id, 1)),
            cod: Some(7),
            subject: "GM COD".into(),
        },
        &pool,
    )
    .await;
    let gm_cod = newest(&pool, owner).await;
    let row = expiry_row(&pool, gm_cod).await.unwrap();
    assert_eq!(row.flags & MAIL_COD, MAIL_COD, "the GM COD mail: {row:?}");
    assert_ttl(&row, "GM COD mail");

    cleanup(&pool, BASE).await;
}

/// D-SS04 path 2: an expired mail with nothing attached is deleted, and
/// `mail.expired path=deleted` names the mailbox and the sender. A mail not
/// yet due is untouched by the same sweep.
#[tokio::test]
async fn live_db_expired_plain_mail_is_deleted() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (owner, sender) = two_players(&pool, BASE + 0x08, "Plain").await;
    let due = AttachedMail::from(owner, sender, "SsmFourExpSPlain")
        .insert(&pool)
        .await;
    let later = AttachedMail::from(owner, sender, "SsmFourExpSPlain")
        .insert(&pool)
        .await;
    set_expiry_state(&pool, due, Some(NOW), false, false).await;
    set_expiry_state(&pool, later, Some(NOW + 1), false, false).await;

    let summary = sweep_mailbox(&pool, owner, NOW, None).await;

    assert_eq!(expiry_row(&pool, due).await, None, "deleted");
    assert!(expiry_row(&pool, later).await.is_some(), "not yet due");
    assert_eq!((summary.deleted, summary.scanned), (1, 1), "{summary:?}");
    let ev = capture
        .all()
        .into_iter()
        .find(|e| {
            e.level == tracing::Level::INFO
                && e.has_field("event", "mail.expired")
                && e.has_field("mail_id", &due.to_string())
        })
        .expect("mail.expired for the due mail");
    assert!(ev.has_field("path", "deleted"));
    assert!(ev.has_field("player_id", &owner.to_string()));
    assert!(ev.has_field("target_player_id", &sender.to_string()));

    cleanup(&pool, BASE + 0x08).await;
}

/// D-SS04 path 1 for an unpaid COD: it goes back to its sender once, its
/// item still in escrow, the COD flag cleared **and the price zeroed**, so
/// the seller can never take their own price as gift cash. It arrives
/// marked returned with a fresh 30-day expiry. Fails if the sweep returns
/// the COD with its price, or deletes it.
#[tokio::test]
async fn live_db_expired_cod_returns_without_its_price() {
    let pool = require_db_or_skip!();
    let (owner, sender) = two_players(&pool, BASE + 0x10, "Cod").await;
    let type_id = any_type_id(&pool).await;
    let mail_id = AttachedMail::from(owner, sender, "SsmFourExpSCod")
        .cod(400)
        .item(ITEMS + 1, type_id, 2)
        .insert(&pool)
        .await;
    set_expiry_state(&pool, mail_id, Some(NOW - 5), false, false).await;

    let summary = sweep_mailbox(&pool, owner, NOW, None).await;

    assert_eq!(summary.returned, 1, "{summary:?}");
    let row = expiry_row(&pool, mail_id)
        .await
        .expect("returned, not deleted");
    assert_eq!(row.character_id, sender, "back to its sender: {row:?}");
    assert_eq!(row.cash, 0, "the COD price is zeroed, never gift cash");
    assert_eq!(row.flags & MAIL_COD, 0, "the COD flag is cleared");
    assert!(row.returned && !row.quarantined, "{row:?}");
    assert_eq!(row.sent_time, NOW);
    assert_eq!(row.expires_at, Some(NOW + MAIL_TTL_SECS), "a fresh 30 days");
    assert!(has_escrow(&pool, mail_id).await, "the item went with it");
    assert!(inventory_rows(&pool, ITEMS + 1).await.is_empty());
    // The sender cannot take the price: there is none.
    assert_eq!(naquadah(&pool, sender).await, 0);

    // Once returned, the next expiry can only quarantine it, never loop.
    set_expiry_state(&pool, mail_id, Some(NOW), true, false).await;
    let again = sweep_mailbox(&pool, sender, NOW, None).await;
    assert_eq!(again.quarantined, 1, "{again:?}");
    assert_eq!(
        expiry_row(&pool, mail_id).await.unwrap().character_id,
        sender
    );

    cleanup(&pool, BASE + 0x10).await;
}

/// D-SS04 path 1 for gift cash and an item: returned once with both, and
/// nothing is credited to anyone by the sweep itself.
#[tokio::test]
async fn live_db_expired_gift_mail_returns_with_cash_and_item() {
    let pool = require_db_or_skip!();
    let (owner, sender) = two_players(&pool, BASE + 0x18, "Gift").await;
    let type_id = any_type_id(&pool).await;
    let mail_id = AttachedMail::from(owner, sender, "SsmFourExpSGift")
        .cash(250)
        .item(ITEMS + 2, type_id, 1)
        .insert(&pool)
        .await;
    set_expiry_state(&pool, mail_id, Some(NOW), false, false).await;

    sweep_mailbox(&pool, owner, NOW, None).await;

    let row = expiry_row(&pool, mail_id).await.unwrap();
    assert_eq!((row.character_id, row.cash), (sender, 250), "{row:?}");
    assert!(row.returned);
    assert!(has_escrow(&pool, mail_id).await);
    assert_eq!(
        (naquadah(&pool, owner).await, naquadah(&pool, sender).await),
        (0, 0)
    );

    cleanup(&pool, BASE + 0x18).await;
}

/// D-SS04 path 3: an already-returned mail that expires still holding an
/// item and gift cash is quarantined, not deleted: the row and its escrow
/// row survive, it is out of the expiry index (`expires_at` NULL), and a
/// WARN `mail.expired path=quarantined reason=already_returned` names it.
/// Fails if path 3 deletes (the escrow row would cascade away) or returns
/// it again.
#[tokio::test]
async fn live_db_expired_returned_mail_is_quarantined_not_deleted() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (owner, sender) = two_players(&pool, BASE + 0x20, "Ret").await;
    let type_id = any_type_id(&pool).await;
    let mail_id = AttachedMail::from(owner, sender, "SsmFourExpSRet")
        .cash(60)
        .item(ITEMS + 3, type_id, 1)
        .insert(&pool)
        .await;
    set_expiry_state(&pool, mail_id, Some(NOW), true, false).await;

    let summary = sweep_mailbox(&pool, owner, NOW, None).await;

    assert_eq!(summary.quarantined, 1, "{summary:?}");
    let row = expiry_row(&pool, mail_id).await.expect("never deleted");
    assert!(row.quarantined, "{row:?}");
    assert_eq!(
        (row.character_id, row.cash, row.expires_at),
        (owner, 60, None)
    );
    assert!(has_escrow(&pool, mail_id).await, "the escrow row survives");
    let ev = capture
        .find_event(
            tracing::Level::WARN,
            "expired gate-mail quarantined",
            "already_returned",
        )
        .expect("mail.expired path=quarantined");
    assert!(ev.has_field("mail_id", &mail_id.to_string()));
    assert!(ev.has_field("item_id", &(ITEMS + 3).to_string()));
    assert!(ev.has_field("cash", "60"));

    cleanup(&pool, BASE + 0x20).await;
}

/// SS-M3 integration edit: a paid COD whose item was never taken belongs to
/// its recipient. Expiry never returns it (the seller would get the item
/// and the price): it is quarantined in the recipient's name with its
/// escrow row. Fails if the sweep takes path 1 for it.
#[tokio::test]
async fn live_db_expired_paid_cod_is_quarantined_not_returned() {
    let pool = require_db_or_skip!();
    let (owner, sender) = two_players(&pool, BASE + 0x28, "Paid").await;
    let type_id = any_type_id(&pool).await;
    // As `pay_cod_tx` leaves it: flag cleared, price zeroed, `cod_paid`.
    let mail_id = AttachedMail::from(owner, sender, "SsmFourExpSPaid")
        .item(ITEMS + 4, type_id, 1)
        .insert(&pool)
        .await;
    set_expiry_state(&pool, mail_id, Some(NOW), false, true).await;

    sweep_mailbox(&pool, owner, NOW, None).await;

    let row = expiry_row(&pool, mail_id).await.unwrap();
    assert_eq!(row.character_id, owner, "still the payer's: {row:?}");
    assert!(row.quarantined && !row.returned, "{row:?}");
    assert!(has_escrow(&pool, mail_id).await);

    cleanup(&pool, BASE + 0x28).await;
}

/// Server mail (no `sender_id`) cannot be returned (D-SS10). Expired with
/// cash or an item, it is quarantined, never deleted with value on it;
/// expired empty, it is deleted like any mail.
#[tokio::test]
async fn live_db_expired_system_mail_is_quarantined_only_with_value() {
    let pool = require_db_or_skip!();
    let base = BASE + 0x30;
    cleanup(&pool, base).await;
    let owner = base + 1;
    insert_players(&pool, base, &[(owner, "SsmFourExpSys")]).await;
    let type_id = any_type_id(&pool).await;
    let valued = AttachedMail {
        owner,
        sender_id: None,
        sender_name: "Black Market",
        cash: 90,
        flags: 0,
        item: Some((ITEMS + 5, type_id, 1)),
    }
    .insert(&pool)
    .await;
    let empty = AttachedMail {
        owner,
        sender_id: None,
        sender_name: "Black Market",
        cash: 0,
        flags: 0,
        item: None,
    }
    .insert(&pool)
    .await;
    for id in [valued, empty] {
        set_expiry_state(&pool, id, Some(NOW), false, false).await;
    }

    let summary = sweep_mailbox(&pool, owner, NOW, None).await;

    assert_eq!(
        (summary.quarantined, summary.deleted),
        (1, 1),
        "{summary:?}"
    );
    let row = expiry_row(&pool, valued).await.expect("kept");
    assert!(row.quarantined && row.cash == 90, "{row:?}");
    assert!(has_escrow(&pool, valued).await);
    assert_eq!(expiry_row(&pool, empty).await, None);

    cleanup(&pool, base).await;
}

/// D-SS04: archived mail never expires. Archiving clears `expires_at`, and
/// the sweep skips an archived row even if one still carries a past expiry
/// (two separate guards: revert either and one half fails).
#[tokio::test]
async fn live_db_archived_mail_never_expires() {
    let pool = require_db_or_skip!();
    let (owner, sender) = two_players(&pool, BASE + 0x38, "Arch").await;
    let o = Client::new(BASE as u32 + 0x42, owner, 55_202, "SsmFourExpOArch");
    let archived_by_player = AttachedMail::from(owner, sender, "SsmFourExpSArch")
        .insert(&pool)
        .await;
    set_expiry_state(&pool, archived_by_player, Some(NOW), false, false).await;
    o.op(
        MailOp::Archive {
            mail_id: archived_by_player,
        },
        Some(&pool),
        Instant::now(),
    )
    .await;
    let row = expiry_row(&pool, archived_by_player).await.unwrap();
    assert_eq!(row.flags & MAIL_ARCHIVE, MAIL_ARCHIVE);
    assert_eq!(row.expires_at, None, "archiving clears the expiry");

    let stale = AttachedMail::from(owner, sender, "SsmFourExpSArch")
        .insert(&pool)
        .await;
    sqlx::query("UPDATE sgw_gate_mail SET flags = $2, expires_at = $3 WHERE mail_id = $1")
        .bind(stale)
        .bind(MAIL_ARCHIVE)
        .bind(NOW)
        .execute(&pool)
        .await
        .unwrap();

    sweep_mailbox(&pool, owner, NOW, None).await;
    sweep_due(&pool, NOW, None).await;

    assert!(expiry_row(&pool, archived_by_player).await.is_some());
    assert!(
        expiry_row(&pool, stale).await.is_some(),
        "an archived row is never swept, whatever its expires_at"
    );

    cleanup(&pool, BASE + 0x38).await;
}

/// No escrow row is ever lost or orphaned by a sweep. Every shape at once
/// (plain, gift cash, item, unpaid COD, returned with an item, a paid COD
/// with its item, server mail with an item), through the global sweep:
/// afterwards every escrowed instance is still in `sgw_gate_mail_item`,
/// on a mail row that exists. Fails if any path deletes a mail that still
/// holds an item (the escrow row cascades with its mail).
#[tokio::test]
async fn live_db_no_orphaned_escrow_after_sweep() {
    let pool = require_db_or_skip!();
    let (owner, sender) = two_players(&pool, BASE + 0x40, "Orph").await;
    let type_id = any_type_id(&pool).await;
    let items: Vec<i32> = (10..15).map(|n| ITEMS + n).collect();
    let mut mails = vec![
        AttachedMail::from(owner, sender, "SsmFourExpSOrph")
            .insert(&pool)
            .await,
        AttachedMail::from(owner, sender, "SsmFourExpSOrph")
            .cash(20)
            .insert(&pool)
            .await,
        AttachedMail::from(owner, sender, "SsmFourExpSOrph")
            .item(items[0], type_id, 1)
            .insert(&pool)
            .await,
        AttachedMail::from(owner, sender, "SsmFourExpSOrph")
            .cod(30)
            .item(items[1], type_id, 1)
            .insert(&pool)
            .await,
    ];
    let returned = AttachedMail::from(owner, sender, "SsmFourExpSOrph")
        .item(items[2], type_id, 1)
        .insert(&pool)
        .await;
    let paid = AttachedMail::from(owner, sender, "SsmFourExpSOrph")
        .item(items[3], type_id, 1)
        .insert(&pool)
        .await;
    let system = AttachedMail {
        owner,
        sender_id: None,
        sender_name: "Black Market",
        cash: 0,
        flags: 0,
        item: Some((items[4], type_id, 1)),
    }
    .insert(&pool)
    .await;
    for &id in &mails {
        set_expiry_state(&pool, id, Some(NOW), false, false).await;
    }
    set_expiry_state(&pool, returned, Some(NOW), true, false).await;
    set_expiry_state(&pool, paid, Some(NOW), false, true).await;
    set_expiry_state(&pool, system, Some(NOW), false, false).await;
    mails.extend([returned, paid, system]);

    let summary = sweep_due(&pool, NOW, None).await;
    assert_eq!(summary.failed, 0, "{summary:?}");
    assert!(summary.scanned >= mails.len(), "{summary:?}");

    let orphans: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sgw_gate_mail_item i \
         WHERE NOT EXISTS (SELECT 1 FROM sgw_gate_mail m WHERE m.mail_id = i.mail_id)",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(orphans, 0);
    for item_id in &items {
        let kept: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM sgw_gate_mail_item WHERE item_id = $1")
                .bind(item_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(kept, 1, "escrowed item {item_id} survives the sweep");
    }
    // What stayed where: the returnable ones went to the sender, the three
    // that cannot go back are quarantined in the owner's name.
    let quarantined: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sgw_gate_mail WHERE character_id = $1 AND quarantined",
    )
    .bind(owner)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(quarantined, 3);
    assert_eq!(mail_count(&pool, sender).await, 3, "gift, item and COD");
    assert_eq!(expiry_row(&pool, mails[0]).await, None, "plain: deleted");

    cleanup(&pool, BASE + 0x40).await;
}

/// Paying a COD restarts the mail's 30 days (security review of SS-M4,
/// MEDIUM): the item is now the payer's, so a COD paid a moment before it
/// expires is not quarantined out of their reach. Fails if `clear_cod`
/// leaves `expires_at` as it was.
#[tokio::test]
async fn live_db_paid_cod_restarts_its_expiry() {
    let pool = require_db_or_skip!();
    let (owner, sender) = two_players(&pool, BASE + 0x48, "Rest").await;
    set_naquadah(&pool, owner, 500).await;
    let type_id = any_type_id(&pool).await;
    let mail_id = AttachedMail::from(owner, sender, "SsmFourExpSRest")
        .cod(100)
        .item(ITEMS + 20, type_id, 1)
        .insert(&pool)
        .await;
    set_expiry_state(&pool, mail_id, Some(NOW), false, false).await;
    let o = Client::new(BASE as u32 + 0x43, owner, 55_203, "SsmFourExpORest");

    let before = unix_now();
    o.op(MailOp::PayCod { mail_id }, Some(&pool), Instant::now())
        .await;
    let row = expiry_row(&pool, mail_id).await.unwrap();
    let at = row.expires_at.expect("still expires");
    assert!(
        (before + MAIL_TTL_SECS..=unix_now() + MAIL_TTL_SECS).contains(&at),
        "30 days from the payment: {row:?}"
    );

    let summary = sweep_mailbox(&pool, owner, NOW, None).await;
    assert_eq!(summary.scanned, 0, "no longer due: {summary:?}");
    let row = expiry_row(&pool, mail_id).await.unwrap();
    assert!(!row.quarantined && row.character_id == owner, "{row:?}");
    assert!(
        has_escrow(&pool, mail_id).await,
        "the item is still takeable"
    );

    cleanup(&pool, BASE + 0x48).await;
}

/// A COD price is not value. An unpaid COD with no item and nobody to
/// return it to (its sender's character is gone, `sender_id` NULL) is
/// deleted when it expires, not quarantined empty for a GM.
#[tokio::test]
async fn live_db_expired_itemless_cod_with_no_sender_is_deleted() {
    let pool = require_db_or_skip!();
    let base = BASE + 0x50;
    cleanup(&pool, base).await;
    let owner = base + 1;
    insert_players(&pool, base, &[(owner, "SsmFourExpNoS")]).await;
    let mail_id = AttachedMail {
        owner,
        sender_id: None,
        sender_name: "Gone",
        cash: 70,
        flags: MAIL_COD,
        item: None,
    }
    .insert(&pool)
    .await;
    set_expiry_state(&pool, mail_id, Some(NOW), false, false).await;

    let summary = sweep_mailbox(&pool, owner, NOW, None).await;

    assert_eq!(
        (summary.deleted, summary.quarantined),
        (1, 0),
        "{summary:?}"
    );
    assert_eq!(expiry_row(&pool, mail_id).await, None);
    assert_eq!(
        naquadah(&pool, owner).await,
        0,
        "the price is nobody's cash"
    );

    cleanup(&pool, base).await;
}
