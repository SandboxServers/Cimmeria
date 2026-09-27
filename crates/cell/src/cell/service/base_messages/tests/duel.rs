//! `BaseToCellMsg::Duel` routing (SS-D1): a forwarded challenge reaches
//! `cell::duel::challenge` and the target gets `onDuelChallenge` [143].

use super::*;
use crate::cell::messages::DuelBaseToCell;

#[tokio::test]
async fn duel_challenge_reaches_the_duel_registry() {
    let mut mgr = crate::test_support::make_space_manager();
    for (eid, pid, x) in [(10u32, 1000i32, 0.0f32), (20, 2000, 5.0)] {
        mgr.create_entity(eid, "Agnos", [x, 0.0, 0.0], [0.0; 3])
            .unwrap();
        mgr.connect_entity(eid);
        mgr.get_entity_mut(eid).unwrap().player_id = Some(pid);
    }
    let (tx, mut rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let msg = BaseToCellMsg::Duel(DuelBaseToCell::Challenge {
        player_id: 1000,
        entity_id: 10,
        account_id: 1,
        target_player_id: 2000,
        target_entity_id: 20,
    });
    handle_base_message(msg, &tx, &mut mgr, &engine, &[]).await;
    assert!(mgr.duels.pending_for(2000).is_some());
    match rx.try_recv() {
        Ok(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        }) => {
            assert_eq!((entity_id, method_index), (20, 143));
            assert_eq!(args, vec![10, 0, 0, 0, 0, 0, 0, 0]);
        }
        _ => panic!("expected onDuelChallenge to the target first"),
    }
}
