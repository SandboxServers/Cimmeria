//! Per-pet runtime state (pets campaign PT-01, issue #570).
//!
//! A pet is an ordinary NPC `CellEntity` (wire class `SGWPet`, 0x05) that
//! carries one extra box of state: who owns it, its stance, and the ability
//! lists the owner's pet bar shows. Everything here is server-side runtime
//! state: never persisted (D-PT01 blocks persistence), and only the ability
//! and stance lists ever reach a client, through the owner-only
//! `onPetAbilityList` / `onPetStanceList` / `onPetStanceUpdate` sends.
//!
//! Field meanings follow `entities/defs/SGWPet.def` (`ownerID`,
//! `transferXP`, `toggledAbilities`, `lastTeleportTime`, `petStance`,
//! `abilityToResolve`). The `.def` keeps them as properties on the pet; they
//! live in one boxed struct here so an NPC that is not a pet pays one
//! pointer, and `entity_struct.rs` (already over the file cap) grows by one
//! line.

use std::time::Instant;

/// `EPetStance` (`entities/defs/enumerations.xml:199-206`,
/// `db/resources/AI/Types/EPetStance.sql`). The discriminants are the wire
/// values of `onPetStanceUpdate(INT8)` and the elements of
/// `onPetStanceList(ARRAY<INT8>)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i8)]
pub enum PetStance {
    /// Never engages on its own.
    Passive = 0,
    /// The `.def` default (`petStance` defaults to 1): defends itself and
    /// its owner.
    Defensive = 1,
    /// Engages hostiles near it.
    Aggressive = 2,
}

impl PetStance {
    /// Every stance, in `EPetStance` order. The stance list sent to the
    /// client is this, filtered by the pet's [`PetState::stance_mask`].
    pub const ALL: [PetStance; 3] = [Self::Passive, Self::Defensive, Self::Aggressive];

    /// The wire / enum value.
    pub fn wire(self) -> i8 {
        self as i8
    }

    /// This stance's bit in [`PetState::stance_mask`] (`1 << value`).
    pub fn mask_bit(self) -> u8 {
        1u8 << (self as u8)
    }

    /// Lower-case name for logs and GM feedback.
    pub fn label(self) -> &'static str {
        match self {
            Self::Passive => "passive",
            Self::Defensive => "defensive",
            Self::Aggressive => "aggressive",
        }
    }
}

/// A stance value outside `EPetStance`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnknownPetStance(pub i8);

impl std::fmt::Display for UnknownPetStance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown EPetStance value {}", self.0)
    }
}

impl std::error::Error for UnknownPetStance {}

impl TryFrom<i8> for PetStance {
    type Error = UnknownPetStance;

    /// Rejects anything outside 0..=2. The client's `changePetStance` (cell
    /// method 90) is untrusted input, so an unknown value must never be
    /// coerced to a default.
    fn try_from(value: i8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Passive),
            1 => Ok(Self::Defensive),
            2 => Ok(Self::Aggressive),
            other => Err(UnknownPetStance(other)),
        }
    }
}

/// Every stance allowed: the mask of a template with none of the
/// `ENTITYFLAG_NoPassive` / `NoDefensive` / `NoAggressive` bits.
pub const ALL_STANCES_MASK: u8 = 0b111;

/// The pet half of a pet's `CellEntity` (`CellEntity::pet`).
#[derive(Debug, Clone, PartialEq)]
pub struct PetState {
    /// Entity id of the owning player (`SGWPet.ownerID`). The ownership
    /// source of truth is `PetRegistry` on the `SpaceManager`; this copy
    /// lets per-entity code (the AoI replay, the AI) read the owner without
    /// a map lookup. The two are written together by the spawn path.
    pub owner_id: u32,
    /// Current stance. Defensive at summon (the `.def` default).
    pub stance: PetStance,
    /// Abilities on the owner's pet bar (`onPetAbilityList`), from the
    /// template's ability set.
    pub ability_list: Vec<i32>,
    /// Abilities the owner toggled OFF (`SGWPet.toggledAbilities`). The AI
    /// never picks one of these.
    pub toggled_off: Vec<i32>,
    /// Share of the pet's kill XP the owner receives (`SGWPet.transferXP`,
    /// `.def` default 1.0; D-PT02).
    pub transfer_xp: f32,
    /// Which stances this pet may take: bit `1 << stance` per allowed
    /// stance, from the template's `ENTITYFLAG_NoPassive` / `NoDefensive` /
    /// `NoAggressive` bits. [`ALL_STANCES_MASK`] when none is set.
    pub stance_mask: u8,
    /// The summon ability that created the pet, `0` for a GM or test spawn.
    pub summon_ability_id: i32,
    /// When the pet was last teleported beside its owner
    /// (`SGWPet.lastTeleportTime`); rate-limits the follow teleport.
    pub last_teleport_at: Option<Instant>,
    /// When a timed pet (or a pet corpse) despawns. `None` means it lives
    /// until its owner or the teardown sweep removes it.
    pub despawn_at: Option<Instant>,
}

impl PetState {
    /// A freshly summoned pet: Defensive when allowed (the `.def` default),
    /// otherwise the first allowed stance, and full XP transfer.
    pub fn new(
        owner_id: u32,
        ability_list: Vec<i32>,
        stance_mask: u8,
        summon_ability_id: i32,
    ) -> Self {
        let stance = if stance_mask & PetStance::Defensive.mask_bit() != 0 {
            PetStance::Defensive
        } else {
            PetStance::ALL
                .into_iter()
                .find(|s| stance_mask & s.mask_bit() != 0)
                // A template that forbids every stance still needs a value;
                // the `.def` default is the least surprising one.
                .unwrap_or(PetStance::Defensive)
        };
        Self {
            owner_id,
            stance,
            ability_list,
            toggled_off: Vec::new(),
            transfer_xp: 1.0,
            stance_mask,
            summon_ability_id,
            last_teleport_at: None,
            despawn_at: None,
        }
    }

    /// The stances this pet may take, in `EPetStance` order: the payload of
    /// `onPetStanceList`.
    pub fn allowed_stances(&self) -> Vec<PetStance> {
        PetStance::ALL
            .into_iter()
            .filter(|s| self.stance_mask & s.mask_bit() != 0)
            .collect()
    }

    /// Whether `stance` is in this pet's stance list.
    pub fn allows(&self, stance: PetStance) -> bool {
        self.stance_mask & stance.mask_bit() != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stance_try_from_accepts_enum_values_and_rejects_the_rest() {
        assert_eq!(PetStance::try_from(0), Ok(PetStance::Passive));
        assert_eq!(PetStance::try_from(1), Ok(PetStance::Defensive));
        assert_eq!(PetStance::try_from(2), Ok(PetStance::Aggressive));
        for bad in [-1i8, 3, 4, i8::MAX, i8::MIN] {
            assert_eq!(PetStance::try_from(bad), Err(UnknownPetStance(bad)));
        }
    }

    #[test]
    fn stance_wire_values_match_epetstance() {
        assert_eq!(PetStance::Passive.wire(), 0);
        assert_eq!(PetStance::Defensive.wire(), 1);
        assert_eq!(PetStance::Aggressive.wire(), 2);
    }

    #[test]
    fn new_pet_defaults_to_defensive_with_full_xp_transfer() {
        let pet = PetState::new(7, vec![592], ALL_STANCES_MASK, 1643);
        assert_eq!(pet.stance, PetStance::Defensive);
        assert_eq!(pet.transfer_xp, 1.0);
        assert_eq!(pet.owner_id, 7);
        assert_eq!(pet.summon_ability_id, 1643);
        assert!(pet.toggled_off.is_empty());
        assert_eq!(
            pet.allowed_stances(),
            vec![
                PetStance::Passive,
                PetStance::Defensive,
                PetStance::Aggressive
            ]
        );
    }

    #[test]
    fn new_pet_without_defensive_takes_the_first_allowed_stance() {
        let mask = PetStance::Aggressive.mask_bit();
        let pet = PetState::new(7, vec![], mask, 0);
        assert_eq!(pet.stance, PetStance::Aggressive);
        assert_eq!(pet.allowed_stances(), vec![PetStance::Aggressive]);
        assert!(!pet.allows(PetStance::Defensive));
    }
}
