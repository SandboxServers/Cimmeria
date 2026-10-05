//! The loot hand-back: a loot grant that commits nothing answers the cell
//! with `LootGrantRefused` (so the item goes back on its corpse), and a
//! grant that commits answers nothing (so there is no second copy).
//!
//! Removing the `LootGrantRefused` send makes the refused-grant tests find
//! an empty channel and fail.
//!
//! Sentinels: accounts/players `0x7000_C410..=0x7000_C415`, entities
//! `0x7000_C4E4..=0x7000_C4E7`, the synthetic item type `0x7000_C4F1`.

use tokio::sync::mpsc;
use tracing::Level;

use super::fall_through_tests::{bags, cleanup, insert_account_and_player, seeded_item, state};
use super::*;
use crate::cell::messages::{BaseToCellMsg, GrantRefusal, LootGrantSource};
use crate::test_support::{require_db_or_skip, LogCapture};

const STORAGE_ONLY_TYPE_ID: i32 = 0x7000_C4F1;

fn source() -> LootGrantSource {
    LootGrantSource {
        corpse_id: 0x7000_C4D0,
        index: 3,
        corpse_respawn_at: None,
        corpse_template_id: Some(304),
    }
}

/// Everything the cell received, as `(source, design, qty, container,
/// reason)` per `LootGrantRefused`.
fn refusals(
    rx: &mut mpsc::Receiver<BaseToCellMsg>,
) -> Vec<(LootGrantSource, i32, i32, i32, GrantRefusal)> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let BaseToCellMsg::LootGrantRefused {
            source,
            design_id,
            quantity,
            container_id,
            reason,
            ..
        } = msg
        {
            out.push((source, design_id, quantity, container_id, reason));
        }
    }
    out
}

/// A full crafting bag refuses the grant: nothing is written and the cell
/// gets the item back, with the bag it was going to.
#[tokio::test]
async fn live_db_full_bag_hands_the_loot_back_to_the_cell() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (0x7000_C410, 0x7000_C411, 0x7000_C4E4_u32);
    cleanup(&pool, account_id, player_id, entity_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;
    let type_id = seeded_item(&pool, "{17,15}", false).await;
    // Fill all 100 crafting-bag slots.
    sqlx::query(
        "INSERT INTO sgw_inventory (character_id, type_id, stack_size, slot_id, container_id, \
                                    bound, durability, charges) \
         SELECT $1, $2, 1, s, 15, false, 100, 0 FROM generate_series(0, 99) s",
    )
    .bind(player_id)
    .bind(type_id)
    .execute(&pool)
    .await
    .expect("fill the crafting bag");
    let (transport, conn, e2a) = state();
    let db_pool = Some(Arc::new(pool.clone()));
    let (tx, mut rx) = mpsc::channel(8);
    let capture = LogCapture::install();

    handle_loot_grant(
        entity_id,
        player_id,
        type_id,
        15,
        1,
        source(),
        &db_pool,
        &Some(tx),
        &transport,
        &conn,
        &e2a,
    )
    .await;

    assert_eq!(
        bags(&pool, player_id).await,
        vec![(15, 100, 100)],
        "nothing written"
    );
    assert_eq!(
        refusals(&mut rx),
        vec![(source(), type_id, 1, 15, GrantRefusal::ContainerFull)],
        "the refused item must go back to the cell"
    );
    let event = capture
        .find_event(Level::INFO, "grant_refused", "container_full")
        .expect("grant_refused reason=container_full");
    assert_eq!(event.target, "inventory");
    for (key, value) in [
        ("account_id", account_id.to_string()),
        ("player_id", player_id.to_string()),
        ("entity_id", entity_id.to_string()),
        ("item_type_id", type_id.to_string()),
        ("container_id", "15".to_string()),
    ] {
        assert_eq!(event.fields.get(key), Some(&value), "field `{key}`");
    }

    cleanup(&pool, account_id, player_id, entity_id).await;
}

/// A storage-only item is refused by the vault guard, and the refusal goes
/// back to the cell too.
#[tokio::test]
async fn live_db_storage_only_loot_is_handed_back_to_the_cell() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (0x7000_C412, 0x7000_C413, 0x7000_C4E5_u32);
    cleanup(&pool, account_id, player_id, entity_id).await;
    let _ = sqlx::query("DELETE FROM resources.items WHERE item_id = $1")
        .bind(STORAGE_ONLY_TYPE_ID)
        .execute(&pool)
        .await;
    insert_account_and_player(&pool, account_id, player_id).await;
    sqlx::query(
        "INSERT INTO resources.items (item_id, description, name, quality_id, tech_comp, tier, \
                                      max_stack_size, container_sets) \
         VALUES ($1, '', 'storage-only', 'ITEM_QUALITY_Normal', 0, 1, 1, '{17}')",
    )
    .bind(STORAGE_ONLY_TYPE_ID)
    .execute(&pool)
    .await
    .expect("insert the storage-only item type");
    let (transport, conn, e2a) = state();
    let db_pool = Some(Arc::new(pool.clone()));
    let (tx, mut rx) = mpsc::channel(8);

    handle_loot_grant(
        entity_id,
        player_id,
        STORAGE_ONLY_TYPE_ID,
        17,
        1,
        source(),
        &db_pool,
        &Some(tx),
        &transport,
        &conn,
        &e2a,
    )
    .await;

    assert!(bags(&pool, player_id).await.is_empty(), "nothing written");
    assert_eq!(
        refusals(&mut rx),
        vec![(
            source(),
            STORAGE_ONLY_TYPE_ID,
            1,
            17,
            GrantRefusal::StorageOnly
        )]
    );

    cleanup(&pool, account_id, player_id, entity_id).await;
    let _ = sqlx::query("DELETE FROM resources.items WHERE item_id = $1")
        .bind(STORAGE_ONLY_TYPE_ID)
        .execute(&pool)
        .await;
}

/// A committed loot grant answers nothing: the corpse must not get a copy
/// of an item the player now holds.
#[tokio::test]
async fn live_db_committed_loot_grant_hands_nothing_back() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (0x7000_C414, 0x7000_C415, 0x7000_C4E6_u32);
    cleanup(&pool, account_id, player_id, entity_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;
    let type_id = seeded_item(&pool, "{17,15}", false).await;
    let (transport, conn, e2a) = state();
    let db_pool = Some(Arc::new(pool.clone()));
    let (tx, mut rx) = mpsc::channel(8);

    handle_loot_grant(
        entity_id,
        player_id,
        type_id,
        15,
        1,
        source(),
        &db_pool,
        &Some(tx),
        &transport,
        &conn,
        &e2a,
    )
    .await;

    assert_eq!(bags(&pool, player_id).await, vec![(15, 1, 1)]);
    assert!(
        refusals(&mut rx).is_empty(),
        "a committed grant must not hand the item back"
    );

    cleanup(&pool, account_id, player_id, entity_id).await;
}

/// No database: the grant is refused before anything starts, and the item
/// goes back. No database needed.
#[tokio::test]
async fn no_database_hands_the_loot_back() {
    let (transport, conn, e2a) = state();
    let (tx, mut rx) = mpsc::channel(8);

    handle_loot_grant(
        0x7000_C4E7,
        0x7000_C416,
        4242,
        15,
        2,
        source(),
        &None,
        &Some(tx),
        &transport,
        &conn,
        &e2a,
    )
    .await;

    assert_eq!(
        refusals(&mut rx),
        vec![(source(), 4242, 2, 15, GrantRefusal::NoDatabase)]
    );
}

/// With no cell channel the refused item cannot go back; that loss is a
/// WARN with the corpse, the item and the refusal.
#[tokio::test]
async fn refused_loot_with_no_cell_channel_logs_loot_restore_failed() {
    let (transport, conn, e2a) = state();
    let capture = LogCapture::install();

    handle_loot_grant(
        0x7000_C4E7,
        0x7000_C416,
        4242,
        15,
        2,
        source(),
        &None,
        &None,
        &transport,
        &conn,
        &e2a,
    )
    .await;

    let event = capture
        .find_event(Level::WARN, "loot_restore_failed", "cell_channel_closed")
        .expect("loot_restore_failed reason=cell_channel_closed");
    assert_eq!(event.target, "inventory");
    for (key, value) in [
        ("player_id", 0x7000_C416.to_string()),
        ("entity_id", 0x7000_C4E7_u32.to_string()),
        ("corpse_id", 0x7000_C4D0_u32.to_string()),
        ("index", "3".to_string()),
        ("item_type_id", "4242".to_string()),
        ("qty", "2".to_string()),
        ("refusal", "no_database".to_string()),
    ] {
        assert_eq!(event.fields.get(key), Some(&value), "field `{key}`");
    }
}
