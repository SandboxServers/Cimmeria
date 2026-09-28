//! A pet ability with a warmup fires later, from the warmup tick. The tick
//! re-applies the owner's target rule and line of sight for a pet caster,
//! so a target that turns friendly (or is a being) during the warmup, or a
//! wall that comes between them, is not hit: the cast is interrupted
//! instead of fired. The pet engages the ordered target only when the cast
//! fires, so an interrupted order leaves it not fighting.
//!
//! The fixture's seeded ability has no effects, so the fire does no damage
//! either way; the observable is the tick's own row on target `abilities`
//! (`warmup_complete` when it fires, `warmup_interrupted` with `reason` when
//! it does not), which is also what an operator would read.

use cimmeria_entity::cell_entity::PetState;
use std::time::{Duration, Instant};

use cimmeria_entity::cell_entity::AiState;
use tracing::Level;

use super::*;
use crate::test_support::{Captured, LogCapture};

/// How the warmup ended.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Fired,
    Interrupted(String),
}

/// Give the pet's ability a 1 s warmup, order the cast at `MOB`, let
/// `before_fire` change the world, then run the warmup tick as if the
/// warmup had elapsed.
async fn warmup_cast_then(before_fire: impl FnOnce(&mut SpaceManager, u32)) -> Outcome {
    warmup_cast_with(before_fire).await.0
}

/// What [`warmup_cast_with`] hands back: how the warmup ended, the world
/// and the pet afterwards, every row the warmup tick logged, and what the
/// tick sent.
type WarmupRun = (Outcome, SpaceManager, u32, Vec<Captured>, Sent);

/// [`warmup_cast_then`], also returning the world, the pet and the tick's
/// log rows.
async fn warmup_cast_with(before_fire: impl FnOnce(&mut SpaceManager, u32)) -> WarmupRun {
    let World { mut mgr, pet, .. } = world();
    mgr.ability_defs.get_mut(&PET_ABILITY).unwrap().warmup = 1.0;

    let sent = invoke(&mut mgr, OWNER, pet, PET_ABILITY, MOB).await;
    assert_eq!(sent.all_error_codes(), 0);
    let e = mgr.get_entity(pet).unwrap();
    assert!(e.pending_cast.is_some(), "the cast is warming up");
    assert!(
        e.threat_list.is_empty() && e.ai_state() != AiState::Fighting,
        "a warming order does not engage yet"
    );
    assert_eq!(
        e.extensions.get::<PetState>().unwrap().deferred_order,
        Some(MOB)
    );

    before_fire(&mut mgr, pet);
    mgr.get_entity_mut(pet)
        .unwrap()
        .pending_cast
        .as_mut()
        .unwrap()
        .fire_at = Instant::now() - Duration::from_millis(1);

    let capture = LogCapture::install();
    let (tx, mut rx) = mpsc::channel(512);
    let engine = ChainEngine::new();
    crate::cell::abilities::warmup_tick(
        &tx,
        &mut mgr,
        &crate::cell::content::EngineEvents(&engine),
    )
    .await;
    drop(tx);
    let mut tick_sent = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        tick_sent.push(msg);
    }
    let tick_sent = Sent(tick_sent);
    assert!(
        mgr.get_entity(pet).unwrap().pending_cast.is_none(),
        "the warmup resolved one way or the other"
    );

    let pet_id = pet.to_string();
    let rows: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.target == "abilities" && c.has_field("entity_id", &pet_id))
        .collect();
    if rows.iter().any(|c| c.has_field("event", "warmup_complete")) {
        return (Outcome::Fired, mgr, pet, capture.all(), tick_sent);
    }
    let interrupted = rows
        .iter()
        .find(|c| c.level == Level::INFO && c.has_field("event", "warmup_interrupted"))
        .unwrap_or_else(|| panic!("neither fired nor interrupted: {:#?}", capture.all()));
    let outcome = Outcome::Interrupted(
        interrupted
            .fields
            .get("reason")
            .cloned()
            .unwrap_or_default(),
    );
    (outcome, mgr, pet, capture.all(), tick_sent)
}

/// Control: nothing changes during the warmup, and the cast fires. Without
/// it the two refusals below could pass because the fixture never fires.
#[tokio::test]
async fn an_unchanged_target_is_fired_on_when_the_warmup_ends() {
    assert_eq!(warmup_cast_then(|_, _| {}).await, Outcome::Fired);
}

/// Content turns the mob friendly during the warmup: the fire is refused.
#[tokio::test]
async fn a_target_turned_friendly_during_the_warmup_is_not_hit() {
    let outcome = warmup_cast_then(|mgr, _| {
        mgr.get_entity_mut(MOB).unwrap().faction = 1;
    })
    .await;
    assert_eq!(outcome, Outcome::Interrupted("target_lost".into()));
}

/// A wall comes between pet and mob during the warmup: the fire is refused.
#[tokio::test]
async fn a_target_hidden_during_the_warmup_is_not_hit() {
    let outcome = warmup_cast_then(|mgr, pet| {
        let pet_x = mgr.get_entity(pet).unwrap().position.x;
        let mob_x = mgr.get_entity(MOB).unwrap().position.x;
        let wall_x = (pet_x + mob_x) / 2.0;
        let wall = crate::test_support::occluder_fixtures::synthetic(&[(
            [wall_x - 0.15, 0.0, 0.0],
            [wall_x + 0.15, 4.0, 40.0],
        )]);
        let sid = mgr.get_entity_space_id(pet).unwrap();
        mgr.spaces.get_mut(&sid).unwrap().occluder = Some(wall);
    })
    .await;
    assert_eq!(outcome, Outcome::Interrupted("no_line_of_sight".into()));
}

/// A being (class 0x01) is never an order target: content swapping the mob
/// for a being-class entity during the warmup interrupts the fire, the same
/// rule the command pre-check applies (Copilot, #901).
#[tokio::test]
async fn a_target_that_is_a_being_at_fire_time_is_not_hit() {
    let outcome = warmup_cast_then(|mgr, _| {
        mgr.get_entity_mut(MOB).unwrap().class_id = 0x01;
    })
    .await;
    assert_eq!(outcome, Outcome::Interrupted("target_lost".into()));
}

/// The deferred engagement: the fired cast engages the ordered target, with
/// an `order_engaged` row; this is the control for the guard below.
#[tokio::test]
async fn a_fired_warmup_engages_the_ordered_target() {
    let (outcome, mgr, pet, rows, _) = warmup_cast_with(|_, _| {}).await;
    assert_eq!(outcome, Outcome::Fired);
    let engaged: Vec<_> = rows
        .iter()
        .filter(|c| c.target == "pets.command" && c.has_field("event", "order_engaged"))
        .collect();
    assert_eq!(engaged.len(), 1, "one order_engaged row: {rows:#?}");
    let row = engaged[0];
    assert_eq!(row.level, Level::DEBUG);
    for (field, value) in [
        ("pet_id", pet.to_string()),
        ("owner_id", OWNER.to_string()),
        ("account_id", OWNER_ACCOUNT_ID.to_string()),
        ("player_id", OWNER_PLAYER_ID.to_string()),
        ("target_id", MOB.to_string()),
        ("engaged", "true".to_string()),
    ] {
        assert!(row.has_field(field, &value), "{field}: {row:?}");
    }
    let e = mgr.get_entity(pet).unwrap();
    assert_eq!(e.ai_state(), AiState::Fighting, "the pet engages on fire");
    assert!(e.threat_list.contains_key(&MOB));
    assert_eq!(
        e.extensions.get::<PetState>().unwrap().deferred_order,
        None,
        "taken"
    );
    let mut mgr = mgr;
    assert_two_sided_engagement(&mut mgr, pet, MOB).await;
}

/// An order whose warmup is interrupted (the target turned friendly) leaves
/// the pet with no seeded threat and not Fighting, so the AI does not keep
/// fighting a target the owner may no longer attack (Copilot, #901).
#[tokio::test]
async fn an_interrupted_warmup_leaves_the_pet_not_fighting() {
    let (outcome, mgr, pet, rows, sent) = warmup_cast_with(|mgr, _| {
        mgr.get_entity_mut(MOB).unwrap().faction = 1;
    })
    .await;
    assert_eq!(outcome, Outcome::Interrupted("target_lost".into()));
    let e = mgr.get_entity(pet).unwrap();
    assert!(e.threat_list.is_empty(), "no command-seeded threat");
    assert_ne!(e.ai_state(), AiState::Fighting);
    assert!(
        mgr.get_entity(MOB).unwrap().threat_list.is_empty(),
        "the mob was never engaged"
    );
    assert!(
        !rows.iter().any(|c| c.has_field("event", "order_engaged")),
        "an interrupted cast never reaches the engagement"
    );
    assert_eq!(
        e.extensions.get::<PetState>().unwrap().deferred_order,
        None,
        "the interrupt drops the order, so no later fire can engage it"
    );
    assert_eq!(
        sent.error_codes_to(OWNER),
        vec![(PET_ABILITY, 0)],
        "the owner hears why, keyed by the ability id"
    );
    assert_eq!(sent.feedback_lines_to(OWNER), 1);
    let row = rows
        .iter()
        .find(|c| c.target == "pets.command" && c.has_field("event", "order_interrupted"))
        .unwrap_or_else(|| panic!("no order_interrupted row: {rows:#?}"));
    assert!(row.has_field("reason", "target_lost"), "{row:?}");
    assert!(row.has_field("player_id", &OWNER_PLAYER_ID.to_string()));
}

/// The target starts walking home (Leashing) during the warmup: the pet's
/// fire-time check refuses it before any damage, the cast is interrupted,
/// and the owner gets `onErrorCode` (keyed by the ability id) plus a chat
/// line (Copilot, #901).
#[tokio::test]
async fn a_target_that_starts_leashing_mid_warmup_takes_no_damage() {
    let health = |mgr: &SpaceManager| {
        mgr.get_entity(MOB)
            .unwrap()
            .stats
            .get(cimmeria_entity::stats::HEALTH)
            .map(|s| s.cur)
    };
    let mut before = None;
    let (outcome, mgr, pet, _, sent) = warmup_cast_with(|mgr, _| {
        cimmeria_cell_combat::cell::service::npc_ai::force_ai_state(
            mgr.get_entity_mut(MOB).unwrap(),
            AiState::Leashing,
        );
        before = health(mgr);
    })
    .await;
    assert_eq!(outcome, Outcome::Interrupted("target_lost".into()));
    assert_eq!(health(&mgr), before, "no damage");
    assert!(mgr.get_entity(MOB).unwrap().threat_list.is_empty());
    assert!(mgr.get_entity(pet).unwrap().threat_list.is_empty());
    assert_eq!(sent.error_codes_to(OWNER), vec![(PET_ABILITY, 0)]);
    assert_eq!(sent.feedback_lines_to(OWNER), 1);
}

/// The owner switches the pet to Passive while an order warms up: the
/// order is dropped, so the cast lands but the pet does not engage.
#[tokio::test]
async fn going_passive_mid_warmup_drops_the_order() {
    let World { mut mgr, pet, .. } = world();
    mgr.ability_defs.get_mut(&PET_ABILITY).unwrap().warmup = 1.0;
    let _ = invoke(&mut mgr, OWNER, pet, PET_ABILITY, MOB).await;
    let deferred = |mgr: &SpaceManager| {
        mgr.get_entity(pet)
            .unwrap()
            .extensions
            .get::<PetState>()
            .unwrap()
            .deferred_order
    };
    assert_eq!(deferred(&mgr), Some(MOB));

    // Through the real CM 90 path, as the pet info window sends it.
    let passive = cimmeria_entity::cell_entity::PetStance::Passive.wire();
    let _ = stance(&mut mgr, OWNER, pet, passive).await;
    assert_eq!(deferred(&mgr), None, "Passive drops the pending order");

    mgr.get_entity_mut(pet)
        .unwrap()
        .pending_cast
        .as_mut()
        .unwrap()
        .fire_at = Instant::now() - Duration::from_millis(1);
    let capture = LogCapture::install();
    let (tx, _rx) = mpsc::channel(512);
    let engine = ChainEngine::new();
    crate::cell::abilities::warmup_tick(
        &tx,
        &mut mgr,
        &crate::cell::content::EngineEvents(&engine),
    )
    .await;
    let e = mgr.get_entity(pet).unwrap();
    assert!(e.pending_cast.is_none(), "the cast still fired");
    assert!(!e.threat_list.contains_key(&MOB), "the pet did not engage");
    assert_ne!(e.ai_state(), AiState::Fighting);
    assert!(!capture
        .all()
        .iter()
        .any(|c| c.has_field("event", "order_engaged")));
}
