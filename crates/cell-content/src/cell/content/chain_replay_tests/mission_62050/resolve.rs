//! CB-N05-3 — the gates of mission 62050 resolved through the full seeded
//! engine (`build_engine`, so the stock chains 1051, 1052 and 1152 are
//! present). Each fire keeps only the `(chain_id, action)` pairs of
//! [`CHAINS`] unless a test says otherwise.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::super::engine_loader::build_engine;
use super::{ctx as new_ctx, CELLBLOCK_WORLD_ID, CHAINS};
use crate::test_support::require_db_or_skip;

/// `(key, value)` pairs: string state params, or event params.
type Pairs = &'static [(&'static str, &'static str)];

/// Offer state: 638 completed; 641 and 62050 unset (`not_active`).
const OFFER_STATE: Pairs = &[("mission_638_status", "completed")];

/// Step B: 62050 active with 62054 active, 638 completed, 641 not active.
const STEP_B_STATE: Pairs = &[
    ("mission_638_status", "completed"),
    ("mission_641_status", "not_active"),
    ("mission_62050_status", "active"),
    ("mission_62050_step_62054_status", "active"),
];

/// Marsh state: 640 completed, 641 not active, step B active.
const MARSH_STATE: Pairs = &[
    ("mission_640_status", "completed"),
    ("mission_641_status", "not_active"),
    ("mission_62050_status", "active"),
    ("mission_62050_step_62054_status", "active"),
];

/// An offer or accept trigger: its chain, its trigger type and the event
/// key and value that fire it.
struct Gate {
    chain: i32,
    trigger: TriggerType,
    key: &'static str,
    value: &'static str,
}

const OFFERS: [Gate; 3] = [
    Gate {
        chain: 11000,
        trigger: TriggerType::MissionCompleted,
        key: "mission_id",
        value: "638",
    },
    Gate {
        chain: 11001,
        trigger: TriggerType::PlayerLoaded,
        key: "world_name",
        value: "Castle_CellBlock",
    },
    Gate {
        chain: 11002,
        trigger: TriggerType::MissionAbandoned,
        key: "mission_id",
        value: "62050",
    },
];

/// Refusals shared by the offer and accept gates: a world, or a 638, 641 or
/// 62050 state the gate must not fire in.
const REFUSALS: [(&str, Option<i32>, Pairs); 7] = [
    ("world unset", None, OFFER_STATE),
    ("world 13", Some(13), OFFER_STATE),
    (
        "638 active",
        Some(CELLBLOCK_WORLD_ID),
        &[("mission_638_status", "active")],
    ),
    (
        "641 active",
        Some(CELLBLOCK_WORLD_ID),
        &[
            ("mission_638_status", "completed"),
            ("mission_641_status", "active"),
        ],
    ),
    (
        "641 completed",
        Some(CELLBLOCK_WORLD_ID),
        &[
            ("mission_638_status", "completed"),
            ("mission_641_status", "completed"),
        ],
    ),
    (
        "62050 active",
        Some(CELLBLOCK_WORLD_ID),
        &[
            ("mission_638_status", "completed"),
            ("mission_62050_status", "active"),
        ],
    ),
    (
        "62050 completed",
        Some(CELLBLOCK_WORLD_ID),
        &[
            ("mission_638_status", "completed"),
            ("mission_62050_status", "completed"),
        ],
    ),
];

/// Build the context for one event: `world`, the `state` params, the
/// `event` params (a value that parses as an integer becomes an i64, the
/// rest strings) and the `archetype`.
fn context(
    world: Option<i32>,
    state: &[(&str, &str)],
    event: &[(&str, &str)],
    archetype: Option<i64>,
) -> ExecutionContext {
    let mut ctx = new_ctx(world, state, archetype);
    for (key, value) in event {
        let json = match value.parse::<i64>() {
            Ok(n) => serde_json::json!(n),
            Err(_) => serde_json::json!(value),
        };
        ctx.set_param((*key).to_string(), json);
    }
    ctx
}

/// Resolve `trigger` against `ctx` through the whole engine.
fn resolve(engine: &ChainEngine, trigger: TriggerType, ctx: &ExecutionContext) -> ResolvedActions {
    let event = TriggerEvent {
        trigger_type: trigger,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    engine.resolve_event(&event, ctx)
}

/// Is `chain` one of the six chains of mission 62050?
fn in_mission(chain: i64) -> bool {
    CHAINS.iter().any(|id| i64::from(*id) == chain)
}

/// Resolve `trigger` and keep only the actions of the 62050 chains.
fn fire(engine: &ChainEngine, trigger: TriggerType, ctx: &ExecutionContext) -> Vec<(i64, Action)> {
    resolve(engine, trigger, ctx)
        .actions
        .into_iter()
        .filter(|(chain, _)| in_mission(*chain))
        .collect()
}

#[tokio::test]
async fn live_db_mission_62050_offer_gates() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let dialog_set = Action::AddDialogSet {
        dialog_set_id: 2795,
        slot: 17,
        mission_id: None,
    };
    for gate in &OFFERS {
        let chain = i64::from(gate.chain);
        let positive = fire(
            &engine,
            gate.trigger.clone(),
            &context(
                Some(CELLBLOCK_WORLD_ID),
                OFFER_STATE,
                &[(gate.key, gate.value)],
                None,
            ),
        );
        assert_eq!(
            positive,
            vec![(chain, dialog_set.clone())],
            "chain {chain} must offer 62050 in the offer state",
        );
        for (label, world, state) in REFUSALS {
            let fired = fire(
                &engine,
                gate.trigger.clone(),
                &context(world, state, &[(gate.key, gate.value)], None),
            );
            assert!(
                fired.is_empty(),
                "chain {chain} must refuse {label}: {fired:?}"
            );
        }
        let wrong = if gate.key == "mission_id" {
            "637"
        } else {
            "Castle"
        };
        let fired = fire(
            &engine,
            gate.trigger.clone(),
            &context(
                Some(CELLBLOCK_WORLD_ID),
                OFFER_STATE,
                &[(gate.key, wrong)],
                None,
            ),
        );
        assert!(
            fired.is_empty(),
            "chain {chain} must refuse {}={wrong}",
            gate.key
        );
    }
}

#[tokio::test]
async fn live_db_mission_62050_accept_gate() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let accept = vec![(11003, Action::AcceptMission { mission_id: 62050 })];
    let tag = [("entity_tag", "Prisoner_329")];
    let fired = fire(
        &engine,
        TriggerType::InteractTag,
        &context(Some(CELLBLOCK_WORLD_ID), OFFER_STATE, &tag, None),
    );
    assert_eq!(fired, accept, "the validated Prisoner click must accept");
    for (label, world, state) in REFUSALS {
        let fired = fire(
            &engine,
            TriggerType::InteractTag,
            &context(world, state, &tag, None),
        );
        assert!(fired.is_empty(), "accept must refuse {label}: {fired:?}");
    }
    let door = [("entity_tag", "329_CellDoorButton")];
    let fired = fire(
        &engine,
        TriggerType::InteractTag,
        &context(Some(CELLBLOCK_WORLD_ID), OFFER_STATE, &door, None),
    );
    assert!(
        fired.is_empty(),
        "the cell door button must not accept: {fired:?}"
    );
}

#[tokio::test]
async fn live_db_mission_62050_accepted_follow_up() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let step_a: Pairs = &[("mission_62050_step_62051_status", "active")];
    let accepted = [("mission_id", "62050")];
    let follow_up = vec![
        (
            11004,
            Action::AdvanceStep {
                mission_id: 62050,
                step_id: 62054,
            },
        ),
        (
            11004,
            Action::RemoveDialogSet {
                dialog_set_id: 2795,
                slot: 17,
            },
        ),
        (11004, Action::DisplayDialog { dialog_id: 60150 }),
    ];
    let fired = fire(
        &engine,
        TriggerType::MissionAccepted,
        &context(Some(CELLBLOCK_WORLD_ID), step_a, &accepted, None),
    );
    assert_eq!(
        fired, follow_up,
        "the accept must advance A to B and show 60150"
    );

    let refusals: [(&str, Option<i32>, Pairs); 3] = [
        ("world unset", None, step_a),
        (
            "step 62051 completed",
            Some(CELLBLOCK_WORLD_ID),
            &[("mission_62050_step_62051_status", "completed")],
        ),
        ("step 62051 unset", Some(CELLBLOCK_WORLD_ID), &[]),
    ];
    for (label, world, state) in refusals {
        let fired = fire(
            &engine,
            TriggerType::MissionAccepted,
            &context(world, state, &accepted, None),
        );
        assert!(fired.is_empty(), "11004 must refuse {label}: {fired:?}");
    }

    // Mission 641 accepted: 62050's follow-up stays silent, and the stock
    // 1152 dialog for 641 still resolves (unfiltered).
    let resolved = resolve(
        &engine,
        TriggerType::MissionAccepted,
        &context(
            Some(CELLBLOCK_WORLD_ID),
            step_a,
            &[("mission_id", "641")],
            None,
        ),
    );
    assert!(
        resolved
            .actions
            .iter()
            .all(|(chain, _)| !in_mission(*chain)),
        "no 62050 chain may fire on mission 641 accepted",
    );
    assert!(
        resolved
            .actions
            .contains(&(1152, Action::DisplayDialog { dialog_id: 4000 })),
        "stock chain 1152 must still show 4000 for mission 641",
    );
}

#[tokio::test]
async fn live_db_mission_62050_marsh_completes_b() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let marsh = [("entity_tag", "Preparation_ColMarsh")];
    let complete = (
        11005,
        Action::CompleteObjective {
            mission_id: 62050,
            objective_id: 62055,
        },
    );

    // Unfiltered: the core Marsh chains fire beside 11005, per archetype.
    let male = resolve(
        &engine,
        TriggerType::InteractTag,
        &context(Some(CELLBLOCK_WORLD_ID), MARSH_STATE, &marsh, Some(1)),
    );
    assert!(male.actions.contains(&complete));
    assert!(male
        .actions
        .contains(&(1051, Action::DisplayDialog { dialog_id: 4001 })));
    assert!(!male
        .actions
        .contains(&(1052, Action::DisplayDialog { dialog_id: 5022 })));
    let other = resolve(
        &engine,
        TriggerType::InteractTag,
        &context(Some(CELLBLOCK_WORLD_ID), MARSH_STATE, &marsh, Some(8)),
    );
    assert!(other.actions.contains(&complete));
    assert!(other
        .actions
        .contains(&(1052, Action::DisplayDialog { dialog_id: 5022 })));
    assert!(!other
        .actions
        .contains(&(1051, Action::DisplayDialog { dialog_id: 4001 })));

    let refusals: [(&str, Option<i32>, Pairs); 4] = [
        (
            "640 active",
            Some(CELLBLOCK_WORLD_ID),
            &[
                ("mission_640_status", "active"),
                ("mission_641_status", "not_active"),
                ("mission_62050_status", "active"),
                ("mission_62050_step_62054_status", "active"),
            ],
        ),
        (
            "step 62054 unset",
            Some(CELLBLOCK_WORLD_ID),
            &[
                ("mission_640_status", "completed"),
                ("mission_641_status", "not_active"),
                ("mission_62050_status", "active"),
            ],
        ),
        (
            "step 62054 completed",
            Some(CELLBLOCK_WORLD_ID),
            &[
                ("mission_640_status", "completed"),
                ("mission_641_status", "not_active"),
                ("mission_62050_status", "active"),
                ("mission_62050_step_62054_status", "completed"),
            ],
        ),
        ("world 13", Some(13), MARSH_STATE),
    ];
    for (label, world, state) in refusals {
        let fired = fire(
            &engine,
            TriggerType::InteractTag,
            &context(world, state, &marsh, Some(1)),
        );
        assert!(
            !fired.iter().any(|(chain, _)| *chain == 11005),
            "11005 must refuse {label}: {fired:?}",
        );
    }
}

#[tokio::test]
async fn live_db_mission_62050_no_dialog_path_moves_the_mission() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let cases: [(TriggerType, Pairs); 3] = [
        (
            TriggerType::DialogChoice,
            &[("dialog_id", "60150"), ("button_id", "-1")],
        ),
        (
            TriggerType::DialogChoice,
            &[("dialog_id", "60150"), ("button_id", "0")],
        ),
        (TriggerType::DialogOpen, &[("dialog_id", "60150")]),
    ];
    for state in [OFFER_STATE, STEP_B_STATE] {
        for (trigger, event) in &cases {
            let fired = fire(
                &engine,
                trigger.clone(),
                &context(Some(CELLBLOCK_WORLD_ID), state, event, None),
            );
            assert!(
                fired.is_empty(),
                "{trigger:?} {event:?} must not move 62050: {fired:?}"
            );
        }
    }
}
