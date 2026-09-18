//! `fire_player_flanked_npc` — the player-perspective flank dispatcher
//! behind Castle Cellblock C06's flank objectives. The chain-replay tests
//! in `chain_replay_tests/mission_681_686_flank.rs` pin the seed; these pin
//! the executor boundary: actions must land on the flanking PLAYER (not the
//! NPC, unlike `fire_npc_flanked`), and completing the flank objective must
//! not trip the all-objectives-done auto-complete.

use tokio::sync::mpsc;

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{Chain, ChainEngine};
use cimmeria_content_engine::triggers::Trigger;
use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_ACTIVE, STATUS_ACTIVE, STATUS_COMPLETED,
};

use super::fire_player_flanked_npc;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Player = entity 1 (db player 100), NPC = entity 2.
fn make_mgr_with_player_and_npc() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    mgr.create_entity(2, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(100);
    }
    mgr.connect_entity(1);
    mgr
}

fn flank_chain(id: i64, template: Option<&str>, actions: Vec<Action>) -> Chain {
    Chain {
        action_delays: Vec::new(),
        id,
        name: "test: player flanked npc".to_string(),
        enabled: true,
        trigger: Trigger::OnPlayerFlankedNpc {
            npc_template: template.map(str::to_string),
        },
        conditions: vec![],
        actions,
        priority: 0,
    }
}

fn objective(objective_id: i32) -> MissionObjective {
    MissionObjective {
        objective_id,
        status: STATUS_ACTIVE,
        hidden: false,
        optional: false,
    }
}

/// The dispatcher must execute against the flanking player: the counter
/// lands on entity 1 (the player), not entity 2 (the NPC) — the opposite of
/// `fire_npc_flanked`.
#[tokio::test]
async fn actions_land_on_the_flanking_player_not_the_npc() {
    let mut mgr = make_mgr_with_player_and_npc();
    let mut engine = ChainEngine::new();
    engine.register_chain(flank_chain(
        0x7000_6C01,
        Some("NID Guard"),
        vec![Action::IncrementCounter {
            counter_name: "test_player_flank".to_string(),
            amount: 1,
        }],
    ));
    let (tx, _rx) = mpsc::channel(16);

    fire_player_flanked_npc(2, 1, "NID Guard", &engine, &tx, &mut mgr).await;

    let player = mgr.get_entity(1).expect("player must still exist");
    assert_eq!(
        player.counters.get("test_player_flank"),
        Some(&1),
        "matched chain must execute on the flanking player"
    );
    let npc = mgr.get_entity(2).expect("npc must still exist");
    assert!(
        !npc.counters.contains_key("test_player_flank"),
        "the NPC must not receive the action; counters: {:?}",
        npc.counters
    );
}

#[tokio::test]
async fn typed_chain_rejects_a_different_template() {
    let mut mgr = make_mgr_with_player_and_npc();
    let mut engine = ChainEngine::new();
    engine.register_chain(flank_chain(
        0x7000_6C02,
        Some("NID Guard"),
        vec![Action::IncrementCounter {
            counter_name: "test_wrong_template".to_string(),
            amount: 1,
        }],
    ));
    let (tx, _rx) = mpsc::channel(16);

    fire_player_flanked_npc(2, 1, "Jaffa Warrior", &engine, &tx, &mut mgr).await;

    assert!(
        !mgr.get_entity(1)
            .unwrap()
            .counters
            .contains_key("test_wrong_template"),
        "a chain typed to 'NID Guard' must not fire for another template"
    );
}

/// An NPC "threat" (entity 2 has no `player_id`) and an unknown threat id
/// must both no-op — there is no player to act on.
#[tokio::test]
async fn no_ops_when_the_threat_is_not_a_player() {
    let mut mgr = make_mgr_with_player_and_npc();
    let mut engine = ChainEngine::new();
    engine.register_chain(flank_chain(
        0x7000_6C03,
        None,
        vec![Action::IncrementCounter {
            counter_name: "test_no_player".to_string(),
            amount: 1,
        }],
    ));
    let (tx, mut rx) = mpsc::channel(16);

    // Threat is the NPC (entity 2): it carries no db player_id.
    fire_player_flanked_npc(1, 2, "NID Guard", &engine, &tx, &mut mgr).await;
    // Threat id that doesn't exist in any space.
    fire_player_flanked_npc(2, 999, "NID Guard", &engine, &tx, &mut mgr).await;

    for eid in [1u32, 2] {
        assert!(
            !mgr.get_entity(eid)
                .unwrap()
                .counters
                .contains_key("test_no_player"),
            "entity {eid} must not receive an action when the threat isn't a player"
        );
    }
    assert!(rx.try_recv().is_err(), "no wire messages expected");
}

/// End-to-end through the executor with C06's real shape: mission 681 on
/// step 2348 with the kill objective 2724 and the flank objective 2725 both
/// open. Flanking completes ONLY 2725 — the player gets an
/// `onObjectiveUpdate` — and the mission stays active. Guards the
/// auto-complete trap (`cell::missions::complete_objective` ends the
/// mission when every required objective is done): if 2724 were ever
/// pre-completed, or the flank chain grew a second objective action, this
/// would end mission 681 with no kills.
#[tokio::test]
async fn flank_completes_only_the_flank_objective_and_keeps_the_mission_active() {
    let mut mgr = make_mgr_with_player_and_npc();
    mgr.get_entity_mut(1)
        .unwrap()
        .missions
        .add_mission(MissionInstance::new(
            681,
            2348,
            vec![objective(2724), objective(2725)],
        ));
    let mut engine = ChainEngine::new();
    engine.register_chain(flank_chain(
        0x7000_6C04,
        Some("NID Guard"),
        vec![Action::CompleteObjective {
            mission_id: 681,
            objective_id: 2725,
        }],
    ));
    let (tx, mut rx) = mpsc::channel(16);

    fire_player_flanked_npc(2, 1, "NID Guard", &engine, &tx, &mut mgr).await;

    let mission = mgr
        .get_entity(1)
        .unwrap()
        .missions
        .get_mission(681)
        .expect("mission 681 must still be tracked");
    let status_of = |id: i32| {
        mission
            .active_objectives
            .iter()
            .find(|o| o.objective_id == id)
            .map(|o| o.status)
    };
    assert_eq!(
        status_of(2725),
        Some(STATUS_COMPLETED),
        "flank objective done"
    );
    assert_eq!(
        status_of(2724),
        Some(STATUS_ACTIVE),
        "kill objective untouched"
    );
    assert_eq!(
        mission.status, MISSION_ACTIVE,
        "flanking alone must not complete mission 681"
    );

    let mut objective_updates = 0;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            ..
        } = msg
        {
            if method_index == crate::cell::missions::ON_OBJECTIVE_UPDATE {
                assert_eq!(entity_id, 1, "onObjectiveUpdate must target the player");
                objective_updates += 1;
            }
        }
    }
    assert_eq!(
        objective_updates, 1,
        "exactly one onObjectiveUpdate expected"
    );
}
