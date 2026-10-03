//! The `vitals` `damage_taken` row is wired into the hit pipeline: a player
//! target logs its pools before and after, an NPC target logs nothing.
//! Removing the `log_damage_taken` call in `apply_hit` fails the first test.

use super::tests::{make_ability, make_effect, make_mgr_player_vs_npc};
use super::*;
use crate::test_support::LogCapture;
use cimmeria_entity::stats::HEALTH;
use tracing::Level;

#[tokio::test]
async fn player_target_logs_damage_taken_with_pools() {
    let mut mgr = make_mgr_player_vs_npc();
    // Swap roles: the NPC (1) hits the player (2).
    let a = mgr.get_entity_mut(1).unwrap();
    a.is_player = false;
    a.player_id = None;
    let t = mgr.get_entity_mut(2).unwrap();
    t.is_player = true;
    t.player_id = Some(72);
    t.account_id = Some(6);
    t.stats.get_mut(HEALTH).unwrap().update(0, 100, 100);
    let ability = make_ability(7, vec![100]);
    mgr.ability_defs.insert(7, ability.clone());
    mgr.effect_defs.insert(100, make_effect(100, 30));
    let (tx, _rx) = mpsc::channel(64);

    let capture = LogCapture::install();
    apply_damage_to_target(1, 2, 7, &Some(ability), 1, false, &tx, &mut mgr).await;

    let row = capture
        .find_message(Level::DEBUG, "vitals: player took damage")
        .expect("a hit on a player logs vitals damage_taken");
    assert_eq!(row.target, "vitals");
    assert!(row.has_field("player_id", "72"), "{row:#?}");
    assert!(row.has_field("account_id", "6"));
    assert!(row.has_field("attacker_entity_id", "1"));
    assert!(row.has_field("ability_id", "7"));
    assert!(row.has_field("health_before", "100"));
    let hp = mgr.get_entity(2).unwrap().stats.get(HEALTH).unwrap().cur;
    assert!(row.has_field("health", &hp.to_string()));
    assert!(row.has_field("health_damage", &(100 - hp).to_string()));
}

#[tokio::test]
async fn npc_target_logs_no_vitals() {
    let mut mgr = make_mgr_player_vs_npc();
    let ability = make_ability(7, vec![100]);
    mgr.ability_defs.insert(7, ability.clone());
    mgr.effect_defs.insert(100, make_effect(100, 30));
    let (tx, _rx) = mpsc::channel(64);

    let capture = LogCapture::install();
    apply_damage_to_target(1, 2, 7, &Some(ability), 1, false, &tx, &mut mgr).await;

    assert!(
        capture.all().iter().all(|c| c.target != "vitals"),
        "NPC pools are not vitals rows"
    );
}
