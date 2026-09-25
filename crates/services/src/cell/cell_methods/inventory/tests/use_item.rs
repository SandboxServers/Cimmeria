use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;

use super::super::dispatch::dispatch;
use super::super::USE_ITEM;
use super::make_test_space_mgr;

/// Dispatch one `useItem(item 7001, target 0)` from player entity 1 and drain
/// every message the cell sent.
async fn use_item(dead: bool) -> Vec<CellToBaseMsg> {
    use crate::cell::content::build_engine;

    let mut mgr = make_test_space_mgr();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
        if dead {
            e.set_state_flag(crate::cell::combat::BSF_DEAD);
        }
    }
    mgr.connect_entity(1);

    let (tx, mut rx) = mpsc::channel(8);
    let engine = build_engine(None).await;
    let mut args = Vec::with_capacity(8);
    args.extend_from_slice(&7001i32.to_le_bytes());
    args.extend_from_slice(&0i32.to_le_bytes());
    dispatch(1, USE_ITEM, &args, &tx, &mut mgr, &engine).await;

    let mut out = Vec::new();
    while let Ok(m) = rx.try_recv() {
        out.push(m);
    }
    out
}

/// NA24 (UAT-1 A): a dead player's medkit healed the corpse to 500 HP, and
/// the NPC that killed it kept it as a live target through the respawn. The
/// cell must refuse `useItem` while `BSF_DEAD` is set -- no
/// `UseInventoryItem` reaches base, so no `OnItemUse` chain can heal -- and
/// answer the press with the legacy `@mustBeAlive` reply
/// `onErrorCode(ERRORCODE_SYSTEM_Ability=0, 0, CONDITION_FEEDBACK_NotLiving=14)`.
#[tokio::test]
async fn use_item_is_refused_with_not_living_feedback_while_dead() {
    let out = use_item(true).await;
    assert!(
        !out.iter()
            .any(|m| matches!(m, CellToBaseMsg::UseInventoryItem { .. })),
        "a dead player's useItem must not reach base: {out:?}"
    );
    let feedback: Vec<&Vec<u8>> = out
        .iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                entity_id: 1,
                method_index,
                args,
            } if *method_index == crate::cell::client_methods::player::ON_ERROR_CODE => Some(args),
            _ => None,
        })
        .collect();
    assert_eq!(
        feedback,
        vec![&vec![0u8, 0, 0, 0, 0, 14, 0]],
        "exactly one onErrorCode(0, 0, NotLiving) so the press is not silent"
    );
}

/// The living path is unchanged: the use goes to base and nothing else.
#[tokio::test]
async fn use_item_forwards_to_base_while_alive() {
    let out = use_item(false).await;
    assert_eq!(out.len(), 1, "{out:?}");
    assert!(matches!(
        out[0],
        CellToBaseMsg::UseInventoryItem {
            entity_id: 1,
            player_id: 100,
            item_id: 7001,
            target_id: 0,
        }
    ));
}
