//! `engage_pet_target`, the one pet-engagement entry (stance picks and an
//! owner's attack order): both sides of the fight are seeded, and a refusal
//! changes nothing.

use super::*;
use crate::cell::service::npc_ai::pet::{engage_pet_target, PetEngagement};

fn lists(mgr: &SpaceManager, who: u32, whom: u32) -> bool {
    mgr.get_entity(who).unwrap().threat_list.contains_key(&whom)
}

/// Both sides: the mob lists the pet and fights back, the pet lists the mob
/// and fights. An explicit engagement ignores the stance, so an owner's
/// order is obeyed even by a Passive pet.
#[test]
fn an_engagement_is_two_sided_whatever_the_stance() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    add_mob(&mut mgr, MOB, [14.0, 0.0, 10.0], HOSTILE);
    set_stance(&mut mgr, pet, PetStance::Passive);

    assert_eq!(
        engage_pet_target(&mut mgr, pet, MOB, PetEngagement::Automatic),
        Ok(())
    );

    assert_eq!(state(&mgr, MOB), AiState::Fighting, "the mob fights back");
    assert!(lists(&mgr, MOB, pet), "the mob lists the pet");
    assert_eq!(state(&mgr, pet), AiState::Fighting);
    assert!(lists(&mgr, pet, MOB), "the pet lists the mob");
}

/// A refusal says why and touches neither side.
#[test]
fn a_refused_engagement_changes_nothing() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    add_mob(&mut mgr, MOB, [14.0, 0.0, 10.0], 0);
    add_mob(&mut mgr, MOB_2, [16.0, 0.0, 10.0], HOSTILE);
    mgr.get_entity_mut(MOB_2)
        .unwrap()
        .stats
        .get_mut(cimmeria_entity::stats::HEALTH)
        .unwrap()
        .update(0, 0, 100);

    assert_eq!(
        engage_pet_target(&mut mgr, pet, MOB, PetEngagement::Automatic),
        Err("target_not_hostile")
    );
    assert_eq!(
        engage_pet_target(&mut mgr, pet, MOB_2, PetEngagement::Automatic),
        Err("target_dead")
    );
    assert_eq!(
        engage_pet_target(&mut mgr, MOB, pet, PetEngagement::Automatic),
        Err("not_a_pet")
    );

    assert!(mgr.get_entity(pet).unwrap().threat_list.is_empty());
    assert!(mgr.get_entity(MOB).unwrap().threat_list.is_empty());
    assert!(mgr.get_entity(MOB_2).unwrap().threat_list.is_empty());
    assert_ne!(state(&mgr, pet), AiState::Fighting);
}

/// A surrendered (`Submit`) NPC is not engageable on the pet's own
/// initiative, but an owner's order still reaches it, as a player's own
/// attack does (`handle_use_ability` does not refuse a submitted target).
#[test]
fn a_surrendered_npc_takes_an_order_but_not_an_automatic_engagement() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    add_mob(&mut mgr, MOB, [14.0, 0.0, 10.0], HOSTILE);
    crate::cell::service::npc_ai::force_ai_state(mgr.get_entity_mut(MOB).unwrap(), AiState::Submit);

    assert_eq!(
        engage_pet_target(&mut mgr, pet, MOB, PetEngagement::Automatic),
        Err("target_not_engageable")
    );
    assert!(mgr.get_entity(pet).unwrap().threat_list.is_empty());
    assert_eq!(
        engage_pet_target(&mut mgr, pet, MOB, PetEngagement::OwnerOrder),
        Ok(())
    );
    assert!(lists(&mgr, pet, MOB));
}

/// A target in another space (an owner order carries a client-chosen id) is
/// refused before either side is seeded: no threat on the pet, none on the
/// target, and no combat entry for the owner.
#[test]
fn a_target_in_another_space_is_refused_and_nothing_is_seeded() {
    use crate::test_support::LogCapture;

    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces>
        <Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" />
        <Space WorldName="Castle" Instanced="false" MinX="-2400" MaxX="2400" MinY="-2400" MaxY="2400" />
        </Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    seed_pet_template(&mut mgr, PET_FIXTURE_TEMPLATE_ID);
    add_pet_owner(&mut mgr, OWNER, "Agnos", [10.0, 0.0, 10.0], 12);
    mgr.get_entity_mut(OWNER).unwrap().player_id = Some(OWNER_PLAYER_ID);
    let pet = mgr
        .spawn_pet_from_template(OWNER, PET_FIXTURE_TEMPLATE_ID, 0)
        .unwrap();
    // Same coordinates as the pet, another space.
    mgr.spawn_npc(MOB, "Castle", [10.0, 0.0, 8.0], [0.0; 3])
        .unwrap();
    let mob = mgr.get_entity_mut(MOB).unwrap();
    mob.faction = HOSTILE;
    if let Some(h) = mob.stats.get_mut(cimmeria_entity::stats::HEALTH) {
        h.update(0, 100, 100);
        h.clear_dirty();
    }
    assert_ne!(
        mgr.get_entity_space_id(pet),
        mgr.get_entity_space_id(MOB),
        "precondition"
    );

    let logs = LogCapture::install();
    for kind in [PetEngagement::OwnerOrder, PetEngagement::Automatic] {
        assert_eq!(
            engage_pet_target(&mut mgr, pet, MOB, kind),
            Err("target_other_space")
        );
    }

    assert!(mgr.get_entity(pet).unwrap().threat_list.is_empty());
    assert!(mgr.get_entity(MOB).unwrap().threat_list.is_empty());
    assert_ne!(state(&mgr, pet), AiState::Fighting);
    assert_ne!(state(&mgr, MOB), AiState::Fighting);
    assert!(mgr.get_entity(OWNER).unwrap().threatened_mobs.is_empty());
    let row = pets_ai_row(&logs, "pet_cross_space_refused").expect("refusal row");
    assert!(row.has_field("reason", "target_other_space"), "{row:?}");
    assert!(row.has_field("target_id", &MOB.to_string()), "{row:?}");
    assert_owner_identity(&row);
}
