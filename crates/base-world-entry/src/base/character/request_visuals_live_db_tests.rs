//! Live-DB guards for `handle_request_character_visuals`.
//!
//! Two branches matter: the happy path (player row found, visuals
//! query joins through `sgw_inventory` and `resources.items`,
//! `onCharacterVisuals` goes out on the wire) and the
//! `fetch_optional → Ok(None)` branch where the (player_id,
//! account_id) pair doesn't match — the handler must log WARN and
//! NOT send a packet.

use super::*;
use crate::mercury::{build_character_visuals, SKIN_TINTS};
use crate::test_support::{require_db_or_skip, LogCapture, TestTransport};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tracing::Level;

/// Sentinel base for the visuals live-DB tests. Stepped to the next
/// free window past the delete-character module's `0x7000_1700`
/// reservation. Sibling reservations occupy `0x7000_1100..0x7000_1700`
/// — see the doc-comment on `delete_live_db_tests::TEST_BASE` for the
/// neighbouring slot map.
const TEST_BASE: i32 = 0x7000_1800;

/// Synthetic `resources.items` rows used by these tests. Picked
/// outside any production / seed range. Idempotent `ON CONFLICT
/// DO NOTHING` insert, and NEVER deleted in cleanup so concurrent
/// tests don't fight over the shared FK target. See the matching
/// pattern in `world_entry/methods/vendor/recharge.rs`.
const SYNTH_ITEM_EQUIP: i32 = 0x7FFF_C001;
const SYNTH_ITEM_BANDOLIER: i32 = 0x7FFF_C002;

/// Equipment-container id used to place the equip item. Must be
/// inside `EQUIPMENT_CONTAINERS` — pick 4 (head slot) to match the
/// production constant in `player_load/core.rs`.
const CONTAINER_HEAD: i32 = 4;
/// Bandolier container id mirrors the production
/// `CONTAINER_BANDOLIER` constant. Hard-coded here as 3 so a
/// regression that re-numbers the bandolier container would
/// surface as a test failure (the handler's SQL would still bind
/// the renamed constant, but this test's seeded row would no
/// longer match — exactly the signal we want).
const CONTAINER_BANDOLIER_LOCAL: i32 = 3;
/// `bandolier_slot` we set on the player row; the visuals query
/// only returns the bandolier row whose `slot_id` matches this.
const ACTIVE_BAND_SLOT: i32 = 0;

/// Bodyset string seeded into `sgw_player.bodyset`. Re-used when
/// reconstructing the expected payload for the byte-exact assertion.
const TEST_BODYSET: &str = "BS_HumanMale.BS_HumanMale";
/// Single `components` array entry seeded on the `sgw_player` row.
/// The visuals query prepends this with `sgw_inventory ⋈ resources.items`
/// rows in indeterminate result order — see the happy-path test's
/// payload-matching comment.
const TEST_BASE_COMPONENT: &str = "base.Component";

async fn cleanup(pool: &PgPool, account_ids: &[i32], player_ids: &[i32]) {
    // Per-test rows only — the synthetic resources.items rows are
    // shared FK targets, see SYNTH_ITEM_* doc above.
    for player_id in player_ids {
        let _ = sqlx::query("DELETE FROM sgw_inventory WHERE character_id = $1")
            .bind(player_id)
            .execute(pool)
            .await;
    }
    for account_id in account_ids {
        let _ = sqlx::query("DELETE FROM sgw_player WHERE account_id = $1")
            .bind(account_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(account_id)
            .execute(pool)
            .await;
    }
}

async fn insert_account(pool: &PgPool, account_id: i32) {
    sqlx::query(
        "INSERT INTO account (account_id, account_name, password) \
         VALUES ($1, $2, '')",
    )
    .bind(account_id)
    .bind(format!("visuals-{account_id}"))
    .execute(pool)
    .await
    .expect("insert account");
}

async fn insert_character(pool: &PgPool, account_id: i32, player_id: i32, bandolier_slot: i32) {
    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, alignment, archetype, gender, \
            player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id, naquadah, bandolier_slot, \
            components\
         ) VALUES ($1, $2, 1, 1, 1, 1, $3, '', 'CombatSim', $4, \
                   0.0, 0.0, 0.0, 0, 0, $5, ARRAY[$6]::varchar[])",
    )
    .bind(account_id)
    .bind(player_id)
    .bind(format!("visuals-char-{player_id}"))
    .bind(TEST_BODYSET)
    .bind(bandolier_slot)
    .bind(TEST_BASE_COMPONENT)
    .execute(pool)
    .await
    .expect("insert character");
}

/// Insert a synthetic `resources.items` row whose `visual_component`
/// is the canonical column the visuals query joins on. `ON CONFLICT
/// DO NOTHING` so concurrent test binaries don't fight over the PK.
async fn insert_synthetic_item(pool: &PgPool, item_id: i32, visual_component: &str) {
    sqlx::query(
        "INSERT INTO resources.items (\
            item_id, description, name, quality_id, tech_comp, tier, \
            max_stack_size, visual_component\
         ) VALUES ($1, '', $2, 'ITEM_QUALITY_Normal', 0, 1, 1, $3) \
         ON CONFLICT (item_id) DO NOTHING",
    )
    .bind(item_id)
    .bind(format!("synth-visual-{item_id}"))
    .bind(visual_component)
    .execute(pool)
    .await
    .expect("insert synthetic resources.items row");
}

async fn insert_inventory_row(
    pool: &PgPool,
    player_id: i32,
    type_id: i32,
    container_id: i32,
    slot_id: i32,
) {
    sqlx::query(
        "INSERT INTO sgw_inventory \
            (character_id, type_id, stack_size, slot_id, container_id, \
             bound, durability, charges) \
         VALUES ($1, $2, 1, $3, $4, false, 100, 0)",
    )
    .bind(player_id)
    .bind(type_id)
    .bind(slot_id)
    .bind(container_id)
    .execute(pool)
    .await
    .expect("insert inventory row");
}

fn make_connected(
    addr: SocketAddr,
    account_id: u32,
    account_entity_id: u32,
) -> Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> {
    let mut state = crate::test_support::test_default_connected_client_state();
    state.account_id = account_id;
    state.account_entity_id = account_entity_id;
    let mut m = HashMap::new();
    m.insert(addr, state);
    Arc::new(Mutex::new(m))
}

/// Happy path: player row exists for (player_id, account_id), the
/// inventory join produces two visual rows (one equip-container,
/// one bandolier active slot), and the handler sends exactly one
/// `onCharacterVisuals` packet. Bug shape: a regression that
/// silently swallows the inventory join error (or drops the
/// `components.extend(item_visuals)` line) would still send a
/// packet, so the assertion here is "send_count == 1" — not just
/// non-empty.
///
/// **Payload check** (added per PR review): the captured packet
/// bytes are reconstructed from `build_character_visuals` with both
/// possible orderings of the two inventory-derived visuals.
/// Postgres returns the `sgw_inventory ⋈ resources.items` rows in
/// indeterminate order (the SELECT has no `ORDER BY`), so the
/// assertion accepts either permutation. A regression that drops
/// either visual, or changes the bodyset / account_entity_id /
/// skin_tint inputs, would no longer match either permutation.
#[tokio::test]
async fn handle_request_character_visuals_happy_path_sends_one_packet() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE;
    let player_id = TEST_BASE + 10;

    cleanup(&pool, &[account_id], &[player_id]).await;
    insert_synthetic_item(&pool, SYNTH_ITEM_EQUIP, "vis.Equip").await;
    insert_synthetic_item(&pool, SYNTH_ITEM_BANDOLIER, "vis.Bandolier").await;
    insert_account(&pool, account_id).await;
    insert_character(&pool, account_id, player_id, ACTIVE_BAND_SLOT).await;
    // Equipment container row: slot_id MUST be 0 to match the
    // `(container_id <> $3 AND slot_id = 0)` arm of the visuals
    // query's OR predicate.
    insert_inventory_row(&pool, player_id, SYNTH_ITEM_EQUIP, CONTAINER_HEAD, 0).await;
    // Bandolier active slot row: container_id = bandolier AND
    // slot_id = bandolier_slot ($4) — the other arm of the OR.
    insert_inventory_row(
        &pool,
        player_id,
        SYNTH_ITEM_BANDOLIER,
        CONTAINER_BANDOLIER_LOCAL,
        ACTIVE_BAND_SLOT,
    )
    .await;

    let transport = Arc::new(TestTransport::new());
    let dyn_transport: Arc<dyn Transport> = transport.clone();
    let addr: SocketAddr = "127.0.0.1:55803".parse().unwrap();
    let account_entity_id: u32 = 0xBBBB_0001;
    let connected = make_connected(addr, account_id as u32, account_entity_id);
    let key = [0u8; 32];
    let db_pool = Some(Arc::new(pool.clone()));

    let result = handle_request_character_visuals(
        &dyn_transport,
        addr,
        key,
        player_id,
        &connected,
        &db_pool,
    )
    .await;
    assert!(
        result.is_ok(),
        "visuals handler must return Ok on happy path"
    );

    // Exactly one packet — not "at least one". The bug shape we're
    // pinning is "extra fan-out" as well as "no fan-out".
    assert_eq!(
        transport.send_count_to(addr),
        1,
        "happy-path requestCharacterVisuals must emit exactly one \
         onCharacterVisuals packet to the requester; got {} packets",
        transport.send_count_to(addr),
    );
    assert_eq!(
        transport.len(),
        1,
        "no other fan-out is expected — the handler is a 1-in/1-out RPC",
    );

    // Byte-exact payload check (PR review): reconstruct the expected
    // packet from the same `build_character_visuals` the handler
    // calls, with both possible orderings of the two inventory-
    // derived visuals. The SELECT has no `ORDER BY` so Postgres can
    // return them in either order — accept both. Catches: wrong
    // bodyset, missing visual, wrong account_entity_id, wrong
    // skin_tint, wrong message type, packet-truncation bugs.
    let actual_packets = transport.filter_to(addr);
    assert_eq!(actual_packets.len(), 1, "expected exactly one packet");
    let actual = &actual_packets[0];

    // skin_color_id = 0 (set in insert_character) → SKIN_TINTS[0].
    let skin_tint = SKIN_TINTS[0];
    let components_a = vec![
        TEST_BASE_COMPONENT.to_string(),
        "vis.Equip".to_string(),
        "vis.Bandolier".to_string(),
    ];
    let components_b = vec![
        TEST_BASE_COMPONENT.to_string(),
        "vis.Bandolier".to_string(),
        "vis.Equip".to_string(),
    ];
    // Test state starts at next_seq=0 with all-zero key, no acks
    // (see test_default_connected_client_state).
    let expected_a = build_character_visuals(
        &key,
        0,
        &[],
        player_id,
        TEST_BODYSET,
        &components_a,
        0xFF,
        0xFF,
        skin_tint,
        account_entity_id,
        cimmeria_mercury::encryption::EncryptionVersion::V1,
    );
    let expected_b = build_character_visuals(
        &key,
        0,
        &[],
        player_id,
        TEST_BODYSET,
        &components_b,
        0xFF,
        0xFF,
        skin_tint,
        account_entity_id,
        cimmeria_mercury::encryption::EncryptionVersion::V1,
    );
    assert!(
        actual == &expected_a || actual == &expected_b,
        "packet bytes must match build_character_visuals with both \
         seeded item visuals present (in either SQL-returned order). \
         A wrong bodyset, missing visual, wrong account_entity_id, \
         or wrong message type would trip here. \
         actual.len()={}, expected_a.len()={}, expected_b.len()={}",
        actual.len(),
        expected_a.len(),
        expected_b.len(),
    );

    cleanup(&pool, &[account_id], &[player_id]).await;
}

/// `fetch_optional → Ok(None)` branch: the player row exists but
/// under a DIFFERENT account, so the `(player_id AND account_id)`
/// predicate yields no row. Handler must NOT send a packet and
/// MUST emit the documented WARN. Bug shape: dropping the
/// `account_id = $2` predicate from the SELECT (mirroring the
/// delete-character bug shape) would leak another account's
/// visuals on the character-select screen.
#[tokio::test]
async fn handle_request_character_visuals_account_mismatch_logs_warn() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();

    let owning_account = TEST_BASE + 20;
    let requesting_account = TEST_BASE + 21;
    let other_player = TEST_BASE + 30;

    cleanup(
        &pool,
        &[owning_account, requesting_account],
        &[other_player],
    )
    .await;
    insert_account(&pool, owning_account).await;
    insert_account(&pool, requesting_account).await;
    // Player belongs to owning_account, NOT requesting_account.
    insert_character(&pool, owning_account, other_player, ACTIVE_BAND_SLOT).await;

    let transport = Arc::new(TestTransport::new());
    let dyn_transport: Arc<dyn Transport> = transport.clone();
    let addr: SocketAddr = "127.0.0.1:55804".parse().unwrap();
    // The session is for requesting_account — but the player_id
    // they're asking about belongs to owning_account. The SELECT
    // WHERE filter must yield None.
    let connected = make_connected(addr, requesting_account as u32, 0xBBBB_0002);
    let key = [0u8; 32];
    let db_pool = Some(Arc::new(pool.clone()));

    let result = handle_request_character_visuals(
        &dyn_transport,
        addr,
        key,
        other_player,
        &connected,
        &db_pool,
    )
    .await;
    assert!(
        result.is_ok(),
        "Ok(None) branch must still return Ok — the function does \
         not surface 'not found' as an error to the caller",
    );

    // No packet sent — the Ok(None) arm has no transport.send_to.
    // A regression that fell through to the happy-path build_packet
    // line would trip this.
    assert!(
        transport.is_empty(),
        "account-mismatch lookup must NOT send a visuals packet — \
         that would leak another account's character visuals; \
         got {} packets",
        transport.len(),
    );

    let event = capture
        .find_message(Level::WARN, "requestCharacterVisuals: player not found")
        .expect(
            "Ok(None) branch must emit a WARN — negative-logging \
             convention: the only signal ops has that a session asked \
             for a player it doesn't own (or a stale id)",
        );
    assert!(
        event.has_field("player_id", &other_player.to_string()),
        "warn must carry the queried player_id for ops triage: {event:#?}"
    );

    cleanup(
        &pool,
        &[owning_account, requesting_account],
        &[other_player],
    )
    .await;
}
