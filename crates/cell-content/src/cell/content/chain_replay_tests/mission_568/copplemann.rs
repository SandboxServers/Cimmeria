//! Mission 568 chains 9001-9010: the Copplemann click, accept, turn-in,
//! progress bind, Tollana entry restore and abandon repaint.
//!
//! Each test walks the seeded engine (`build_engine`, the whole DB) so a
//! collision or a missing chain shows up as a wrong fired set, not as a
//! per-chain pass.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::triggers::{Trigger, TriggerType};

use super::super::super::engine_loader::build_engine;
use super::{
    actions_of, ctx, fired, load, resolve_one, with_dialog, with_mission, with_tag,
    with_world_name, ALL, S, TOLLANA,
};
use crate::test_support::require_db_or_skip;

/// Whether `trigger` is the F5 trigger for `chain_id`.
fn trigger_is(trigger: &Trigger, chain_id: i32) -> bool {
    match chain_id {
        9001..=9003 => {
            matches!(trigger, Trigger::OnInteractTag { entity_tag } if entity_tag == "Tollana_Copplemann")
        }
        9004 => matches!(trigger, Trigger::OnDialogChoice { dialog_id: 2153 }),
        9005 => matches!(trigger, Trigger::OnDialogChoice { dialog_id: 2154 }),
        9006 => matches!(trigger, Trigger::OnMissionAccepted { mission_id: 568 }),
        9007..=9009 => {
            matches!(trigger, Trigger::OnPlayerLoaded { world_name: Some(w) } if w == "Tollana")
        }
        9010 => matches!(trigger, Trigger::OnMissionAbandoned { mission_id: 568 }),
        _ => false,
    }
}

#[tokio::test]
async fn live_db_568_chains_9001_to_9010_load_with_exact_shapes() {
    let pool = require_db_or_skip!();
    let slot = 420;
    let expected: [(i32, usize, Vec<Action>); 10] = [
        (9001, 1, vec![Action::DisplayDialog { dialog_id: 2153 }]),
        (9002, 2, vec![Action::DisplayDialog { dialog_id: 2153 }]),
        (9003, 1, vec![Action::DisplayDialog { dialog_id: 2154 }]),
        (
            9004,
            2,
            vec![
                Action::RemoveDialogSet {
                    dialog_set_id: 2656,
                    slot,
                },
                Action::AcceptMission { mission_id: 568 },
                Action::AdvanceStep {
                    mission_id: 568,
                    step_id: 1964,
                },
            ],
        ),
        (
            9005,
            2,
            vec![
                Action::RemoveDialogSet {
                    dialog_set_id: 6393,
                    slot,
                },
                Action::CompleteMission { mission_id: 568 },
            ],
        ),
        (
            9006,
            1,
            vec![Action::AddDialogSet {
                dialog_set_id: 4959,
                slot,
                mission_id: Some(568),
            }],
        ),
        (
            9007,
            2,
            vec![Action::AddDialogSet {
                dialog_set_id: 2656,
                slot,
                mission_id: None,
            }],
        ),
        (
            9008,
            3,
            vec![Action::AddDialogSet {
                dialog_set_id: 4959,
                slot,
                mission_id: Some(568),
            }],
        ),
        (
            9009,
            2,
            vec![Action::AddDialogSet {
                dialog_set_id: 6393,
                slot,
                mission_id: Some(568),
            }],
        ),
        (
            9010,
            2,
            vec![
                Action::RemoveDialogSet {
                    dialog_set_id: 4959,
                    slot,
                },
                Action::RemoveDialogSet {
                    dialog_set_id: 6393,
                    slot,
                },
                Action::AddDialogSet {
                    dialog_set_id: 2656,
                    slot,
                    mission_id: None,
                },
            ],
        ),
    ];
    for (id, cond, actions) in expected {
        let chain = load(&pool, id).await;
        assert!(chain.enabled, "chain {id} must be enabled");
        assert_eq!(chain.conditions.len(), cond, "chain {id} condition count");
        assert_eq!(chain.actions, actions, "chain {id} action list");
        assert!(
            trigger_is(&chain.trigger, id),
            "chain {id} trigger {:?}",
            chain.trigger
        );
    }
}

#[tokio::test]
async fn live_db_568_copplemann_click_is_disjoint_and_exhaustive() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    for s in ALL {
        let want = match s {
            S::S0 => vec![9001],
            S::S1964 | S::S1965 => vec![9002],
            S::S1966 => vec![9003],
            S::Done => vec![],
        };
        let got = fired(
            &engine,
            TriggerType::InteractTag,
            &with_tag(ctx(TOLLANA, s), "Tollana_Copplemann"),
        )
        .await;
        assert_eq!(got, want, "Copplemann click in {s:?}");
    }
}

#[tokio::test]
async fn live_db_568_dialog_choice_gates() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    for s in ALL {
        let want_accept = if matches!(s, S::S0) {
            vec![9004]
        } else {
            vec![]
        };
        let got = fired(
            &engine,
            TriggerType::DialogChoice,
            &with_dialog(ctx(TOLLANA, s), 2153),
        )
        .await;
        assert_eq!(got, want_accept, "dialog 2153 close in {s:?}");

        let want_turn_in = if matches!(s, S::S1966) {
            vec![9005]
        } else {
            vec![]
        };
        let got = fired(
            &engine,
            TriggerType::DialogChoice,
            &with_dialog(ctx(TOLLANA, s), 2154),
        )
        .await;
        assert_eq!(got, want_turn_in, "dialog 2154 close in {s:?}");
    }
    let got = fired(
        &engine,
        TriggerType::DialogChoice,
        &with_dialog(ctx(1, S::S0), 2153),
    )
    .await;
    assert!(got.is_empty(), "dialog 2153 in world 1 must fire nothing");
    let got = fired(
        &engine,
        TriggerType::DialogChoice,
        &with_dialog(ctx(1, S::S1966), 2154),
    )
    .await;
    assert!(got.is_empty(), "dialog 2154 in world 1 must fire nothing");
}

#[tokio::test]
async fn live_db_568_mission_accepted_fires_9006_only_on_tollana() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    for s in ALL {
        let got = fired(
            &engine,
            TriggerType::MissionAccepted,
            &with_mission(ctx(TOLLANA, s), 568),
        )
        .await;
        assert_eq!(got, vec![9006], "mission 568 accepted in {s:?}");
    }
    let got = fired(
        &engine,
        TriggerType::MissionAccepted,
        &with_mission(ctx(1, S::S0), 568),
    )
    .await;
    assert!(
        got.is_empty(),
        "mission 568 accepted in world 1 must fire nothing"
    );

    let resolved = resolve_one(
        &pool,
        9006,
        &with_mission(ctx(TOLLANA, S::S0), 568),
        TriggerType::MissionAccepted,
    )
    .await;
    assert_eq!(
        actions_of(&resolved),
        vec![Action::AddDialogSet {
            dialog_set_id: 4959,
            slot: 420,
            mission_id: Some(568),
        }],
        "9006 progress bind"
    );
}

#[tokio::test]
async fn live_db_568_restore_and_abandon_gates() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    for s in ALL {
        let want = match s {
            S::S0 => vec![9007],
            S::S1964 | S::S1965 => vec![9008],
            S::S1966 => vec![9009],
            S::Done => vec![],
        };
        let got = fired(
            &engine,
            TriggerType::PlayerLoaded,
            &with_world_name(ctx(TOLLANA, s), "Tollana"),
        )
        .await;
        assert_eq!(got, want, "Tollana entry restore in {s:?}");
    }
    let got = fired(
        &engine,
        TriggerType::PlayerLoaded,
        &with_world_name(ctx(1, S::S0), "Tollana"),
    )
    .await;
    assert!(got.is_empty(), "restore in world 1 must fire nothing");

    let got = fired(
        &engine,
        TriggerType::MissionAbandoned,
        &with_mission(ctx(TOLLANA, S::S0), 568),
    )
    .await;
    assert_eq!(got, vec![9010], "abandon 568 in S0 (post-abandon state)");
    let got = fired(
        &engine,
        TriggerType::MissionAbandoned,
        &with_mission(ctx(1, S::S0), 568),
    )
    .await;
    assert!(got.is_empty(), "abandon in world 1 must fire nothing");
}

#[test]
fn seed_has_no_forbidden_actions() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../db/resources/Content/Seed/tollana_568_chains.sql"
    );
    let seed = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("read {path}: {e}"))
        .replace("\r\n", "\n");
    for banned in [
        "'grant_xp'",
        "'grant_cash'",
        "'add_item'",
        "'remove_item'",
        "'set_interaction_type'",
        "'complete_objective', 568, '2311'",
    ] {
        assert!(!seed.contains(banned), "seed must not contain {banned}");
    }

    let lines: Vec<&str> = seed.lines().collect();
    let mut checked = 0;
    for (i, line) in lines.iter().enumerate() {
        if !line.starts_with("INSERT INTO content_actions") {
            continue;
        }
        let values = lines
            .get(i + 1)
            .copied()
            .unwrap_or_else(|| panic!("content_actions row at line {} has no VALUES", i + 1));
        assert!(values.starts_with("VALUES ("), "line {}: {values}", i + 2);
        let body = values
            .strip_suffix(");")
            .unwrap_or_else(|| panic!("line {}: {values}", i + 2));
        let delay_ok = body
            .rsplit_once(", ")
            .is_some_and(|(head, _sort)| head.ends_with(", 0"));
        assert!(delay_ok, "delay_ms must be 0 on line {}: {values}", i + 2);
        checked += 1;
    }
    assert!(checked > 0, "seed must contain content_actions rows");
}
