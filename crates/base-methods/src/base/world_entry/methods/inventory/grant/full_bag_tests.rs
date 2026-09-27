//! Live-DB guards: the fall-through never writes buyback (16) or a vault
//! (17-20), even when the carried bag it wants is full.
//!
//! A `{17,15}` component whose crafting bag is full is refused with
//! `container_full` and a visible line (the GM's feedback, or the loot
//! hand-back that leaves the item on the corpse); it does not fall on into
//! the vault or buyback. An item that resolves to buyback is refused with
//! `not_grantable_container`.
//!
//! Sentinels: accounts/players `0x7000_C420..=0x7000_C427`, entities
//! `0x7000_C4E8..=0x7000_C4EB`, synthetic item types `0x7000_C4F2` (`{16}`)
//! and `0x7000_C4F3` (`{16,17,15}`).

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

async fn fill_crafting_bag(pool: &PgPool, player_id: i32, type_id: i32) {
    sqlx::query(
        "INSERT INTO sgw_inventory (character_id, type_id, stack_size, slot_id, container_id, \
                                    bound, durability, charges) \
         SELECT $1, $2, 1, s, 15, false, 100, 0 FROM generate_series(0, 99) s",
    )
    .bind(player_id)
    .bind(type_id)
    .execute(pool)
    .await
    .expect("fill the crafting bag");
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
    for id in [BUYBACK_ONLY, BUYBACK_FIRST] {
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
                let units: Vec<u16> = pt[skip..]
                    .chunks_exact(2)
                    .map(|b| u16::from_le_bytes([b[0], b[1]]))
                    .collect();
                String::from_utf16_lossy(&units)
            })
        })
        .collect()
}

/// A `{17,15}` component asked into the vault while the crafting bag is
/// full: nothing is written (not in the vault, not in buyback), the refusal
/// is logged with its reason and identity, and the GM is told why.
#[tokio::test]
async fn full_crafting_bag_refuses_instead_of_using_the_vault() {
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
async fn full_crafting_bag_hands_vault_requested_loot_back() {
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
async fn buyback_is_never_a_grant_target() {
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
