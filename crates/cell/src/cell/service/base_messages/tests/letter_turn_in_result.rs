//! The base message receiver must forward a fenced result to the content consumer.

use std::collections::HashMap;
use std::time::Instant;

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ResolvedActions;
use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_COMPLETED, STATUS_ACTIVE,
};
use cimmeria_wire::cell::messages::{
    CommittedLetterTurnIn, LetterTurnInEntryEpoch, LetterTurnInOutcome, LetterTurnInRequest,
    LetterTurnInResult, LetterTurnInSession, LetterTurnInToken, SavedMission,
};

use super::*;
use crate::cell::spawner::WorldRow;

fn rig() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(r#"<?xml version="1.0"?><Spaces><Space WorldName="Harset_CmdCenter" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.stamp_world_rows(&HashMap::from([(
        "Harset_CmdCenter".to_string(),
        WorldRow::enforcing(68),
    )]));
    let space = mgr
        .create_entity(101, "Harset_CmdCenter", [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.create_entity_in_space(102, space, [1.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(101);
    let marsh = mgr.get_entity_mut(102).unwrap();
    marsh.tag = Some("CmdCenter_Marsh".into());
    marsh.template_id = Some(10);
    let player = mgr.get_entity_mut(101).unwrap();
    player.is_player = true;
    player.player_id = Some(4242);
    player.last_interaction_target = Some(102);
    player.witnesses.insert(cimmeria_common::EntityId(102));
    player
        .available_interactions
        .insert(10, vec![(120101, None, 268435456)]);
    player.extensions.insert(LetterTurnInSession {
        player_id: 4242,
        token: LetterTurnInToken::from_bytes([4; 16]),
        generation: Instant::now(),
        epoch: LetterTurnInEntryEpoch::new(1),
    });
    player.missions.add_mission(MissionInstance::new(
        1360,
        4038,
        vec![MissionObjective {
            objective_id: 4651,
            status: STATUS_ACTIVE,
            hidden: false,
            optional: false,
        }],
    ));
    mgr
}

fn request_msg() -> ResolvedActions {
    ResolvedActions {
        actions: vec![(
            6501,
            Action::CompleteMissionWithItem {
                mission_id: 1360,
                expected_step_id: 4038,
                item_id: 3730,
                quantity: 1,
                dialog_id: 4576,
                dialog_set_id: 120101,
                slot: 10,
            },
        )],
        action_delays: vec![0],
        params: HashMap::from([
            ("target_entity_id".into(), serde_json::json!(102)),
            ("entity_tag".into(), serde_json::json!("CmdCenter_Marsh")),
        ]),
    }
}

async fn accepted_request(
    mgr: &mut SpaceManager,
    tx: &mpsc::Sender<CellToBaseMsg>,
    rx: &mut mpsc::Receiver<CellToBaseMsg>,
) -> LetterTurnInRequest {
    crate::cell::content::execute_actions(request_msg(), 101, 4242, tx, mgr, &ChainEngine::new())
        .await;
    let _feedback = rx.try_recv().unwrap();
    let plugin = rx.try_recv().unwrap();
    *plugin
        .plugin_payload::<LetterTurnInRequest>()
        .expect("accepted request")
}

#[tokio::test]
async fn receiver_delivers_actual_terminal_frames_and_rejects_duplicate() {
    let mut mgr = rig();
    let (tx, mut rx) = mpsc::channel(32);
    let request = accepted_request(&mut mgr, &tx, &mut rx).await;
    let result = LetterTurnInResult {
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
            item_id: 9001,
            container_id: 2,
            stack_before: 1,
            stack_after: 0,
        }),
    };
    handle_base_message(
        BaseToCellMsg::LetterTurnInResult(result.clone()),
        &tx,
        &mut mgr,
        &ChainEngine::new(),
        &[],
    )
    .await;
    let mut methods = Vec::new();
    let mut interaction = false;
    while let Ok(msg) = rx.try_recv() {
        match msg {
            CellToBaseMsg::EntityMethodCall {
                method_index, args, ..
            } => methods.push((method_index, args)),
            CellToBaseMsg::WitnessEntityMethod {
                witness_id: 101,
                entity_id: 102,
                args,
                ..
            } => {
                interaction = true;
                assert_eq!(args, 0u64.to_le_bytes());
            }
            other => panic!("unexpected output: {other:?}"),
        }
    }
    assert_eq!(
        methods.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
        [crate::mercury::method_idx::ON_DIALOG_DISPLAY, 82, 81, 80]
    );
    assert_eq!(
        methods[0].1,
        [
            &102i32.to_le_bytes()[..],
            &4576i32.to_le_bytes()[..],
            &0i32.to_le_bytes()[..],
            &[1],
            &0i32.to_le_bytes()[..],
        ]
        .concat()
    );
    assert_eq!(
        methods[1].1,
        [&4651i32.to_le_bytes()[..], &[1, 0, 0]].concat()
    );
    assert_eq!(methods[2].1, [&4038i32.to_le_bytes()[..], &[1]].concat());
    assert_eq!(
        methods[3].1,
        [&1360i32.to_le_bytes()[..], &[2], &0i32.to_le_bytes()[..]].concat()
    );
    assert!(interaction, "DSM removal changed the visible Marsh flags");
    assert!(!mgr
        .get_entity(101)
        .unwrap()
        .available_interactions
        .contains_key(&10));
    handle_base_message(
        BaseToCellMsg::LetterTurnInResult(result),
        &tx,
        &mut mgr,
        &ChainEngine::new(),
        &[],
    )
    .await;
    assert!(rx.try_recv().is_err());
}
