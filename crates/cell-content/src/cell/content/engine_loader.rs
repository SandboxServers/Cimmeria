//! Content engine construction — builds a [`ChainEngine`] from chains stored
//! in the database.
//!
//! All chain data lives in the `resources.content_*` tables. Startup loads
//! every enabled chain; if the DB is unavailable or the tables are missing,
//! we return an empty engine and the server runs without content scripting.

use sqlx::PgPool;

use cimmeria_content_engine::chain::{Chain, ChainEngine};
use cimmeria_content_engine::loader::{
    build_chains_from_rows, DbActionRow, DbChainRow, DbConditionRow, DbTriggerRow,
};

/// Build the content engine by loading chains from the database.
///
/// Returns an empty engine if the DB pool is unavailable or the content
/// tables don't exist yet — all chain data lives in the database.
pub async fn build_engine(db_pool: Option<&PgPool>) -> ChainEngine {
    if let Some(pool) = db_pool {
        match load_chains_from_db(pool).await {
            Ok(chains) => {
                let mut engine = ChainEngine::new();
                for chain in chains {
                    engine.register_chain(chain);
                }
                tracing::info!(
                    chains = engine.chain_count(),
                    "Content engine loaded from database"
                );
                return engine;
            }
            Err(e) => {
                tracing::error!(
                    "Failed to load content chains from DB: {e} — content engine will be empty"
                );
            }
        }
    } else {
        tracing::warn!("No DB pool available — content engine will be empty");
    }

    ChainEngine::new()
}

/// Load all enabled content chains from the database.
async fn load_chains_from_db(pool: &PgPool) -> Result<Vec<Chain>, sqlx::Error> {
    use sqlx::Row;

    let chain_rows: Vec<DbChainRow> = sqlx::query(
        "SELECT chain_id, description, scope_type, scope_id, enabled, priority \
         FROM resources.content_chains ORDER BY chain_id",
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| DbChainRow {
        chain_id: r.get("chain_id"),
        description: r.get("description"),
        scope_type: r.get("scope_type"),
        scope_id: r.get("scope_id"),
        enabled: r.get("enabled"),
        priority: r.get("priority"),
    })
    .collect();

    let trigger_rows: Vec<DbTriggerRow> = sqlx::query(
        "SELECT chain_id, event_type, event_key, scope, once, sort_order \
         FROM resources.content_triggers ORDER BY chain_id, sort_order",
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| DbTriggerRow {
        chain_id: r.get("chain_id"),
        event_type: r.get("event_type"),
        event_key: r.get("event_key"),
        scope: r.get("scope"),
        once: r.get("once"),
        sort_order: r.get("sort_order"),
    })
    .collect();

    let condition_rows: Vec<DbConditionRow> = sqlx::query(
        "SELECT chain_id, condition_type, target_id, target_key, operator, value, sort_order \
         FROM resources.content_conditions ORDER BY chain_id, sort_order",
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| DbConditionRow {
        chain_id: r.get("chain_id"),
        condition_type: r.get("condition_type"),
        target_id: r.get("target_id"),
        target_key: r.get("target_key"),
        operator: r.get("operator"),
        value: r.get("value"),
        sort_order: r.get("sort_order"),
    })
    .collect();

    let action_rows: Vec<DbActionRow> = sqlx::query(
        "SELECT chain_id, action_type, target_id, target_key, params, delay_ms, sort_order \
         FROM resources.content_actions ORDER BY chain_id, sort_order",
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| DbActionRow {
        chain_id: r.get("chain_id"),
        action_type: r.get("action_type"),
        target_id: r.get("target_id"),
        target_key: r.get("target_key"),
        params: r.get("params"),
        delay_ms: r.get("delay_ms"),
        sort_order: r.get("sort_order"),
    })
    .collect();

    tracing::info!(
        chains = chain_rows.len(),
        triggers = trigger_rows.len(),
        conditions = condition_rows.len(),
        actions = action_rows.len(),
        "Loaded content engine rows from database"
    );

    Ok(build_chains_from_rows(
        chain_rows,
        trigger_rows,
        condition_rows,
        action_rows,
    ))
}

/// Load a single content chain by id, fully assembled (rows from all four
/// `content_*` tables joined into a `Chain`). Returns `None` if no row exists
/// in `content_chains` for the id.
///
/// Used by the chain-replay tests in `chain_replay_tests` to exercise a
/// specific seeded chain through the engine without loading the entire content
/// surface. Mirrors `load_chains_from_db` but scopes every query by
/// `chain_id = $1` so the test runs in milliseconds even on the full seed.
/// Behind `test-support` for the content tests that stay in
/// `cimmeria-services` (`cell::content_tests`).
#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub async fn load_single_chain_for_test(
    pool: &PgPool,
    chain_id: i32,
) -> Result<Option<Chain>, sqlx::Error> {
    use sqlx::Row;

    let chain_rows: Vec<DbChainRow> = sqlx::query(
        "SELECT chain_id, description, scope_type, scope_id, enabled, priority \
         FROM resources.content_chains WHERE chain_id = $1",
    )
    .bind(chain_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| DbChainRow {
        chain_id: r.get("chain_id"),
        description: r.get("description"),
        scope_type: r.get("scope_type"),
        scope_id: r.get("scope_id"),
        enabled: r.get("enabled"),
        priority: r.get("priority"),
    })
    .collect();

    if chain_rows.is_empty() {
        return Ok(None);
    }

    let trigger_rows: Vec<DbTriggerRow> = sqlx::query(
        "SELECT chain_id, event_type, event_key, scope, once, sort_order \
         FROM resources.content_triggers WHERE chain_id = $1 ORDER BY sort_order",
    )
    .bind(chain_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| DbTriggerRow {
        chain_id: r.get("chain_id"),
        event_type: r.get("event_type"),
        event_key: r.get("event_key"),
        scope: r.get("scope"),
        once: r.get("once"),
        sort_order: r.get("sort_order"),
    })
    .collect();

    let condition_rows: Vec<DbConditionRow> = sqlx::query(
        "SELECT chain_id, condition_type, target_id, target_key, operator, value, sort_order \
         FROM resources.content_conditions WHERE chain_id = $1 ORDER BY sort_order",
    )
    .bind(chain_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| DbConditionRow {
        chain_id: r.get("chain_id"),
        condition_type: r.get("condition_type"),
        target_id: r.get("target_id"),
        target_key: r.get("target_key"),
        operator: r.get("operator"),
        value: r.get("value"),
        sort_order: r.get("sort_order"),
    })
    .collect();

    let action_rows: Vec<DbActionRow> = sqlx::query(
        "SELECT chain_id, action_type, target_id, target_key, params, delay_ms, sort_order \
         FROM resources.content_actions WHERE chain_id = $1 ORDER BY sort_order",
    )
    .bind(chain_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| DbActionRow {
        chain_id: r.get("chain_id"),
        action_type: r.get("action_type"),
        target_id: r.get("target_id"),
        target_key: r.get("target_key"),
        params: r.get("params"),
        delay_ms: r.get("delay_ms"),
        sort_order: r.get("sort_order"),
    })
    .collect();

    Ok(
        build_chains_from_rows(chain_rows, trigger_rows, condition_rows, action_rows)
            .into_iter()
            .next(),
    )
}

/// Test helper: load every in-memory `Chain` expansion produced by the
/// chain id. The loader materializes one `Chain` per `content_triggers`
/// row, so chains with multi-trigger OR semantics (e.g., chain 1103
/// firing on any of `Barracks_Guard1/2/3` deaths) need all expansions
/// registered for the engine to match correctly. `load_single_chain_for_test`
/// only returns the first expansion, which would silently mask drift in
/// the 2nd+ trigger rows.
#[cfg(test)]
pub(super) async fn load_chain_expansions_for_test(
    pool: &PgPool,
    chain_id: i32,
) -> Result<Vec<Chain>, sqlx::Error> {
    use sqlx::Row;

    let chain_rows: Vec<DbChainRow> = sqlx::query(
        "SELECT chain_id, description, scope_type, scope_id, enabled, priority \
         FROM resources.content_chains WHERE chain_id = $1",
    )
    .bind(chain_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| DbChainRow {
        chain_id: r.get("chain_id"),
        description: r.get("description"),
        scope_type: r.get("scope_type"),
        scope_id: r.get("scope_id"),
        enabled: r.get("enabled"),
        priority: r.get("priority"),
    })
    .collect();

    if chain_rows.is_empty() {
        return Ok(vec![]);
    }

    let trigger_rows: Vec<DbTriggerRow> = sqlx::query(
        "SELECT chain_id, event_type, event_key, scope, once, sort_order \
         FROM resources.content_triggers WHERE chain_id = $1 ORDER BY sort_order",
    )
    .bind(chain_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| DbTriggerRow {
        chain_id: r.get("chain_id"),
        event_type: r.get("event_type"),
        event_key: r.get("event_key"),
        scope: r.get("scope"),
        once: r.get("once"),
        sort_order: r.get("sort_order"),
    })
    .collect();

    let condition_rows: Vec<DbConditionRow> = sqlx::query(
        "SELECT chain_id, condition_type, target_id, target_key, operator, value, sort_order \
         FROM resources.content_conditions WHERE chain_id = $1 ORDER BY sort_order",
    )
    .bind(chain_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| DbConditionRow {
        chain_id: r.get("chain_id"),
        condition_type: r.get("condition_type"),
        target_id: r.get("target_id"),
        target_key: r.get("target_key"),
        operator: r.get("operator"),
        value: r.get("value"),
        sort_order: r.get("sort_order"),
    })
    .collect();

    let action_rows: Vec<DbActionRow> = sqlx::query(
        "SELECT chain_id, action_type, target_id, target_key, params, delay_ms, sort_order \
         FROM resources.content_actions WHERE chain_id = $1 ORDER BY sort_order",
    )
    .bind(chain_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| DbActionRow {
        chain_id: r.get("chain_id"),
        action_type: r.get("action_type"),
        target_id: r.get("target_id"),
        target_key: r.get("target_key"),
        params: r.get("params"),
        delay_ms: r.get("delay_ms"),
        sort_order: r.get("sort_order"),
    })
    .collect();

    Ok(build_chains_from_rows(
        chain_rows,
        trigger_rows,
        condition_rows,
        action_rows,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `build_engine(None)` returns an empty engine — the codepath
    /// the server takes when started without a DB pool. Pin so a
    /// regression that panics on the None branch (e.g. via unwrap)
    /// gets caught.
    #[tokio::test]
    async fn build_engine_with_none_returns_empty_engine() {
        let engine = build_engine(None).await;
        assert_eq!(engine.chain_count(), 0);
    }

    /// Live-DB sanity: against the seeded `resources.content_*` tables,
    /// `build_engine` returns a non-empty engine. Catches a JOIN
    /// breakage, a column rename, or a content_chains schema drift
    /// that the rest of the test suite wouldn't surface.
    #[tokio::test]
    async fn live_db_build_engine_with_db_pool_loads_seeded_chains() {
        let pool = crate::test_support::require_db_or_skip!();
        let engine = build_engine(Some(&pool)).await;
        assert!(
            engine.chain_count() > 0,
            "seeded resources.content_chains has rows; engine must load them"
        );
    }

    /// Live-DB guard: the Health Slappack (2893) has no `item_use` chain.
    /// It is a native consumable (`consumable_use`: `items_event_sets`
    /// event 5 -> ability 648 -> `HealHealth`, `HealAmount` 500), and a chain
    /// for the same item would own it and switch the native path off (no
    /// refusal feedback at full health, and the chain's apply-then-consume
    /// order). Its old chain 4001 is retired; this fails if it comes back.
    #[tokio::test]
    async fn live_db_the_health_slappack_has_no_item_use_chain() {
        let pool = crate::test_support::require_db_or_skip!();
        let engine = build_engine(Some(&pool)).await;
        assert!(engine.chain_count() > 0, "the seeded chains must load");
        assert!(
            !engine.has_item_use_chain(2893),
            "an item_use chain for 2893 would take the slappack off the native \
             consumable path; retire it or document the switch"
        );
    }

    /// Live-DB guard: using the Ambernol vial (item 19) on its mission step
    /// resolves to chain 1034's actions in order, cast first and then the
    /// consume: `launch_ability 1374`, `remove_item 19 x1`. The vial is the
    /// contrast to the native consumables: a chain owns it (the mission
    /// gate, the completion), so its event-5 binding to 1374 is not applied
    /// natively. Asserted by behaviour, not by chain id.
    ///
    /// Also the C08a zero-delay guard: every seeded action row has
    /// `delay_ms = 0`, so `resolved.action_delays` must be index-aligned
    /// with `resolved.actions` and all zero for this real chain.
    #[tokio::test]
    async fn live_db_item_use_19_resolves_to_the_ambernol_cast_then_consume() {
        use cimmeria_content_engine::actions::Action;
        use cimmeria_content_engine::context::ExecutionContext;
        use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

        const AMBERNOL_ITEM_ID: i32 = 19;
        const CURE_STASIS_SICKNESS: i32 = 1374;

        let pool = crate::test_support::require_db_or_skip!();
        let engine = build_engine(Some(&pool)).await;
        assert!(engine.has_item_use_chain(AMBERNOL_ITEM_ID));

        // The chain gates on mission 639 step 2343 being active; the live
        // dispatch site fills this from the player's missions.
        let mut ctx = ExecutionContext::new();
        ctx.set_param("item_id".to_string(), serde_json::json!(AMBERNOL_ITEM_ID));
        ctx.set_param(
            "mission_639_step_2343_status".to_string(),
            serde_json::json!("active"),
        );
        let event = TriggerEvent {
            trigger_type: TriggerType::ItemUse,
            source_entity: None,
            target_entity: None,
            params: ctx.params.clone(),
        };

        let resolved = engine.resolve_event(&event, &ctx);
        let actions: Vec<&Action> = resolved.actions.iter().map(|(_, a)| a).collect();
        assert!(
            actions.len() >= 2,
            "useItem(19) on step 2343 must resolve chain 1034; got {actions:?}"
        );
        match actions[0] {
            Action::LaunchAbility { ability_id, .. } => {
                assert_eq!(*ability_id, CURE_STASIS_SICKNESS)
            }
            other => panic!("expected LaunchAbility first, got {other:?}"),
        }
        match actions[1] {
            Action::RemoveItem { item_id, count } => {
                assert_eq!(*item_id, AMBERNOL_ITEM_ID);
                assert_eq!(*count, 1, "consume one vial per use");
            }
            other => panic!("expected RemoveItem second, got {other:?}"),
        }
        assert_eq!(
            resolved.action_delays,
            vec![0; actions.len()],
            "a real seeded chain with no delay_ms rows must resolve with \
             action_delays all zero, index-aligned with actions"
        );
    }

    /// `load_single_chain_for_test` returns `Ok(None)` for a chain id
    /// that doesn't exist in the seed. Boundary used by the
    /// chain-replay tests; pin so a regression that returns
    /// `Some(empty_chain)` instead can't slip through.
    ///
    /// Sentinel uses the project's reserved 0x7000_xxxx range per
    /// TESTING.md "Sentinel id discipline" rather than a raw negative
    /// id. The seed's content_chains rows are positive low-thousands,
    /// so 0x7000_2000 is guaranteed to miss without the loader needing
    /// to special-case negatives.
    #[tokio::test]
    async fn live_db_load_single_chain_returns_none_for_missing_id() {
        let pool = crate::test_support::require_db_or_skip!();
        const TEST_MISSING_CHAIN_ID: i32 = 0x7000_2000;
        let result = load_single_chain_for_test(&pool, TEST_MISSING_CHAIN_ID)
            .await
            .unwrap();
        assert!(result.is_none());
    }
}
