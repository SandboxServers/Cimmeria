//! The `abilityBitfield` a minigame launch carries: the player's known
//! abilities, one bit per group of ability ids (MG-22, D-MG18).
//!
//! The bit numbers and their ability ids come from the original client's
//! launch flow (bits 0-14, the cheat set) and its basic humanoid converse
//! (bits 15-18, the Trump set). Bit 5 has no ability.

use cimmeria_entity::cell_entity::CellEntity;

/// Bit n of abilityBitfield and the abilities that grant it
/// (spec-launch-flow § 4 for 0-14, spec-conversebasichumanoid § 7 for 15-18).
pub const ABILITY_BITS: &[(u32, &[i32])] = &[
    (0, &[677, 690, 698]),
    (1, &[680, 691]),
    (2, &[683, 692]),
    (3, &[685, 693, 699]),
    (4, &[678]),
    (6, &[684]),
    (7, &[687, 700]),
    (8, &[688, 704]),
    (9, &[681, 689, 701]),
    (10, &[694, 702]),
    (11, &[695, 703]),
    (12, &[679, 696]),
    (13, &[682, 697]),
    (14, &[686]),
    (15, &[778]),
    (16, &[779]),
    (17, &[792]),
    (18, &[793]),
];

/// The mask for the abilities `knows` reports as known. A bit is set when any
/// ability that grants it is known.
pub fn abilities_mask_from(knows: impl Fn(i32) -> bool) -> u32 {
    let mut mask = 0;
    for &(bit, ids) in ABILITY_BITS {
        if ids.iter().any(|&id| knows(id)) {
            mask |= 1 << bit;
        }
    }
    mask
}

/// The mask for a live cell entity's known abilities.
pub fn abilities_mask(entity: &CellEntity) -> u32 {
    abilities_mask_from(|id| entity.abilities.has_ability(id))
}
