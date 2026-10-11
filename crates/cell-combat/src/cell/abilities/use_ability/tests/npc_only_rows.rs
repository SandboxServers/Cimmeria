//! TG-CMB-09: the launch row of an NPC-on-NPC cast is background traffic and
//! logs at DEBUG; a cast by or at a player stays INFO. Reverting the level
//! choice in `handle.rs` makes the NPC row INFO and fails the first assert.
//! The kill rows (`target_killed`, the respawn row) follow the same rule.

use super::duel_gate::{duel_mgr, A, A_PID, MOB};
use super::warmup::INSTANT_ABILITY;
use super::*;
use crate::cell::combat::{mark_npc_dead_with_kill, HOSTILE_FACTION};
use crate::test_support::{
    add_pet_owner, seed_pet_template, Captured, LogCapture, PET_FIXTURE_TEMPLATE_ID,
};
use cimmeria_entity::stats::HEALTH;

/// A second hostile NPC for the NPC caster to strike.
const NPC_TARGET: u32 = 5;
/// The owner of the pet in the pet-kill test.
const OWNER: u32 = 7;
const RESPAWN: &str = "NPC death: respawn scheduled";

/// The `ability_launched` row whose `entity_id` is `eid`.
fn launch_of(all: &[Captured], eid: u32) -> Option<Captured> {
    all.iter()
        .find(|c| {
            c.has_field("event", "ability_launched") && c.has_field("entity_id", &eid.to_string())
        })
        .cloned()
}

/// The `target_killed` row whose `attacker_id` is `attacker`.
fn kill_by(all: &[Captured], attacker: u32) -> Option<Captured> {
    all.iter()
        .find(|c| {
            c.has_field("event", "target_killed")
                && c.has_field("attacker_id", &attacker.to_string())
        })
        .cloned()
}

/// `duel_mgr` with `NPC_TARGET` at 1 HP, so one cast kills it.
fn mgr_with_weak_npc() -> SpaceManager {
    let mut mgr = duel_mgr();
    mgr.spawn_npc(NPC_TARGET, "Castle", [3.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(NPC_TARGET).unwrap().faction = HOSTILE_FACTION;
    mgr.get_entity_mut(NPC_TARGET)
        .unwrap()
        .stats
        .get_mut(HEALTH)
        .unwrap()
        .update(0, 1, 100_000);
    mgr
}

/// **TG-CMB-09.** NPC 4 casts at NPC 5: DEBUG. Player 1 casts at NPC 4: INFO.
#[tokio::test]
async fn npc_only_launch_logs_at_debug() {
    let mut mgr = duel_mgr();
    mgr.spawn_npc(NPC_TARGET, "Castle", [3.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(NPC_TARGET).unwrap().faction = HOSTILE_FACTION;
    mgr.get_entity_mut(MOB)
        .unwrap()
        .abilities
        .add_ability(INSTANT_ABILITY);
    let (tx, _rx) = mpsc::channel(1024);
    let logs = LogCapture::install();

    handle_use_ability(MOB, INSTANT_ABILITY, NPC_TARGET as i32, &tx, &mut mgr).await;
    let npc = launch_of(&logs.all(), MOB).expect("the NPC cast logs an ability_launched row");
    assert_eq!(npc.level, tracing::Level::DEBUG, "{npc:?}");

    handle_use_ability(A, INSTANT_ABILITY, MOB as i32, &tx, &mut mgr).await;
    let player = launch_of(&logs.all(), A).expect("the player cast logs an ability_launched row");
    assert_eq!(player.level, tracing::Level::INFO, "{player:?}");
    assert!(
        player.has_field("player_id", &A_PID.to_string()),
        "{player:?}"
    );
}

/// **TG-CMB-09.** An NPC-only kill: NPC 4 kills NPC 5, so `target_killed`
/// is DEBUG.
#[tokio::test]
async fn npc_on_npc_kill_logs_target_killed_at_debug() {
    let mut mgr = mgr_with_weak_npc();
    mgr.get_entity_mut(MOB)
        .unwrap()
        .abilities
        .add_ability(INSTANT_ABILITY);
    let (tx, _rx) = mpsc::channel(1024);
    let logs = LogCapture::install();
    tracing::callsite::rebuild_interest_cache();

    handle_use_ability(MOB, INSTANT_ABILITY, NPC_TARGET as i32, &tx, &mut mgr).await;
    let killed = kill_by(&logs.all(), MOB).expect("the NPC kill logs a target_killed row");
    assert_eq!(killed.level, tracing::Level::DEBUG, "{killed:?}");
}

/// **TG-CMB-09.** A pet's kill of an NPC is a player-side kill: INFO.
#[tokio::test]
async fn pet_kill_of_npc_logs_target_killed_at_info() {
    let mut mgr = duel_mgr();
    seed_pet_template(&mut mgr, PET_FIXTURE_TEMPLATE_ID);
    add_pet_owner(&mut mgr, OWNER, "Castle", [0.0; 3], 12);
    let pet = mgr
        .spawn_pet_from_template(OWNER, PET_FIXTURE_TEMPLATE_ID, 2826)
        .expect("fixture: the pet spawns");
    mgr.get_entity_mut(pet)
        .unwrap()
        .abilities
        .add_ability(INSTANT_ABILITY);
    mgr.get_entity_mut(MOB)
        .unwrap()
        .stats
        .get_mut(HEALTH)
        .unwrap()
        .update(0, 1, 100_000);
    let (tx, _rx) = mpsc::channel(1024);
    let logs = LogCapture::install();
    tracing::callsite::rebuild_interest_cache();

    handle_use_ability(pet, INSTANT_ABILITY, MOB as i32, &tx, &mut mgr).await;
    let killed = kill_by(&logs.all(), pet).expect("the pet kill logs a target_killed row");
    assert_eq!(killed.level, tracing::Level::INFO, "{killed:?}");
}

/// **TG-CMB-09.** The respawn row follows the kill's side: an NPC-only kill
/// (`player_side = false`) writes it at DEBUG, a player-side kill at INFO.
#[test]
fn mark_npc_dead_with_kill_levels_the_respawn_row() {
    let mut mgr = duel_mgr();
    mgr.spawn_npc(NPC_TARGET, "Castle", [3.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(NPC_TARGET).unwrap().respawn_secs = Some(30);
    let logs = LogCapture::install();
    tracing::callsite::rebuild_interest_cache();

    mark_npc_dead_with_kill(mgr.get_entity_mut(NPC_TARGET).unwrap(), "Castle", false);
    assert!(
        logs.find_message(tracing::Level::DEBUG, RESPAWN).is_some(),
        "an NPC-only kill writes the respawn row at DEBUG"
    );
    assert!(
        logs.find_message(tracing::Level::INFO, RESPAWN).is_none(),
        "an NPC-only kill does not write it at INFO"
    );

    mark_npc_dead_with_kill(mgr.get_entity_mut(NPC_TARGET).unwrap(), "Castle", true);
    assert!(
        logs.find_message(tracing::Level::INFO, RESPAWN).is_some(),
        "a player-side kill writes the respawn row at INFO"
    );
}
