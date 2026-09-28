//! The pulse tick: who a pulse hits, whose damage it is, the lifetime, and
//! the owner-lifecycle cleanup.

use std::time::{Duration, Instant};

use cimmeria_cell_world::test_fixtures::{RecordedContentEvent, RecordingContentEvents};
use cimmeria_wire::state_field::BSF_DEAD;

use super::super::tick::pulse_targets;
use super::super::{deployable_tick, deployable_tick_at};
use super::*;

/// The emitter is at `SPOT` (15, 0, 10); its radius is 8 m.
const IN_RANGE: [f32; 3] = [20.0, 0.0, 10.0];
/// Exactly on the 8 m edge.
const ON_EDGE: [f32; 3] = [23.0, 0.0, 10.0];
/// Just past it.
const PAST_EDGE: [f32; 3] = [23.01, 0.0, 10.0];

/// The time the `n`th pulse of `deployable` is due.
fn pulse_due(mgr: &SpaceManager, deployable: u32, n: u32) -> Instant {
    let s = mgr.deployables.get(deployable).unwrap();
    s.spawned_at + s.pulse_interval * n
}

/// **Targeting.** A pulse hits the hostile NPCs within 8 m (the edge
/// included), nearest first, and nothing else: not an NPC just past the
/// edge, a friendly NPC, a dead one, another player, the owner, or the
/// object itself.
#[tokio::test]
async fn a_pulse_targets_live_hostile_npcs_in_its_radius_only() {
    let mut mgr = deploy_mgr();
    let (tx, _rx) = mpsc::channel(512);
    let d = deploy_at(&mut mgr, SPOT, &tx).await;

    hostile(&mut mgr, 50, IN_RANGE);
    hostile(&mut mgr, 51, ON_EDGE);
    hostile(&mut mgr, 52, PAST_EDGE);
    npc(&mut mgr, 53, [16.0, 0.0, 10.0], 1); // friendly, beside the object
    hostile(&mut mgr, 54, [17.0, 0.0, 10.0]);
    mgr.get_entity_mut(54).unwrap().state_field |= BSF_DEAD;
    add_pet_owner(&mut mgr, 2, "Castle", [15.0, 0.0, 11.0], 10); // a bystander player
    mgr.get_entity_mut(2).unwrap().faction = HOSTILE_FACTION;

    assert_eq!(pulse_targets(&mgr, d), vec![50, 51]);
}

/// **Attribution.** A pulse's damage is the owner's: the target's Focus
/// drops, its threat list names the owner, and the owner is in combat with
/// it, though the owner stands 10 m from the object.
#[tokio::test]
async fn a_pulse_damages_as_the_owner() {
    let mut mgr = deploy_mgr();
    let (tx, mut rx) = mpsc::channel(512);
    let d = deploy_at(&mut mgr, SPOT, &tx).await;
    hostile(&mut mgr, 50, IN_RANGE);
    drain(&mut rx);

    assert_eq!(
        deployable_tick_at(pulse_due(&mgr, d, 1), &tx, &mut mgr, &NoContentEvents).await,
        1
    );
    let mob = mgr.get_entity(50).unwrap();
    assert!(
        mob.stats.get(FOCUS).unwrap().cur < 200,
        "the pulse drained Focus (5066 is -100F)"
    );
    assert!(
        mob.threat_list.contains_key(&OWNER),
        "the mob hates the owner"
    );
    assert!(
        mgr.get_entity(OWNER).unwrap().threatened_mobs.contains(&50),
        "the owner is in combat with it"
    );
    let effect_results_from_owner = calls(&drain(&mut rx)).into_iter().any(|(e, m, a)| {
        e == OWNER
            && m == method_idx::ON_EFFECT_RESULTS
            && i32::from_le_bytes([a[0], a[1], a[2], a[3]]) == OWNER as i32
    });
    assert!(
        effect_results_from_owner,
        "onEffectResults names the owner as the source"
    );
    let s = mgr.deployables.get(d).unwrap();
    assert_eq!((s.totals.pulses, s.totals.hits), (1, 1));
    assert!(s.totals.focus_damage > 0);
}

/// Focus goes first, then the overflow bleeds into Health (the
/// `RangedPhysicalDamage` script on 5066): a target with no Focus loses
/// Health on the next pulse.
#[tokio::test]
async fn once_focus_is_gone_the_pulse_bleeds_health() {
    let mut mgr = deploy_mgr();
    let (tx, _rx) = mpsc::channel(512);
    let d = deploy_at(&mut mgr, SPOT, &tx).await;
    hostile(&mut mgr, 50, IN_RANGE);
    mgr.get_entity_mut(50)
        .unwrap()
        .stats
        .get_mut(FOCUS)
        .unwrap()
        .update(0, 0, 200);

    deployable_tick_at(pulse_due(&mgr, d, 1), &tx, &mut mgr, &NoContentEvents).await;
    assert!(mgr.get_entity(50).unwrap().stats.get(HEALTH).unwrap().cur < FULL);
    assert!(mgr.deployables.get(d).unwrap().totals.health_damage > 0);
}

/// **Lifetime.** Nothing between pulses; exactly 30 pulses, one a second;
/// the 30th removes the object in the same tick, and every witness sees it
/// go.
#[tokio::test]
async fn thirty_pulses_then_the_object_is_removed() {
    let mut mgr = deploy_mgr();
    let (tx, mut rx) = mpsc::channel(4096);
    let d = deploy_at(&mut mgr, SPOT, &tx).await;
    hostile(&mut mgr, 50, IN_RANGE);
    let _ = mgr.compute_aoi_changes();
    drain(&mut rx);

    let half_way = pulse_due(&mgr, d, 1) - Duration::from_millis(500);
    assert_eq!(
        deployable_tick_at(half_way, &tx, &mut mgr, &NoContentEvents).await,
        0,
        "no pulse before the first interval"
    );
    let mut fired = 0;
    for n in 1..=30 {
        let at = pulse_due(&mgr, d, n);
        fired += deployable_tick_at(at, &tx, &mut mgr, &NoContentEvents).await;
        if n < 30 {
            assert!(
                mgr.get_entity(d).is_some(),
                "still standing after pulse {n}"
            );
        }
    }
    assert_eq!(fired, 30);
    assert!(mgr.get_entity(d).is_none(), "gone after the 30th pulse");
    assert!(mgr.deployables.is_empty());
    let left = drain(&mut rx).into_iter().any(|m| {
        matches!(m, CellToBaseMsg::LeftAoI { witness_id, entity_id } if witness_id == OWNER && entity_id == d)
    });
    assert!(left, "the owner sees it go");
    assert_eq!(
        deployable_tick_at(
            Instant::now() + Duration::from_secs(60),
            &tx,
            &mut mgr,
            &NoContentEvents
        )
        .await,
        0
    );
}

/// **Owner death.** The object goes on the next tick and never pulses
/// again, even with a pulse due.
#[tokio::test]
async fn the_owners_death_removes_the_object_before_it_pulses() {
    let mut mgr = deploy_mgr();
    let (tx, _rx) = mpsc::channel(512);
    let d = deploy_at(&mut mgr, SPOT, &tx).await;
    hostile(&mut mgr, 50, IN_RANGE);
    mgr.get_entity_mut(OWNER).unwrap().state_field |= BSF_DEAD;

    assert_eq!(
        deployable_tick_at(pulse_due(&mgr, d, 1), &tx, &mut mgr, &NoContentEvents).await,
        0,
        "a dead owner's object does not pulse"
    );
    assert!(mgr.get_entity(d).is_none());
    assert_eq!(
        mgr.get_entity(50).unwrap().stats.get(FOCUS).unwrap().cur,
        200
    );
}

/// **Owner logout.** The owner's entity is destroyed (every logout, gate
/// travel and zone change ends that way): the object goes on the next tick,
/// through the real per-tick entry point.
#[tokio::test]
async fn the_owners_logout_removes_the_object() {
    let mut mgr = deploy_mgr();
    let (tx, _rx) = mpsc::channel(512);
    let d = deploy_at(&mut mgr, SPOT, &tx).await;
    mgr.destroy_entity(OWNER);

    deployable_tick(&tx, &mut mgr, &NoContentEvents).await;
    assert!(mgr.get_entity(d).is_none());
    assert!(mgr.deployables.is_empty());
}

/// **Kill credit.** A pulse that kills a tagged mob credits the owner: the
/// mob is a corpse, the owner is paid kill XP, and the mission
/// `EntityDeath` names the owner.
#[tokio::test]
async fn a_pulse_kill_is_the_owners_kill() {
    let mut mgr = deploy_mgr();
    let (tx, mut rx) = mpsc::channel(512);
    let d = deploy_at(&mut mgr, SPOT, &tx).await;
    hostile(&mut mgr, 50, IN_RANGE);
    {
        let mob = mgr.get_entity_mut(50).unwrap();
        mob.tag = Some("emitter_target".to_string());
        mob.stats.get_mut(FOCUS).unwrap().update(0, 0, 200);
        mob.stats.get_mut(HEALTH).unwrap().update(0, 1, FULL);
    }
    drain(&mut rx);

    let events = RecordingContentEvents::new();
    deployable_tick_at(pulse_due(&mgr, d, 1), &tx, &mut mgr, &events).await;

    assert_ne!(
        mgr.get_entity(50).unwrap().state_field & BSF_DEAD,
        0,
        "a corpse"
    );
    let xp_to_owner = drain(&mut rx).into_iter().any(|m| {
        matches!(m, CellToBaseMsg::GrantXP { entity_id, xp_amount, .. } if entity_id == OWNER && xp_amount > 0)
    });
    assert!(xp_to_owner, "the owner is paid the kill XP");
    let credited = events.events().into_iter().any(|e| {
        matches!(e, RecordedContentEvent::EntityDeath { killer_entity_id, ref entity_tag, .. }
            if killer_entity_id == OWNER && entity_tag == "emitter_target")
    });
    assert!(credited, "mission credit is the owner's");
    assert_eq!(mgr.deployables.get(d).unwrap().totals.kills, 1);
}

/// The lifetime effect never reaches a target: a pulse registers no
/// 30-pulse "Pulser" instance on the mob it hits.
#[tokio::test]
async fn a_pulse_registers_no_lifetime_effect_on_its_target() {
    let mut mgr = deploy_mgr();
    let (tx, _rx) = mpsc::channel(512);
    let d = deploy_at(&mut mgr, SPOT, &tx).await;
    hostile(&mut mgr, 50, IN_RANGE);

    deployable_tick_at(pulse_due(&mgr, d, 1), &tx, &mut mgr, &NoContentEvents).await;
    assert!(
        mgr.get_entity(50).unwrap().active_effects.is_empty(),
        "no active effect on the target: {:?}",
        mgr.get_entity(50).unwrap().active_effects
    );
}
