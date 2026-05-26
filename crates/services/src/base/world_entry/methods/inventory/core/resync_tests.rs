//! Live-DB regression guard for `send_full_inventory_resync`.
//!
//! Pins that the post-`ReanchorPlayer` resync ships the FULL
//! client-init bundle (onBagInfo + onActiveSlotUpdate +
//! onCashChanged + onUpdateItem), not just the item snapshot. Without
//! the bag declaration, the freshly-instantiated client-side
//! `InventoryComponent` has no containers registered and every
//! subsequent `onUpdateItem` silently fails to slot — both the
//! initial post-respawn snapshot AND every later pickup.
//!
//! Skips cleanly when `DATABASE_URL` is unset (same pattern as
//! sibling live-DB tests).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::send_full_inventory_resync;
use crate::base::ConnectedClientState;
use crate::mercury::{build_player_entity_method_packet, method_idx};
use crate::test_support::{require_db_or_skip, test_default_connected_client_state, TestTransport};

const TEST_BASE: i32 = 0x7000_0500;
const SLAPPACK_TYPE_ID: i32 = 2893;
const TEST_NAQUADAH: i32 = 4242;
const TEST_BANDOLIER_SLOT: i32 = 2;

async fn cleanup(pool: &PgPool, account_id: i32, player_id: i32) {
    let _ = sqlx::query("DELETE FROM sgw_inventory WHERE character_id = $1")
        .bind(player_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(account_id)
        .execute(pool)
        .await;
}

async fn insert_account_player_and_item(pool: &PgPool, account_id: i32, player_id: i32) {
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(account_id)
        .bind(format!("resync-{account_id}"))
        .execute(pool)
        .await
        .expect("insert account");

    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, alignment, archetype, gender, \
            player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id, naquadah, bandolier_slot\
         ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                   0.0, 0.0, 0.0, 0, $4, $5)",
    )
    .bind(account_id)
    .bind(player_id)
    .bind(format!("test-{player_id}"))
    .bind(TEST_NAQUADAH)
    .bind(TEST_BANDOLIER_SLOT)
    .execute(pool)
    .await
    .expect("insert player");

    // One slappack in main bag so onUpdateItem has something to ship.
    sqlx::query(
        "INSERT INTO sgw_inventory \
            (character_id, type_id, stack_size, slot_id, container_id, \
             bound, durability, charges, \
             ammo_type, ammo_types, ammo, flags) \
         SELECT $1, ri.item_id, 1, 0, 1, false, 100, 0, \
                COALESCE(ri.default_ammo_type, 'AMMO_NONE'::resources.\"EAmmoType\"), \
                ri.ammo_types, ri.charges, 0 \
         FROM resources.items ri WHERE ri.item_id = $2",
    )
    .bind(player_id)
    .bind(SLAPPACK_TYPE_ID)
    .execute(pool)
    .await
    .expect("insert slappack");
}

/// End-to-end shape pin: `send_full_inventory_resync` must emit
/// exactly four packets in the order
/// `[onBagInfo, onActiveSlotUpdate, onCashChanged, onUpdateItem]`,
/// each addressed to the player's own client. The order matters —
/// onBagInfo MUST land before onUpdateItem or the client's new
/// `InventoryComponent` has no containers and silently drops every
/// item entry.
///
/// Bug shape pre-fix: only `onUpdateItem` fired. The other three
/// init packets stayed in the world-entry path and were never
/// re-sent on respawn, so the freshly-recreated pawn never knew
/// about its bags. Post-respawn the inventory UI stayed empty,
/// equipment was invisible, and even fresh pickups didn't show
/// because their `onUpdateItem` landed against the same broken
/// bag-less state.
#[tokio::test]
async fn resync_sends_bag_info_active_slot_cash_then_items_in_order() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE;
    let player_id = TEST_BASE + 1;
    let entity_id: u32 = 0x7000_0501;

    cleanup(&pool, account_id, player_id).await;
    insert_account_player_and_item(&pool, account_id, player_id).await;

    let transport = Arc::new(TestTransport::new());
    let dyn_transport: Arc<dyn Transport> = transport.clone();
    let addr: SocketAddr = "127.0.0.1:40400".parse().unwrap();
    let entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>> =
        Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));
    let connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> = Arc::new(Mutex::new(
        HashMap::from([(addr, test_default_connected_client_state())]),
    ));

    let arc_pool = Arc::new(pool.clone());
    send_full_inventory_resync(
        entity_id,
        player_id,
        &arc_pool,
        &dyn_transport,
        &connected,
        &entity_to_addr,
    )
    .await;

    let sent = transport.drain();
    assert_eq!(
        sent.len(),
        4,
        "resync must emit exactly 4 packets (onBagInfo + onActiveSlotUpdate + onCashChanged + onUpdateItem); got {}. \
         A regression that drops onBagInfo (or any of the init three) leaves the client-side \
         InventoryComponent without registered containers and silently breaks every subsequent \
         onUpdateItem.",
        sent.len(),
    );
    for (i, (recipient, _)) in sent.iter().enumerate() {
        assert_eq!(
            *recipient, addr,
            "packet {i} must target the owning client's addr (resync is owner-only, no AoI fan-out)"
        );
    }

    // Predict the exact wire bytes per packet. test_default_connected_client_state
    // starts next_seq at 0; send_to_witness_reliable fetch_adds, so the
    // four packets land at seq 0, 1, 2, 3 in send order.
    let key = [0u8; 32];

    let bag_args = cimmeria_entity::inventory::Inventory::new(0).serialize_bag_info();
    let expected_bag = build_player_entity_method_packet(
        &key,
        0,
        &[],
        entity_id,
        method_idx::ON_BAG_INFO,
        &bag_args,
    );
    assert_eq!(
        sent[0].1, expected_bag,
        "packet 0 must be onBagInfo — declares the container set on the new InventoryComponent"
    );

    const CONTAINER_BANDOLIER: i32 = 3;
    let mut slot_args = Vec::with_capacity(8);
    slot_args.extend_from_slice(&CONTAINER_BANDOLIER.to_le_bytes());
    slot_args.extend_from_slice(&(TEST_BANDOLIER_SLOT + 1).to_le_bytes());
    let expected_slot = build_player_entity_method_packet(
        &key,
        1,
        &[],
        entity_id,
        method_idx::ON_ACTIVE_SLOT_UPDATE,
        &slot_args,
    );
    assert_eq!(
        sent[1].1, expected_slot,
        "packet 1 must be onActiveSlotUpdate with the persisted bandolier_slot+1 (wire is 1-indexed)"
    );

    let cash_args = TEST_NAQUADAH.to_le_bytes().to_vec();
    let expected_cash = build_player_entity_method_packet(
        &key,
        2,
        &[],
        entity_id,
        method_idx::ON_CASH_CHANGED,
        &cash_args,
    );
    assert_eq!(
        sent[2].1, expected_cash,
        "packet 2 must be onCashChanged with the persisted naquadah balance"
    );

    // Packet 3 is the onUpdateItem call. We don't compare its bytes
    // directly: the encrypted body includes ammo_type / charges / etc.
    // derived from the seed `resources.items` row for the slappack,
    // and the Mercury encryption's body-length and CRC fields make a
    // raw prefix-byte comparison brittle. The bundle-shape pin above
    // (exactly 4 packets in order, first three matching method-idx
    // byte-exact) already excludes any reordering or omission — the
    // 4th packet is structurally the onUpdateItem call. Stronger
    // assertions on the item payload belong in a dedicated
    // `send_full_inventory_update` byte-shape test.
    assert!(
        !sent[3].1.is_empty(),
        "packet 3 (onUpdateItem) must be a non-empty packet"
    );

    cleanup(&pool, account_id, player_id).await;
}
