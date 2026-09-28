//! A dial the cell refuses must never reach the arrival write, and the same
//! dial accepted must: the cell's `handle_dial_gate` against the base's
//! `persist_arrival`, on a live `sgw_player` row.
//!
//! Was `base::world_entry::gate_travel::persist_arrival::tests::
//! a_dial_to_an_unheld_address_arms_nothing_and_leaves_the_row_untouched`,
//! which moved to `cimmeria-base-world-entry` (wave B3 of
//! docs/architecture/services-crate-split.md) without this test: it drives the
//! cell's dial handler (`cimmeria-cell-interactions` since wave C4), which
//! that crate cannot reach. The sentinels and the four
//! fixture helpers are copies of that module's. Its base moved off that
//! module's `0x7000_0600` to its own block (#800); the offsets it had
//! (`TEST_BASE + 4` and `+ 14`) are unchanged.

use std::sync::Arc;

use sqlx::PgPool;

use crate::base::world_entry::persist_arrival;
use crate::test_support::require_db_or_skip;

/// Sentinel base, own block since #800 (was `persist_arrival`'s
/// `0x7006_0600`). Cleanup deletes by exact id, never by range.
const TEST_BASE: i32 = 0x7000_8700;

/// `Castle` (world 8) has a seeded gate (`stargate_id = 2`).
/// Only used for the world_location FK, which must name a real world.
const A_REAL_WORLD: &str = "Castle";

async fn cleanup(pool: &PgPool, account_id: i32, player_id: i32) {
    let _ = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
        .bind(player_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(account_id)
        .execute(pool)
        .await;
}

async fn seed(pool: &PgPool, account_id: i32, player_id: i32, known: &[i32]) {
    seed_in(pool, account_id, player_id, known, A_REAL_WORLD).await
}

async fn seed_in(pool: &PgPool, account_id: i32, player_id: i32, known: &[i32], world: &str) {
    cleanup(pool, account_id, player_id).await;
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(account_id)
        .bind(format!("h06-{account_id}"))
        .execute(pool)
        .await
        .expect("insert sentinel account");
    // `extra_name` and `bodyset` are NOT NULL with a `NULL::varchar`
    // default, so they have to be named explicitly even though nothing
    // here reads them.
    sqlx::query(
        "INSERT INTO sgw_player \
           (account_id, player_id, player_name, extra_name, bodyset, world_location, \
            alignment, archetype, gender, pos_x, pos_y, pos_z, skin_color_id, \
            known_stargates) \
         VALUES ($1, $2, $3, '', 'BS_HumanMale.BS_HumanMale', $4, \
                 1, 1, 1, 0, 0, 0, 0, $5::integer[])",
    )
    .bind(account_id)
    .bind(player_id)
    .bind(format!("H06Pin{player_id}"))
    .bind(world)
    .bind(known)
    .execute(pool)
    .await
    .expect("insert sentinel player");
}

async fn row_of(pool: &PgPool, player_id: i32) -> (Vec<i32>, String, f32) {
    let r: (Vec<i32>, String, f32) = sqlx::query_as(
        "SELECT known_stargates, world_location, pos_x FROM sgw_player WHERE player_id = $1",
    )
    .bind(player_id)
    .fetch_one(pool)
    .await
    .expect("sentinel player row must exist");
    r
}

/// Packet acceptance, cross-layer: a dial to a gate the player does not
/// hold arms nothing, sends nothing, and leaves the `sgw_player` row
/// exactly as it was — then the *same* dial, with the address granted,
/// travels and rewrites the row.
///
/// The second half is what stops the first from being vacuous. The cell
/// never touches the database itself, so "the row did not change" only
/// means something alongside a demonstration that it changes when the
/// dial is accepted. Deleting the `player_knows_stargate` call from
/// `handle_dial_gate` fails phase 1 on all three counts.
#[tokio::test]
async fn live_db_a_dial_to_an_unheld_address_arms_nothing_and_leaves_the_row_untouched() {
    use crate::cell::gate_travel::handle_dial_gate;
    use crate::cell::messages::CellToBaseMsg;
    use crate::cell::space_manager::SpaceManager;
    use crate::cell::spawner::StargateEntry;

    let pool = require_db_or_skip!();
    let (account_id, player_id) = (TEST_BASE + 4, TEST_BASE + 14);
    seed(&pool, account_id, player_id, &[]).await;

    const ENTITY_ID: u32 = 42;
    const TARGET_GATE: i32 = 2;

    // Agnos has no `REGION_FLAG_Stargate` volume here, so an accepted
    // dial travels in one call and the `GateTravel` is observable.
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces>
            <Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" />
            <Space WorldName="Castle" Instanced="false" MinX="0" MaxX="1000" MinY="0" MaxY="1000" />
        </Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces>
            <Space WorldName="Agnos" /><Space WorldName="Castle" />
        </Spaces>"#,
    )
    .unwrap();
    mgr.stargates.insert(
        TARGET_GATE,
        StargateEntry {
            world_name: "Castle".to_string(),
            x: 761.677,
            y: 63.466,
            z: 551.716,
            yaw: 2.152,
            address_origin: 18,
            arrival: None,
            event_set_id: None,
        },
    );
    mgr.create_entity(ENTITY_ID, "Agnos", [10.0; 3], [0.0; 3])
        .unwrap();
    mgr.connect_entity(ENTITY_ID);
    let space_before = mgr.get_entity_space_id(ENTITY_ID);

    let db = Some(Arc::new(pool.clone()));
    let before = row_of(&pool, player_id).await;

    // ── Phase 1: the player does not hold the address ──
    let (tx, mut rx) = tokio::sync::mpsc::channel::<CellToBaseMsg>(16);
    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    assert!(
        !handle_dial_gate(ENTITY_ID, TARGET_GATE, 0, &tx, &mut mgr, &engine).await,
        "a dial to an unheld address must report refusal"
    );
    assert!(
        mgr.get_entity(ENTITY_ID).is_some(),
        "a refused dial must not tear the traveller out of their space"
    );
    assert_eq!(mgr.get_entity_space_id(ENTITY_ID), space_before);
    assert!(
        mgr.gate_dial(ENTITY_ID).is_none(),
        "a refused dial must arm nothing"
    );
    let mut travelled = None;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::GateTravel { position, .. } = msg {
            travelled = Some(position);
        }
    }
    assert!(
        travelled.is_none(),
        "a refused dial must enqueue no GateTravel, so nothing downstream can \
         ever reach persist_arrival"
    );
    assert_eq!(
        row_of(&pool, player_id).await,
        before,
        "world_location, pos_x and known_stargates must all be untouched"
    );

    // ── Phase 2: same dial, address granted ──
    mgr.get_entity_mut(ENTITY_ID)
        .expect("traveller")
        .known_stargates = vec![TARGET_GATE];
    assert!(
        handle_dial_gate(ENTITY_ID, TARGET_GATE, 0, &tx, &mut mgr, &engine).await,
        "the identical dial must succeed once the address is held"
    );
    let position = loop {
        match rx.try_recv() {
            Ok(CellToBaseMsg::GateTravel { position, .. }) => break position,
            Ok(_) => continue,
            Err(_) => panic!("an accepted dial must enqueue a GateTravel"),
        }
    };
    persist_arrival(
        &db,
        player_id,
        account_id as u32,
        "Agnos",
        position,
        &[TARGET_GATE],
    )
    .await;
    let after = row_of(&pool, player_id).await;
    assert_ne!(
        after, before,
        "the accepted dial's arrival must rewrite the row -- otherwise phase 1 \
         proves nothing"
    );
    assert_eq!(after.1, "Agnos", "destination world persisted");
    assert!(
        after.0.contains(&TARGET_GATE),
        "the arrival must learn the dialled address, got {:?}",
        after.0
    );

    cleanup(&pool, account_id, player_id).await;
}
