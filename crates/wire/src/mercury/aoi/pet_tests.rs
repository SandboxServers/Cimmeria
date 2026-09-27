//! Byte-exact tests for a pet's introduction (pets campaign PT-01): the
//! class-0x05 CREATE_ENTITY and the owner binding in the `createOnClient`
//! cascade.
//!
//! The client binds a pet to its owner's `Unit.Pet1..4` from
//! `ENTITYFLAG_Pet` in `onEntityFlags` plus `onEntityProperty(PetOwnerId,
//! owner)` plus the owner-only `onPetStanceList`
//! (`docs/reverse-engineering/findings/pet-client-contract.md`). The first two
//! are in the cascade and pinned here; the list is pinned in `cell-world`.

use super::{compose_create_entity_base_body, compose_create_entity_cascade_body};
use crate::cell::client_methods::pet::GENERICPROPERTY_PET_OWNER_ID;
use crate::cell::messages::NpcAoIData;
use crate::mercury::{method_idx, SGWPET_CLASS_ID};

const PET_ID: u32 = 0x0001_86A5; // 100_005
const OWNER_ID: u32 = 0x0000_0102; // 258

fn pet_npc_data(owner: Option<u32>) -> NpcAoIData {
    NpcAoIData {
        static_mesh: Some("CA-Props.CA-Crate".to_string()),
        body_set: Some("GLB_Components.WorldObject_Small".to_string()),
        pet_owner_id: owner,
        ..NpcAoIData::default()
    }
}

/// The exact `onEntityProperty(GENERICPROPERTY_PetOwnerId, owner)` message:
/// direct-encoded method 7 (`0x87`), `u16` payload length 12 (entity id +
/// two INT32 args), the pet's id, property id 5, the owner's id.
fn owner_binding_bytes() -> Vec<u8> {
    let mut expected = vec![0x87, 0x0C, 0x00];
    expected.extend_from_slice(&PET_ID.to_le_bytes());
    expected.extend_from_slice(&5i32.to_le_bytes());
    expected.extend_from_slice(&(OWNER_ID as i32).to_le_bytes());
    expected
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// CREATE_ENTITY for a pet carries class byte 0x05 at body offset 8 —
/// the byte that makes the client build a `GamePet` rather than a
/// `GameMob`.
#[test]
fn pet_create_entity_carries_class_0x05() {
    let body = compose_create_entity_base_body(PET_ID, SGWPET_CLASS_ID, [1.0, 2.0, 3.0], [0.0; 3]);
    assert_eq!(
        &body[..11],
        &[
            0x09, // BASEMSG_CREATE_ENTITY
            0x08, 0x00, // wordLength = 8
            0xA5, 0x86, 0x01, 0x00, // entity id 100_005
            0xFF, // idAlias: none
            0x05, // class id: SGWPet
            0x00, 0x00,
        ]
    );
}

/// A pet's cascade contains the owner binding exactly once.
#[test]
fn pet_cascade_emits_owner_binding() {
    assert_eq!(GENERICPROPERTY_PET_OWNER_ID, 5);
    assert_eq!(method_idx::ON_ENTITY_PROPERTY, 7);
    let body = compose_create_entity_cascade_body(
        PET_ID,
        SGWPET_CLASS_ID,
        12,
        Some(&pet_npc_data(Some(OWNER_ID))),
    );
    let binding = owner_binding_bytes();
    let at = find(&body, &binding).expect("cascade must carry onEntityProperty(PetOwnerId, owner)");
    assert!(
        find(&body[at + binding.len()..], &binding).is_none(),
        "the owner binding is sent once"
    );
}

/// The owner binding must come right AFTER `onEntityFlags`:
/// `GamePet__OnOwnerIdChanged` is gated on `ENTITYFLAG_Pet` already being
/// set, so an owner property sent before the flags is ignored and the pet
/// never binds to `Unit.PetN` (Copilot review on #870). With a speaker id,
/// `DatabaseId` still opens the cascade.
#[test]
fn pet_owner_binding_follows_entity_flags() {
    use crate::cell::client_methods::pet::ENTITYFLAG_PET;
    let mut npc = pet_npc_data(Some(OWNER_ID));
    npc.speaker_id = Some(77);
    npc.entity_flags = ENTITYFLAG_PET;
    let body = compose_create_entity_cascade_body(PET_ID, SGWPET_CLASS_ID, 12, Some(&npc));
    let mut database_id = vec![0x87, 0x0C, 0x00];
    database_id.extend_from_slice(&PET_ID.to_le_bytes());
    database_id.extend_from_slice(&9i32.to_le_bytes());
    database_id.extend_from_slice(&77i32.to_le_bytes());
    assert_eq!(&body[..database_id.len()], database_id.as_slice());
    let mut flags = vec![0x84, 0x0C, 0x00];
    flags.extend_from_slice(&PET_ID.to_le_bytes());
    flags.extend_from_slice(&1024u64.to_le_bytes());
    let flags_at = find(&body, &flags).expect("onEntityFlags(ENTITYFLAG_Pet) present");
    let binding = owner_binding_bytes();
    let after = flags_at + flags.len();
    assert_eq!(
        &body[after..after + binding.len()],
        binding.as_slice(),
        "onEntityProperty(PetOwnerId) must immediately follow onEntityFlags"
    );
}

/// Negative: an ordinary NPC (no owner) gets no `PetOwnerId` property.
/// Guards against the binding leaking onto every mob.
#[test]
fn non_pet_cascade_has_no_owner_binding() {
    let body =
        compose_create_entity_cascade_body(PET_ID, SGWPET_CLASS_ID, 12, Some(&pet_npc_data(None)));
    let mut prefix = vec![0x87, 0x0C, 0x00];
    prefix.extend_from_slice(&PET_ID.to_le_bytes());
    prefix.extend_from_slice(&5i32.to_le_bytes());
    assert!(
        find(&body, &prefix).is_none(),
        "no onEntityProperty(PetOwnerId) without an owner"
    );
}

/// The bind's third leg: `ENTITYFLAG_Pet` (1024) must reach the client in
/// the cascade's `onEntityFlags` (`GamePet__OnOwnerIdChanged` is gated on
/// it). Pins the exact message: direct method 4 (`0x84`), payload length 12
/// (entity id + UINT64), the flags.
#[test]
fn pet_cascade_carries_the_pet_flag() {
    use crate::cell::client_methods::pet::ENTITYFLAG_PET;
    let mut npc = pet_npc_data(Some(OWNER_ID));
    npc.entity_flags = ENTITYFLAG_PET;
    let body = compose_create_entity_cascade_body(PET_ID, SGWPET_CLASS_ID, 12, Some(&npc));
    let mut flags = vec![0x84, 0x0C, 0x00];
    flags.extend_from_slice(&PET_ID.to_le_bytes());
    flags.extend_from_slice(&1024u64.to_le_bytes());
    assert!(
        find(&body, &flags).is_some(),
        "onEntityFlags(ENTITYFLAG_Pet) missing from the pet cascade"
    );
}

/// A pet (class 0x05, not 0x00) still gets the SGWBeing half of the
/// cascade: level, target, alignment, faction, state, stats. The level
/// the spawn path stamps (the owner's, D-PT02) must reach the client.
#[test]
fn pet_cascade_includes_the_being_half() {
    let body = compose_create_entity_cascade_body(
        PET_ID,
        SGWPET_CLASS_ID,
        12,
        Some(&pet_npc_data(Some(OWNER_ID))),
    );
    // onLevelUpdate: 0x80|15, len 8, pet id, INT32 level 12.
    let mut level = vec![0x8F, 0x08, 0x00];
    level.extend_from_slice(&PET_ID.to_le_bytes());
    level.extend_from_slice(&12i32.to_le_bytes());
    assert!(
        find(&body, &level).is_some(),
        "class 0x05 must get onLevelUpdate(level)"
    );
}
