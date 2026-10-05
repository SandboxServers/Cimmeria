//! A GM-removed NPC leaves no player in combat with it.
//!
//! Filter prefix: `despawn_releases_combat_`.
//!
//! Bug shape: `.despawn`, `/gmdespawn` and `.respawnall` removed or reset an
//! NPC without draining it from the players' `threatened_mobs`, so a player
//! who had hit it kept `BSF_InCombat` (no regen, no holster) until relog. The
//! player who only fought that NPC must leave combat and be told; one still
//! fighting another NPC must stay in combat and hear nothing.
//!
//! Revert proof: route any of these paths back to a bare `despawn_npc` (or
//! drop the `.respawnall` drain) and its test fails on `threatened_mobs`;
//! make the shared helper send to the owner only and the observer check fails.

use tokio::sync::mpsc;

use super::pt07_giveability::{console, world, CALLER};
use crate::cell::client_methods::being::ON_STATE_FIELD_UPDATE;
use crate::cell::combat::{generate_threat, AggroCause, BSF_IN_COMBAT};
use crate::cell::console::gm::{dispatch, GM_DESPAWN_BY_CMD};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

const WITNESS: u32 = 2;
/// A connected bystander who sees the caller (and fights nothing).
const OBSERVER: u32 = 4;

/// The caller hits `npc` only; the witness hits `npc` and a second NPC in
/// another space (so a reset of the caller's space does not touch it).
/// Returns `(world, npc, other)`.
fn fight() -> (SpaceManager, u32, u32) {
    let (mut mgr, npc) = world(2);
    let other = mgr.allocate_npc_id();
    mgr.spawn_npc(other, "Harset", [12.0, 0.0, 12.0], [0.0; 3])
        .unwrap();
    mgr.create_entity(OBSERVER, "Agnos", [14.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(OBSERVER);
    let o = mgr.get_entity_mut(OBSERVER).unwrap();
    o.is_player = true;
    o.player_id = Some(74);
    o.witnesses.insert(cimmeria_common::EntityId(CALLER as i32));
    let _ = generate_threat(&mut mgr, CALLER, npc, 10.0, AggroCause::Damage);
    let _ = generate_threat(&mut mgr, WITNESS, npc, 10.0, AggroCause::Damage);
    let _ = generate_threat(&mut mgr, WITNESS, other, 10.0, AggroCause::Damage);
    for p in [CALLER, WITNESS] {
        let e = mgr.get_entity(p).unwrap();
        assert!(e.threatened_mobs.contains(&npc), "fixture: {p} fights it");
        assert!(e.state_field & BSF_IN_COMBAT != 0, "fixture: {p} in combat");
    }
    (mgr, npc, other)
}

fn assert_released(mgr: &SpaceManager, msgs: &[CellToBaseMsg], npc: u32, other: u32) {
    let caller = mgr.get_entity(CALLER).unwrap();
    assert!(caller.threatened_mobs.is_empty(), "the NPC is drained");
    assert_eq!(
        caller.state_field & BSF_IN_COMBAT,
        0,
        "the caller left combat"
    );
    let witness = mgr.get_entity(WITNESS).unwrap();
    assert!(!witness.threatened_mobs.contains(&npc));
    assert!(witness.threatened_mobs.contains(&other), "still fighting");
    assert!(
        witness.state_field & BSF_IN_COMBAT != 0,
        "so still in combat"
    );
    let updates: Vec<(u32, u32)> = msgs
        .iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index: ON_STATE_FIELD_UPDATE,
                args,
            } => Some((*entity_id, u32::from_le_bytes(args[..4].try_into().ok()?))),
            _ => None,
        })
        .collect();
    assert_eq!(
        updates,
        vec![(CALLER, caller.state_field)],
        "only the caller's client is told, with its new state"
    );
    // Whoever sees the caller must see the in-combat bit drop too (the
    // client draws the combat stance on the entity it renders).
    let seen: Vec<(u32, u32, u32)> = msgs
        .iter()
        .filter_map(|m| match m {
            CellToBaseMsg::WitnessEntityMethod {
                witness_id,
                entity_id,
                method_index: ON_STATE_FIELD_UPDATE,
                args,
                ..
            } => Some((
                *witness_id,
                *entity_id,
                u32::from_le_bytes(args[..4].try_into().ok()?),
            )),
            _ => None,
        })
        .collect();
    assert_eq!(
        seen,
        vec![(OBSERVER, CALLER, caller.state_field)],
        "exactly one witness update, to the observer, about the caller"
    );
}

#[tokio::test]
async fn despawn_releases_combat_dot_despawn() {
    let (mut mgr, npc, other) = fight();
    let msgs = console(&mut mgr, Some(npc), ".despawn").await;
    assert!(mgr.get_entity(npc).is_none());
    assert_released(&mgr, &msgs, npc, other);
}

#[tokio::test]
async fn despawn_releases_combat_native_gmdespawn() {
    let (mut mgr, npc, other) = fight();
    let (tx, mut rx) = mpsc::channel(64);
    assert!(
        dispatch(
            CALLER,
            GM_DESPAWN_BY_CMD,
            &(npc as i32).to_le_bytes(),
            &tx,
            &mut mgr,
            &cimmeria_content_engine::chain::ChainEngine::new(),
        )
        .await
    );
    let msgs: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    assert!(mgr.get_entity(npc).is_none());
    assert_released(&mgr, &msgs, npc, other);
}

#[tokio::test]
async fn despawn_releases_combat_delspawn() {
    let (mut mgr, npc, other) = fight();
    let e = mgr.get_entity_mut(npc).unwrap();
    e.spawn_id = Some(77);
    e.template_id = Some(1);
    let msgs = console(&mut mgr, Some(npc), ".delspawn").await;
    assert!(mgr.get_entity(npc).is_none());
    assert_released(&mgr, &msgs, npc, other);
}

/// `.respawnall` keeps the NPC but forgets its threat list: the players it
/// forgot must leave combat with it too.
#[tokio::test]
async fn despawn_releases_combat_respawnall() {
    let (mut mgr, npc, other) = fight();
    let msgs = console(&mut mgr, None, ".respawnall").await;
    assert!(mgr.get_entity(npc).unwrap().threat_list.is_empty());
    assert_released(&mgr, &msgs, npc, other);
}
