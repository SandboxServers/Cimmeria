//! Live-DB guard: the personal vault loads with the player (D-BV06, A-22).
//!
//! A row placed in container 17 by hand must load with the player and reach
//! the client in the world-entry `onUpdateItem` batch, and `onBagInfo` must
//! declare 17 at the player's own `sgw_player.bank_slots`. BV-02 onward put
//! real items in 17; this pins that login already carries them.
//!
//! Sentinels: account `0x7000_B150`, player `0x7000_B151`.

use super::query_player_load_data;
use crate::mercury::{build_map_loaded_body, WorldEntryInfo};
use crate::test_support::require_db_or_skip;
use sqlx::PgPool;
use std::sync::Arc;

const ACCOUNT_ID: i32 = 0x7000_B150;
const PLAYER_ID: i32 = 0x7000_B151;
/// Not the column default (40), so the load is proven to read the column.
const BANK_SLOTS: i16 = 60;

async fn cleanup(pool: &PgPool) {
    let _ = sqlx::query("DELETE FROM sgw_inventory WHERE character_id = $1")
        .bind(PLAYER_ID)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(ACCOUNT_ID)
        .execute(pool)
        .await;
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

#[tokio::test]
async fn live_db_vault_row_placed_by_hand_loads_and_is_sent() {
    let pool = require_db_or_skip!();
    cleanup(&pool).await;
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(ACCOUNT_ID)
        .bind(format!("bv01-load-{ACCOUNT_ID}"))
        .execute(&pool)
        .await
        .expect("insert account");
    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, alignment, archetype, gender, \
            player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id, naquadah, bank_slots\
         ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                   0.0, 0.0, 0.0, 0, 0, $4)",
    )
    .bind(ACCOUNT_ID)
    .bind(PLAYER_ID)
    .bind(format!("test-{PLAYER_ID}"))
    .bind(BANK_SLOTS)
    .execute(&pool)
    .await
    .expect("insert player");
    // Any seeded item the vault accepts.
    let type_id: i32 = sqlx::query_scalar(
        "SELECT item_id FROM resources.items WHERE 17 = ANY(container_sets) \
         ORDER BY item_id LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .expect("the seed must hold an item the vault accepts");
    let vault_item: i32 = sqlx::query_scalar(
        "INSERT INTO sgw_inventory \
            (character_id, type_id, stack_size, slot_id, container_id, bound, durability, charges) \
         VALUES ($1, $2, 1, 9, 17, false, 100, 0) RETURNING item_id",
    )
    .bind(PLAYER_ID)
    .bind(type_id)
    .fetch_one(&pool)
    .await
    .expect("insert vault row");

    let db_pool = Some(Arc::new(pool.clone()));
    let data = query_player_load_data(&db_pool, ACCOUNT_ID as u32, PLAYER_ID).await;

    assert_eq!(data.player_id, PLAYER_ID, "the player must load");
    assert_eq!(
        data.bank_slots,
        i32::from(BANK_SLOTS),
        "bank_slots must load from sgw_player"
    );
    let loaded = data
        .items
        .iter()
        .find(|item| item.id == vault_item)
        .expect("the vault row must load with the player");
    assert_eq!(loaded.container_id, 17);
    assert_eq!(loaded.slot_id, 10, "wire slot is DB slot + 1");

    let entry = WorldEntryInfo {
        player_entity_id: 100,
        space_id: 65552,
        pos: [0.0; 3],
        rot: [0.0; 3],
        world_name: "CombatSim".into(),
        class_id: 0x02,
        world_stargates: vec![],
    };
    let body = build_map_loaded_body(100, &data, &entry);

    let mut item_bytes = Vec::new();
    loaded.serialize(&mut item_bytes);
    assert!(
        contains(&body, &item_bytes),
        "the vault row must reach the client in the world-entry onUpdateItem batch"
    );
    let mut vault_bag = Vec::new();
    vault_bag.extend_from_slice(&17i32.to_le_bytes());
    vault_bag.extend_from_slice(&i32::from(BANK_SLOTS).to_le_bytes());
    vault_bag.extend_from_slice(&18i32.to_le_bytes());
    assert!(
        contains(&body, &vault_bag),
        "onBagInfo must declare container 17 at the player's bank_slots"
    );

    cleanup(&pool).await;
}
