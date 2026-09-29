//! Live-DB guard for the crafting-options hook in
//! `send_full_inventory_update`: every inventory commit ends in that
//! resync, so it is where a Field Crafting Tool entering or leaving the
//! crafting bag reaches `onUpdateCraftingOptions` (140).
//!
//! With the `refresh_tools_from_rows` call removed, no 140 follows the
//! `onUpdateItem` and both assertions on the packet count fail.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::encryption::EncryptionVersion;
use cimmeria_mercury::transport::Transport;
use cimmeria_wire::cell::client_methods::player::ON_UPDATE_CRAFTING_OPTIONS;
use cimmeria_wire::crafting::{crafting_options_args, CraftingInfo, CraftingOptions};
use sqlx::PgPool;

use super::send_full_inventory_update;
use crate::mercury::build_player_entity_method_packet;
use crate::test_support::{require_db_or_skip, test_default_connected_client_state, TestTransport};
use cimmeria_base_crafting::base::crafting::options::CraftingOptionsExt;

// Crafting live-DB sentinels (`0x7000_Cxxx`): `0x7000_CD40..0x7000_CD42`.
const ACCOUNT: i32 = 0x7000_CD40;
const PLAYER: i32 = 0x7000_CD41;
const TOOL: i32 = 0x7000_CD42;
/// BMAS-5 Field Crafting Tool.
const TOOL_TYPE: i32 = 5369;
const ENTITY: u32 = 4310;

async fn cleanup(pool: &PgPool) {
    let _ = sqlx::query("DELETE FROM sgw_inventory WHERE item_id = $1")
        .bind(TOOL)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
        .bind(PLAYER)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(ACCOUNT)
        .execute(pool)
        .await;
}

async fn setup(pool: &PgPool) {
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(ACCOUNT)
        .bind(format!("craft-hook-{ACCOUNT}"))
        .execute(pool)
        .await
        .expect("insert account");
    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, alignment, archetype, gender, \
            player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id\
         ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                   0.0, 0.0, 0.0, 0)",
    )
    .bind(ACCOUNT)
    .bind(PLAYER)
    .bind(format!("craft-hook-{PLAYER}"))
    .execute(pool)
    .await
    .expect("insert player");
    sqlx::query(
        "INSERT INTO sgw_inventory (item_id, stack_size, container_id, slot_id, type_id, character_id) \
         VALUES ($1, 1, 15, 0, $2, $3)",
    )
    .bind(TOOL)
    .bind(TOOL_TYPE)
    .bind(PLAYER)
    .execute(pool)
    .await
    .expect("insert tool");
}

fn options_packet(options: &CraftingOptions, seq: u32) -> Vec<u8> {
    build_player_entity_method_packet(
        &[0u8; 32],
        seq,
        &[],
        ENTITY,
        ON_UPDATE_CRAFTING_OPTIONS,
        &crafting_options_args(options),
        EncryptionVersion::V1,
    )
}

#[tokio::test]
async fn live_db_a_tool_entering_and_leaving_the_crafting_bag_updates_the_options() {
    let pool = require_db_or_skip!();
    cleanup(&pool).await;
    setup(&pool).await;

    let typed = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed.clone();
    let addr: SocketAddr = "127.0.0.1:55760".parse().unwrap();
    let mut state = test_default_connected_client_state();
    state.plugins = cimmeria_base_session::base::plugin::BasePlugins::build(&[
        &cimmeria_base_crafting::CraftingPlugin,
    ])
    .unwrap();
    // The login send has happened, with no tool.
    state.crafting_options_mut().armed = true;
    state.crafting_options_mut().last_sent = Some(CraftingOptions::default());
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(ENTITY, addr)])));
    let pool = Arc::new(pool);

    send_full_inventory_update(
        ENTITY,
        PLAYER,
        &pool,
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;
    let with_tool = typed.filter_to(addr);

    sqlx::query("UPDATE sgw_inventory SET container_id = 1 WHERE item_id = $1")
        .bind(TOOL)
        .execute(pool.as_ref())
        .await
        .expect("move tool");
    send_full_inventory_update(
        ENTITY,
        PLAYER,
        &pool,
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;
    let after_move = typed.filter_to(addr);
    cleanup(&pool).await;

    let tool = CraftingInfo {
        items: vec![TOOL],
        entities: vec![],
    };
    let expected = CraftingOptions {
        crafting: tool.clone(),
        research: tool.clone(),
        reverse_engineering: tool,
        alloying: CraftingInfo::default(),
    };
    assert_eq!(with_tool.len(), 2, "onUpdateItem, then 140");
    assert_eq!(with_tool[1], options_packet(&expected, 1));
    assert_eq!(after_move.len(), 4, "onUpdateItem, then the empty 140");
    assert_eq!(
        after_move[3],
        options_packet(&CraftingOptions::default(), 3)
    );
}
