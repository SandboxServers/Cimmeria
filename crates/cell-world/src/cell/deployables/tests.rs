//! Deployables Phase 0, world half: the spawn, the registry, the lifetime
//! verdict and the observer-visible despawn.

use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use cimmeria_common::Vector3;
use cimmeria_entity::abilities::EffectDef;
use cimmeria_wire::state_field::BSF_DEAD;

use super::*;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{DespawnOutcome, SpaceManager};
use crate::mercury::aoi::compose_create_entity_cascade_body;
use crate::mercury::write_wstring;
use crate::test_fixtures::{
    add_pet_owner, drain_left_aoi_for, make_pet_world, seed_deployable, DEPLOYABLE_ABILITY,
    DEPLOYABLE_LIFETIME_EFFECT, DEPLOYABLE_SPEC, DEPLOYABLE_TEMPLATE,
};

const OWNER: u32 = 7;
const OTHER: u32 = 8;
/// The owner's faction, distinct from the template's placeholder 1, so the
/// copy is observable.
const OWNER_FACTION: u8 = 3;
const SPOT: Vector3 = Vector3 {
    x: 15.0,
    y: 0.0,
    z: 15.0,
};

/// Agnos with `OWNER` (level 12, faction 3) standing at (10, 0, 10) and the
/// 1012 fixture cached.
fn world() -> SpaceManager {
    let mut mgr = make_pet_world();
    add_pet_owner(&mut mgr, OWNER, "Agnos", [10.0, 0.0, 10.0], 12);
    mgr.get_entity_mut(OWNER).unwrap().faction = OWNER_FACTION;
    seed_deployable(&mut mgr);
    mgr
}

fn place(mgr: &mut SpaceManager, now: Instant) -> u32 {
    mgr.spawn_deployable(OWNER, DEPLOYABLE_SPEC, SPOT, 0.5, now)
        .expect("the emitter is placed")
}

/// The spawn builds a stationary `SGWBeing` at the point, in the owner's
/// faction, with everything a placed NPC carries stripped, and registers it
/// with the schedule the cooked effects give: 30 pulses of 1 s (5065) in
/// an 8 m radius (5066 "Medium").
#[test]
fn spawn_places_a_stationary_being_owned_by_the_caster() {
    let mut mgr = world();
    let now = Instant::now();
    let id = place(&mut mgr, now);

    let e = mgr.get_entity(id).expect("the entity exists");
    assert_eq!(e.class_id, 0x01, "an SGWBeing: no AoE list, no fight pass");
    assert!(!e.is_player);
    assert_eq!(
        e.position, SPOT,
        "at the validated point, not snapped again"
    );
    assert!(
        (e.direction.y - 0.5).abs() < 1e-6,
        "faces the caster's heading"
    );
    assert!(e.is_stationary);
    assert_eq!(
        e.faction, OWNER_FACTION,
        "the owner's faction, not the template's"
    );
    assert_eq!(e.template_id, Some(DEPLOYABLE_TEMPLATE));
    assert_eq!(e.body_set.as_deref(), Some("WP-Human.BS_DeployableLow"));
    assert_eq!(e.components, vec!["WP-Human.Dp_Standard100".to_string()]);
    assert_eq!(e.loot_table_id, None);
    assert_eq!(e.respawn_secs, None);
    assert!(e.patrol_path.is_empty());
    assert_eq!(e.wander_radius, 0.0);
    assert_eq!(e.tag, None);
    assert_eq!(e.spawn_id, None);
    assert_eq!(mgr.get_entity_space_id(id), mgr.get_entity_space_id(OWNER));

    let s = mgr.deployables.get(id).expect("registered");
    assert_eq!(s.owner, OWNER);
    assert_eq!(s.owner_identity, mgr.player_identity(OWNER));
    assert_eq!(s.ability_id, DEPLOYABLE_ABILITY);
    assert_eq!(s.pulses_total, 30);
    assert_eq!(s.pulse_interval, Duration::from_secs(1));
    assert_eq!(s.radius, 8.0);
    assert_eq!(s.next_pulse_at, now + Duration::from_secs(1));
    assert_eq!(s.totals, PulseTotals::default());
}

/// A spawn that cannot happen says why and places nothing.
#[test]
fn spawn_refusals_carry_their_reason_and_place_nothing() {
    let mut mgr = world();
    let now = Instant::now();

    let mut no_template = DEPLOYABLE_SPEC;
    no_template.template_id = 409;
    let e = mgr
        .spawn_deployable(OWNER, no_template, SPOT, 0.0, now)
        .unwrap_err();
    assert_eq!(e.reason(), "unknown_template");

    mgr.effect_defs
        .get_mut(&DEPLOYABLE_LIFETIME_EFFECT)
        .unwrap()
        .pulse_count = 0;
    let e = mgr
        .spawn_deployable(OWNER, DEPLOYABLE_SPEC, SPOT, 0.0, now)
        .unwrap_err();
    assert_eq!(e.reason(), "bad_lifetime_effect");

    let npc = mgr.allocate_npc_id();
    mgr.spawn_npc(npc, "Agnos", [1.0, 0.0, 1.0], [0.0; 3])
        .unwrap();
    let e = mgr
        .spawn_deployable(npc, DEPLOYABLE_SPEC, SPOT, 0.0, now)
        .unwrap_err();
    assert_eq!(e.reason(), "owner_not_player");

    assert!(mgr.deployables.is_empty(), "nothing registered");
}

/// The schedule: nothing before the interval, a pulse at it, and once the
/// last pulse has run the object expires.
#[test]
fn verdict_holds_then_pulses_on_the_interval_then_expires() {
    let mut mgr = world();
    let now = Instant::now();
    let id = place(&mut mgr, now);

    assert_eq!(deployable_verdict(&mgr, id, now), DeployableVerdict::Hold);
    assert_eq!(
        deployable_verdict(&mgr, id, now + Duration::from_millis(999)),
        DeployableVerdict::Hold
    );
    assert_eq!(
        deployable_verdict(&mgr, id, now + Duration::from_secs(1)),
        DeployableVerdict::Pulse
    );

    mgr.deployables.get_mut(id).unwrap().totals.pulses = 29;
    assert_eq!(
        deployable_verdict(&mgr, id, now + Duration::from_secs(1)),
        DeployableVerdict::Pulse,
        "29 of 30 pulses: one to go"
    );
    mgr.deployables.get_mut(id).unwrap().totals.pulses = 30;
    assert_eq!(
        deployable_verdict(&mgr, id, now),
        DeployableVerdict::Despawn(DeployableDespawnReason::Expired)
    );
}

/// Owner death, logout and zone change each end the object, and each wins
/// over a pulse that is due.
#[test]
fn verdict_ends_the_object_on_owner_death_logout_and_zone_change() {
    let due = Instant::now() + Duration::from_secs(5);

    let mut mgr = world();
    let id = place(&mut mgr, Instant::now());
    mgr.get_entity_mut(OWNER).unwrap().state_field |= BSF_DEAD;
    assert_eq!(
        deployable_verdict(&mgr, id, due),
        DeployableVerdict::Despawn(DeployableDespawnReason::OwnerDead)
    );

    let mut mgr = world();
    let id = place(&mut mgr, Instant::now());
    mgr.destroy_entity(OWNER);
    assert_eq!(
        deployable_verdict(&mgr, id, due),
        DeployableVerdict::Despawn(DeployableDespawnReason::OwnerGone)
    );

    let mut mgr = world();
    let id = place(&mut mgr, Instant::now());
    let identity = mgr.player_identity(OWNER);
    mgr.destroy_entity(OWNER);
    add_pet_owner(&mut mgr, OWNER, "Castle", [10.0, 0.0, 10.0], 12);
    let owner = mgr.get_entity_mut(OWNER).unwrap();
    owner.account_id = identity.account_id;
    owner.player_id = identity.player_id;
    assert_eq!(
        deployable_verdict(&mgr, id, due),
        DeployableVerdict::Despawn(DeployableDespawnReason::OwnerLeftSpace)
    );
}

/// Entity ids are reused: a different player given the owner's id does not
/// inherit the object.
#[test]
fn verdict_refuses_a_player_who_reused_the_owner_id() {
    let mut mgr = world();
    let id = place(&mut mgr, Instant::now());
    mgr.destroy_entity(OWNER);
    add_pet_owner(&mut mgr, OWNER, "Agnos", [10.0, 0.0, 10.0], 12);
    let impostor = mgr.get_entity_mut(OWNER).unwrap();
    impostor.account_id = Some(4242);
    impostor.player_id = Some(4243);
    assert_eq!(
        deployable_verdict(&mgr, id, Instant::now()),
        DeployableVerdict::Despawn(DeployableDespawnReason::OwnerGone)
    );
}

/// Fan-out, both ends. Every player in range meets the object as an
/// SGWBeing whose cascade carries `BeingAppearance` with the deployable
/// body, the owner included; the despawn sends every one of them a
/// `LeftAoI` and leaves nothing behind.
#[tokio::test]
async fn observers_meet_the_object_with_its_body_and_see_it_leave() {
    let mut mgr = world();
    add_pet_owner(&mut mgr, OTHER, "Agnos", [12.0, 0.0, 12.0], 5);
    let id = place(&mut mgr, Instant::now());

    let events = mgr.compute_aoi_changes();
    for witness in [OWNER, OTHER] {
        let (class_id, data) = events
            .iter()
            .find_map(|e| match e {
                CellToBaseMsg::EnteredAoI {
                    witness_id,
                    entity_id,
                    class_id,
                    npc_data,
                    ..
                } if *witness_id == witness && *entity_id == id => {
                    Some((*class_id, npc_data.clone()))
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("witness {witness} must meet the deployable"));
        assert_eq!(class_id, 0x01);
        let data = data.expect("NPC create data");
        assert_eq!(data.body_set.as_deref(), Some("WP-Human.BS_DeployableLow"));
        assert_eq!(data.components, vec!["WP-Human.Dp_Standard100".to_string()]);
        assert_eq!(data.faction, OWNER_FACTION);
        assert_eq!(data.pet_owner_id, None, "never bound into the pet bar");
        let cascade = compose_create_entity_cascade_body(id, class_id, 1, Some(&data));
        // BeingAppearance's arguments, byte-exact: the body set, then the
        // one-entry component list.
        let mut appearance = Vec::new();
        write_wstring(&mut appearance, "WP-Human.BS_DeployableLow");
        appearance.extend_from_slice(&1u32.to_le_bytes());
        write_wstring(&mut appearance, "WP-Human.Dp_Standard100");
        assert!(
            cascade
                .windows(appearance.len())
                .any(|w| w == appearance.as_slice()),
            "the cascade carries BeingAppearance(body set, [component])"
        );
    }

    let (tx, mut rx) = mpsc::channel(64);
    let outcome =
        despawn_deployable(&mut mgr, id, DeployableDespawnReason::Expired, "sweep", &tx).await;
    assert!(matches!(
        outcome,
        DespawnOutcome::Despawned {
            witnesses_notified: 2
        }
    ));
    assert_eq!(drain_left_aoi_for(&mut rx, id), vec![OWNER, OTHER]);
    assert!(mgr.get_entity(id).is_none());
    assert!(mgr.deployables.is_empty());
    for p in [OWNER, OTHER] {
        assert!(!mgr
            .get_entity(p)
            .unwrap()
            .witnesses
            .contains(&cimmeria_common::EntityId(id as i32)));
    }
}

/// A departing owner's client is being reset: it gets no `LeftAoI` (it
/// would reach its next world), every other witness does. An expiry tells
/// the owner too.
#[tokio::test]
async fn a_departing_owner_is_spared_the_leave_other_witnesses_are_not() {
    let mut mgr = world();
    add_pet_owner(&mut mgr, OTHER, "Agnos", [12.0, 0.0, 12.0], 5);
    let id = place(&mut mgr, Instant::now());
    let _ = mgr.compute_aoi_changes();

    let (tx, mut rx) = mpsc::channel(64);
    let _ = despawn_deployable(
        &mut mgr,
        id,
        DeployableDespawnReason::OwnerLeftSpace,
        "sweep",
        &tx,
    )
    .await;
    assert_eq!(drain_left_aoi_for(&mut rx, id), vec![OTHER]);

    let id = place(&mut mgr, Instant::now());
    let _ = mgr.compute_aoi_changes();
    let _ = despawn_deployable(
        &mut mgr,
        id,
        DeployableDespawnReason::OwnerDead,
        "sweep",
        &tx,
    )
    .await;
    assert_eq!(drain_left_aoi_for(&mut rx, id), vec![OWNER, OTHER]);
}

/// `destroy_entity` and `destroy_space` drop the registry entry, so no
/// path that removes the object leaves an orphan behind.
#[test]
fn every_removal_path_scrubs_the_registry() {
    let mut mgr = world();
    let id = place(&mut mgr, Instant::now());
    mgr.destroy_entity(id);
    assert!(mgr.deployables.get(id).is_none());

    let mut mgr = make_pet_world();
    add_pet_owner(&mut mgr, OWNER, "Castle_CellBlock", [10.0, 0.0, 10.0], 12);
    seed_deployable(&mut mgr);
    let id = place(&mut mgr, Instant::now());
    let space = mgr.get_entity_space_id(id).unwrap();
    mgr.destroy_space(space);
    assert!(mgr.deployables.is_empty());
}

/// A staged point is only ever used by the ability it was validated for.
#[test]
fn a_staged_point_is_taken_once_and_only_by_its_ability() {
    let mut reg = DeployableRegistry::default();
    reg.stage(OWNER, DEPLOYABLE_ABILITY, SPOT);
    assert_eq!(reg.staged_for(OWNER, DEPLOYABLE_ABILITY), Some(SPOT));
    assert_eq!(reg.staged_for(OWNER, 1236), None);
    assert_eq!(reg.take_staged(OWNER, DEPLOYABLE_ABILITY), Some(SPOT));
    assert_eq!(
        reg.take_staged(OWNER, DEPLOYABLE_ABILITY),
        None,
        "taken once"
    );

    reg.stage(OWNER, DEPLOYABLE_ABILITY, SPOT);
    assert_eq!(
        reg.take_staged(OWNER, 1236),
        None,
        "another ability never uses it"
    );
    assert_eq!(
        reg.staged_for(OWNER, DEPLOYABLE_ABILITY),
        None,
        "and the mismatched take drops it"
    );
}

/// The radius is the `Radius` NVP when set, else the range tier; a
/// lifetime effect with no pulses or no interval gives no schedule.
#[test]
fn radius_and_schedule_come_from_the_effects() {
    let mut e = EffectDef {
        tcm_param1: "Medium".to_string(),
        ..Default::default()
    };
    assert_eq!(pulse_radius(&e), 8.0);
    e.params.insert("Radius".to_string(), "12.5".to_string());
    assert_eq!(pulse_radius(&e), 12.5);

    let mut life = EffectDef {
        pulse_count: 30,
        pulse_duration: 1.0,
        ..Default::default()
    };
    assert_eq!(pulse_schedule(&life), Some((30, Duration::from_secs(1))));
    life.pulse_duration = 0.0;
    assert_eq!(pulse_schedule(&life), None);
    life.pulse_duration = 1.0;
    life.pulse_count = 0;
    assert_eq!(pulse_schedule(&life), None);
}
