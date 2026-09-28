//! #715: `.missionfail` on a hidden mission fails it but sends the target no
//! `onMissionUpdate` (reference `MissionManager.py:724`). The visible case is
//! the control, so a gate that dropped every frame would fail it.
//!
//! Fails without the `suppress_hidden_mission_frames` call in
//! `console::mission::fail`.

use cimmeria_entity::missions::{MissionInstance, MISSION_FAILED};

use super::*;
use crate::cell::client_methods::missionary::ON_MISSION_UPDATE;

/// Fail mission 689 on the NPC-turned-player target and return how many
/// `onMissionUpdate` frames went to that target.
async fn missionfail_frames_to_target(is_hidden: bool) -> usize {
    let (mut mgr, gm, npc) = setup();
    if let Some(e) = mgr.get_entity_mut(npc) {
        e.is_player = true;
        e.player_id = Some(500);
        let mut m = MissionInstance::new(689, 2200, vec![]);
        m.is_hidden = is_hidden;
        e.missions.add_mission(m);
    }
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);
    handle_console_command(gm, ".missionfail 689", &tx, &mut mgr, &engine).await;

    assert_eq!(
        mgr.get_entity(npc)
            .unwrap()
            .missions
            .get_mission(689)
            .unwrap()
            .status,
        MISSION_FAILED,
        "the state change happens whether or not the mission is hidden"
    );

    let mut frames = 0;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            ..
        } = msg
        {
            if entity_id == npc && method_index == ON_MISSION_UPDATE {
                frames += 1;
            }
        }
    }
    frames
}

#[tokio::test]
async fn missionfail_on_a_hidden_mission_sends_no_frame() {
    assert_eq!(missionfail_frames_to_target(true).await, 0);
}

#[tokio::test]
async fn missionfail_on_a_visible_mission_still_sends_its_frame() {
    assert_eq!(missionfail_frames_to_target(false).await, 1);
}
