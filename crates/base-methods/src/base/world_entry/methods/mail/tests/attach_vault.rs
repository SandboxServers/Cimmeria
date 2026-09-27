//! Live-DB guard for the owner's backpack-only rule (Bank campaign,
//! 2026-09-27): an item in a vault (17-20) or on the buyback list (16) can
//! not be mailed. And the crafting bag (15) can be, since the owner's
//! decision relayed by the crafting campaign (2026-09-27, SS-M4).
//! Sentinels `0x7300_144x` and items `0x7300_164x`; `0x7300_245x` and
//! `0x7300_246x` for the crafting-bag test.

use std::time::{Duration, Instant};

use super::attach_live::{assert_untouched, attached, reply, setup};
use super::packets::Client;
use super::*;
use crate::cell::mail::codes::MailResult;
use crate::test_support::LogCapture;
use cimmeria_entity::inventory::{
    INV_AUCTION, INV_BANK, INV_BUYBACK, INV_COMMAND_BANK, INV_CRAFTING, INV_TEAM_BANK,
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

/// Owner decision (2026-09-27): crafting components live in the crafting
/// bag (15) after crafting CR-16, and bag 15 is a mail source. A component
/// type (`container_sets` holds 15) sent from bag 15 is escrowed like a
/// backpack item: `Sent`, postage debited, the row gone from bag 15 and in
/// `sgw_gate_mail_item` under the same instance id. Fails when the source
/// allowlist is back to the backpack alone (`item_not_in_main_bag`).
#[tokio::test]
async fn send_escrows_a_crafting_component_from_bag_15() {
    let pool = require_db_or_skip!();
    let (acct, sender, rcpt) = (0x7300_2450, 0x7300_2451, 0x7300_2452);
    let any = setup(
        &pool,
        acct,
        (sender, "SsmFourCraftSend"),
        (rcpt, "SsmFourCraftRcpt"),
        100,
    )
    .await;
    let component: Option<i32> = sqlx::query_scalar(
        "SELECT item_id FROM resources.items WHERE $1 = ANY(container_sets) \
         ORDER BY item_id LIMIT 1",
    )
    .bind(INV_CRAFTING)
    .fetch_optional(&pool)
    .await
    .unwrap();
    let type_id = component.unwrap_or(any);
    let item = TestItem {
        container_id: INV_CRAFTING,
        ..TestItem::main(0x7300_2460, sender, 0, 3)
    };
    insert_item(&pool, item, type_id).await;

    let c = Client::new(0x7300_2455, sender, 55_270, "SsmFourCraftSend");
    c.op(
        MailOp::Send(attached("SsmFourCraftRcpt", 0, false, item.item_id, 3)),
        Some(&pool),
        Instant::now(),
    )
    .await;

    let r = reply(c.take());
    assert_eq!(r.code, Some(MailResult::Sent.code()), "{r:?}");
    assert_eq!(naquadah(&pool, sender).await, 75, "postage only");
    assert!(
        inventory_row(&pool, item.item_id).await.is_none(),
        "left bag 15"
    );
    let escrow = escrow_for(&pool, rcpt).await;
    assert_eq!(escrow.len(), 1, "{escrow:?}");
    assert_eq!(
        (escrow[0].item_id, escrow[0].type_id, escrow[0].stack_size),
        (item.item_id, type_id, 3)
    );

    cleanup(&pool, acct).await;
}
