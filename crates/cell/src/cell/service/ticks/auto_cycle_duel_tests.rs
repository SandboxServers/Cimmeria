//! `auto_cycle_tick` and duels (SS-D2): a loop aimed at a player keeps
//! firing only while that player is the caster's engaged duel partner.
//! Once the duel ends, the loop stops with a feedback line instead of
//! re-firing into the #444 gate every tick.

use cimmeria_cell_world::cell::duel::DuelResources;
use std::time::Instant;

use cimmeria_common::Vector3;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::abilities::AbilityDef;
use cimmeria_wire::state_field::BSF_AUTO_CYCLING;

use super::*;
use crate::test_support::LogCapture;

const A: u32 = 1;
const B: u32 = 2;
const A_PID: i32 = 100;
const B_PID: i32 = 200;

/// A (armed, auto-cycling ability 7 at B) and B, two connected players in
/// Castle, five units apart, engaged in a duel.
fn duel_loop_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    for (eid, pid, x) in [(A, A_PID, 0.0), (B, B_PID, 5.0)] {
        mgr.create_entity(eid, "Castle", [x, 0.0, 0.0], [0.0; 3])
            .unwrap();
        mgr.connect_entity(eid);
        let e = mgr.get_entity_mut(eid).unwrap();
        e.player_id = Some(pid);
        e.account_id = Some(eid);
        e.archetype_id = Some(1);
    }
    {
        let a = mgr.get_entity_mut(A).unwrap();
        a.abilities.add_ability(7);
        a.abilities.auto_cycle = true;
        a.abilities.auto_cycle_ability_id = Some(7);
        a.current_target_id = Some(B as i32);
        a.weapon_holstered = false;
        a.state_field |= BSF_AUTO_CYCLING;
    }
    let _ = mgr.compute_aoi_changes();
    crate::test_support::seed_mechanic_effect(&mut mgr);
    mgr.ability_defs.insert(
        7,
        AbilityDef {
            ability_id: 7,
            name: "test".to_string(),
            cooldown: 0.5,
            warmup: 0.0,
            flags: 0,
            is_ranged: false,
            min_range: 0.0,
            max_range: 30.0,
            target_type_id: 0,
            effect_ids: vec![crate::test_support::MECHANIC_FIXTURE_EFFECT],
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id: None,
            velocity: 0.0,
            type_id: Default::default(),
            passive: false,
        },
    );
    let now = Instant::now();
    mgr.resources
        .duels_mut()
        .open_challenge(A_PID, B_PID, now)
        .unwrap();
    let space = mgr.get_entity_space_id(A).expect("A has a space");
    let p = mgr
        .resources
        .duels_mut()
        .take_pending_for(B_PID, now)
        .unwrap();
    let duel = mgr
        .resources
        .duels_mut()
        .start_duel(&p, space, Vector3::new(2.5, 0.0, 0.0), now);
    mgr.resources
        .duels_mut()
        .engage(duel.duel_id, [A, B], now)
        .unwrap();
    mgr
}

/// Feedback lines (method 28) sent to `entity`.
fn lines_to(msgs: &[CellToBaseMsg], entity: u32) -> usize {
    msgs.iter()
        .filter(|m| {
            matches!(m, CellToBaseMsg::EntityMethodCall { entity_id, method_index: 28, .. } if *entity_id == entity)
        })
        .count()
}

#[tokio::test]
async fn auto_cycle_on_the_partner_fires_during_the_duel_and_stops_after_it() {
    let logs = LogCapture::install();
    let mut mgr = duel_loop_mgr();
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(1024);

    // Engaged: the loop fires at the partner (the cooldown starts).
    auto_cycle_tick(&tx, &mut mgr, &engine).await;
    assert!(
        mgr.get_entity(A).unwrap().abilities.is_on_cooldown(7),
        "the loop fired at the duel partner"
    );
    assert!(mgr.get_entity(A).unwrap().abilities.auto_cycle);
    while rx.try_recv().is_ok() {}

    // The duel ends: the next tick clears the loop, un-lights the button,
    // and tells the player once, even mid-cooldown.
    let duel_id = mgr.resources.duels().duel_of(A_PID).unwrap().duel_id;
    mgr.resources.duels_mut().end_duel(duel_id);
    auto_cycle_tick(&tx, &mut mgr, &engine).await;
    let mut msgs = Vec::new();
    while let Ok(m) = rx.try_recv() {
        msgs.push(m);
    }
    let a = mgr.get_entity(A).unwrap();
    assert!(!a.abilities.auto_cycle, "the loop is cleared");
    assert_eq!(a.state_field & BSF_AUTO_CYCLING, 0, "the button is un-lit");
    assert!(
        msgs.iter().any(|m| matches!(m, CellToBaseMsg::EntityMethodCall { entity_id: A, method_index, .. } if *method_index == crate::mercury::method_idx::ON_STATE_FIELD_UPDATE)),
        "onStateFieldUpdate to the caster: {msgs:?}"
    );
    assert_eq!(lines_to(&msgs, A), 1, "one feedback line");
    let row = logs
        .all()
        .into_iter()
        .find(|c| c.has_field("event", "duel.auto_cycle_stopped"))
        .expect("the stop row");
    for (k, v) in [
        ("reason", "not_duel_opponent"),
        ("player_id", "100"),
        ("target_player_id", "200"),
    ] {
        assert!(row.has_field(k, v), "{k}={v}: {row:?}");
    }

    // Nothing further: no re-fire into the #444 gate, no second line.
    mgr.get_entity_mut(A)
        .unwrap()
        .abilities
        .clear_all_cooldowns();
    auto_cycle_tick(&tx, &mut mgr, &engine).await;
    let mut more = Vec::new();
    while let Ok(m) = rx.try_recv() {
        more.push(m);
    }
    assert!(more.is_empty(), "the stopped loop sent more: {more:?}");
    assert!(
        logs.all()
            .iter()
            .all(|c| !c.message_contains("forged target")),
        "a #444 WARN fired"
    );
}
