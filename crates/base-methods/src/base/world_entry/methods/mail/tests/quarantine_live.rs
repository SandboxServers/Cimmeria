//! Live-DB guards for quarantined mail (SS-M4, D-SS04 path 3): out of its
//! owner's reach, out of the mailbox list, out of the D-SS03 cap. And the
//! full header list resets the client's list (`ResetCategory` 1), so mail
//! that left server-side drops out. Sentinels: accounts, players and
//! entities `0x7300_2200` up, items `0x7300_2280` up.

use std::time::Instant;

use super::super::read::{archive_unless_cod, delete_if_empty};
use super::packets::{plain_send, Client, Received};
use super::*;
use crate::cell::messages::MailOp;

const BASE: i32 = 0x7300_2200;
const ITEMS: i32 = 0x7300_2280;

async fn quarantine(pool: &PgPool, mail_id: i32) {
    sqlx::query(
        "UPDATE sgw_gate_mail SET quarantined = true, expires_at = NULL WHERE mail_id = $1",
    )
    .bind(mail_id)
    .execute(pool)
    .await
    .unwrap();
}

/// A quarantined mail holding 80 gift cash and an item: its owner's take
/// cash, take item, pay and return all find nothing (`not_found_for_owner`,
/// and the stale header is removed); archive and delete change nothing.
/// The cash, the item and the escrow row stay exactly where they were.
/// Fails if `AND NOT quarantined` leaves `claim::lock_mail`, the archive
/// `UPDATE` or the delete guard.
#[tokio::test]
async fn live_db_quarantined_mail_is_out_of_every_player_op() {
    let pool = require_db_or_skip!();
    cleanup(&pool, BASE).await;
    let (owner, sender) = (BASE + 1, BASE + 2);
    insert_players(&pool, BASE, &[(owner, "SsmFourQO"), (sender, "SsmFourQS")]).await;
    set_naquadah(&pool, owner, 1_000).await;
    let type_id = any_type_id(&pool).await;
    let mail_id = AttachedMail::from(owner, sender, "SsmFourQS")
        .cash(80)
        .item(ITEMS, type_id, 1)
        .insert(&pool)
        .await;
    quarantine(&pool, mail_id).await;
    let before = expiry_row(&pool, mail_id).await.unwrap();

    let c = Client::new(BASE as u32 + 0x10, owner, 55_220, "SsmFourQO");
    let now = Instant::now();
    for op in [
        MailOp::TakeCash { mail_id },
        MailOp::TakeItem {
            mail_id,
            container_id: -1,
            slot_id: -1,
        },
        MailOp::PayCod { mail_id },
        MailOp::Return { mail_id },
    ] {
        let name = format!("{op:?}");
        c.op(op, Some(&pool), now).await;
        let got = c.take();
        assert!(
            got.contains(&Received::Other(
                crate::mercury::method_idx::ON_MAIL_HEADER_REMOVE
            )),
            "{name} answers as for a missing mail: {got:?}"
        );
        assert_eq!(
            expiry_row(&pool, mail_id).await.as_ref(),
            Some(&before),
            "{name}"
        );
    }
    assert_eq!(archive_unless_cod(&pool, mail_id, owner).await.unwrap(), 0);
    assert_eq!(expiry_row(&pool, mail_id).await.as_ref(), Some(&before));

    assert_eq!(naquadah(&pool, owner).await, 1_000, "nothing credited");
    assert!(
        inventory_rows(&pool, ITEMS).await.is_empty(),
        "nothing taken"
    );
    assert!(has_escrow(&pool, mail_id).await);

    // The delete guard, on a quarantined mail that holds nothing (it would
    // otherwise pass `cash = 0` and no escrow row).
    let empty = AttachedMail::from(owner, sender, "SsmFourQS")
        .insert(&pool)
        .await;
    quarantine(&pool, empty).await;
    assert_eq!(delete_if_empty(&pool, empty, owner).await.unwrap(), 0);
    c.op(MailOp::Delete { mail_id: empty }, Some(&pool), now)
        .await;
    assert!(expiry_row(&pool, empty).await.is_some(), "never deleted");

    cleanup(&pool, BASE).await;
}

/// A quarantined mail is out of the mailbox: the inbox list does not show
/// it, and it does not count toward the 100-message cap (D-SS03), so a
/// mailbox with 99 open mails and three quarantined ones still takes a
/// player's mail. The list reply sets `ResetCategory` (SS-E1 M-Q7), so the
/// client drops any mail that left server-side since the last open. Fails
/// if the cap count, the header list or the reset flag is reverted.
#[tokio::test]
async fn live_db_quarantined_mail_is_not_listed_or_capped() {
    let pool = require_db_or_skip!();
    let base = BASE + 0x08;
    cleanup(&pool, base).await;
    let (owner, sender) = (base + 1, base + 2);
    insert_players(
        &pool,
        base,
        &[(owner, "SsmFourQCapO"), (sender, "SsmFourQCapS")],
    )
    .await;
    fill_mailbox(&pool, owner, 99, 0).await;
    let mut hidden = Vec::new();
    for _ in 0..3 {
        let id = AttachedMail::from(owner, sender, "SsmFourQCapS")
            .cash(5)
            .insert(&pool)
            .await;
        quarantine(&pool, id).await;
        hidden.push(id);
    }

    let s = Client::new(base as u32 + 0x10, sender, 55_221, "SsmFourQCapS");
    s.op(
        MailOp::Send(plain_send(&["SsmFourQCapO"])),
        Some(&pool),
        Instant::now(),
    )
    .await;
    let got = s.take();
    assert!(
        matches!(
            got.first(),
            Some(Received::SendMailResult { result: 0, .. })
        ),
        "delivered, not MailboxFull: {got:?}"
    );
    assert_eq!(mail_count(&pool, owner).await, 99 + 3 + 1);

    let o = Client::new(base as u32 + 0x11, owner, 55_222, "SsmFourQCapO");
    o.op(
        MailOp::RequestHeaders { b_archive: 0 },
        Some(&pool),
        Instant::now(),
    )
    .await;
    match o.take().as_slice() {
        [Received::HeaderInfo { reset, headers, .. }] => {
            assert_eq!(*reset, 1, "a full list resets the client's list");
            assert_eq!(headers.len(), 100, "99 filler and the new mail");
            for id in &hidden {
                assert!(!headers.iter().any(|(h, _)| h == id), "{id} is hidden");
            }
        }
        other => panic!("expected one onMailHeaderInfo, got {other:?}"),
    }

    cleanup(&pool, base).await;
}
