//! SS-D2: nothing the duel started outlives it, and nothing else ends it
//! early.
//!
//! - A DoT the partner applied during the duel stops at the end
//!   (`end_engaged` strips the partner's effects), while an effect from
//!   anyone else keeps ticking.
//! - A pet owner stays in combat with the duel opponent while the pet's
//!   own combat bookkeeping runs (`pet::defend::sync_owner_combat` must not
//!   treat the opponent as a stale mob).

use std::collections::HashMap;
use std::time::{Duration, Instant};

use cimmeria_common::Vector3;
use cimmeria_entity::abilities::EffectDef;
use cimmeria_entity::stats::HEALTH;
use cimmeria_wire::state_field::BSF_IN_COMBAT;

use super::*;
use crate::cell::duel::{end_engaged, EndReason};
use crate::cell::effects::{effect_pulse_tick, register_active_effect};
use crate::test_support::{
    add_pet_owner, make_pet_world, LogCapture, NoContentEvents, PET_FIXTURE_TEMPLATE_ID,
};

const A: u32 = 1;
const B: u32 = 2;
const C: u32 = 3;
const FULL: i32 = 10_000;

/// A and B dueling (engaged) in Agnos, C a bystander; every player at full
/// health.
fn dueling_world() -> SpaceManager {
    let mut mgr = make_pet_world();
    for (eid, pos) in [
        (A, [0.0, 0.0, 0.0]),
        (B, [5.0, 0.0, 0.0]),
        (C, [0.0, 0.0, 3.0]),
    ] {
        add_pet_owner(&mut mgr, eid, "Agnos", pos, 10);
        let hp = mgr
            .get_entity_mut(eid)
            .unwrap()
            .stats
            .get_mut(HEALTH)
            .unwrap();
        hp.update(0, FULL, FULL);
        hp.clear_dirty();
    }
    let _ = mgr.compute_aoi_changes();
    let (a, b) = (pid(&mgr, A), pid(&mgr, B));
    let now = Instant::now();
    mgr.duels.open_challenge(a, b, now).unwrap();
    let space = mgr.get_entity_space_id(A).expect("A has a space");
    let p = mgr.duels.take_pending_for(b, now).unwrap();
    let duel = mgr
        .duels
        .start_duel(&p, space, Vector3::new(2.5, 0.0, 0.0), now);
    mgr.duels.engage(duel.duel_id, [A, B], now).unwrap();
    mgr
}

fn pid(mgr: &SpaceManager, eid: u32) -> i32 {
    mgr.get_entity(eid).unwrap().player_id.unwrap()
}

fn health(mgr: &SpaceManager, eid: u32) -> i32 {
    mgr.get_entity(eid).unwrap().stats.get(HEALTH).unwrap().cur
}

fn dot(effect_id: i32) -> EffectDef {
    let mut params = HashMap::new();
    params.insert("HealthDamage".to_string(), "10".to_string());
    EffectDef {
        effect_id,
        ability_id: 1234,
        pulse_count: 5,
        pulse_duration: 1.0,
        params,
        ..Default::default()
    }
}

/// Make every pulse on `eid` due now.
fn make_due(mgr: &mut SpaceManager, eid: u32) {
    let past = Instant::now() - Duration::from_secs(2);
    for inst in &mut mgr.get_entity_mut(eid).unwrap().active_effects {
        inst.next_pulse_at = past;
    }
}

/// A partner's DoT lands while engaged and stops at the end; a bystander's
/// DoT on the same duelist keeps ticking.
#[tokio::test]
async fn partner_dot_stops_when_the_duel_ends() {
    let logs = LogCapture::install();
    let mut mgr = dueling_world();
    let (partner_dot, other_dot) = (dot(7001), dot(7002));
    mgr.effect_defs.insert(7001, partner_dot.clone());
    mgr.effect_defs.insert(7002, other_dot.clone());
    let (tx, mut rx) = mpsc::channel(4096);
    let now = Instant::now();
    register_active_effect(&mut mgr, B, A, &partner_dot, now, &tx).await;
    register_active_effect(&mut mgr, B, C, &other_dot, now, &tx).await;

    // Engaged: both DoTs pulse.
    make_due(&mut mgr, B);
    let before = health(&mgr, B);
    effect_pulse_tick(&NoContentEvents, &tx, &mut mgr).await;
    assert_eq!(
        health(&mgr, B),
        before - 20,
        "both DoTs pulsed while engaged"
    );
    drain(&mut rx);

    // The end strips the partner's DoT, with the zero timer for its icon.
    let duel_id = mgr.duels.duel_of(pid(&mgr, A)).unwrap().duel_id;
    end_engaged(&tx, &mut mgr, duel_id, EndReason::GmAborted).await;
    let msgs = drain(&mut rx);
    let invokers: Vec<u32> = mgr
        .get_entity(B)
        .unwrap()
        .active_effects
        .iter()
        .map(|i| i.invoker_id)
        .collect();
    assert_eq!(invokers, vec![C], "only the bystander's effect remains");
    let zero_timer = cimmeria_entity::abilities::serialize_timer_update(
        7001,
        cimmeria_entity::abilities::TIMER_DURATION_EFFECT,
        A as i32,
        7001,
        0.0,
        0.0,
    );
    assert!(
        msgs.iter().any(|m| matches!(m, CellToBaseMsg::EntityMethodCall { entity_id: B, method_index: 12, args } if *args == zero_timer)),
        "the partner DoT's icon is cleared"
    );
    let row = logs
        .all()
        .into_iter()
        .find(|c| c.has_field("event", "duel.ended"))
        .expect("duel.ended");
    assert!(row.has_field("target_effects_removed", "1"), "{row:?}");

    // After the end: only the bystander's DoT lands.
    make_due(&mut mgr, B);
    let before = health(&mgr, B);
    effect_pulse_tick(&NoContentEvents, &tx, &mut mgr).await;
    assert_eq!(
        health(&mgr, B),
        before - 10,
        "a partner pulse landed after the end"
    );
}

/// B owns a pet. The pet's combat bookkeeping runs on the AI tick; B stays
/// in combat with the duel opponent A the whole time.
#[tokio::test]
async fn pet_owner_stays_in_combat_during_a_duel() {
    let mut mgr = dueling_world();
    let pet = mgr
        .spawn_pet_from_template(B, PET_FIXTURE_TEMPLATE_ID, 0)
        .expect("B's pet spawns");
    let _ = mgr.compute_aoi_changes();
    // The engage's combat source, as `duel::engage` sets it.
    {
        let b = mgr.get_entity_mut(B).unwrap();
        b.threatened_mobs.insert(A);
        b.state_field |= BSF_IN_COMBAT;
    }
    assert!(mgr.get_entity(pet).is_some());

    let (tx, _rx) = mpsc::channel(4096);
    for _ in 0..3 {
        crate::cell::service::npc_ai::npc_ai_tick_for_test(&tx, &mut mgr, &NoContentEvents).await;
    }
    let b = mgr.get_entity(B).unwrap();
    assert!(
        b.threatened_mobs.contains(&A),
        "the pet sweep dropped the duel opponent as a stale mob"
    );
    assert_ne!(b.state_field & BSF_IN_COMBAT, 0, "B left combat mid-duel");
}
