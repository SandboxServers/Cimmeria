//! **#411 end-to-end guard.** A mis-gated `player_loaded` chain (one
//! whose author forgot the `mission_status eq not_active` condition)
//! fires `accept_mission` on every world entry. Pre-fix, that
//! overwrote a relog-restored COMPLETED mission with a fresh ACTIVE
//! instance and persisted `MissionUpdate status=1` over the saved
//! row — completed missions reappeared as active in the quest log
//! after relog. The server-side offer guard must hold even when the
//! chain data is wrong.

use super::super::*;
use crate::cell::messages::SavedMission;
use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::Chain;
use cimmeria_content_engine::triggers::Trigger;
use cimmeria_entity::missions::MISSION_COMPLETED;

#[tokio::test]
async fn mis_gated_player_loaded_chain_cannot_resurrect_completed_mission() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.connect_entity(1);
    mgr.mission_defs.insert(
        622,
        crate::cell::spawner::MissionDefEntry {
            step_id: 2113,
            objectives: vec![],
            is_hidden: false,
            num_repeats: 1,
            can_repeat_on_fail: true,
        },
    );

    // The mis-gated chain: player_loaded, NO conditions, accepts 622.
    let mut engine = ChainEngine::new();
    engine.register_chain(Chain {
        id: 9999,
        name: "mis-gated 622 grant (no not_active condition)".into(),
        enabled: true,
        trigger: Trigger::OnPlayerLoaded { world_name: None },
        conditions: vec![],
        actions: vec![Action::AcceptMission { mission_id: 622 }],
        priority: 0,
    });

    // Relog payload: 622 completed, repeat counter at the cap.
    let saved = vec![SavedMission {
        mission_id: 622,
        status: MISSION_COMPLETED,
        current_step_id: None,
        completed_step_ids: vec![2113],
        completed_objective_ids: vec![],
        active_objective_ids: vec![],
        failed_objective_ids: vec![],
        repeats: 2, // > num_repeats = 1 → not re-offerable
    }];

    let (tx, mut rx) = mpsc::channel(64);
    handle_init_player_state(
        1,
        100,
        "Castle_CellBlock".into(),
        1,
        saved,
        vec![],
        0,
        vec![],
        cimmeria_entity::cell_entity::SystemOptions::default(),
        0, // state_field
        0, // access_level
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    // Cell-side state: still completed, repeat counter intact.
    let m = mgr
        .get_entity(1)
        .unwrap()
        .missions
        .get_mission(622)
        .expect("restored mission must still be tracked");
    assert_eq!(
        m.status, MISSION_COMPLETED,
        "the offer guard must keep the restored mission COMPLETED even \
         when a mis-gated player_loaded chain re-fires accept_mission"
    );
    assert_eq!(m.repeats, 2, "repeat counter must survive the relog");

    // Wire/persist side: no MissionUpdate(status=1) may have been sent —
    // that's the message that UPSERTs "active" over the saved row.
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::MissionUpdate {
            mission_id, status, ..
        } = msg
        {
            assert!(
                !(mission_id == 622 && status == 1),
                "relog must not persist MissionUpdate(622, status=1) — \
                 this is the exact write that resurrected completed \
                 missions as active (#411)"
            );
        }
    }
}
