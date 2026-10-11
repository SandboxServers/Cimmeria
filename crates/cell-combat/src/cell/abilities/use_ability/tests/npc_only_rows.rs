//! TG-CMB-09: the launch row of an NPC-on-NPC cast is background traffic and
//! logs at DEBUG; a cast by or at a player stays INFO. Reverting the level
//! choice in `handle.rs` makes the NPC row INFO and fails the first assert.

use super::duel_gate::{duel_mgr, A, A_PID, MOB};
use super::warmup::INSTANT_ABILITY;
use super::*;
use crate::cell::combat::HOSTILE_FACTION;
use crate::test_support::{Captured, LogCapture};

/// A second hostile NPC for the NPC caster to strike.
const NPC_TARGET: u32 = 5;

/// The `ability_launched` row whose `entity_id` is `eid`.
fn launch_of(all: &[Captured], eid: u32) -> Option<Captured> {
    all.iter()
        .find(|c| {
            c.has_field("event", "ability_launched") && c.has_field("entity_id", &eid.to_string())
        })
        .cloned()
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
