//! SGWPet's wire contract: its own ClientMethods (indices 29-31), the
//! generic-property id that binds a pet to its owner, and the pet
//! `EEntityFlags` bits (pets campaign PT-01, issue #570).
//!
//! # These indices are SGWPet's, not SGWPlayer's
//!
//! Every other module here numbers SGWPlayer's flattened table. SGWPet
//! (`entities/defs/SGWPet.def`, `<Parent>SGWMob</Parent>`, no
//! `<Implements>`) flattens as SGWMob's 0-28 (SGWSpawnableEntity + SGWBeing
//! 0-26, then SGWMob's own `onAggressionOverrideUpdate` 27 and
//! `onAggressionOverrideCleared` 28) followed by its own three methods, 32 in
//! total, which matches the SGWPet row of `entity-property-sync` App. B.
//! Index 29 on a **player** is a Communicator method; sending one of these
//! against a player id calls the wrong handler. Always address them to the
//! pet's entity id.
//!
//! All three sit below the NPC idbase (62, `IDBASE_NPC_DEFAULT`), so they
//! direct-encode as `0x80 | idx`: `0x9D`, `0x9E`, `0x9F`.
//!
//! Confirmed against the client by PT-E1
//! (`docs/reverse-engineering/findings/pet-client-contract.md`), and pinned
//! by `pet_method_indices_are_pinned`. `pet-restoration.md`'s "idx 0/1/2" is
//! the client's handler registration order, not the wire index.

/// `onPetAbilityList(ARRAY<INT32> aAbilityList)` — the owner's pet bar.
pub const ON_PET_ABILITY_LIST: u16 = 29;
/// `onPetStanceList(ARRAY<INT8> aStanceList)` — the stances the pet may take.
pub const ON_PET_STANCE_LIST: u16 = 30;
/// `onPetStanceUpdate(INT8 aStance)` — the pet's current stance.
pub const ON_PET_STANCE_UPDATE: u16 = 31;

/// `GENERICPROPERTY_PetOwnerId` (`entities/defs/enumerations.xml:1727`).
/// Sent as `onEntityProperty(5, ownerEntityId)` in a pet's
/// `createOnClient` cascade: the server sends NPCs no BigWorld property
/// stream (`propCount = 0`), so the `.def`'s CELL_PUBLIC `ownerID` has no
/// other way to reach the client. The owner's client binds the pet into
/// `Unit.Pet1..4` once the entity carries [`ENTITYFLAG_PET`] and has this
/// property plus `onPetStanceList`, in either order (`pet-client-contract.md`).
pub const GENERICPROPERTY_PET_OWNER_ID: i32 = 5;

// ── EEntityFlags pet bits (`entities/defs/enumerations.xml:1479-1493`) ──
//
// `entity_templates.flags` / `CellEntity::entity_flags` carry these; the
// cascade's `onEntityFlags` ships the whole mask.

/// The pet does not take its owner's level.
pub const ENTITYFLAG_NO_PET_LEVELING: u64 = 8;
/// Other pets may not target this entity.
pub const ENTITYFLAG_NO_PET_TARGETING: u64 = 16;
/// Despawn when the pet gets too far from its owner.
pub const ENTITYFLAG_DESPAWN_ON_OWNER_LEASH: u64 = 32;
/// The pet may not take the Passive stance.
pub const ENTITYFLAG_NO_PASSIVE: u64 = 64;
/// The pet may not take the Defensive stance.
pub const ENTITYFLAG_NO_DEFENSIVE: u64 = 128;
/// The pet may not take the Aggressive stance.
pub const ENTITYFLAG_NO_AGGRESSIVE: u64 = 256;
/// A detection pet.
pub const ENTITYFLAG_DETECTION_PET: u64 = 512;
/// The entity is a pet.
pub const ENTITYFLAG_PET: u64 = 1024;
/// Despawn when the (mob) owner leashes.
pub const ENTITYFLAG_DESPAWN_ON_LEASH_FROM_OWNER: u64 = 32768;
/// The pet keeps its own faction instead of taking its owner's.
pub const ENTITYFLAG_PET_USE_OWN_FACTION: u64 = 65536;
/// The pet waits before despawning.
pub const ENTITYFLAG_PET_WAIT_TO_DESPAWN: u64 = 131072;

/// Args of `onPetAbilityList`: `[count u32 LE][count × i32 LE]` (BigWorld
/// `ARRAY<INT32>`).
pub fn build_pet_ability_list(abilities: &[i32]) -> Vec<u8> {
    let mut args = Vec::with_capacity(4 + abilities.len() * 4);
    args.extend_from_slice(&(abilities.len() as u32).to_le_bytes());
    for &id in abilities {
        args.extend_from_slice(&id.to_le_bytes());
    }
    args
}

/// Args of `onPetStanceList`: `[count u32 LE][count × i8]` (BigWorld
/// `ARRAY<INT8>`). `pet-wire-formats.md` once had these as INT32; the
/// `.def` says INT8.
pub fn build_pet_stance_list(stances: &[i8]) -> Vec<u8> {
    let mut args = Vec::with_capacity(4 + stances.len());
    args.extend_from_slice(&(stances.len() as u32).to_le_bytes());
    args.extend(stances.iter().map(|&s| s as u8));
    args
}

/// Args of `onPetStanceUpdate`: one `INT8`.
pub fn build_pet_stance_update(stance: i8) -> Vec<u8> {
    vec![stance as u8]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Derived from the `.def` flattening (SGWMob 0-28 + 3) and confirmed
    /// against the client by PT-E1 (`pet-client-contract.md`). Changing one
    /// means changing the SGWPet table in
    /// `docs/protocol/client-method-dispatch-table.md` too.
    #[test]
    fn pet_method_indices_are_pinned() {
        assert_eq!(ON_PET_ABILITY_LIST, 29);
        assert_eq!(ON_PET_STANCE_LIST, 30);
        assert_eq!(ON_PET_STANCE_UPDATE, 31);
        assert_eq!(GENERICPROPERTY_PET_OWNER_ID, 5);
    }

    #[test]
    fn pet_flag_values_match_enumerations_xml() {
        assert_eq!(ENTITYFLAG_NO_PET_LEVELING, 8);
        assert_eq!(ENTITYFLAG_NO_PET_TARGETING, 16);
        assert_eq!(ENTITYFLAG_DESPAWN_ON_OWNER_LEASH, 32);
        assert_eq!(ENTITYFLAG_NO_PASSIVE, 64);
        assert_eq!(ENTITYFLAG_NO_DEFENSIVE, 128);
        assert_eq!(ENTITYFLAG_NO_AGGRESSIVE, 256);
        assert_eq!(ENTITYFLAG_DETECTION_PET, 512);
        assert_eq!(ENTITYFLAG_PET, 1024);
        assert_eq!(ENTITYFLAG_DESPAWN_ON_LEASH_FROM_OWNER, 32768);
        assert_eq!(ENTITYFLAG_PET_USE_OWN_FACTION, 65536);
        assert_eq!(ENTITYFLAG_PET_WAIT_TO_DESPAWN, 131072);
    }

    #[test]
    fn ability_list_is_u32_count_then_i32s() {
        assert_eq!(
            build_pet_ability_list(&[1643, -1]),
            vec![
                0x02, 0x00, 0x00, 0x00, // count = 2
                0x6B, 0x06, 0x00, 0x00, // 1643
                0xFF, 0xFF, 0xFF, 0xFF, // -1
            ]
        );
        assert_eq!(build_pet_ability_list(&[]), vec![0, 0, 0, 0]);
    }

    #[test]
    fn stance_list_is_u32_count_then_one_byte_per_stance() {
        assert_eq!(
            build_pet_stance_list(&[0, 1, 2]),
            vec![0x03, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02]
        );
        assert_eq!(build_pet_stance_list(&[]), vec![0, 0, 0, 0]);
    }

    #[test]
    fn stance_update_is_one_byte() {
        assert_eq!(build_pet_stance_update(1), vec![0x01]);
        assert_eq!(build_pet_stance_update(2), vec![0x02]);
    }
}
