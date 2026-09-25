//! NA24 (UAT-1 A, colo 2026-09-25 12:17): an NPC that kills a player lets go
//! of the corpse, and does not pick the player back up after the respawn.
//!
//! The playtest sequence: Hallway02_Guard kills the player; 0.6 s later the
//! dead player uses a medkit and a chain heals the corpse to 500 HP; the next
//! fight pass reads HEALTH, keeps the "living" target, and its attack is
//! refused (the target is dead), which arms the 500 ms retry sweep; the player
//! respawns 106 u away; the sweep's fight pass re-targets the living player
//! and the guard leaves cover to chase it across the floor.

use super::{make_ai_fixture, seed_default_ability, seed_target_with_threat};
use crate::cell::combat::{BSF_DEAD, NPC_DEFAULT_ABILITY};
use cimmeria_entity::cell_entity::AiState;
use cimmeria_entity::stats::HEALTH;
use tokio::sync::mpsc;

const NPC: u32 = 200;
const PLAYER: u32 = 100;

/// A Fighting guard at the origin holding a player 5 u away on its threat
/// list, with a real in-range ability so a pass that keeps the target
/// visibly fights.
fn fighting_guard() -> crate::cell::space_manager::SpaceManager {
    let mut mgr = make_ai_fixture([0.0; 3], [0.0; 3]);
    seed_default_ability(&mut mgr, 0, 30);
    seed_target_with_threat(&mut mgr, NPC, PLAYER, [5.0, 0.0, 0.0]);
    if let Some(npc) = mgr.get_entity_mut(NPC) {
        npc.abilities.add_ability(NPC_DEFAULT_ABILITY);
        npc.faction = crate::cell::combat::HOSTILE_FACTION;
    }
    if let Some(p) = mgr.get_entity_mut(PLAYER) {
        p.threatened_mobs.insert(NPC);
    }
    mgr
}

fn set_player_health(mgr: &mut crate::cell::space_manager::SpaceManager, cur: i32) {
    if let Some(h) = mgr
        .get_entity_mut(PLAYER)
        .and_then(|p| p.stats.get_mut(HEALTH))
    {
        h.update(0, cur, 100);
        h.clear_dirty();
    }
}

/// Death, medkit heal, respawn 106 u away -- with no AI pass in between,
/// exactly the window the old code left open -- then the retry sweep and a
/// natural tick. The guard must not be fighting the respawned player.
///
/// Revert proof: remove the `purge_dead_player_from_threat` call from
/// `abilities::death::resolve_death` and the player is still on the threat
/// list after the death; the sweep's fight pass then sees a living target
/// (HEALTH 100, `BSF_DEAD` cleared by the respawn) and keeps fighting it.
#[tokio::test]
async fn npc_drops_a_dead_player_and_does_not_reacquire_it_after_respawn() {
    let mut mgr = fighting_guard();
    let (tx, _rx) = mpsc::channel(1024);

    // 1. The guard kills the player.
    set_player_health(&mut mgr, 0);
    assert!(crate::cell::abilities::resolve_death_for_test(PLAYER, NPC, &tx, &mut mgr).await);
    let npc = mgr.get_entity(NPC).unwrap();
    assert!(
        !npc.threat_list.contains_key(&PLAYER),
        "the corpse must leave the killer's threat list at the moment of death, \
         not on the next AI pass: {:?}",
        npc.threat_list
    );
    assert_eq!(
        npc.ai_state(),
        AiState::Leashing,
        "with nobody left to fight the guard starts home now"
    );
    assert!(
        !mgr.get_entity(PLAYER)
            .unwrap()
            .threatened_mobs
            .contains(&NPC),
        "the corpse leaves the guard's combat set with the drop"
    );

    // 2. The dead player's medkit heals the corpse (the chain path the cell
    //    now refuses; the AI must hold even if some other path heals).
    set_player_health(&mut mgr, 100);
    // 3. A retry was pending from the refused attack on the corpse.
    if let Some(npc) = mgr.get_entity_mut(NPC) {
        npc.ai_retry_at = Some(std::time::Instant::now());
    }
    mgr.pending_ai_retries.insert(NPC);
    // 4. Respawn 106 u away: state flags reset, HEALTH full.
    if let Some(p) = mgr.get_entity_mut(PLAYER) {
        p.clear_all_state_flags();
    }
    mgr.update_entity_position(PLAYER, [106.0, 0.0, 0.0], [0, 0, 0], [0.0; 3]);

    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    crate::cell::service::npc_ai::npc_ai_retry_sweep(&tx, &mut mgr, &engine).await;
    crate::cell::service::npc_ai::npc_ai_tick(&tx, &mut mgr, &engine).await;

    let npc = mgr.get_entity(NPC).unwrap();
    assert!(
        !npc.threat_list.contains_key(&PLAYER),
        "the respawned player must not be re-acquired: {:?}",
        npc.threat_list
    );
    assert_ne!(npc.ai_state(), AiState::Fighting);
}

/// The fight pass itself treats a `BSF_DEAD` target as dead even when its
/// HEALTH says otherwise (the healed corpse). Revert proof: drop the
/// `is_dead_state` arm in `fight_target::select_target` and the pass keeps
/// the corpse and stays `Fighting`.
#[tokio::test]
async fn fight_pass_drops_a_target_carrying_bsf_dead_despite_health() {
    let mut mgr = fighting_guard();
    // Healed corpse: dead bit set, HEALTH full. Put there directly, the way
    // a death that predates the purge left it.
    if let Some(p) = mgr.get_entity_mut(PLAYER) {
        p.set_state_flag(BSF_DEAD);
    }
    let (tx, _rx) = mpsc::channel(1024);
    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    crate::cell::service::npc_ai::npc_ai_tick(&tx, &mut mgr, &engine).await;

    let npc = mgr.get_entity(NPC).unwrap();
    assert!(
        !npc.threat_list.contains_key(&PLAYER),
        "a BSF_DEAD target must be pruned whatever its HEALTH: {:?}",
        npc.threat_list
    );
    assert_eq!(npc.ai_state(), AiState::Leashing);
}
