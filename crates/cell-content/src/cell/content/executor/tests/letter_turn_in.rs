//! Request admission and same-attempt retry for the Marsh hand-in.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, STATUS_ACTIVE, STATUS_COMPLETED,
};
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};
use cimmeria_wire::cell::messages::{LetterTurnInRequest, LetterTurnInSession, LetterTurnInToken};

use super::super::letter_turn_in::{letter_turn_in_retry_tick, PendingLetterTurnIn};
use super::*;
use crate::cell::spawner::WorldRow;

const PLAYER: u32 = 101;
const MARSH: u32 = 102;
const PLAYER_ID: i32 = 4242;

fn rig() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Harset_CmdCenter" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /><Space WorldName="Harset" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Harset" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr.stamp_world_rows(&HashMap::from([
        ("Harset_CmdCenter".to_string(), WorldRow::enforcing(68)),
        ("Harset".to_string(), WorldRow::enforcing(57)),
    ]));
    let space = mgr
        .create_entity(PLAYER, "Harset_CmdCenter", [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.create_entity_in_space(MARSH, space, [1.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(PLAYER);
    let marsh = mgr.get_entity_mut(MARSH).unwrap();
    marsh.tag = Some("CmdCenter_Marsh".into());
    marsh.template_id = Some(10);
    let player = mgr.get_entity_mut(PLAYER).unwrap();
    player.is_player = true;
    player.player_id = Some(PLAYER_ID);
    player.last_interaction_target = Some(MARSH);
    player.extensions.insert(LetterTurnInSession {
        player_id: PLAYER_ID,
        token: LetterTurnInToken::from_bytes([8; 16]),
        generation: Instant::now(),
        epoch: cimmeria_wire::cell::messages::LetterTurnInEntryEpoch::new(1),
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

fn action() -> ResolvedActions {
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
            ("target_entity_id".into(), serde_json::json!(MARSH)),
            ("entity_tag".into(), serde_json::json!("CmdCenter_Marsh")),
        ]),
    }
}

async fn press(mgr: &mut SpaceManager, tx: &mpsc::Sender<CellToBaseMsg>) {
    execute_actions(action(), PLAYER, PLAYER_ID, tx, mgr, &ChainEngine::new()).await;
}

fn requests(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<LetterTurnInRequest> {
    let mut found = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let Some(request) = msg.plugin_payload::<LetterTurnInRequest>() {
            found.push(*request);
        } else {
            assert!(
                matches!(msg, CellToBaseMsg::EntityMethodCall { .. }),
                "only processing feedback may accompany a pending request: {msg:?}"
            );
        }
    }
    found
}

#[tokio::test]
async fn first_and_repeat_click_keep_one_attempt_without_early_success() {
    let mut mgr = rig();
    let (tx, mut rx) = mpsc::channel(8);
    press(&mut mgr, &tx).await;
    let feedback_msg = rx.try_recv().unwrap();
    assert!(
        matches!(feedback_msg, CellToBaseMsg::EntityMethodCall { entity_id: PLAYER, method_index, args }
        if method_index == crate::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION
            && args == serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, "Checking Frost's Letter..."))
    );
    let first_msg = rx.try_recv().unwrap();
    let first = *first_msg
        .plugin_payload::<LetterTurnInRequest>()
        .expect("one plugin request after feedback");
    assert!(rx.try_recv().is_err(), "no precommit success frame");
    let pending = mgr
        .get_entity(PLAYER)
        .unwrap()
        .extensions
        .get::<PendingLetterTurnIn>()
        .unwrap();
    assert_eq!(pending.request.attempt, first.attempt);
    assert_eq!(pending.marsh_entity_id, MARSH);
    assert_eq!(
        mgr.get_entity(PLAYER)
            .unwrap()
            .missions
            .get_mission(1360)
            .unwrap()
            .current_step_id,
        Some(4038)
    );
    assert_eq!(
        mgr.get_entity(PLAYER).unwrap().last_interaction_target,
        Some(MARSH)
    );

    press(&mut mgr, &tx).await;
    assert!(requests(&mut rx).is_empty());
    assert_eq!(
        mgr.get_entity(PLAYER)
            .unwrap()
            .extensions
            .get::<PendingLetterTurnIn>()
            .unwrap()
            .request
            .attempt,
        first.attempt
    );

    mgr.get_entity_mut(PLAYER)
        .unwrap()
        .missions
        .get_mission_mut(1360)
        .unwrap()
        .current_step_id = Some(4037);
    press(&mut mgr, &tx).await;
    assert!(requests(&mut rx).is_empty());
    assert!(!mgr
        .get_entity(PLAYER)
        .unwrap()
        .extensions
        .contains::<PendingLetterTurnIn>());
}

#[tokio::test]
async fn forged_or_stale_actor_target_world_and_mission_never_queue_request() {
    for case in 0..12 {
        let mut mgr = rig();
        let mut resolved = action();
        match case {
            0 => mgr.get_entity_mut(PLAYER).unwrap().player_id = Some(PLAYER_ID + 1),
            1 => mgr.get_entity_mut(MARSH).unwrap().tag = Some("Wrong".into()),
            2 => mgr.get_entity_mut(MARSH).unwrap().template_id = Some(11),
            3 => mgr.get_entity_mut(MARSH).unwrap().position.x = 100.0,
            4 => {
                mgr.get_entity_mut(PLAYER)
                    .unwrap()
                    .missions
                    .get_mission_mut(1360)
                    .unwrap()
                    .current_step_id = Some(4037)
            }
            5 => mgr
                .get_entity_mut(PLAYER)
                .unwrap()
                .extensions
                .remove::<LetterTurnInSession>()
                .map(|_| ())
                .unwrap(),
            6 => {
                resolved
                    .params
                    .insert("entity_tag".into(), serde_json::json!("Wrong"));
            }
            7 => {
                resolved
                    .params
                    .insert("target_entity_id".into(), serde_json::json!(PLAYER));
            }
            8 => mgr.stamp_world_rows(&HashMap::from([(
                "Harset_CmdCenter".to_string(),
                WorldRow::enforcing(57),
            )])),
            9 => mgr
                .get_entity_mut(PLAYER)
                .unwrap()
                .missions
                .get_mission_mut(1360)
                .unwrap()
                .active_objectives
                .clear(),
            10 => {
                mgr.get_entity_mut(PLAYER)
                    .unwrap()
                    .missions
                    .get_mission_mut(1360)
                    .unwrap()
                    .active_objectives[0]
                    .status = STATUS_COMPLETED
            }
            11 => {
                resolved.params.remove("target_entity_id");
            }
            _ => unreachable!(),
        }
        let (tx, mut rx) = mpsc::channel(8);
        execute_actions(
            resolved,
            PLAYER,
            PLAYER_ID,
            &tx,
            &mut mgr,
            &ChainEngine::new(),
        )
        .await;
        assert!(requests(&mut rx).is_empty(), "case {case}");
        assert!(!mgr
            .get_entity(PLAYER)
            .unwrap()
            .extensions
            .contains::<PendingLetterTurnIn>());
    }
    let mut mgr = rig();
    mgr.get_entity_mut(PLAYER)
        .unwrap()
        .missions
        .add_mission(MissionInstance::new(1361, 4694, vec![]));
    let (tx, mut rx) = mpsc::channel(8);
    press(&mut mgr, &tx).await;
    assert!(requests(&mut rx).is_empty(), "Praxis return overlaps Marsh");
}

#[tokio::test]
async fn one_free_slot_sends_feedback_and_retries_same_attempt_after_drain() {
    let mut mgr = rig();
    let (tx, mut rx) = mpsc::channel(1);
    press(&mut mgr, &tx).await;
    let pending = *mgr
        .get_entity(PLAYER)
        .unwrap()
        .extensions
        .get::<PendingLetterTurnIn>()
        .unwrap();
    let feedback_msg = rx
        .try_recv()
        .expect("processing feedback uses the only slot");
    assert!(
        matches!(feedback_msg, CellToBaseMsg::EntityMethodCall { entity_id: PLAYER, method_index, args }
        if method_index == crate::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION
            && args == serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, "Checking Frost's Letter..."))
    );
    assert!(
        rx.try_recv().is_err(),
        "request was not queued behind feedback"
    );
    mgr.get_entity_mut(PLAYER)
        .unwrap()
        .extensions
        .get_mut::<PendingLetterTurnIn>()
        .unwrap()
        .next_retry_at = Instant::now() - Duration::from_millis(1);
    letter_turn_in_retry_tick(&tx, &mut mgr);
    let retried = requests(&mut rx);
    assert_eq!(retried.len(), 1);
    assert_eq!(retried[0].attempt, pending.request.attempt);
    assert_eq!(retried[0].token, pending.request.token);
    assert_eq!(retried[0].generation, pending.request.generation);
    assert_eq!(retried[0].epoch, pending.request.epoch);
}

#[tokio::test]
async fn changed_token_or_generation_clears_old_pending_before_retry() {
    for change_token in [true, false] {
        let mut mgr = rig();
        let (tx, mut rx) = mpsc::channel(8);
        press(&mut mgr, &tx).await;
        let old = requests(&mut rx);
        assert_eq!(old.len(), 1);
        let session = mgr
            .get_entity_mut(PLAYER)
            .unwrap()
            .extensions
            .get_mut::<LetterTurnInSession>()
            .unwrap();
        if change_token {
            session.token = LetterTurnInToken::from_bytes([9; 16]);
        } else {
            session.generation += Duration::from_secs(1);
        }
        mgr.get_entity_mut(PLAYER)
            .unwrap()
            .extensions
            .get_mut::<PendingLetterTurnIn>()
            .unwrap()
            .next_retry_at = Instant::now() - Duration::from_millis(1);
        letter_turn_in_retry_tick(&tx, &mut mgr);
        assert!(
            requests(&mut rx).is_empty(),
            "old entry retried: change_token={change_token}"
        );
        assert!(!mgr
            .get_entity(PLAYER)
            .unwrap()
            .extensions
            .contains::<PendingLetterTurnIn>());
        press(&mut mgr, &tx).await;
        let new = requests(&mut rx);
        assert_eq!(new.len(), 1);
        assert_ne!(new[0].attempt, old[0].attempt);
        if change_token {
            assert_ne!(new[0].token, old[0].token);
        } else {
            assert_ne!(new[0].generation, old[0].generation);
        }
    }
}

#[tokio::test]
async fn full_queue_retries_same_attempt_and_stale_pending_is_cleared() {
    let mut mgr = rig();
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(CellToBaseMsg::EntityMethodCall {
        entity_id: PLAYER,
        method_index: 0,
        args: vec![],
    })
    .unwrap();
    press(&mut mgr, &tx).await;
    let attempt = mgr
        .get_entity(PLAYER)
        .unwrap()
        .extensions
        .get::<PendingLetterTurnIn>()
        .unwrap()
        .request
        .attempt;
    assert!(requests(&mut rx).is_empty());
    mgr.get_entity_mut(PLAYER)
        .unwrap()
        .extensions
        .get_mut::<PendingLetterTurnIn>()
        .unwrap()
        .next_retry_at = Instant::now() - Duration::from_millis(1);
    letter_turn_in_retry_tick(&tx, &mut mgr);
    let retried = requests(&mut rx);
    assert_eq!(retried.len(), 1);
    assert_eq!(retried[0].attempt, attempt);

    mgr.get_entity_mut(PLAYER).unwrap().last_interaction_target = None;
    mgr.get_entity_mut(MARSH).unwrap().position.x = 100.0;
    letter_turn_in_retry_tick(&tx, &mut mgr);
    assert!(
        mgr.get_entity(PLAYER)
            .unwrap()
            .extensions
            .contains::<PendingLetterTurnIn>(),
        "an accepted commit may still need receipt reconciliation after walking away"
    );

    mgr.get_entity_mut(PLAYER)
        .unwrap()
        .extensions
        .get_mut::<LetterTurnInSession>()
        .unwrap()
        .epoch = cimmeria_wire::cell::messages::LetterTurnInEntryEpoch::new(2);
    letter_turn_in_retry_tick(&tx, &mut mgr);
    assert!(!mgr
        .get_entity(PLAYER)
        .unwrap()
        .extensions
        .contains::<PendingLetterTurnIn>());
}

#[tokio::test]
async fn closed_queue_keeps_attempt_for_uncertain_retry() {
    let mut mgr = rig();
    let (tx, rx) = mpsc::channel(1);
    drop(rx);
    press(&mut mgr, &tx).await;
    let first = mgr
        .get_entity(PLAYER)
        .unwrap()
        .extensions
        .get::<PendingLetterTurnIn>()
        .unwrap()
        .request
        .attempt;
    mgr.get_entity_mut(PLAYER)
        .unwrap()
        .extensions
        .get_mut::<PendingLetterTurnIn>()
        .unwrap()
        .next_retry_at = Instant::now() - Duration::from_millis(1);
    letter_turn_in_retry_tick(&tx, &mut mgr);
    assert_eq!(
        mgr.get_entity(PLAYER)
            .unwrap()
            .extensions
            .get::<PendingLetterTurnIn>()
            .unwrap()
            .request
            .attempt,
        first
    );
}

#[tokio::test]
async fn new_entry_discards_old_pending_before_its_first_click() {
    let mut mgr = rig();
    let (tx, mut rx) = mpsc::channel(8);
    press(&mut mgr, &tx).await;
    let old = requests(&mut rx);
    assert_eq!(old.len(), 1);
    mgr.get_entity_mut(PLAYER)
        .unwrap()
        .extensions
        .get_mut::<LetterTurnInSession>()
        .unwrap()
        .epoch = cimmeria_wire::cell::messages::LetterTurnInEntryEpoch::new(2);
    press(&mut mgr, &tx).await;
    let new = requests(&mut rx);
    assert_eq!(new.len(), 1);
    assert_ne!(new[0].attempt, old[0].attempt);
    assert_eq!(new[0].epoch.value(), 2);
}
