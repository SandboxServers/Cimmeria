//! Live-DB guards: the fall-through never writes buyback (16) or a vault
//! (17-20), even when the carried bag it wants is full.
//!
//! A `{17,15}` component whose crafting bag is full is refused with
//! `container_full` and a visible line (the GM's feedback, or the loot
//! hand-back that leaves the item on the corpse); it does not fall on into
//! the vault or buyback. An item that resolves to buyback is refused with
//! `not_grantable_container`.
//!
//! With both carried bags full, an item that lists the vault first and both
//! carried bags (`{17,1,15}`) is refused the same way, for a plain grant and
//! for loot.
//!
//! Sentinels: accounts/players `0x7000_C420..=0x7000_C42B`, entities
//! `0x7000_C4E8..=0x7000_C4ED`, synthetic item types `0x7000_C4F2` (`{16}`),
//! `0x7000_C4F3` (`{16,17,15}`) and `0x7000_C4F4` (`{17,1,15}`).

use tokio::sync::mpsc;
use tracing::Level;

use super::fall_through_tests::{bags, cleanup, insert_account_and_player, seeded_item, state};
use super::*;
use crate::cell::messages::{BaseToCellMsg, GrantRefusal, LootGrantSource};
use crate::test_support::{
    require_db_or_skip, test_default_connected_client_state, LogCapture, TestTransport,
};

const BUYBACK_ONLY: i32 = 0x7000_C4F2;
const BUYBACK_FIRST: i32 = 0x7000_C4F3;
const VAULT_FIRST_BOTH_BAGS: i32 = 0x7000_C4F4;

async fn fill_crafting_bag(pool: &PgPool, player_id: i32, type_id: i32) {
    fill_bag(pool, player_id, type_id, 15, 100).await;
}

/// Fill `slots` slots (0..slots) of `container_id` with one-item stacks.
async fn fill_bag(pool: &PgPool, player_id: i32, type_id: i32, container_id: i32, slots: i32) {
    sqlx::query(
        "INSERT INTO sgw_inventory (character_id, type_id, stack_size, slot_id, container_id, \
                                    bound, durability, charges) \
         SELECT $1, $2, 1, s, $3, false, 100, 0 FROM generate_series(0, $4 - 1) s",
    )
    .bind(player_id)
    .bind(type_id)
    .bind(container_id)
    .bind(slots)
    .execute(pool)
    .await
    .expect("fill the bag");
}

async fn insert_item_type(pool: &PgPool, item_id: i32, container_sets: &str) {
    let _ = sqlx::query("DELETE FROM resources.items WHERE item_id = $1")
        .bind(item_id)
        .execute(pool)
        .await;
    sqlx::query(
        "INSERT INTO resources.items (item_id, description, name, quality_id, tech_comp, tier, \
                                      max_stack_size, container_sets) \
         VALUES ($1, '', 'cr16-synthetic', 'ITEM_QUALITY_Normal', 0, 1, 1, $2::integer[])",
    )
    .bind(item_id)
    .bind(container_sets)
    .execute(pool)
    .await
    .expect("insert synthetic item type");
}

async fn delete_item_types(pool: &PgPool) {
    for id in [BUYBACK_ONLY, BUYBACK_FIRST, VAULT_FIRST_BOTH_BAGS] {
        let _ = sqlx::query("DELETE FROM resources.items WHERE item_id = $1")
            .bind(id)
            .execute(pool)
            .await;
    }
}

/// Rows the player holds in buyback or a vault.
async fn ungrantable_rows(pool: &PgPool, player_id: i32) -> i64 {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM sgw_inventory WHERE character_id = $1 AND container_id BETWEEN 16 AND 20",
    )
    .bind(player_id)
    .fetch_one(pool)
    .await
    .expect("count")
}

/// Every text line the transport carried, decrypted with the all-zero test
/// key and read as UTF-16.
fn sent_texts(transport: &TestTransport) -> Vec<String> {
    let enc = cimmeria_mercury::encryption::MercuryEncryption::from_session_key([0u8; 32]);
    transport
        .drain()
        .into_iter()
        .filter_map(|(_, packet)| enc.decrypt(&packet).ok())
        .flat_map(|pt| {
            // The text may start at either byte parity; read both.
            [0usize, 1].map(|skip| {
                let (pairs, _) = pt[skip..].as_chunks::<2>();
                let units: Vec<u16> = pairs.iter().map(|b| u16::from_le_bytes(*b)).collect();
                String::from_utf16_lossy(&units)
            })
        })
        .collect()
}

/// A `{17,15}` component asked into the vault while the crafting bag is
/// full: nothing is written (not in the vault, not in buyback), the refusal
/// is logged with its reason and identity, and the GM is told why.
#[tokio::test]
async fn live_db_full_crafting_bag_refuses_instead_of_using_the_vault() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (0x7000_C420, 0x7000_C421, 0x7000_C4E8_u32);
    cleanup(&pool, account_id, player_id, entity_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;
    let type_id = seeded_item(&pool, "{17,15}", false).await;
    fill_crafting_bag(&pool, player_id, type_id).await;

    let addr: SocketAddr = "127.0.0.1:54620".parse().unwrap();
    let mut session = test_default_connected_client_state();
    session.player_entity_id = Some(entity_id);
    let conn = Arc::new(Mutex::new(HashMap::from([(addr, session)])));
    let e2a = Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));
    let test_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = test_transport.clone();
    let db_pool = Some(Arc::new(pool.clone()));
    let capture = LogCapture::install();

    handle_grant_item(
        entity_id, player_id, type_id, 17, 1, true, &db_pool, &None, &transport, &conn, &e2a,
    )
    .await;

    assert_eq!(
        bags(&pool, player_id).await,
        vec![(15, 100, 100)],
        "nothing written"
    );
    assert_eq!(ungrantable_rows(&pool, player_id).await, 0);
    let event = capture
        .find_event(Level::INFO, "grant_refused", "container_full")
        .expect("grant_refused reason=container_full");
    for (key, value) in [
        ("account_id", account_id.to_string()),
        ("player_id", player_id.to_string()),
        ("entity_id", entity_id.to_string()),
        ("container_id", "15".to_string()),
    ] {
        assert_eq!(event.fields.get(key), Some(&value), "field `{key}`");
    }
    let texts = sent_texts(&test_transport);
    assert!(
        texts.iter().any(|t| t.contains("that bag is full")),
        "the GM must be told the grant was refused: {texts:?}"
    );

    cleanup(&pool, account_id, player_id, entity_id).await;
}

/// The same refusal on a loot pickup asked into the vault: the cell gets
/// the item back (it stays on the corpse, and the cell tells the looter),
/// and nothing lands in the vault.
#[tokio::test]
async fn live_db_full_crafting_bag_hands_vault_requested_loot_back() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (0x7000_C422, 0x7000_C423, 0x7000_C4E9_u32);
    cleanup(&pool, account_id, player_id, entity_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;
    let type_id = seeded_item(&pool, "{17,15}", false).await;
    fill_crafting_bag(&pool, player_id, type_id).await;
    let (transport, conn, e2a) = state();
    let db_pool = Some(Arc::new(pool.clone()));
    let (tx, mut rx) = mpsc::channel(8);
    let source = LootGrantSource {
        corpse_id: 0x7000_C4D1,
        index: 1,
        corpse_respawn_at: None,
        corpse_template_id: None,
    };

    handle_loot_grant(
        entity_id,
        player_id,
        type_id,
        17,
        1,
        source,
        &db_pool,
        &Some(tx),
        &transport,
        &conn,
        &e2a,
    )
    .await;

    assert_eq!(ungrantable_rows(&pool, player_id).await, 0);
    match rx.try_recv() {
        Ok(BaseToCellMsg::LootGrantRefused {
            reason,
            container_id,
            ..
        }) => {
            assert_eq!(reason, GrantRefusal::ContainerFull);
            assert_eq!(container_id, 15);
        }
        _ => panic!("the refused loot must go back to the cell"),
    }

    cleanup(&pool, account_id, player_id, entity_id).await;
}

/// An item that resolves to buyback is refused (`not_grantable_container`),
/// for a plain grant and for loot; one that lists buyback before a carried
/// bag falls through to the carried bag.
#[tokio::test]
async fn live_db_buyback_is_never_a_grant_target() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (0x7000_C424, 0x7000_C425, 0x7000_C4EA_u32);
    cleanup(&pool, account_id, player_id, entity_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;
    insert_item_type(&pool, BUYBACK_ONLY, "{16}").await;
    insert_item_type(&pool, BUYBACK_FIRST, "{16,17,15}").await;
    let (transport, conn, e2a) = state();
    let db_pool = Some(Arc::new(pool.clone()));
    let capture = LogCapture::install();

    handle_grant_item(
        entity_id,
        player_id,
        BUYBACK_ONLY,
        16,
        1,
        false,
        &db_pool,
        &None,
        &transport,
        &conn,
        &e2a,
    )
    .await;
    assert!(bags(&pool, player_id).await.is_empty(), "nothing written");
    assert!(capture
        .find_event(Level::INFO, "grant_refused", "not_grantable_container")
        .is_some());

    let (tx, mut rx) = mpsc::channel(8);
    let source = LootGrantSource {
        corpse_id: 0x7000_C4D2,
        index: 1,
        corpse_respawn_at: None,
        corpse_template_id: None,
    };
    handle_loot_grant(
        entity_id,
        player_id,
        BUYBACK_ONLY,
        16,
        1,
        source,
        &db_pool,
        &Some(tx),
        &transport,
        &conn,
        &e2a,
    )
    .await;
    assert!(matches!(
        rx.try_recv(),
        Ok(BaseToCellMsg::LootGrantRefused {
            reason: GrantRefusal::NotGrantable,
            ..
        })
    ));

    handle_grant_item(
        entity_id,
        player_id,
        BUYBACK_FIRST,
        16,
        1,
        false,
        &db_pool,
        &None,
        &transport,
        &conn,
        &e2a,
    )
    .await;
    assert_eq!(bags(&pool, player_id).await, vec![(15, 1, 1)]);

    cleanup(&pool, account_id, player_id, entity_id).await;
    delete_item_types(&pool).await;
}

/// Both fall-through targets full: a `{17,1,15}` item asked into the vault
/// by `gmGiveItem` (the same request shape as a content `grant_item`) with
/// the main bag (40) and the crafting bag (100) full is refused with
/// `grant_refused reason=container_full` and the full identity, the GM is
/// told why, and it lands in no container: no new row anywhere, nothing in
/// 16-20.
#[tokio::test]
async fn live_db_both_carried_bags_full_refuses_a_vault_first_grant() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (0x7000_C428, 0x7000_C429, 0x7000_C4EC_u32);
    cleanup(&pool, account_id, player_id, entity_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;
    insert_item_type(&pool, VAULT_FIRST_BOTH_BAGS, "{17,1,15}").await;
    fill_bag(&pool, player_id, VAULT_FIRST_BOTH_BAGS, 1, 40).await;
    fill_bag(&pool, player_id, VAULT_FIRST_BOTH_BAGS, 15, 100).await;
    let before = bags(&pool, player_id).await;
    assert_eq!(before, vec![(1, 40, 40), (15, 100, 100)]);

    let addr: SocketAddr = "127.0.0.1:54621".parse().unwrap();
    let mut session = test_default_connected_client_state();
    session.player_entity_id = Some(entity_id);
    let conn = Arc::new(Mutex::new(HashMap::from([(addr, session)])));
    let e2a = Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));
    let test_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = test_transport.clone();
    let db_pool = Some(Arc::new(pool.clone()));
    let capture = LogCapture::install();

    handle_grant_item(
        entity_id,
        player_id,
        VAULT_FIRST_BOTH_BAGS,
        17,
        1,
        true,
        &db_pool,
        &None,
        &transport,
        &conn,
        &e2a,
    )
    .await;

    assert_eq!(
        bags(&pool, player_id).await,
        before,
        "no new row in any container"
    );
    assert_eq!(
        ungrantable_rows(&pool, player_id).await,
        0,
        "nothing in 16-20"
    );
    let event = capture
        .find_event(Level::INFO, "grant_refused", "container_full")
        .expect("grant_refused reason=container_full");
    assert_eq!(event.target, "inventory");
    for (key, value) in [
        ("event", "grant_refused".to_string()),
        ("account_id", account_id.to_string()),
        ("player_id", player_id.to_string()),
        ("entity_id", entity_id.to_string()),
        ("type_id", VAULT_FIRST_BOTH_BAGS.to_string()),
        ("quantity", "1".to_string()),
    ] {
        assert_eq!(event.fields.get(key), Some(&value), "field `{key}`");
    }
    assert!(
        capture
            .find_event(
                Level::WARN,
                "grant_rejected",
                "grant_into_storage_container"
            )
            .is_none(),
        "an item that lists a carried bag is not the vault guard's refusal"
    );
    let texts = sent_texts(&test_transport);
    assert!(
        texts.iter().any(|t| t.contains("that bag is full")),
        "the player must see why the grant was refused: {texts:?}"
    );

    cleanup(&pool, account_id, player_id, entity_id).await;
    delete_item_types(&pool).await;
}

/// The loot variant: with both carried bags full the loot grant is refused
/// and handed back to the cell (which puts it back on the corpse and tells
/// the looter, pinned by the cell's
/// `loot::restore_tests::refused_grant_goes_back_on_the_emptied_corpse`);
/// no row is written, nothing lands in 16-20.
#[tokio::test]
async fn live_db_both_carried_bags_full_hands_vault_first_loot_back() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (0x7000_C42A, 0x7000_C42B, 0x7000_C4ED_u32);
    cleanup(&pool, account_id, player_id, entity_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;
    insert_item_type(&pool, VAULT_FIRST_BOTH_BAGS, "{17,1,15}").await;
    fill_bag(&pool, player_id, VAULT_FIRST_BOTH_BAGS, 1, 40).await;
    fill_bag(&pool, player_id, VAULT_FIRST_BOTH_BAGS, 15, 100).await;
    let before = bags(&pool, player_id).await;
    let (transport, conn, e2a) = state();
    let db_pool = Some(Arc::new(pool.clone()));
    let (tx, mut rx) = mpsc::channel(8);
    let source = LootGrantSource {
        corpse_id: 0x7000_C4D3,
        index: 2,
        corpse_respawn_at: None,
        corpse_template_id: None,
    };
    let capture = LogCapture::install();

    handle_loot_grant(
        entity_id,
        player_id,
        VAULT_FIRST_BOTH_BAGS,
        17,
        1,
        source,
        &db_pool,
        &Some(tx),
        &transport,
        &conn,
        &e2a,
    )
    .await;

    assert_eq!(
        bags(&pool, player_id).await,
        before,
        "no new row in any container"
    );
    assert_eq!(
        ungrantable_rows(&pool, player_id).await,
        0,
        "nothing in 16-20"
    );
    match rx.try_recv() {
        Ok(BaseToCellMsg::LootGrantRefused {
            source: back,
            reason,
            design_id,
            quantity,
            ..
        }) => {
            assert_eq!(back, source);
            assert_eq!(reason, GrantRefusal::ContainerFull);
            assert_eq!((design_id, quantity), (VAULT_FIRST_BOTH_BAGS, 1));
        }
        _ => panic!("the refused loot must go back to the cell"),
    }
    let event = capture
        .find_event(Level::INFO, "grant_refused", "container_full")
        .expect("grant_refused reason=container_full");
    assert_eq!(
        event.fields.get("account_id"),
        Some(&account_id.to_string())
    );

    cleanup(&pool, account_id, player_id, entity_id).await;
    delete_item_types(&pool).await;
}
