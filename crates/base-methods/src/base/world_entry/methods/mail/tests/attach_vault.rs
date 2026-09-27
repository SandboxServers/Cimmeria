//! Live-DB guard for the owner's backpack-only rule (Bank campaign,
//! 2026-09-27): an item in a vault (17-20) or on the buyback list (16) can
//! not be mailed. Sentinels `0x7300_144x` and items `0x7300_164x`.

use std::time::{Duration, Instant};

use super::attach_live::{assert_untouched, attached, reply, setup};
use super::packets::Client;
use super::*;
use crate::cell::mail::codes::MailResult;
use crate::test_support::LogCapture;
use cimmeria_entity::inventory::{
    INV_AUCTION, INV_BANK, INV_BUYBACK, INV_COMMAND_BANK, INV_TEAM_BANK,
};

const BASE: i32 = 0x7300_1440;
const ITEMS: i32 = 0x7300_1640;

/// Each of the four vault containers and the buyback list is refused with
/// `ItemNotAvailable`, its own `reason` and a feedback line saying where
/// the item is; nothing is debited and the item stays in its slot. Fails
/// without the main-bag check (the item would be mailed out of the vault).
#[tokio::test]
async fn send_rejects_banked_item() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (acct, sender, rcpt) = (BASE, BASE + 1, BASE + 2);
    let type_id = setup(
        &pool,
        acct,
        (sender, "SsmTwoVaultSend"),
        (rcpt, "SsmTwoVaultRcpt"),
        500,
    )
    .await;
    let cases = [
        (INV_BANK, "item_in_vault"),
        (INV_AUCTION, "item_in_vault"),
        (INV_TEAM_BANK, "item_in_vault"),
        (INV_COMMAND_BANK, "item_in_vault"),
        (INV_BUYBACK, "item_in_buyback"),
    ];
    let items: Vec<TestItem> = cases
        .iter()
        .enumerate()
        .map(|(i, (container_id, _))| TestItem {
            container_id: *container_id,
            ..TestItem::main(ITEMS + i as i32, sender, 0, 1)
        })
        .collect();
    for item in &items {
        insert_item(&pool, *item, type_id).await;
    }

    let c = Client::new(0x7300_1480, sender, 54_740, "SsmTwoVaultSend");
    let t0 = Instant::now();
    for (i, (item, (container_id, reason))) in items.iter().zip(cases).enumerate() {
        c.op(
            MailOp::Send(attached("SsmTwoVaultRcpt", 10, false, item.item_id, 1)),
            Some(&pool),
            t0 + Duration::from_secs(11 * i as u64),
        )
        .await;
        let r = reply(c.take());
        assert_eq!(
            r.code,
            Some(MailResult::ItemNotAvailable.code()),
            "container {container_id}: {r:?}"
        );
        assert_eq!(r.lines.len(), 1, "container {container_id}");
        assert!(
            r.lines[0].contains(if reason == "item_in_vault" {
                "vault"
            } else {
                "buyback"
            }),
            "container {container_id}: {:?}",
            r.lines
        );
        assert!(r.cash.is_empty() && r.removed.is_empty());
        assert!(
            capture
                .all()
                .iter()
                .filter(|e| e.message_contains("sendMailMessage refused")
                    && e.has_field("reason", reason))
                .count()
                > 0,
            "container {container_id}: reason={reason}"
        );
    }
    assert_untouched(&pool, sender, rcpt, 500, &items).await;
    for item in &items {
        let container: i32 =
            sqlx::query_scalar("SELECT container_id FROM sgw_inventory WHERE item_id = $1")
                .bind(item.item_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(container, item.container_id, "the item stays where it was");
    }

    cleanup(&pool, acct).await;
}
