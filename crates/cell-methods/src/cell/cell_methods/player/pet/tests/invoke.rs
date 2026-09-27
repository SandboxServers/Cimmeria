//! CM 88 `petInvokeAbility`: a good order casts and engages; every refusal
//! answers the owner with `onErrorCode` and leaves the pet idle.

use cimmeria_entity::cell_entity::AiState;

use super::*;
use crate::cell::cell_methods::player::pet::{
    FEEDBACK_INVALID_ENTITY, FEEDBACK_NOT_LIVING, FEEDBACK_NOT_READY, FEEDBACK_NO_LINE_OF_SIGHT,
    FEEDBACK_NO_SUCH_PET_ABILITY, FEEDBACK_OUTSIDE_WEAPON_RANGE, FEEDBACK_RELATIONSHIP_FRIEND,
};

/// The pet did nothing: no cooldown started, no threat, not fighting.
fn assert_pet_idle(mgr: &SpaceManager, pet: u32, ability: i32) {
    let e = mgr.get_entity(pet).unwrap();
    assert!(!e.abilities.is_on_cooldown(ability), "no cast started");
    assert!(e.threat_list.is_empty(), "no target seeded");
    assert_ne!(e.ai_state(), AiState::Fighting);
}

/// The good path: the pet casts (its cooldown starts), no refusal is sent,
/// and the commanded mob is its top threat with the pet Fighting.
#[tokio::test]
async fn owner_order_casts_and_engages_the_target() {
    let World { mut mgr, pet, .. } = world();
    let sent = invoke(&mut mgr, OWNER, pet, PET_ABILITY, MOB).await;

    assert_eq!(sent.all_error_codes(), 0, "no refusal");
    let e = mgr.get_entity(pet).unwrap();
    assert!(
        e.abilities.is_on_cooldown(PET_ABILITY),
        "handle_use_ability committed the pet's cast"
    );
    assert_eq!(e.ai_state(), AiState::Fighting, "the pet engages");
    let top = e
        .threat_list
        .iter()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .map(|(&id, _)| id);
    assert_eq!(top, Some(MOB), "the commanded mob is the top threat");
    assert_two_sided_engagement(&mut mgr, pet, MOB).await;
}

/// An order's threat is capped at `OWNER_ORDER_THREAT_CAP`, even when the
/// target's own entry was already above it (Copilot, #901).
#[tokio::test]
async fn an_order_s_threat_is_capped() {
    use cimmeria_cell_combat::cell::service::npc_ai::pet::OWNER_ORDER_THREAT_CAP;
    let World { mut mgr, pet, .. } = world();
    mgr.get_entity_mut(pet)
        .unwrap()
        .threat_list
        .insert(MOB, OWNER_ORDER_THREAT_CAP * 5.0);
    let sent = invoke(&mut mgr, OWNER, pet, PET_ABILITY, MOB).await;
    assert_eq!(sent.all_error_codes(), 0);
    assert_eq!(
        mgr.get_entity(pet).unwrap().threat_list[&MOB],
        OWNER_ORDER_THREAT_CAP
    );
}

/// A hostile-faction SGWBeing (class 0x01: a prop or story actor) is not a
/// combatant, so the owner cannot order the pet at it, even on faction 10.
/// Without the class check the order would pass the faction test and the
/// pet would shoot a crate or Col Marsh (Copilot, #901).
#[tokio::test]
async fn a_hostile_faction_being_is_refused() {
    let World { mut mgr, pet, .. } = world();
    const BEING: u32 = 200_004;
    let pos = mgr.get_entity(pet).unwrap().position;
    mgr.spawn_npc(BEING, "Agnos", [pos.x, pos.y, pos.z + 5.0], [0.0; 3])
        .unwrap();
    let being = mgr.get_entity_mut(BEING).unwrap();
    being.faction = HOSTILE_FACTION;
    being.class_id = 0x01;

    let capture = crate::test_support::LogCapture::install();
    let sent = invoke(&mut mgr, OWNER, pet, PET_ABILITY, BEING).await;
    assert_eq!(
        sent.error_codes_to(OWNER),
        vec![(PET_ABILITY, FEEDBACK_RELATIONSHIP_FRIEND)]
    );
    assert_eq!(sent.feedback_lines_to(OWNER), 1, "a visible chat line");
    assert!(capture
        .all()
        .iter()
        .any(|c| c.target == "pets.command" && c.has_field("reason", "target_not_combatant")));
    assert_pet_idle(&mgr, pet, PET_ABILITY);
    assert!(mgr.get_entity(BEING).unwrap().threat_list.is_empty());
}

/// A mob walking home (Leashing) evades: the order is refused before the
/// cast, with feedback, and nothing is engaged.
#[tokio::test]
async fn a_leashing_target_is_refused() {
    let World { mut mgr, pet, .. } = world();
    cimmeria_cell_combat::cell::service::npc_ai::force_ai_state(
        mgr.get_entity_mut(MOB).unwrap(),
        cimmeria_entity::cell_entity::AiState::Leashing,
    );
    let sent = invoke(&mut mgr, OWNER, pet, PET_ABILITY, MOB).await;
    assert_eq!(
        sent.error_codes_to(OWNER),
        vec![(PET_ABILITY, FEEDBACK_INVALID_ENTITY)]
    );
    assert_eq!(sent.feedback_lines_to(OWNER), 1);
    assert_pet_idle(&mgr, pet, PET_ABILITY);
    assert!(mgr.get_entity(MOB).unwrap().threat_list.is_empty());
}

/// A second order at a new target outranks the fight the pet is already in.
#[tokio::test]
async fn a_new_order_outranks_existing_threat() {
    let World { mut mgr, pet, .. } = world();
    const MOB2: u32 = 200_003;
    let pos = mgr.get_entity(pet).unwrap().position;
    mgr.spawn_npc(MOB2, "Agnos", [pos.x, pos.y, pos.z + 5.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(MOB2).unwrap().faction = HOSTILE_FACTION;
    mgr.get_entity_mut(pet)
        .unwrap()
        .threat_list
        .insert(MOB, 500.0);

    let _ = invoke(&mut mgr, OWNER, pet, PET_ABILITY, MOB2).await;
    let e = mgr.get_entity(pet).unwrap();
    assert!(e.threat_list[&MOB2] > e.threat_list[&MOB]);
}

/// `targetId = 0`: an untargeted cast commits and seeds nothing.
#[tokio::test]
async fn an_untargeted_cast_commits_without_engaging() {
    let World { mut mgr, pet, .. } = world();
    let sent = invoke(&mut mgr, OWNER, pet, PET_ABILITY, 0).await;
    assert_eq!(sent.all_error_codes(), 0);
    let e = mgr.get_entity(pet).unwrap();
    assert!(e.abilities.is_on_cooldown(PET_ABILITY));
    assert!(e.threat_list.is_empty());
}

/// An ability that is not on the pet's bar (the owner's own, say):
/// EntityDoesNotHaveAbility, nothing cast.
#[tokio::test]
async fn an_ability_off_the_pets_bar_is_refused() {
    let World { mut mgr, pet, .. } = world();
    let sent = invoke(&mut mgr, OWNER, pet, NOT_A_PET_ABILITY, MOB).await;
    assert_eq!(
        sent.error_codes_to(OWNER),
        vec![(NOT_A_PET_ABILITY, FEEDBACK_NO_SUCH_PET_ABILITY)]
    );
    assert_pet_idle(&mgr, pet, NOT_A_PET_ABILITY);
}

/// A toggled-off ability is refused with the same code.
#[tokio::test]
async fn a_toggled_off_ability_is_refused() {
    let World { mut mgr, pet, .. } = world();
    mgr.get_entity_mut(pet)
        .unwrap()
        .pet
        .as_deref_mut()
        .unwrap()
        .toggled_off
        .push(PET_ABILITY);
    let sent = invoke(&mut mgr, OWNER, pet, PET_ABILITY, MOB).await;
    assert_eq!(
        sent.error_codes_to(OWNER),
        vec![(PET_ABILITY, FEEDBACK_NO_SUCH_PET_ABILITY)]
    );
    assert_pet_idle(&mgr, pet, PET_ABILITY);
}

/// Pressing again while the pet's ability cools down: the second press is
/// answered (NotReady) instead of being dropped by `handle_use_ability`'s
/// silent cooldown refusal.
#[tokio::test]
async fn a_press_during_the_cooldown_gets_feedback() {
    let World { mut mgr, pet, .. } = world();
    let first = invoke(&mut mgr, OWNER, pet, PET_ABILITY, MOB).await;
    assert_eq!(first.all_error_codes(), 0);
    let capture = crate::test_support::LogCapture::install();
    let second = invoke(&mut mgr, OWNER, pet, PET_ABILITY, MOB).await;
    assert_eq!(
        second.error_codes_to(OWNER),
        vec![(PET_ABILITY, FEEDBACK_NOT_READY)]
    );
    // The pre-check, not `handle_use_ability`'s silent refusal caught by
    // the `cast_refused` fallback (which sends the same code).
    assert!(
        capture
            .all()
            .iter()
            .any(|c| c.target == "pets.command" && c.has_field("reason", "ability_on_cooldown")),
        "refused by the cooldown pre-check; captured: {:#?}",
        capture.all()
    );
}

/// A target beyond the ability's reach: OutsideWeaponRange to the **owner**.
/// `handle_use_ability` would address that code to the pet id, which no
/// client receives.
#[tokio::test]
async fn an_out_of_range_target_is_answered_to_the_owner() {
    let World { mut mgr, pet, .. } = world();
    let pos = mgr.get_entity(pet).unwrap().position;
    mgr.get_entity_mut(MOB).unwrap().position.x = pos.x + 100.0;
    let sent = invoke(&mut mgr, OWNER, pet, PET_ABILITY, MOB).await;
    assert_eq!(
        sent.error_codes_to(OWNER),
        vec![(PET_ABILITY, FEEDBACK_OUTSIDE_WEAPON_RANGE)]
    );
    assert_eq!(
        sent.error_codes_to(pet),
        vec![],
        "nothing addressed to the pet"
    );
    assert_pet_idle(&mgr, pet, PET_ABILITY);
}

/// The #444 rule for the owner's order: a friendly NPC, another player, the
/// owner, the pet itself and another player's pet are all refused, and none
/// of them takes damage.
#[tokio::test]
async fn a_non_hostile_target_is_refused() {
    let World {
        mut mgr,
        pet,
        other_pet,
    } = world();
    for target in [FRIENDLY, OTHER, OWNER, pet, other_pet] {
        let health_before = mgr
            .get_entity(target)
            .unwrap()
            .stats
            .get(cimmeria_entity::stats::HEALTH)
            .map(|s| s.cur);
        let sent = invoke(&mut mgr, OWNER, pet, PET_ABILITY, target).await;
        assert_eq!(
            sent.error_codes_to(OWNER),
            vec![(PET_ABILITY, FEEDBACK_RELATIONSHIP_FRIEND)],
            "target {target}"
        );
        let health_after = mgr
            .get_entity(target)
            .unwrap()
            .stats
            .get(cimmeria_entity::stats::HEALTH)
            .map(|s| s.cur);
        assert_eq!(health_before, health_after, "target {target} untouched");
        assert_pet_idle(&mgr, pet, PET_ABILITY);
    }
}

/// A dead pet and a dead target: NotLiving.
#[tokio::test]
async fn a_dead_pet_or_a_dead_target_is_refused() {
    let World { mut mgr, pet, .. } = world();
    mgr.get_entity_mut(MOB).unwrap().state_field |= crate::cell::combat::BSF_DEAD;
    let sent = invoke(&mut mgr, OWNER, pet, PET_ABILITY, MOB).await;
    assert_eq!(
        sent.error_codes_to(OWNER),
        vec![(PET_ABILITY, FEEDBACK_NOT_LIVING)]
    );

    let World { mut mgr, pet, .. } = world();
    mgr.get_entity_mut(pet).unwrap().state_field |= crate::cell::combat::BSF_DEAD;
    let sent = invoke(&mut mgr, OWNER, pet, PET_ABILITY, MOB).await;
    assert_eq!(
        sent.error_codes_to(OWNER),
        vec![(PET_ABILITY, FEEDBACK_NOT_LIVING)]
    );
    assert_pet_idle(&mgr, pet, PET_ABILITY);
}

/// A wall between the pet and the mob: `CONDITION_FEEDBACK_LOS` (39) to the
/// owner, and the mob takes nothing. `fire_los` skips NPC shooters, so
/// without the pet's own sight check the order would hit through the wall.
#[tokio::test]
async fn a_target_behind_a_wall_is_refused() {
    let World { mut mgr, pet, .. } = world();
    let pet_pos = mgr.get_entity(pet).unwrap().position;
    let mob_pos = mgr.get_entity(MOB).unwrap().position;
    let wall_x = (pet_pos.x + mob_pos.x) / 2.0;
    let wall = crate::test_support::occluder_fixtures::synthetic(&[(
        [wall_x - 0.15, 0.0, 0.0],
        [wall_x + 0.15, 4.0, 40.0],
    )]);
    let sid = mgr.get_entity_space_id(pet).unwrap();
    mgr.spaces.get_mut(&sid).unwrap().occluder = Some(wall);
    let health_before = mob_health(&mgr);

    let sent = invoke(&mut mgr, OWNER, pet, PET_ABILITY, MOB).await;
    assert_eq!(
        sent.error_codes_to(OWNER),
        vec![(PET_ABILITY, FEEDBACK_NO_LINE_OF_SIGHT)]
    );
    assert_eq!(mob_health(&mgr), health_before, "the mob took nothing");
    assert_pet_idle(&mgr, pet, PET_ABILITY);
}

/// The same scene without the wall casts: the refusal above is the wall's.
#[tokio::test]
async fn the_same_target_in_the_open_is_hit() {
    let World { mut mgr, pet, .. } = world();
    let sid = mgr.get_entity_space_id(pet).unwrap();
    mgr.spaces.get_mut(&sid).unwrap().occluder =
        Some(crate::test_support::occluder_fixtures::synthetic(&[]));
    let sent = invoke(&mut mgr, OWNER, pet, PET_ABILITY, MOB).await;
    assert_eq!(sent.all_error_codes(), 0);
    assert!(mgr
        .get_entity(pet)
        .unwrap()
        .abilities
        .is_on_cooldown(PET_ABILITY));
}

/// A hostile NPC in another space at the pet's own coordinates: refused as
/// `target_other_space`, and it takes nothing. `get_entity` searches every
/// space, and the range test alone would pass it.
#[tokio::test]
async fn a_target_in_another_space_is_refused() {
    let World { mut mgr, pet, .. } = world();
    const ELSEWHERE: u32 = 200_010;
    let p = mgr.get_entity(pet).unwrap().position;
    mgr.spawn_npc(ELSEWHERE, "Castle", [p.x + 1.0, p.y, p.z], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(ELSEWHERE).unwrap().faction = HOSTILE_FACTION;
    assert_ne!(
        mgr.get_entity_space_id(ELSEWHERE),
        mgr.get_entity_space_id(pet)
    );
    let before = mgr
        .get_entity(ELSEWHERE)
        .unwrap()
        .stats
        .get(cimmeria_entity::stats::HEALTH)
        .map(|s| s.cur);

    let capture = crate::test_support::LogCapture::install();
    let sent = invoke(&mut mgr, OWNER, pet, PET_ABILITY, ELSEWHERE).await;
    assert_eq!(
        sent.error_codes_to(OWNER),
        vec![(PET_ABILITY, FEEDBACK_INVALID_ENTITY)]
    );
    assert_eq!(sent.feedback_lines_to(OWNER), 1, "a visible chat line");
    assert!(
        mgr.get_entity(ELSEWHERE).unwrap().threat_list.is_empty(),
        "no threat across spaces"
    );
    assert!(capture
        .all()
        .iter()
        .any(|c| c.target == "pets.command" && c.has_field("reason", "target_other_space")));
    let after = mgr
        .get_entity(ELSEWHERE)
        .unwrap()
        .stats
        .get(cimmeria_entity::stats::HEALTH)
        .map(|s| s.cur);
    assert_eq!(before, after);
    assert_pet_idle(&mgr, pet, PET_ABILITY);
}

/// An ability id the server has no definition for (only a forged packet
/// sends one) is refused at DEBUG, so it cannot flood the WARN index; a
/// known ability that is not on the bar still WARNs.
#[tokio::test]
async fn an_unknown_ability_id_is_refused_quietly() {
    use tracing::Level;
    let World { mut mgr, pet, .. } = world();
    let capture = crate::test_support::LogCapture::install();
    let sent = invoke(&mut mgr, OWNER, pet, 999_999, MOB).await;
    assert_eq!(
        sent.error_codes_to(OWNER),
        vec![(999_999, FEEDBACK_NO_SUCH_PET_ABILITY)]
    );
    let rows: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.target == "pets.command" && c.has_field("reason", "ability_not_in_list"))
        .collect();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].level, Level::DEBUG);

    let capture = crate::test_support::LogCapture::install();
    let _ = invoke(&mut mgr, OWNER, pet, NOT_A_PET_ABILITY, MOB).await;
    assert!(capture.all().iter().any(|c| c.level == Level::WARN
        && c.target == "pets.command"
        && c.has_field("reason", "ability_not_in_list")));
}

fn mob_health(mgr: &SpaceManager) -> Option<i32> {
    mgr.get_entity(MOB)
        .unwrap()
        .stats
        .get(cimmeria_entity::stats::HEALTH)
        .map(|s| s.cur)
}
