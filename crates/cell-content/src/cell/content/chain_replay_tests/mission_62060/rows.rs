//! CB-N06-3 — the seeded rows of mission 62060 (chains 11030-11041) and
//! their load path. `live_db_mission_62060_rows_are_exact` pins the three
//! content tables; `live_db_mission_62060_rows_load_as_sealed` assembles each
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
/// sort_order)` as `content_actions` stores them. `params` is read with
/// `params::text`, so jsonb reorders its keys (shorter keys first).
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
    op: &str,
    value: Option<&str>,
    ord: i32,
) -> ConditionRow {
    (
        chain,
        ctype.to_string(),
        target,
        key.map(str::to_string),
        op.to_string(),
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

/// The three gates of an offer or accept chain (`world` 12 first).
fn offer_gates(chain: i32) -> Vec<ConditionRow> {
    vec![
        cond(chain, "world", Some(12), None, "eq", None, 0),
        cond(
            chain,
            "mission_status",
            Some(638),
            None,
            "eq",
            Some("completed"),
            1,
        ),
        cond(
            chain,
            "mission_status",
            Some(641),
            None,
            "eq",
            Some("not_active"),
            2,
        ),
        cond(
            chain,
            "mission_status",
            Some(62060),
            None,
            "eq",
            Some("not_active"),
            3,
        ),
    ]
}

/// The world gate plus `step_status(62060, 62061) = 'active'`.
fn step_gate(chain: i32) -> Vec<ConditionRow> {
    vec![
        cond(chain, "world", Some(12), None, "eq", None, 0),
        cond(
            chain,
            "step_status",
            Some(62060),
            Some("62061"),
            "eq",
            Some("active"),
            1,
        ),
    ]
}

/// The four actions of an aid or sabotage click: grant, unbind both, complete.
fn choice_actions(chain: i32) -> Vec<ActionRow> {
    vec![
        act(
            chain,
            "add_item",
            Some(2893),
            None,
            r#"{"qty": 2, "container": 0}"#,
            0,
        ),
        act(
            chain,
            "remove_dialog_set",
            Some(7509),
            None,
            r#"{"slot": 22}"#,
            1,
        ),
        act(
            chain,
            "remove_dialog_set",
            Some(3862),
            None,
            r#"{"slot": 176}"#,
            2,
        ),
        act(
            chain,
            "complete_objective",
            Some(62060),
            Some("62062"),
            "{}",
            3,
        ),
    ]
}

#[tokio::test]
async fn live_db_mission_62060_rows_are_exact() {
    let pool = require_db_or_skip!();
    let triggers = sqlx::query_as::<_, TriggerRow>(
        "SELECT chain_id, event_type, event_key, scope, once FROM resources.content_triggers \
         WHERE chain_id BETWEEN 11030 AND 11041 ORDER BY chain_id, sort_order",
    )
    .fetch_all(&pool)
    .await
    .expect("content_triggers query");
    assert_eq!(
        triggers,
        vec![
            trig(11030, "mission_completed", Some("638"), false),
            trig(11031, "player_loaded", Some("Castle_CellBlock"), true),
            trig(11032, "player_loaded", Some("Castle_CellBlock"), true),
            trig(
                11033,
                "interact_tag",
                Some("Cellblock_Ambient_Prisoner1"),
                false
            ),
            trig(
                11034,
                "interact_tag",
                Some("Cellblock_Ambient_Prisoner2"),
                false
            ),
            trig(11035, "mission_accepted", Some("62060"), false),
            trig(
                11036,
                "interact_tag",
                Some("Cellblock_Ambient_Prisoner1"),
                false
            ),
            trig(
                11037,
                "interact_tag",
                Some("Cellblock_Ambient_Prisoner2"),
                false
            ),
            trig(11038, "interact_tag", Some("329_CellDoorButton"), false),
            trig(11039, "mission_abandoned", Some("62060"), false),
            trig(11040, "mission_accepted", Some("641"), false),
            trig(11041, "mission_abandoned", Some("62060"), false),
        ],
    );

    let conditions = sqlx::query_as::<_, ConditionRow>(
        "SELECT chain_id, condition_type, target_id, target_key, operator, value, sort_order \
         FROM resources.content_conditions WHERE chain_id BETWEEN 11030 AND 11041 \
         ORDER BY chain_id, sort_order",
    )
    .fetch_all(&pool)
    .await
    .expect("content_conditions query");
    let mut expected = Vec::new();
    expected.extend(offer_gates(11030));
    expected.extend(offer_gates(11031));
    expected.extend(step_gate(11032));
    expected.extend(offer_gates(11033));
    expected.extend(offer_gates(11034));
    expected.extend(step_gate(11035));
    expected.extend(step_gate(11036));
    expected.extend(step_gate(11037));
    expected.extend(step_gate(11038));
    expected.push(cond(11039, "world", Some(12), None, "eq", None, 0));
    expected.push(cond(11040, "world", Some(12), None, "eq", None, 0));
    expected.push(cond(
        11040,
        "mission_status",
        Some(62060),
        None,
        "eq",
        Some("not_active"),
        1,
    ));
    expected.push(cond(11041, "world", Some(12), None, "eq", None, 0));
    expected.push(cond(
        11041,
        "mission_status",
        Some(641),
        None,
        "neq",
        Some("not_active"),
        1,
    ));
    assert_eq!(conditions, expected);

    let actions = sqlx::query_as::<_, ActionRow>(
        "SELECT chain_id, action_type, target_id, target_key, params::text, delay_ms, sort_order \
         FROM resources.content_actions WHERE chain_id BETWEEN 11030 AND 11041 \
         ORDER BY chain_id, sort_order",
    )
    .fetch_all(&pool)
    .await
    .expect("content_actions query");
    let mut expected = vec![
        act(
            11030,
            "add_dialog_set",
            Some(3862),
            None,
            r#"{"slot": 176}"#,
            0,
        ),
        act(
            11031,
            "add_dialog_set",
            Some(3862),
            None,
            r#"{"slot": 176}"#,
            0,
        ),
        act(
            11032,
            "add_dialog_set",
            Some(3862),
            None,
            r#"{"slot": 176}"#,
            0,
        ),
        act(
            11032,
            "add_dialog_set",
            Some(7509),
            None,
            r#"{"slot": 22}"#,
            1,
        ),
        act(11033, "accept_mission", Some(62060), None, "{}", 0),
        act(11034, "accept_mission", Some(62060), None, "{}", 0),
        act(
            11035,
            "add_dialog_set",
            Some(7509),
            None,
            r#"{"slot": 22}"#,
            0,
        ),
        act(11035, "display_dialog", Some(60151), None, "{}", 1),
    ];
    expected.extend(choice_actions(11036));
    expected.extend(choice_actions(11037));
    expected.extend(choice_actions(11038));
    expected.push(act(
        11039,
        "remove_dialog_set",
        Some(7509),
        None,
        r#"{"slot": 22}"#,
        0,
    ));
    expected.push(act(
        11040,
        "remove_dialog_set",
        Some(3862),
        None,
        r#"{"slot": 176}"#,
        0,
    ));
    expected.push(act(
        11041,
        "remove_dialog_set",
        Some(3862),
        None,
        r#"{"slot": 176}"#,
        0,
    ));
    assert_eq!(actions, expected);
}

#[tokio::test]
async fn live_db_mission_62060_rows_load_as_sealed() {
    let pool = require_db_or_skip!();
    let mut engine = ChainEngine::new();
    for id in CHAINS {
        let chain = load_single_chain_for_test(&pool, id)
            .await
            .unwrap_or_else(|e| panic!("DB query for chain {id} must succeed: {e}"))
            .unwrap_or_else(|| panic!("chain {id} must exist and assemble"));
        engine.register_chain(chain);
    }
    let action_counts = [1, 1, 2, 1, 1, 2, 4, 4, 4, 1, 1, 1];
    for (id, count) in CHAINS.into_iter().zip(action_counts) {
        let id64 = i64::from(id);
        let actions = engine.get_chain_actions(id64);
        assert_eq!(actions.len(), count, "chain {id} action count");
        let once = matches!(id, 11031 | 11032);
        assert_eq!(engine.is_once(id64), once, "chain {id} once");
        // Delays are read back from the engine, so a deferred row fails here.
        let resolved = ResolvedActions {
            actions: actions.iter().map(|(a, _)| (id64, a.clone())).collect(),
            action_delays: actions.iter().map(|(_, d)| *d).collect(),
            ..Default::default()
        };
        assert_no_deferred_actions(&resolved, id64);
    }
}
