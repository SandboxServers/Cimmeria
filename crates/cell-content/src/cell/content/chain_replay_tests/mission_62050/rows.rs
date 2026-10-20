//! CB-N05-3 — the seeded rows of mission 62050 (chains 11000-11005) and
//! their load path. `live_db_mission_62050_rows_are_exact` pins the three
//! content tables; `live_db_mission_62050_rows_load_as_sealed` assembles each
//! chain and checks that no action is deferred.

use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};

use super::super::super::engine_loader::load_single_chain_for_test;
use super::super::assert_no_deferred_actions;
use super::CHAINS;
use crate::test_support::require_db_or_skip;

/// `(chain_id, event_type, event_key, scope, once)` as `content_triggers`
/// stores them.
type TriggerRow = (i32, String, Option<String>, String, bool);
/// `(chain_id, condition_type, target_id, target_key, operator, value,
/// sort_order)` as `content_conditions` stores them.
type ConditionRow = (
    i32,
    String,
    Option<i32>,
    Option<String>,
    String,
    Option<String>,
    i32,
);
/// `(chain_id, action_type, target_id, target_key, params, delay_ms,
/// sort_order)` as `content_actions` stores them.
type ActionRow = (i32, String, Option<i32>, Option<String>, String, i32, i32);

fn trig(chain: i32, event: &str, key: Option<&str>, once: bool) -> TriggerRow {
    (
        chain,
        event.to_string(),
        key.map(str::to_string),
        "player".to_string(),
        once,
    )
}

fn cond(
    chain: i32,
    ctype: &str,
    target: Option<i32>,
    key: Option<&str>,
    value: Option<&str>,
    ord: i32,
) -> ConditionRow {
    (
        chain,
        ctype.to_string(),
        target,
        key.map(str::to_string),
        "eq".to_string(),
        value.map(str::to_string),
        ord,
    )
}

fn act(
    chain: i32,
    atype: &str,
    target: Option<i32>,
    key: Option<&str>,
    params: &str,
    ord: i32,
) -> ActionRow {
    (
        chain,
        atype.to_string(),
        target,
        key.map(str::to_string),
        params.to_string(),
        0,
        ord,
    )
}

#[tokio::test]
async fn live_db_mission_62050_rows_are_exact() {
    let pool = require_db_or_skip!();
    let triggers = sqlx::query_as::<_, TriggerRow>(
        "SELECT chain_id, event_type, event_key, scope, once FROM resources.content_triggers \
         WHERE chain_id BETWEEN 11000 AND 11005 ORDER BY chain_id, sort_order",
    )
    .fetch_all(&pool)
    .await
    .expect("content_triggers query");
    assert_eq!(
        triggers,
        vec![
            trig(11000, "mission_completed", Some("638"), false),
            trig(11001, "player_loaded", Some("Castle_CellBlock"), true),
            trig(11002, "mission_abandoned", Some("62050"), false),
            trig(11003, "interact_tag", Some("Prisoner_329"), false),
            trig(11004, "mission_accepted", Some("62050"), false),
            trig(11005, "interact_tag", Some("Preparation_ColMarsh"), false),
        ],
    );

    let conditions = sqlx::query_as::<_, ConditionRow>(
        "SELECT chain_id, condition_type, target_id, target_key, operator, value, sort_order \
         FROM resources.content_conditions WHERE chain_id BETWEEN 11000 AND 11005 \
         ORDER BY chain_id, sort_order",
    )
    .fetch_all(&pool)
    .await
    .expect("content_conditions query");
    let offer_gates = |chain: i32| {
        vec![
            cond(chain, "world", Some(12), None, None, 0),
            cond(
                chain,
                "mission_status",
                Some(638),
                None,
                Some("completed"),
                1,
            ),
            cond(
                chain,
                "mission_status",
                Some(641),
                None,
                Some("not_active"),
                2,
            ),
            cond(
                chain,
                "mission_status",
                Some(62050),
                None,
                Some("not_active"),
                3,
            ),
        ]
    };
    let mut expected_conditions = Vec::new();
    for chain in [11000, 11001, 11002, 11003] {
        expected_conditions.extend(offer_gates(chain));
    }
    expected_conditions.extend([
        cond(11004, "world", Some(12), None, None, 0),
        cond(
            11004,
            "step_status",
            Some(62050),
            Some("62051"),
            Some("active"),
            1,
        ),
        cond(11005, "world", Some(12), None, None, 0),
        cond(
            11005,
            "mission_status",
            Some(640),
            None,
            Some("completed"),
            1,
        ),
        cond(
            11005,
            "step_status",
            Some(62050),
            Some("62054"),
            Some("active"),
            2,
        ),
    ]);
    assert_eq!(conditions, expected_conditions);

    let actions = sqlx::query_as::<_, ActionRow>(
        "SELECT chain_id, action_type, target_id, target_key, params::text, delay_ms, sort_order \
         FROM resources.content_actions WHERE chain_id BETWEEN 11000 AND 11005 \
         ORDER BY chain_id, sort_order",
    )
    .fetch_all(&pool)
    .await
    .expect("content_actions query");
    assert_eq!(
        actions,
        vec![
            act(
                11000,
                "add_dialog_set",
                Some(2795),
                None,
                r#"{"slot": 17}"#,
                0
            ),
            act(
                11001,
                "add_dialog_set",
                Some(2795),
                None,
                r#"{"slot": 17}"#,
                0
            ),
            act(
                11002,
                "add_dialog_set",
                Some(2795),
                None,
                r#"{"slot": 17}"#,
                0
            ),
            act(11003, "accept_mission", Some(62050), None, "{}", 0),
            act(11004, "advance_step", Some(62050), Some("62054"), "{}", 0),
            act(
                11004,
                "remove_dialog_set",
                Some(2795),
                None,
                r#"{"slot": 17}"#,
                1
            ),
            act(11004, "display_dialog", Some(60150), None, "{}", 2),
            act(
                11005,
                "complete_objective",
                Some(62050),
                Some("62055"),
                "{}",
                0
            ),
        ],
    );
}

#[tokio::test]
async fn live_db_mission_62050_rows_load_as_sealed() {
    let pool = require_db_or_skip!();
    let mut engine = ChainEngine::new();
    for id in CHAINS {
        let chain = load_single_chain_for_test(&pool, id)
            .await
            .unwrap_or_else(|e| panic!("DB query for chain {id} must succeed: {e}"))
            .unwrap_or_else(|| panic!("chain {id} must exist and assemble"));
        engine.register_chain(chain);
    }
    let action_counts = [1, 1, 1, 1, 3, 1];
    for (id, count) in CHAINS.into_iter().zip(action_counts) {
        let id64 = i64::from(id);
        let actions = engine.get_chain_actions(id64);
        assert_eq!(actions.len(), count, "chain {id} action count");
        assert_eq!(engine.is_once(id64), id == 11001, "chain {id} once");
        // Delays are read back from the engine, so a deferred row fails here.
        let resolved = ResolvedActions {
            actions: actions.iter().map(|(a, _)| (id64, a.clone())).collect(),
            action_delays: actions.iter().map(|(_, d)| *d).collect(),
            ..Default::default()
        };
        assert_no_deferred_actions(&resolved, id64);
    }
}
