//! AB-N2: the ability-testing GM commands through the full router.
//!
//! For each of 136, 142, 153, 154 and 158 a player (access level 0) with
//! well-formed args gets only the gate's `onErrorCode`: no grant, no bulk
//! write, no god mode, no change to a mob. A GM's same call reaches the
//! handler. Reverting the `index >= 109` arm of `requires_gm` (or routing
//! these indices ahead of the gate) fails the player half.

use tokio::sync::mpsc;

use super::super::messages::CellToBaseMsg;
use super::super::space_manager::SpaceManager;
use super::*;
use crate::cell::console::gm::{
    GM_GIVE_ABILITY, GM_GIVE_ALL_ABILITIES, GM_RESET_ABILITIES, GM_SET_GOD_MODE,
    GM_SET_MOB_ABILITY_SET,
};

const CALLER: u32 = 1;
const MOB: u32 = 50;
/// `onErrorCode`.
const ON_ERROR_CODE: u16 = 121;

fn world(access_level: u32) -> SpaceManager {
    let mut mgr = crate::test_support::make_space_manager();
    mgr.create_entity(CALLER, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    let p = mgr.get_entity_mut(CALLER).unwrap();
    p.is_player = true;
    p.player_id = Some(100);
    p.access_level = access_level;
    p.archetype_id = Some(2);
    p.current_target_id = Some(MOB as i32);
    mgr.spawn_npc(MOB, "Agnos", [2.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(MOB).unwrap().abilities.add_ability(592);
    mgr.ability_sets.insert(350, vec![221, 1156]);
    crate::test_support::seed_ability_defs(&mut mgr, &[2826, 597]);
    mgr.ability_tree_catalog = crate::ability_tree::AbilityTreeCatalog::from_nodes([
        crate::ability_tree::TreeNode::with_defaults(2, 0, 597, 1, vec![]),
    ]);
    mgr
}

/// Each AB-N2 index with valid args.
fn calls() -> [(u16, Vec<u8>); 5] {
    [
        (GM_GIVE_ABILITY, 2826i32.to_le_bytes().to_vec()),
        (GM_SET_GOD_MODE, vec![1]),
        (GM_RESET_ABILITIES, vec![]),
        (GM_GIVE_ALL_ABILITIES, vec![]),
        (GM_SET_MOB_ABILITY_SET, 350i32.to_le_bytes().to_vec()),
    ]
}

async fn route(mgr: &mut SpaceManager, index: u16, args: &[u8]) -> Vec<CellToBaseMsg> {
    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(32);
    dispatch_cell_method(CALLER, index, args, &tx, mgr, &engine, None).await;
    std::iter::from_fn(|| rx.try_recv().ok()).collect()
}

fn mob_known(mgr: &SpaceManager) -> Vec<i32> {
    let mut ids = mgr.get_entity(MOB).unwrap().abilities.known_ability_ids();
    ids.sort_unstable();
    ids
}

#[tokio::test]
async fn ab_n2_gm_commands_are_refused_for_a_player_through_the_router() {
    for (index, args) in calls() {
        let mut mgr = world(0);
        let msgs = route(&mut mgr, index, &args).await;
        assert!(
            matches!(
                msgs.as_slice(),
                [CellToBaseMsg::EntityMethodCall {
                    method_index: ON_ERROR_CODE,
                    ..
                }]
            ),
            "{index}: a player gets only onErrorCode, got {msgs:?}"
        );
        assert!(
            !mgr.get_entity(CALLER).unwrap().god_mode,
            "{index}: no god mode"
        );
        assert_eq!(mob_known(&mgr), vec![592], "{index}: the mob is untouched");
    }
}

#[tokio::test]
async fn ab_n2_gm_commands_reach_their_handlers_for_a_gm() {
    for (index, args) in calls() {
        let mut mgr = world(2);
        let msgs = route(&mut mgr, index, &args).await;
        assert!(
            !msgs.iter().any(|m| matches!(
                m,
                CellToBaseMsg::EntityMethodCall {
                    method_index: ON_ERROR_CODE,
                    ..
                }
            )),
            "{index}: a GM is not refused: {msgs:?}"
        );
        let handled = match index {
            GM_GIVE_ABILITY => msgs
                .iter()
                .any(|m| matches!(m, CellToBaseMsg::GmGrantAbility { .. })),
            GM_SET_GOD_MODE => mgr.get_entity(CALLER).unwrap().god_mode,
            GM_RESET_ABILITIES | GM_GIVE_ALL_ABILITIES => msgs
                .iter()
                .any(|m| matches!(m, CellToBaseMsg::GmAbilityBulk(_))),
            _ => mob_known(&mgr) == vec![221, 1156],
        };
        assert!(handled, "{index}: the handler ran: {msgs:?}");
    }
}
