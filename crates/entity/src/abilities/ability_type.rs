//! `resources.abilities.type_id`: the ability's authored category.
//!
//! The column is the Postgres enum `"EAbilityTypes"`
//! (`db/resources/Abilities/Types/EAbilityTypes.sql`), the same six tokens
//! the client's `EAbilityType` carries. The server reads it for one rule so
//! far: an `ABILITY_TYPE_Heal` ability is beneficial (ability-mechanics
//! D-AB02, [`super::ability_is_beneficial`]).

/// An ability's `type_id`. [`AbilityType::Undefined`] is the default, so a
/// hand-built test def is neither a heal nor anything else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AbilityType {
    #[default]
    Undefined,
    Buff,
    Debuff,
    Heal,
    Dot,
    DirectDamage,
}

impl AbilityType {
    /// Parse the enum label as stored in the seed (`ABILITY_TYPE_Heal`).
    /// An unknown label is `None`; the loader logs it and keeps the ability
    /// as [`AbilityType::Undefined`].
    pub fn from_db_label(label: &str) -> Option<Self> {
        Some(match label {
            "ABILITY_TYPE_Undefined" => Self::Undefined,
            "ABILITY_TYPE_Buff" => Self::Buff,
            "ABILITY_TYPE_Debuff" => Self::Debuff,
            "ABILITY_TYPE_Heal" => Self::Heal,
            "ABILITY_TYPE_DOT" => Self::Dot,
            "ABILITY_TYPE_DD" => Self::DirectDamage,
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_seed_label_parses_and_unknown_ones_do_not() {
        for (label, want) in [
            ("ABILITY_TYPE_Undefined", AbilityType::Undefined),
            ("ABILITY_TYPE_Buff", AbilityType::Buff),
            ("ABILITY_TYPE_Debuff", AbilityType::Debuff),
            ("ABILITY_TYPE_Heal", AbilityType::Heal),
            ("ABILITY_TYPE_DOT", AbilityType::Dot),
            ("ABILITY_TYPE_DD", AbilityType::DirectDamage),
        ] {
            assert_eq!(AbilityType::from_db_label(label), Some(want), "{label}");
        }
        assert_eq!(AbilityType::from_db_label("ABILITY_TYPE_heal"), None);
        assert_eq!(AbilityType::from_db_label(""), None);
    }
}
