//! `grant_xp` chain-replay guard (issue #611).
//!
//! Unlike every other module here, this one seeds its own chain. There
//! are **zero** `grant_xp` rows in `db/resources/Content/Seed/` today —
//! the verb had neither a loader arm nor an executor arm, so no content
//! author could reach it. Pinning it against a sentinel chain is the
//! only way to guard both halves before content starts using it; when
//! real seed rows land, the assertions here move to a chain-id module
//! alongside the mission families.
//!
//! Both halves are exercised on purpose:
//!
//! - Removing the `"grant_xp"` arm from
//!   `crates/content-engine/src/loader/action.rs` makes `convert_action`
//!   return `None`, the chain loads with zero actions, and the resolve
//!   assertion fails.
//! - Removing the `Action::GrantXP` arm from
//!   `crates/services/src/cell/content/executor/mod.rs` drops execution
//!   into the `other =>` catch-all and the `CellToBaseMsg::GrantXP`
//!   assertion fails.
//!
//! Sentinel id range: `0x7000_5000`. Sibling reservations in
//! `crates/services` currently run `0x7000_1000..0x7000_1B00`,
//! `0x7000_2000`, `0x7000_3000`, `0x7000_4000` and `0x7000_4242`; this
//! steps past all of them. Cleanup deletes the exact ids inserted, never
//! a range.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::engine_loader::load_single_chain_for_test;
use super::super::executor::execute_actions;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::require_db_or_skip;

/// Sentinel `content_chains.chain_id`. Fits in `i32` (the column type).
const TEST_CHAIN_ID: i32 = 0x7000_5000;
/// Sentinel interact tag — namespaced so it can never collide with a
/// real `content_triggers.event_key`.
const TEST_TAG: &str = "CIMMERIA_TEST_GRANT_XP_TAG";
/// Deliberately not a round number: a hard-coded default or a truncating
/// cast would not reproduce it.
const TEST_XP: u64 = 12_345;

const PLAYER_EID: u32 = 7101;

/// Insert the sentinel chain + trigger + `grant_xp` action.
async fn seed_sentinel_chain(pool: &PgPool) {
    sqlx::query(
        "INSERT INTO resources.content_chains \
         (chain_id, description, scope_type, scope_id, enabled, priority) \
         VALUES ($1, 'grant_xp chain-replay sentinel', 'space', NULL, true, 0)",
    )
    .bind(TEST_CHAIN_ID)
    .execute(pool)
    .await
    .expect("sentinel content_chains insert must succeed");

    sqlx::query(
        "INSERT INTO resources.content_triggers \
         (chain_id, event_type, event_key, scope, once, sort_order) \
         VALUES ($1, 'interact_tag', $2, 'player', false, 0)",
    )
    .bind(TEST_CHAIN_ID)
    .bind(TEST_TAG)
    .execute(pool)
    .await
    .expect("sentinel content_triggers insert must succeed");

    // The `amount` param is the whole point — this is the shape a
    // content author would write.
    sqlx::query(
        "INSERT INTO resources.content_actions \
         (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) \
         VALUES ($1, 'grant_xp', NULL, NULL, $2::jsonb, 0, 0)",
    )
    .bind(TEST_CHAIN_ID)
    .bind(format!(r#"{{"amount": {TEST_XP}}}"#))
    .execute(pool)
    .await
    .expect("sentinel content_actions insert must succeed");
}

/// Delete by exact chain id, children first (FK order).
async fn cleanup_sentinel_chain(pool: &PgPool) {
    for stmt in [
        "DELETE FROM resources.content_actions WHERE chain_id = $1",
        "DELETE FROM resources.content_triggers WHERE chain_id = $1",
        "DELETE FROM resources.content_chains WHERE chain_id = $1",
    ] {
        sqlx::query(stmt)
            .bind(TEST_CHAIN_ID)
            .execute(pool)
            .await
            .expect("sentinel cleanup must succeed");
    }
}

fn make_space_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr
}

/// A `grant_xp` action row must survive the loader as
/// `Action::GrantXP { amount }` and execute as exactly one
/// `CellToBaseMsg::GrantXP` carrying that amount, addressed to the
/// firing player, with `notify_gm: false`.
///
/// `notify_gm` is asserted explicitly: it is the discriminator that
/// makes the base emit a GM-feedback line, and only the `gmGiveXp` path
/// sets it. A chain grant is gameplay, not a GM action — flipping it
/// would spam every player who completes the chain with GM chatter.
#[tokio::test]
async fn grant_xp_action_row_reaches_base_with_the_authored_amount() {
    let pool = require_db_or_skip!();

    // Start from a clean slate in case a previous panicking run leaked.
    cleanup_sentinel_chain(&pool).await;
    seed_sentinel_chain(&pool).await;

    let loaded = load_single_chain_for_test(&pool, TEST_CHAIN_ID).await;

    // Drop the sentinel rows before asserting so a failure can't leave
    // a live chain registered in the shared test database.
    cleanup_sentinel_chain(&pool).await;

    let chain = loaded
        .expect("DB query for the sentinel chain must succeed")
        .expect("sentinel chain must load — a None here means the trigger row was rejected");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param("entity_tag".to_string(), serde_json::json!(TEST_TAG));
    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTag,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    let resolved = engine.resolve_event(&event, &ctx);
    assert_eq!(
        resolved.actions.len(),
        1,
        "the sentinel chain's single grant_xp row must survive \
         convert_action — zero actions means the loader has no \"grant_xp\" \
         arm and the row was dropped with an \"Unknown action_type\" warn",
    );

    let mut mgr = make_space_mgr();
    mgr.create_entity(PLAYER_EID, "Agnos", [0.0; 3], [0.0; 3])
        .expect("Agnos startup space must accept the player entity");
    let p = mgr
        .get_entity_mut(PLAYER_EID)
        .expect("player entity must exist immediately after create_entity");
    p.is_player = true;
    p.player_id = Some(42);
    mgr.connect_entity(PLAYER_EID);

    let (tx, mut rx) = mpsc::channel(16);
    let exec_engine = ChainEngine::new();
    execute_actions(resolved, PLAYER_EID, 42, &tx, &mut mgr, &exec_engine).await;

    let mut grants = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::GrantXP {
            entity_id,
            xp_amount,
            notify_gm,
        } = msg
        {
            grants.push((entity_id, xp_amount, notify_gm));
        }
    }
    assert_eq!(
        grants.len(),
        1,
        "one grant_xp action must produce exactly one CellToBaseMsg::GrantXP; \
         got {grants:?} (zero means the executor has no Action::GrantXP arm \
         and the action fell through the `other =>` catch-all)",
    );
    assert_eq!(
        grants[0],
        (PLAYER_EID, TEST_XP, false),
        "GrantXP must carry (firing entity, authored amount, notify_gm=false)",
    );
}
