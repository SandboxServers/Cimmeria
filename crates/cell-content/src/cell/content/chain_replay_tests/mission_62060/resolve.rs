//! CB-N06-4 — the gates, refusals and mutually exclusive branches of mission
//! 62060 resolved through the full seeded engine (`build_engine`, so the
//! stock chains 1016, 1017, 1028 and 1152 are present). Each fire keeps only
//! the `(chain_id, action)` pairs of [`CHAINS`] unless a test says
//! "unfiltered".

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::super::engine_loader::build_engine;
use super::{ctx as new_ctx, CELLBLOCK_WORLD_ID, CHAINS};
use crate::test_support::require_db_or_skip;

/// `(key, value)` pairs: string state params, or event params.
type Pairs = &'static [(&'static str, &'static str)];

/// Offer state: 638 completed; 641 and 62060 unset (`not_active`).
const OFFER_STATE: Pairs = &[("mission_638_status", "completed")];

/// Step state: 62060 active with step 62061 active, 638 completed.
const STEP_STATE: Pairs = &[
    ("mission_638_status", "completed"),
    ("mission_62060_status", "active"),
    ("mission_62060_step_62061_status", "active"),
];

/// Done state: 62060 completed with step 62061 completed.
const DONE_STATE: Pairs = &[
    ("mission_638_status", "completed"),
    ("mission_62060_status", "completed"),
    ("mission_62060_step_62061_status", "completed"),
];

/// An offer or accept trigger: its chain, its trigger type and the event
/// key and value that fire it.
struct Gate {
    chain: i32,
    trigger: TriggerType,
    key: &'static str,
    value: &'static str,
}

const OFFERS: [Gate; 2] = [
    Gate {
        chain: 11030,
        trigger: TriggerType::MissionCompleted,
        key: "mission_id",
        value: "638",
    },
    Gate {
        chain: 11031,
        trigger: TriggerType::PlayerLoaded,
        key: "world_name",
        value: "Castle_CellBlock",
    },
];

/// Refusals shared by the offer and accept gates: a world, or a 638, 641 or
/// 62060 state the gate must not fire in.
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
        "62060 active",
        Some(CELLBLOCK_WORLD_ID),
        &[
            ("mission_638_status", "completed"),
            ("mission_62060_status", "active"),
        ],
    ),
    (
        "62060 completed",
        Some(CELLBLOCK_WORLD_ID),
        &[
            ("mission_638_status", "completed"),
            ("mission_62060_status", "completed"),
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

/// Is `chain` one of the twelve chains of mission 62060?
fn in_mission(chain: i64) -> bool {
    CHAINS.iter().any(|id| i64::from(*id) == chain)
}

/// Resolve `trigger` and keep only the actions of the 62060 chains.
fn fire(engine: &ChainEngine, trigger: TriggerType, ctx: &ExecutionContext) -> Vec<(i64, Action)> {
    resolve(engine, trigger, ctx)
        .actions
        .into_iter()
        .filter(|(chain, _)| in_mission(*chain))
        .collect()
}

/// The four F4 actions of an aid or sabotage branch (chains 11036-11038):
/// two slappacks, the two dialog-set removals, and objective 62062.
fn branch(chain: i64) -> Vec<(i64, Action)> {
    vec![
        (
            chain,
            Action::GrantItem {
                item_id: 2893,
                count: 2,
                container_id: Some(0),
            },
        ),
        (
            chain,
            Action::RemoveDialogSet {
                dialog_set_id: 7509,
                slot: 22,
            },
        ),
        (
            chain,
            Action::RemoveDialogSet {
                dialog_set_id: 3862,
                slot: 176,
            },
        ),
        (
            chain,
            Action::CompleteObjective {
                mission_id: 62060,
                objective_id: 62062,
            },
        ),
    ]
}

#[tokio::test]
async fn live_db_mission_62060_offer_gates() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let dialog_set = Action::AddDialogSet {
        dialog_set_id: 3862,
        slot: 176,
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
            "chain {chain} must offer 62060 in the offer state",
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
async fn live_db_mission_62060_accept_is_exclusive() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let accept = Action::AcceptMission { mission_id: 62060 };
    let prisoners: [(&str, i64); 2] = [
        ("Cellblock_Ambient_Prisoner1", 11033),
        ("Cellblock_Ambient_Prisoner2", 11034),
    ];
    for (tag, chain) in prisoners {
        let entity = [("entity_tag", tag)];
        let fired = fire(
            &engine,
            TriggerType::InteractTag,
            &context(Some(CELLBLOCK_WORLD_ID), OFFER_STATE, &entity, None),
        );
        assert_eq!(
            fired,
            vec![(chain, accept.clone())],
            "{tag} must accept 62060 in the offer state",
        );
        for (label, world, state) in REFUSALS {
            let fired = fire(
                &engine,
                TriggerType::InteractTag,
                &context(world, state, &entity, None),
            );
            assert!(fired.is_empty(), "accept must refuse {label}: {fired:?}");
        }
    }
    for tag in ["329_CellDoorButton", "Prisoner_329"] {
        let entity = [("entity_tag", tag)];
        let fired = fire(
            &engine,
            TriggerType::InteractTag,
            &context(Some(CELLBLOCK_WORLD_ID), OFFER_STATE, &entity, None),
        );
        assert!(fired.is_empty(), "{tag} must not accept 62060: {fired:?}");
    }
}

#[tokio::test]
async fn live_db_mission_62060_follow_up() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let accepted = [("mission_id", "62060")];
    let panel = vec![
        (
            11035,
            Action::AddDialogSet {
                dialog_set_id: 7509,
                slot: 22,
                mission_id: None,
            },
        ),
        (11035, Action::DisplayDialog { dialog_id: 60151 }),
    ];
    let fired = fire(
        &engine,
        TriggerType::MissionAccepted,
        &context(Some(CELLBLOCK_WORLD_ID), STEP_STATE, &accepted, None),
    );
    assert_eq!(fired, panel, "62060 accepted must bind the panel choice");
    let refusals: [(&str, Option<i32>, Pairs); 3] = [
        ("world unset", None, STEP_STATE),
        (
            "step unset",
            Some(CELLBLOCK_WORLD_ID),
            &[
                ("mission_638_status", "completed"),
                ("mission_62060_status", "active"),
            ],
        ),
        (
            "step completed",
            Some(CELLBLOCK_WORLD_ID),
            &[
                ("mission_638_status", "completed"),
                ("mission_62060_status", "active"),
                ("mission_62060_step_62061_status", "completed"),
            ],
        ),
    ];
    for (label, world, state) in refusals {
        let fired = fire(
            &engine,
            TriggerType::MissionAccepted,
            &context(world, state, &accepted, None),
        );
        assert!(fired.is_empty(), "follow-up must refuse {label}: {fired:?}");
    }
    let other = [("mission_id", "62050")];
    let fired = fire(
        &engine,
        TriggerType::MissionAccepted,
        &context(Some(CELLBLOCK_WORLD_ID), STEP_STATE, &other, None),
    );
    assert!(
        fired.is_empty(),
        "62050 accepted must not fire 62060: {fired:?}"
    );
    let resolved = resolve(
        &engine,
        TriggerType::MissionAccepted,
        &context(
            Some(CELLBLOCK_WORLD_ID),
            &[("mission_62050_step_62051_status", "active")],
            &other,
            None,
        ),
    );
    assert!(
        resolved.actions.contains(&(
            11004,
            Action::AdvanceStep {
                mission_id: 62050,
                step_id: 62054,
            },
        )),
        "stock 62050 follow-up 11004 must still resolve for 62050",
    );
}

#[tokio::test]
async fn live_db_mission_62060_one_branch_per_click() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let taps: [(&str, i64); 3] = [
        ("Cellblock_Ambient_Prisoner1", 11036),
        ("Cellblock_Ambient_Prisoner2", 11037),
        ("329_CellDoorButton", 11038),
    ];
    for (tag, chain) in taps {
        let entity = [("entity_tag", tag)];
        let fired = fire(
            &engine,
            TriggerType::InteractTag,
            &context(Some(CELLBLOCK_WORLD_ID), STEP_STATE, &entity, None),
        );
        assert_eq!(
            fired,
            branch(chain),
            "{tag} must run exactly the {chain} branch in step 62061",
        );
        let fired = fire(
            &engine,
            TriggerType::InteractTag,
            &context(Some(CELLBLOCK_WORLD_ID), DONE_STATE, &entity, None),
        );
        assert!(
            fired.is_empty(),
            "{tag} must not branch once done: {fired:?}"
        );
    }
}

#[tokio::test]
async fn live_db_mission_62060_panel_stock_chains_disjoint() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let door = [("entity_tag", "329_CellDoorButton")];
    let stock: [i64; 3] = [1016, 1017, 1028];
    for state in [STEP_STATE, OFFER_STATE] {
        let resolved = resolve(
            &engine,
            TriggerType::InteractTag,
            &context(Some(CELLBLOCK_WORLD_ID), state, &door, None),
        );
        assert!(
            resolved
                .actions
                .iter()
                .all(|(chain, _)| !stock.contains(chain)),
            "the door panel must not fire chains 1016, 1017 or 1028: {:?}",
            resolved.actions,
        );
    }
    let livewire: Pairs = &[
        ("mission_638_status", "active"),
        ("mission_638_step_2115_status", "active"),
    ];
    let resolved = resolve(
        &engine,
        TriggerType::InteractTag,
        &context(Some(CELLBLOCK_WORLD_ID), livewire, &door, None),
    );
    let minigame = Action::StartMinigame {
        minigame_type: "Livewire".into(),
        difficulty: 1,
        on_victory_chains: vec![1017],
    };
    assert!(
        resolved.actions.contains(&(1016, minigame)),
        "stock chain 1016 must still start the Livewire at step 2115",
    );
    let fired = fire(
        &engine,
        TriggerType::InteractTag,
        &context(Some(CELLBLOCK_WORLD_ID), livewire, &door, None),
    );
    assert!(fired.is_empty(), "no 62060 chain at step 2115: {fired:?}");
}

#[tokio::test]
async fn live_db_mission_62060_restores_and_withdrawals() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let login = [("world_name", "Castle_CellBlock")];
    let restore = vec![
        (
            11032,
            Action::AddDialogSet {
                dialog_set_id: 3862,
                slot: 176,
                mission_id: None,
            },
        ),
        (
            11032,
            Action::AddDialogSet {
                dialog_set_id: 7509,
                slot: 22,
                mission_id: None,
            },
        ),
    ];
    let fired = fire(
        &engine,
        TriggerType::PlayerLoaded,
        &context(Some(CELLBLOCK_WORLD_ID), STEP_STATE, &login, None),
    );
    assert_eq!(
        fired, restore,
        "login in step 62061 must restore both choices"
    );
    let no_step: Pairs = &[
        ("mission_638_status", "completed"),
        ("mission_62060_status", "active"),
    ];
    let refusals: [(&str, Option<i32>, Pairs); 2] = [
        ("step unset", Some(CELLBLOCK_WORLD_ID), no_step),
        ("world 13", Some(13), STEP_STATE),
    ];
    for (label, world, state) in refusals {
        let fired = fire(
            &engine,
            TriggerType::PlayerLoaded,
            &context(world, state, &login, None),
        );
        assert!(fired.is_empty(), "login must refuse {label}: {fired:?}");
    }

    let abandon = [("mission_id", "62060")];
    let fired = fire(
        &engine,
        TriggerType::MissionAbandoned,
        &context(Some(CELLBLOCK_WORLD_ID), STEP_STATE, &abandon, None),
    );
    assert_eq!(
        fired,
        vec![(
            11039,
            Action::RemoveDialogSet {
                dialog_set_id: 7509,
                slot: 22,
            },
        )],
        "abandoning 62060 must unbind the panel choice",
    );

    let accepted_641 = [("mission_id", "641")];
    let withdraw = vec![(
        11040,
        Action::RemoveDialogSet {
            dialog_set_id: 3862,
            slot: 176,
        },
    )];
    let fired = fire(
        &engine,
        TriggerType::MissionAccepted,
        &context(Some(CELLBLOCK_WORLD_ID), OFFER_STATE, &accepted_641, None),
    );
    assert_eq!(fired, withdraw, "641 before 62060 must withdraw the offer");
    let held: [Pairs; 2] = [
        &[
            ("mission_638_status", "completed"),
            ("mission_62060_status", "active"),
        ],
        &[
            ("mission_638_status", "completed"),
            ("mission_62060_status", "completed"),
        ],
    ];
    for state in held {
        let fired = fire(
            &engine,
            TriggerType::MissionAccepted,
            &context(Some(CELLBLOCK_WORLD_ID), state, &accepted_641, None),
        );
        assert!(
            fired.is_empty(),
            "641 must not withdraw in {state:?}: {fired:?}"
        );
    }
    let resolved = resolve(
        &engine,
        TriggerType::MissionAccepted,
        &context(Some(CELLBLOCK_WORLD_ID), OFFER_STATE, &accepted_641, None),
    );
    assert!(
        resolved
            .actions
            .contains(&(1152, Action::DisplayDialog { dialog_id: 4000 })),
        "stock chain 1152 must still show 4000 for mission 641",
    );

    let withdrawn_641 = fire(
        &engine,
        TriggerType::MissionAbandoned,
        &context(
            Some(CELLBLOCK_WORLD_ID),
            &[("mission_641_status", "active")],
            &abandon,
            None,
        ),
    );
    let withdraw = vec![(
        11041,
        Action::RemoveDialogSet {
            dialog_set_id: 3862,
            slot: 176,
        },
    )];
    let only_11041: Vec<_> = withdrawn_641
        .into_iter()
        .filter(|(chain, _)| *chain == 11041)
        .collect();
    assert_eq!(only_11041, withdraw, "abandoning after 641 must withdraw");
    let fired = fire(
        &engine,
        TriggerType::MissionAbandoned,
        &context(Some(CELLBLOCK_WORLD_ID), OFFER_STATE, &abandon, None),
    );
    assert!(
        fired.iter().all(|(chain, _)| *chain != 11041),
        "11041 must refuse 641 unset: {fired:?}",
    );
}

#[tokio::test]
async fn live_db_mission_62060_no_dialog_path_moves_the_mission() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let closed = [("dialog_id", "60151"), ("button_id", "-1")];
    let picked = [("dialog_id", "60151"), ("button_id", "0")];
    let opened = [("dialog_id", "60151")];
    let cases: [(TriggerType, &[(&str, &str)]); 3] = [
        (TriggerType::DialogChoice, &closed),
        (TriggerType::DialogChoice, &picked),
        (TriggerType::DialogOpen, &opened),
    ];
    for state in [OFFER_STATE, STEP_STATE] {
        for (trigger, event) in cases.iter() {
            let fired = fire(
                &engine,
                trigger.clone(),
                &context(Some(CELLBLOCK_WORLD_ID), state, event, None),
            );
            assert!(
                fired.is_empty(),
                "dialog 60151 must not move 62060: {trigger:?} {event:?} {fired:?}",
            );
        }
    }
}
