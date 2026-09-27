//! `world` condition chain-replay guard (Harset H07).
//!
//! The bug shape this guards is "a chain authored for world A fires in
//! world B". `OnRegionEnter` matches a bare `point_sets.name` string and
//! carries no world, so the Harset Command Center door chain
//! (`enter_region Harset.CommandCenterTransition` → teleport) and its
//! mirror inside `Harset_CmdCenter` are only distinguishable by the
//! acting player's world.
//!
//! Like [`grant_xp`](super::grant_xp), this module seeds its own sentinel
//! chain: there are zero `condition_type = 'world'` rows in
//! `db/resources/Content/Seed/` today. It fires the event through the real
//! [`fire_enter_region`](super::super::fire_enter_region) dispatcher rather
//! than calling `resolve_event` directly, because the population step is
//! half of what can break — a `resolve_event`-only test still passes when
//! a dispatcher forgets `populate_world_context`.
//!
//! Each of the four moving parts fails a specific assertion when reverted:
//!
//! - Loader arm (`crates/content-engine/src/loader/condition.rs`,
//!   `"world"`): `convert_condition` returns `None`, the row is dropped
//!   with an "Unknown condition_type" warn, and the chain loads **ungated**
//!   — the `Harset_CmdCenter` case fires and its assertion fails.
//! - Evaluator arm (`Condition::World` in
//!   `crates/content-engine/src/conditions.rs`): same ungated outcome, or
//!   a match in the wrong world.
//! - Populator call in `event_dispatch/region.rs`: `ctx.world_id` stays
//!   `None`, the condition fails closed, and the Harset case produces no
//!   `GrantXP`.
//! - `SpaceManager::stamp_world_ids` / `get_entity_world_id`: same as
//!   above — the world is known by name but has no id.
//!
//! Sentinel id range: `0x7007_0000..0x7007_0001` (packet H07), one id per
//! test — see [`CHAIN_ID_POSITIVE`]. Sibling reservations in
//! `crates/services` run `0x7000_1000..0x7000_1B00`, `0x7000_2000`,
//! `0x7000_3000`, `0x7000_4000`, `0x7000_4242` and `0x7000_5000`; this
//! steps past all of them. Cleanup deletes the exact ids inserted, never a
//! range.

use std::collections::HashMap;

use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::engine_loader::load_single_chain_for_test;
use super::super::fire_enter_region;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::WorldRow;
use crate::test_support::require_db_or_skip;
use cimmeria_content_engine::chain::ChainEngine;

/// Sentinel `content_chains.chain_id` per test — the low byte is
/// partitioned so the two tests never touch the same rows.
///
/// They must not share one id: both call `cleanup → seed → load →
/// cleanup`, so under plain `cargo test` (which, unlike the
/// `ci-live-db` nextest profile, does not serialise them) one test's
/// opening cleanup can delete the other's rows mid-load — surfacing as
/// "sentinel chain must load" or as a zero-condition chain, i.e. a
/// failure that reads exactly like a missing loader arm.
const CHAIN_ID_POSITIVE: i32 = 0x7007_0000;
const CHAIN_ID_NEGATIVE: i32 = 0x7007_0001;

/// Sentinel region tag, deliberately *not* world-qualified. Real seed
/// convention world-qualifies these names (`Castle_Cellblock.Region2`),
/// which is the cheap half of the fix; the bare name here is what the
/// condition has to cope with when an author forgets.
const TEST_REGION_TAG: &str = "CIMMERIA_TEST_H07.CommandCenterTransition";

/// `resources.worlds.world_id` for `Harset` — the world the sentinel
/// chain is gated to.
const HARSET: i32 = 57;
/// `resources.worlds.world_id` for `Harset_CmdCenter` — the adjacent
/// world the chain must NOT fire in.
const HARSET_CMD_CENTER: i32 = 68;

const PLAYER_EID: u32 = 7701;
const PLAYER_ID: i32 = 7702;
/// Not a round number: a truncating cast or a hardcoded default would not
/// reproduce it.
const TEST_XP: u64 = 6_107;

/// Insert the sentinel chain: `enter_region` trigger + `world eq 57`
/// condition + a `grant_xp` action whose `CellToBaseMsg` is observable.
async fn seed_sentinel_chain(pool: &PgPool, chain_id: i32) {
    sqlx::query(
        "INSERT INTO resources.content_chains \
         (chain_id, description, scope_type, scope_id, enabled, priority) \
         VALUES ($1, 'world condition chain-replay sentinel', 'space', NULL, true, 0)",
    )
    .bind(chain_id)
    .execute(pool)
    .await
    .expect("sentinel content_chains insert must succeed");

    sqlx::query(
        "INSERT INTO resources.content_triggers \
         (chain_id, event_type, event_key, scope, once, sort_order) \
         VALUES ($1, 'enter_region', $2, 'player', false, 0)",
    )
    .bind(chain_id)
    .bind(TEST_REGION_TAG)
    .execute(pool)
    .await
    .expect("sentinel content_triggers insert must succeed");

    // The authoring shape a seed author writes: world id in `target_id`,
    // `eq` in `operator`, `target_key` and `value` unused.
    sqlx::query(
        "INSERT INTO resources.content_conditions \
         (chain_id, condition_type, target_id, target_key, operator, value, sort_order) \
         VALUES ($1, 'world', $2, NULL, 'eq', NULL, 0)",
    )
    .bind(chain_id)
    .bind(HARSET)
    .execute(pool)
    .await
    .expect("sentinel content_conditions insert must succeed");

    sqlx::query(
        "INSERT INTO resources.content_actions \
         (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) \
         VALUES ($1, 'grant_xp', NULL, NULL, $2::jsonb, 0, 0)",
    )
    .bind(chain_id)
    .bind(format!(r#"{{"amount": {TEST_XP}}}"#))
    .execute(pool)
    .await
    .expect("sentinel content_actions insert must succeed");
}

/// Delete by exact chain id, children first (FK order).
async fn cleanup_sentinel_chain(pool: &PgPool, chain_id: i32) {
    for stmt in [
        "DELETE FROM resources.content_actions WHERE chain_id = $1",
        "DELETE FROM resources.content_conditions WHERE chain_id = $1",
        "DELETE FROM resources.content_triggers WHERE chain_id = $1",
        "DELETE FROM resources.content_chains WHERE chain_id = $1",
    ] {
        sqlx::query(stmt)
            .bind(chain_id)
            .execute(pool)
            .await
            .expect("sentinel cleanup must succeed");
    }
}

/// Both Harset worlds, with their real world ids stamped on — the same
/// stamping the cell service does at startup from `resources.worlds`.
fn make_space_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces>
        <Space WorldName="Harset" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />
        <Space WorldName="Harset_CmdCenter" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />
    </Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces>
        <Space WorldName="Harset" />
        <Space WorldName="Harset_CmdCenter" />
    </Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr.stamp_world_rows(&HashMap::from([
        ("Harset".to_string(), WorldRow::enforcing(HARSET)),
        (
            "Harset_CmdCenter".to_string(),
            WorldRow::enforcing(HARSET_CMD_CENTER),
        ),
    ]));
    mgr
}

/// Seed the sentinel chain, load it, drop its rows, and register it in a
/// fresh engine. Callers gate on `require_db_or_skip!` first.
async fn load_sentinel_engine(pool: &PgPool, chain_id: i32) -> ChainEngine {
    // Start from a clean slate in case a previous panicking run leaked.
    cleanup_sentinel_chain(pool, chain_id).await;
    seed_sentinel_chain(pool, chain_id).await;

    let loaded = load_single_chain_for_test(pool, chain_id).await;

    // Drop the sentinel rows before asserting so a failure can't leave a
    // live chain registered in the shared test database.
    cleanup_sentinel_chain(pool, chain_id).await;

    let chain = loaded
        .expect("DB query for the sentinel chain must succeed")
        .expect("sentinel chain must load — a None here means the trigger row was rejected");
    assert_eq!(
        chain.conditions.len(),
        1,
        "the `world` condition row must survive convert_condition. Zero conditions \
         means the loader has no \"world\" arm, the row was dropped with an \
         \"Unknown condition_type\" warn, and the chain is now UNGATED — it would \
         fire in every world, which is the exact bug this chain guards against",
    );

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);
    engine
}

/// Put a connected player entity into `world_name` and fire the sentinel
/// region event, returning every `GrantXP` that reached the base.
async fn grants_after_entering_region(
    mgr: &mut SpaceManager,
    world_name: &str,
    engine: &ChainEngine,
) -> Vec<u64> {
    mgr.create_entity(PLAYER_EID, world_name, [0.0; 3], [0.0; 3])
        .unwrap_or_else(|e| panic!("{world_name} must accept the player entity: {e}"));
    {
        let p = mgr
            .get_entity_mut(PLAYER_EID)
            .expect("player entity must exist immediately after create_entity");
        p.is_player = true;
        p.player_id = Some(PLAYER_ID);
    }
    mgr.connect_entity(PLAYER_EID);

    let (tx, mut rx) = mpsc::channel(16);
    fire_enter_region(PLAYER_EID, PLAYER_ID, TEST_REGION_TAG, engine, &tx, mgr).await;

    let mut grants = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::GrantXP { xp_amount, .. } = msg {
            grants.push(xp_amount);
        }
    }
    grants
}

/// A chain gated on `world eq 57` fires when the player is in Harset.
///
/// The positive half proves the populator ran: `Condition::World` fails
/// closed on a context with no `world_id`, so this assertion goes red the
/// moment `populate_world_context` is dropped from `fire_enter_region`, or
/// `stamp_world_ids` stops filling `WorldDef::world_id`.
#[tokio::test]
async fn world_gated_chain_fires_in_the_authored_world() {
    let pool = require_db_or_skip!();
    let engine = load_sentinel_engine(&pool, CHAIN_ID_POSITIVE).await;

    let mut mgr = make_space_mgr();
    let grants = grants_after_entering_region(&mut mgr, "Harset", &engine).await;

    assert_eq!(
        grants,
        vec![TEST_XP],
        "a `world eq 57` chain must fire for a player standing in Harset (57); \
         an empty list means the condition failed closed — ctx.world_id was never \
         populated from the player's space",
    );
}

/// The same chain, the same region tag, one world over: it must not fire.
///
/// This is the regression the packet exists for. Region keys are bare
/// `point_sets.name` strings and the `enter_region` trigger carries no
/// world, so without the condition this event resolves identically in
/// `Harset_CmdCenter`.
#[tokio::test]
async fn world_gated_chain_does_not_fire_in_the_adjacent_world() {
    let pool = require_db_or_skip!();
    let engine = load_sentinel_engine(&pool, CHAIN_ID_NEGATIVE).await;

    let mut mgr = make_space_mgr();
    let grants = grants_after_entering_region(&mut mgr, "Harset_CmdCenter", &engine).await;

    // Note for the next reader: this assertion is also satisfied when the
    // populator is missing entirely (no world_id → fail closed → nothing
    // fires), so it does not on its own bracket the behaviour. The
    // positive test above is what proves the populator ran; read the pair
    // together.
    assert!(
        grants.is_empty(),
        "a `world eq 57` chain must NOT fire for a player in Harset_CmdCenter (68); \
         got {grants:?}. A non-empty list means the chain is ungated — either the \
         loader dropped the condition row or the evaluator matched the wrong world",
    );
}
