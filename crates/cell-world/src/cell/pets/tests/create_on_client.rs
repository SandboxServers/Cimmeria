//! The owner-only `createOnClient` replay on the AoI tick (A-23, A-24).

use super::*;
use crate::cell::messages::CellToBaseMsg;
use crate::mercury::SGWPET_CLASS_ID;
use cimmeria_entity::cell_entity::PetStance;
use cimmeria_wire::cell::client_methods::pet::{
    ENTITYFLAG_PET, ON_PET_ABILITY_LIST, ON_PET_STANCE_LIST, ON_PET_STANCE_UPDATE,
};

/// `(witness_id, method_index, args)` of every pet-list push about `pet`.
fn pet_pushes(events: &[CellToBaseMsg], pet: u32) -> Vec<(u32, u16, Vec<u8>)> {
    events
        .iter()
        .filter_map(|e| match e {
            CellToBaseMsg::WitnessEntityMethod {
                witness_id,
                entity_id,
                method_index,
                args,
                entity_is_player,
            } if *entity_id == pet
                && matches!(
                    *method_index,
                    ON_PET_ABILITY_LIST | ON_PET_STANCE_LIST | ON_PET_STANCE_UPDATE
                ) =>
            {
                assert!(!entity_is_player, "the observee is the pet");
                Some((*witness_id, *method_index, args.clone()))
            }
            _ => None,
        })
        .collect()
}

/// The owner meets its pet: `EnteredAoI` (class 0x05, `ENTITYFLAG_Pet`,
/// owner id in the cascade data) followed by the two lists, byte-exact, in
/// python order. No `onPetStanceUpdate`: the stance is the client's default.
/// A second player in range gets the `EnteredAoI` but **none** of the lists:
/// sending them to every witness would leak the owner's pet bar (A-24), and
/// `onPetStanceList` is half of the client's `Unit.Pet` bind.
#[test]
fn owner_gets_the_pet_lists_and_a_second_witness_does_not() {
    let (mut mgr, pet) = world_with_pet();
    add_pet_owner(&mut mgr, OTHER, "Agnos", [12.0, 0.0, 12.0], 5);

    let events = mgr.compute_aoi_changes();

    // Both players are introduced to the pet as an SGWPet with its owner.
    for witness in [OWNER, OTHER] {
        let entered = events
            .iter()
            .find_map(|e| match e {
                CellToBaseMsg::EnteredAoI {
                    witness_id,
                    entity_id,
                    class_id,
                    npc_data,
                    ..
                } if *witness_id == witness && *entity_id == pet => Some((*class_id, npc_data)),
                _ => None,
            })
            .unwrap_or_else(|| panic!("witness {witness} must meet the pet"));
        assert_eq!(entered.0, SGWPET_CLASS_ID);
        assert_ne!(
            entered.1.as_ref().map_or(0, |d| d.entity_flags) & ENTITYFLAG_PET,
            0,
            "ENTITYFLAG_Pet must ride the cascade's onEntityFlags"
        );
        assert_eq!(
            entered.1.as_ref().and_then(|d| d.pet_owner_id),
            Some(OWNER),
            "the owner binding reaches every witness's cascade"
        );
    }

    let pushes = pet_pushes(&events, pet);
    assert_eq!(
        pushes,
        vec![
            (
                OWNER,
                ON_PET_ABILITY_LIST,
                // [count 2][592][1652]
                vec![2, 0, 0, 0, 0x50, 0x02, 0, 0, 0x74, 0x06, 0, 0],
            ),
            // [count 3][0 1 2]
            (OWNER, ON_PET_STANCE_LIST, vec![3, 0, 0, 0, 0, 1, 2]),
        ],
        "exactly the two lists, to the owner only"
    );

    // Ordering: the lists follow the owner's EnteredAoI for the pet.
    let enter_idx = events
        .iter()
        .position(|e| {
            matches!(e, CellToBaseMsg::EnteredAoI { witness_id, entity_id, .. }
                if *witness_id == OWNER && *entity_id == pet)
        })
        .unwrap();
    let first_list = events
        .iter()
        .position(|e| {
            matches!(e, CellToBaseMsg::WitnessEntityMethod { method_index, .. }
                if *method_index == ON_PET_ABILITY_LIST)
        })
        .unwrap();
    assert!(
        first_list > enter_idx,
        "lists must follow the CREATE_ENTITY"
    );
}

/// Idempotent: the next tick (pet already in the witness set) replays
/// nothing.
#[test]
fn pet_lists_are_sent_once_per_intro() {
    let (mut mgr, pet) = world_with_pet();
    let first = mgr.compute_aoi_changes();
    assert_eq!(pet_pushes(&first, pet).len(), 2);
    let second = mgr.compute_aoi_changes();
    assert!(pet_pushes(&second, pet).is_empty());
}

/// A pet whose stance is not the client default is re-met with
/// `onPetStanceUpdate` after the lists, so the owner's bar shows the real
/// stance.
#[test]
fn non_default_stance_is_replayed_after_the_lists() {
    let (mut mgr, pet) = world_with_pet();
    mgr.get_entity_mut(pet)
        .unwrap()
        .pet
        .as_mut()
        .unwrap()
        .stance = PetStance::Aggressive;
    let events = mgr.compute_aoi_changes();
    let pushes = pet_pushes(&events, pet);
    let methods: Vec<u16> = pushes.iter().map(|p| p.1).collect();
    assert_eq!(
        methods,
        vec![
            ON_PET_ABILITY_LIST,
            ON_PET_STANCE_LIST,
            ON_PET_STANCE_UPDATE
        ]
    );
    assert_eq!(pushes[2], (OWNER, ON_PET_STANCE_UPDATE, vec![2]));
}

/// Copilot, #870: the owner is destroyed and, before the sweep, another
/// player is given its entity id and meets the old pet. That player holds
/// `pet.owner_id` but is not the summoner, so it gets the `EnteredAoI` (the
/// pet is a visible entity) and none of the owner-only lists: a stance list
/// would bind someone else's pet into its `Unit.Pet` slots.
#[test]
fn reused_owner_id_gets_the_pet_but_not_its_lists() {
    let (mut mgr, pet) = world_with_pet();
    super::reuse_owner_id_by_another_player(&mut mgr);

    let events = mgr.compute_aoi_changes();

    assert!(
        events.iter().any(|e| matches!(e,
            CellToBaseMsg::EnteredAoI { witness_id, entity_id, .. }
                if *witness_id == OWNER && *entity_id == pet)),
        "the id's new holder still meets the pet as an entity"
    );
    assert!(
        pet_pushes(&events, pet).is_empty(),
        "no owner-only list may reach a player who only reused the owner's id"
    );
}

/// An ordinary NPC produces no pet pushes for anyone.
#[test]
fn ordinary_npc_sends_no_pet_lists() {
    let mut mgr = make_world();
    add_pet_owner(&mut mgr, OWNER, "Agnos", [10.0, 0.0, 10.0], 5);
    let npc = mgr.allocate_npc_id();
    mgr.spawn_npc(npc, "Agnos", [11.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    let events = mgr.compute_aoi_changes();
    assert!(pet_pushes(&events, npc).is_empty());
    let entered_owner = events.iter().find_map(|e| match e {
        CellToBaseMsg::EnteredAoI {
            entity_id,
            npc_data,
            ..
        } if *entity_id == npc => Some(npc_data.as_ref().and_then(|d| d.pet_owner_id)),
        _ => None,
    });
    assert_eq!(entered_owner, Some(None), "no owner binding on a mob");
}
