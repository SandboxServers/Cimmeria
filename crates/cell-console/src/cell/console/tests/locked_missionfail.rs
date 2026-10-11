//! TG-DB-12: `.missionfail` on a target whose saved missions are unreadable
//! is refused like every other mission change. Fails without the
//! `refuse_if_state_unknown` call in `console::mission::fail`.

use cimmeria_entity::missions::{MissionInstance, MISSION_ACTIVE};

use super::*;

#[tokio::test]
async fn missionfail_on_a_locked_target_is_refused() {
    let (mut mgr, gm, npc) = setup();
    if let Some(e) = mgr.get_entity_mut(npc) {
        e.is_player = true;
        e.player_id = Some(501);
        e.missions
            .add_mission(MissionInstance::new(687, 2200, vec![]));
    }
    crate::cell::missions::state_lock::mark_state_unknown(npc, &mut mgr);
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);
    handle_console_command(gm, ".missionfail 687", &tx, &mut mgr, &engine).await;

    assert_eq!(
        mgr.get_entity(npc)
            .unwrap()
            .missions
            .get_mission(687)
            .unwrap()
            .status,
        MISSION_ACTIVE,
        "a locked tracker must not be failed"
    );
    let mut to_gm = 0;
    while let Ok(msg) = rx.try_recv() {
        if matches!(msg, CellToBaseMsg::EntityMethodCall { entity_id, .. } if entity_id == gm) {
            to_gm += 1;
        }
    }
    assert!(to_gm >= 1, "the GM is told why nothing happened");
}
