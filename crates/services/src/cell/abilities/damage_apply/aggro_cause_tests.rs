//! NA00 review: the damage path is the one real caller of
//! `AggroCause::Damage`. Driven through `apply_damage_to_target` (not
//! `combat::generate_threat` directly), a hit on an Idle NPC must log
//! `cause=damage` and a `threat_preempt` transition.

use super::tests::{make_ability, make_mgr_player_vs_npc};
use super::*;
use crate::test_support::LogCapture;

#[tokio::test]
async fn a_hit_on_an_idle_npc_logs_the_damage_cause() {
    let mut mgr = make_mgr_player_vs_npc();
    if let Some(npc) = mgr.get_entity_mut(2) {
        npc.class_id = 0x04;
        if let Some(h) = npc.stats.get_mut(cimmeria_entity::stats::HEALTH) {
            h.update(0, 100, 100);
            h.clear_dirty();
        }
    }
    let ability = make_ability(579, vec![]);
    mgr.ability_defs.insert(579, ability.clone());
    let (tx, _rx) = mpsc::channel(64);
    let logs = LogCapture::install();

    apply_damage_to_target(1, 2, 579, &Some(ability), 1, false, &tx, &mut mgr).await;

    let all = logs.all();
    let acquired = all
        .iter()
        .find(|c| c.target == "npc_ai.aggro")
        .expect("the hit must put the NPC into Fighting and log npc_ai.aggro");
    assert!(acquired.has_field("cause", "damage"), "{acquired:?}");
    let transition = all
        .iter()
        .find(|c| c.target == "npc_ai.transition")
        .expect("and a transition row");
    assert!(
        transition.has_field("reason", "threat_preempt"),
        "{transition:?}"
    );
}
