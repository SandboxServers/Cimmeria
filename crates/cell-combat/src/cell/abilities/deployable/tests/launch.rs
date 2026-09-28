//! The ground-point launch and the fire: what is placed, where, when, and
//! every refusal's answer.

use std::time::Instant;

use cimmeria_cell_world::test_fixtures::occluder_fixtures::synthetic;
use cimmeria_cell_world::test_fixtures::{
    test_fixture_mesh, test_insert_navmesh_space, DEPLOYABLE_TEMPLATE,
};
use cimmeria_common::Vector3;

use super::super::feedback::DeployRefusal;
use super::super::launch::validate_ground_point;
use super::*;
use crate::cell::abilities::{handle_use_ability, interrupt_pending_cast, InterruptReason};

/// `onTimerUpdate`.
const ON_TIMER_UPDATE: u16 = 12;

/// **Acceptance.** Pressing 1012 on a ground point answers at once (the
/// cooldown and warmup timers), places nothing during the 2 s warmup, and
/// then places template 400 at the point, owned by the caster.
#[tokio::test]
async fn a_ground_cast_places_the_emitter_at_the_point_after_its_warmup() {
    let mut mgr = deploy_mgr();
    let (tx, mut rx) = mpsc::channel(256);

    handle_use_ability_on_ground(OWNER, DEPLOYABLE_ABILITY, SPOT, &tx, &mut mgr).await;
    let press = drain(&mut rx);
    let timers = calls(&press)
        .into_iter()
        .filter(|(e, m, _)| *e == OWNER && *m == ON_TIMER_UPDATE)
        .count();
    assert_eq!(
        timers, 2,
        "cooldown + warmup timers: feedback on the first press"
    );
    assert!(mgr.deployables.is_empty(), "nothing before the warmup ends");
    assert!(
        mgr.get_entity(OWNER)
            .unwrap()
            .abilities
            .is_on_cooldown(DEPLOYABLE_ABILITY),
        "the cooldown is charged at launch"
    );

    assert_eq!(
        resolve_warmups(after_warmup(), &tx, &mut mgr, &NoContentEvents).await,
        1
    );
    let placed = mgr.deployables.of_owner(OWNER, DEPLOYABLE_ABILITY);
    assert_eq!(placed.len(), 1);
    let e = mgr.get_entity(placed[0]).unwrap();
    assert_eq!(e.template_id, Some(DEPLOYABLE_TEMPLATE));
    assert_eq!(e.position, Vector3::new(SPOT[0], SPOT[1], SPOT[2]));
    assert_eq!(e.class_id, 0x01);
    assert_eq!(
        mgr.deployables.staged_for(OWNER, DEPLOYABLE_ABILITY),
        None,
        "the staged point is consumed by the fire"
    );
}

/// A point beyond 1012's 500 range is refused before anything is charged:
/// `onErrorCode` 42, a chat line, no cooldown, no warmup, nothing staged.
#[tokio::test]
async fn an_out_of_range_point_is_refused_with_feedback_and_charges_nothing() {
    let mut mgr = deploy_mgr();
    let (tx, mut rx) = mpsc::channel(256);

    let far = [OWNER_POS[0] + 501.0, 0.0, OWNER_POS[2]];
    handle_use_ability_on_ground(OWNER, DEPLOYABLE_ABILITY, far, &tx, &mut mgr).await;
    let sent = drain(&mut rx);
    assert_eq!(error_codes(&sent, OWNER, DEPLOYABLE_ABILITY), vec![42]);
    assert!(got_chat(&sent, OWNER, "That spot is out of range."));
    let owner = mgr.get_entity(OWNER).unwrap();
    assert!(!owner.abilities.is_on_cooldown(DEPLOYABLE_ABILITY));
    assert!(owner.pending_cast.is_none());
    assert_eq!(mgr.deployables.staged_for(OWNER, DEPLOYABLE_ABILITY), None);

    // 500 exactly is in range.
    let edge = [OWNER_POS[0] + 500.0, 0.0, OWNER_POS[2]];
    assert!(
        validate_ground_point(&mgr, OWNER, mgr.ability_defs.get(&DEPLOYABLE_ABILITY), edge).is_ok()
    );
}

/// Coordinates a client made up (NaN, infinity) are refused, never placed.
#[tokio::test]
async fn a_non_finite_point_is_refused() {
    let mgr = deploy_mgr();
    for bad in [
        [f32::NAN, 0.0, 0.0],
        [0.0, f32::INFINITY, 0.0],
        [0.0, 0.0, f32::NEG_INFINITY],
    ] {
        assert_eq!(
            validate_ground_point(&mgr, OWNER, mgr.ability_defs.get(&DEPLOYABLE_ABILITY), bad),
            Err(DeployRefusal::NotFinite)
        );
    }
}

/// A wall between the caster's eye and the point refuses it (`onErrorCode`
/// 39); the same point with a clear line is accepted. The synthetic wall is
/// 4 m high on x 19.85-20.15, z 0-20.
#[tokio::test]
async fn a_point_behind_a_wall_is_refused_for_line_of_sight() {
    let mut mgr = deploy_mgr();
    let space = mgr.get_entity_space_id(OWNER).unwrap();
    mgr.spaces.get_mut(&space).unwrap().occluder =
        Some(synthetic(&[([19.85, 0.0, 0.0], [20.15, 4.0, 20.0])]));
    let def = mgr.ability_defs.get(&DEPLOYABLE_ABILITY).cloned();

    let behind = [25.0, 0.0, 10.0];
    assert_eq!(
        validate_ground_point(&mgr, OWNER, def.as_ref(), behind),
        Err(DeployRefusal::NoLineOfSight)
    );
    let clear = [15.0, 0.0, 10.0];
    assert!(validate_ground_point(&mgr, OWNER, def.as_ref(), clear).is_ok());

    let (tx, mut rx) = mpsc::channel(256);
    handle_use_ability_on_ground(OWNER, DEPLOYABLE_ABILITY, behind, &tx, &mut mgr).await;
    let sent = drain(&mut rx);
    assert_eq!(error_codes(&sent, OWNER, DEPLOYABLE_ABILITY), vec![39]);
    assert!(got_chat(&sent, OWNER, "You cannot see that spot."));
    assert!(mgr.get_entity(OWNER).unwrap().pending_cast.is_none());
}

/// In a world that enforces navmesh containment, a point off the mesh is
/// refused; a point on it is snapped to the floor. Uses the shipped Castle
/// Cellblock mesh (skipped when the data file is absent).
#[tokio::test]
async fn an_off_mesh_point_is_refused_and_an_on_mesh_point_is_grounded() {
    let Some(mesh) = test_fixture_mesh() else {
        eprintln!("SKIPPED: castle_cellblock.nav is absent");
        return;
    };
    let mut mgr = SpaceManager::new(1);
    test_insert_navmesh_space(&mut mgr, "Castle_CellBlock", mesh);
    // A guard spawn the fixture mesh accepts (the arrival tests' ON_MESH),
    // with the caster standing on it.
    let floor = [-289.465_f32, 68.542, -154.276];
    add_pet_owner(&mut mgr, OWNER, "Castle_CellBlock", floor, 25);
    seed_deployable(&mut mgr);
    let def = mgr.ability_defs.get(&DEPLOYABLE_ABILITY).cloned();
    assert!(mgr.enforces_navmesh_containment(mgr.get_entity_space_id(OWNER).unwrap()));

    // 1.5 m above the floor: on the mesh, placed on the floor.
    let lifted = [floor[0], floor[1] + 1.5, floor[2]];
    let placed = validate_ground_point(&mgr, OWNER, def.as_ref(), lifted)
        .expect("a point over the floor is accepted");
    assert!(
        (placed.y - floor[1]).abs() < 0.5,
        "snapped onto the floor, got y {}",
        placed.y
    );

    // 200 m above it (the arrival tests' OFF_MESH): no mesh there.
    let under = [floor[0], floor[1] + 200.0, floor[2]];
    assert_eq!(
        validate_ground_point(&mgr, OWNER, def.as_ref(), under),
        Err(DeployRefusal::OffNavmesh)
    );
}

/// A plain `useAbility` naming the deployable has no ground point: refused
/// with feedback, nothing charged.
#[tokio::test]
async fn a_plain_use_ability_on_a_deployable_is_refused() {
    let mut mgr = deploy_mgr();
    hostile(&mut mgr, 50, [8.0, 0.0, 10.0]);
    let (tx, mut rx) = mpsc::channel(256);

    assert!(!handle_use_ability(OWNER, DEPLOYABLE_ABILITY, 50, &tx, &mut mgr).await);
    let sent = drain(&mut rx);
    assert_eq!(error_codes(&sent, OWNER, DEPLOYABLE_ABILITY), vec![0]);
    assert!(got_chat(
        &sent,
        OWNER,
        "Choose a spot on the ground to place that."
    ));
    assert!(!mgr
        .get_entity(OWNER)
        .unwrap()
        .abilities
        .is_on_cooldown(DEPLOYABLE_ABILITY));
}

/// A press during the cooldown, or during the warmup, is answered (the
/// ordinary launch refuses both silently).
#[tokio::test]
async fn a_press_during_the_warmup_or_the_cooldown_gets_an_answer() {
    let mut mgr = deploy_mgr();
    let (tx, mut rx) = mpsc::channel(256);

    handle_use_ability_on_ground(OWNER, DEPLOYABLE_ABILITY, SPOT, &tx, &mut mgr).await;
    drain(&mut rx);
    handle_use_ability_on_ground(OWNER, DEPLOYABLE_ABILITY, SPOT, &tx, &mut mgr).await;
    let busy = drain(&mut rx);
    assert_eq!(error_codes(&busy, OWNER, DEPLOYABLE_ABILITY), vec![99]);
    assert!(got_chat(&busy, OWNER, "You are already using an ability."));

    resolve_warmups(after_warmup(), &tx, &mut mgr, &NoContentEvents).await;
    drain(&mut rx);
    handle_use_ability_on_ground(OWNER, DEPLOYABLE_ABILITY, SPOT, &tx, &mut mgr).await;
    let cooling = drain(&mut rx);
    assert_eq!(error_codes(&cooling, OWNER, DEPLOYABLE_ABILITY), vec![99]);
    assert!(got_chat(
        &cooling,
        OWNER,
        "That deployable is not ready yet."
    ));
    assert_eq!(mgr.deployables.len(), 1, "still the first object only");
}

/// An interrupted warmup places nothing and drops the staged point, so a
/// later plain `useAbility` cannot fire on it.
#[tokio::test]
async fn an_interrupted_warmup_places_nothing_and_forgets_the_point() {
    let mut mgr = deploy_mgr();
    let (tx, _rx) = mpsc::channel(256);

    handle_use_ability_on_ground(OWNER, DEPLOYABLE_ABILITY, SPOT, &tx, &mut mgr).await;
    assert!(mgr
        .deployables
        .staged_for(OWNER, DEPLOYABLE_ABILITY)
        .is_some());
    interrupt_pending_cast(OWNER, InterruptReason::CasterMoved, &tx, &mut mgr).await;
    assert_eq!(mgr.deployables.staged_for(OWNER, DEPLOYABLE_ABILITY), None);
    assert_eq!(
        resolve_warmups(after_warmup(), &tx, &mut mgr, &NoContentEvents).await,
        0
    );
    assert!(mgr.deployables.is_empty());
}

/// One active per owner: a re-cast replaces the first object, and every
/// witness is told it left.
#[tokio::test]
async fn a_recast_replaces_the_owners_object() {
    let mut mgr = deploy_mgr();
    let (tx, mut rx) = mpsc::channel(512);
    let first = deploy_at(&mut mgr, SPOT, &tx).await;
    let _ = mgr.compute_aoi_changes();
    drain(&mut rx);

    ready_again(&mut mgr);
    let second = deploy_at(&mut mgr, [12.0, 0.0, 12.0], &tx).await;
    assert_ne!(first, second);
    assert_eq!(
        mgr.deployables.of_owner(OWNER, DEPLOYABLE_ABILITY),
        vec![second]
    );
    assert!(mgr.get_entity(first).is_none());
    let left = drain(&mut rx).into_iter().any(|m| {
        matches!(m, CellToBaseMsg::LeftAoI { witness_id, entity_id } if witness_id == OWNER && entity_id == first)
    });
    assert!(left, "the owner sees the replaced object go");
}

/// An NPC caster never deploys: its launch takes the ordinary path.
#[tokio::test]
async fn an_npc_caster_does_not_deploy() {
    let mut mgr = deploy_mgr();
    hostile(&mut mgr, 60, [30.0, 0.0, 30.0]);
    mgr.get_entity_mut(60)
        .unwrap()
        .abilities
        .add_ability(DEPLOYABLE_ABILITY);
    let (tx, _rx) = mpsc::channel(256);
    handle_use_ability_on_ground(60, DEPLOYABLE_ABILITY, [31.0, 0.0, 30.0], &tx, &mut mgr).await;
    resolve_warmups(
        Instant::now() + std::time::Duration::from_secs(5),
        &tx,
        &mut mgr,
        &NoContentEvents,
    )
    .await;
    assert!(mgr.deployables.is_empty());
}
