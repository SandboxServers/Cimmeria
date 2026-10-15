//! Correlated receipt result, client frames and completion continuation.

use std::collections::HashMap;
use std::time::Instant;

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{Chain, ChainEngine};
use cimmeria_content_engine::triggers::Trigger;
use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_COMPLETED, STATUS_ACTIVE,
};
use cimmeria_wire::cell::messages::{
    CommittedLetterTurnIn, LetterTurnInAttempt, LetterTurnInEntryEpoch, LetterTurnInOutcome,
    LetterTurnInRefusal, LetterTurnInRequest, LetterTurnInResult, LetterTurnInSession,
    LetterTurnInToken, SavedMission,
};
use tokio::sync::mpsc;

use super::executor::letter_turn_in::PendingLetterTurnIn;
use super::letter_turn_in_result::apply_letter_turn_in_result;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::{MissionDefEntry, WorldRow};

const PLAYER: u32 = 101;
const MARSH: u32 = 102;
const PLAYER_ID: i32 = 4242;

fn rig() -> (SpaceManager, LetterTurnInRequest) {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(r#"<?xml version="1.0"?><Spaces><Space WorldName="Harset_CmdCenter" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.stamp_world_rows(&HashMap::from([(
        "Harset_CmdCenter".to_string(),
        WorldRow::enforcing(68),
    )]));
    let space = mgr
        .create_entity(PLAYER, "Harset_CmdCenter", [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.create_entity_in_space(MARSH, space, [1.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(PLAYER);
    let marsh = mgr.get_entity_mut(MARSH).unwrap();
    marsh.tag = Some("CmdCenter_Marsh".into());
    marsh.template_id = Some(10);
    let request = LetterTurnInRequest {
        entity_id: PLAYER,
        player_id: PLAYER_ID,
        attempt: LetterTurnInAttempt::from_bytes([3; 16]),
        token: LetterTurnInToken::from_bytes([4; 16]),
        generation: Instant::now(),
        epoch: LetterTurnInEntryEpoch::new(1),
    };
    let player = mgr.get_entity_mut(PLAYER).unwrap();
    player.is_player = true;
    player.player_id = Some(PLAYER_ID);
    player.last_interaction_target = Some(MARSH);
    player
        .witnesses
        .insert(cimmeria_common::EntityId(MARSH as i32));
    player
        .available_interactions
        .insert(10, vec![(120101, None, 268435456)]);
    player.extensions.insert(LetterTurnInSession {
        player_id: PLAYER_ID,
        token: request.token,
        generation: request.generation,
        epoch: request.epoch,
    });
    player.extensions.insert(PendingLetterTurnIn {
        request,
        marsh_entity_id: MARSH,
        next_retry_at: Instant::now(),
    });
    let mut mission = MissionInstance::new(
        1360,
        4038,
        vec![MissionObjective {
            objective_id: 4651,
            status: STATUS_ACTIVE,
            hidden: false,
            optional: false,
        }],
    );
    mission.completed_steps = vec![4037];
    mission.completed_objectives = vec![4650];
    player.missions.add_mission(mission);
    mgr.mission_defs.insert(
        1361,
        MissionDefEntry {
            step_id: 4694,
            objectives: vec![],
            is_hidden: false,
            num_repeats: 0,
            can_repeat_on_fail: false,
        },
    );
    (mgr, request)
}

fn committed(request: LetterTurnInRequest) -> LetterTurnInResult {
    LetterTurnInResult {
        request,
        outcome: LetterTurnInOutcome::Committed(CommittedLetterTurnIn {
            mission: SavedMission {
                mission_id: 1360,
                status: MISSION_COMPLETED,
                current_step_id: None,
                completed_step_ids: vec![4037, 4038],
                completed_objective_ids: vec![4650, 4651],
                active_objective_ids: vec![4651],
                failed_objective_ids: vec![],
                repeats: 1,
            },
            item_id: 7333,
            container_id: 2,
            stack_before: 1,
            stack_after: 0,
        }),
    }
}

fn engine() -> ChainEngine {
    let mut engine = ChainEngine::new();
    engine.register_chain(Chain {
        id: 6511,
        name: "open Praxis return".into(),
        enabled: true,
        trigger: Trigger::OnMissionCompleted { mission_id: 1360 },
        conditions: vec![],
        actions: vec![Action::AcceptMission { mission_id: 1361 }],
        action_delays: vec![0],
        priority: 0,
        once: false,
    });
    engine
}

fn drained(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
    let mut found = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        found.push(msg);
    }
    found
}

#[tokio::test]
async fn committed_result_installs_postimage_and_orders_visuals_then_opens_1361_once() {
    let (mut mgr, request) = rig();
    let (tx, mut rx) = mpsc::channel(32);
    let engine = engine();
    apply_letter_turn_in_result(committed(request), &tx, &mut mgr, &engine).await;
    let player = mgr.get_entity(PLAYER).unwrap();
    assert!(!player.extensions.contains::<PendingLetterTurnIn>());
    let letter = player.missions.get_mission(1360).unwrap();
    assert_eq!(letter.status, MISSION_COMPLETED);
    assert_eq!(letter.current_step_id, None);
    assert_eq!(letter.completed_steps, [4037, 4038]);
    assert_eq!(letter.completed_objectives, [4650, 4651]);
    assert_eq!(letter.repeats, 1);
    assert!(
        player.missions.get_mission(1361).is_some(),
        "completion event opened 1361"
    );
    assert!(!player.available_interactions.contains_key(&10));
    let messages = drained(&mut rx);
    let methods: Vec<u16> = messages
        .iter()
        .filter_map(|msg| match msg {
            CellToBaseMsg::EntityMethodCall { method_index, .. } => Some(*method_index),
            _ => None,
        })
        .collect();
    assert_eq!(
        &methods[..4],
        &[crate::mercury::method_idx::ON_DIALOG_DISPLAY, 82, 81, 80]
    );
    assert_eq!(
        methods.iter().filter(|&&method| method == 80).count(),
        2,
        "1360 completion and 1361 accept"
    );
    assert!(messages.iter().any(|msg| matches!(msg, CellToBaseMsg::WitnessEntityMethod { witness_id: PLAYER, entity_id: MARSH, method_index, args, .. }
        if *method_index == crate::mercury::method_idx::INTERACTION_TYPE && args == &0u64.to_le_bytes().to_vec())));
    let frame_args: Vec<&[u8]> = messages
        .iter()
        .filter_map(|msg| match msg {
            CellToBaseMsg::EntityMethodCall {
                method_index: 80..=82,
                args,
                ..
            } => Some(args.as_slice()),
            _ => None,
        })
        .collect();
    assert_eq!(&frame_args[0][..4], &4651i32.to_le_bytes());
    assert_eq!(&frame_args[1][..4], &4038i32.to_le_bytes());
    assert_eq!(&frame_args[2][..4], &1360i32.to_le_bytes());
    assert_eq!(frame_args[0][4], 1);
    assert_eq!(frame_args[1][4], 1);
    assert_eq!(frame_args[2][4], 2);

    apply_letter_turn_in_result(committed(request), &tx, &mut mgr, &engine).await;
    apply_letter_turn_in_result(
        LetterTurnInResult {
            request,
            outcome: LetterTurnInOutcome::Refused(LetterTurnInRefusal::NoAccessibleLetter),
        },
        &tx,
        &mut mgr,
        &engine,
    )
    .await;
    assert!(
        drained(&mut rx).is_empty(),
        "duplicate and late refusal are inert"
    );
}

#[tokio::test]
async fn stale_correlations_and_definite_refusal_never_complete() {
    for case in 0..9 {
        let (mut mgr, request) = rig();
        let (tx, mut rx) = mpsc::channel(16);
        let mut result = committed(request);
        match case {
            0 => result.request.entity_id += 1,
            1 => result.request.player_id += 1,
            2 => result.request.token = LetterTurnInToken::from_bytes([9; 16]),
            3 => result.request.epoch = LetterTurnInEntryEpoch::new(2),
            4 => result.request.attempt = LetterTurnInAttempt::from_bytes([9; 16]),
            5 => {
                mgr.get_entity_mut(PLAYER)
                    .unwrap()
                    .extensions
                    .get_mut::<LetterTurnInSession>()
                    .unwrap()
                    .generation += std::time::Duration::from_secs(1)
            }
            6 => mgr.get_entity_mut(PLAYER).unwrap().player_id = Some(PLAYER_ID + 1),
            7 => mgr.stamp_world_rows(&HashMap::from([(
                "Harset_CmdCenter".to_string(),
                WorldRow::enforcing(57),
            )])),
            8 => mgr.get_entity_mut(MARSH).unwrap().tag = Some("Wrong".into()),
            _ => unreachable!(),
        }
        apply_letter_turn_in_result(result, &tx, &mut mgr, &engine()).await;
        assert!(drained(&mut rx).is_empty(), "case {case} emitted success");
        assert_eq!(
            mgr.get_entity(PLAYER)
                .unwrap()
                .missions
                .get_mission(1360)
                .unwrap()
                .status,
            1,
            "case {case}"
        );
    }
    let (mut mgr, request) = rig();
    let (tx, mut rx) = mpsc::channel(16);
    apply_letter_turn_in_result(
        LetterTurnInResult {
            request,
            outcome: LetterTurnInOutcome::Refused(LetterTurnInRefusal::NoAccessibleLetter),
        },
        &tx,
        &mut mgr,
        &engine(),
    )
    .await;
    assert_eq!(
        mgr.get_entity(PLAYER)
            .unwrap()
            .missions
            .get_mission(1360)
            .unwrap()
            .status,
        1
    );
    assert!(!mgr
        .get_entity(PLAYER)
        .unwrap()
        .extensions
        .contains::<PendingLetterTurnIn>());
    let messages = drained(&mut rx);
    assert_eq!(messages.len(), 1, "only refusal feedback");
    assert!(
        matches!(&messages[0], CellToBaseMsg::EntityMethodCall { method_index, args, .. }
        if *method_index == crate::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION
            && *args == cimmeria_wire::cell::chat::serialize_on_player_communication(
                "SYSTEM", 0, cimmeria_wire::cell::chat::CHAN_FEEDBACK,
                "Frost's Letter is not in an accessible bag."))
    );
}

#[tokio::test]
async fn uncertain_result_keeps_attempt_and_walking_away_suppresses_dialog_only() {
    let (mut mgr, request) = rig();
    let (tx, mut rx) = mpsc::channel(32);
    apply_letter_turn_in_result(
        LetterTurnInResult {
            request,
            outcome: LetterTurnInOutcome::UnknownRetryable,
        },
        &tx,
        &mut mgr,
        &engine(),
    )
    .await;
    assert!(mgr
        .get_entity(PLAYER)
        .unwrap()
        .extensions
        .contains::<PendingLetterTurnIn>());
    assert!(drained(&mut rx).is_empty());
    mgr.get_entity_mut(PLAYER).unwrap().last_interaction_target = None;
    mgr.get_entity_mut(MARSH).unwrap().position.x = 20.0;
    apply_letter_turn_in_result(committed(request), &tx, &mut mgr, &engine()).await;
    assert_eq!(
        mgr.get_entity(PLAYER)
            .unwrap()
            .missions
            .get_mission(1360)
            .unwrap()
            .status,
        MISSION_COMPLETED
    );
    assert!(!drained(&mut rx).iter().any(|msg| matches!(msg, CellToBaseMsg::EntityMethodCall { method_index, .. } if *method_index == crate::mercury::method_idx::ON_DIALOG_DISPLAY)));
}
