use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Stat identifiers matching the EStats enum from the database schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Stat {
    MaxHealth,
    MaxEnergy,
    Armor,
    Damage,
    CritChance,
    CritMultiplier,
    MovementSpeed,
    AttackSpeed,
}

/// Source of a stat modifier, used for stacking rules.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModifierSource {
    Base,
    Equipment(i32),
    Buff(i32),
    Debuff(i32),
    Aura(i32),
}

/// A single modifier applied to a stat.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatModifier {
    pub source: ModifierSource,
    pub stat: Stat,
    /// Flat additive bonus applied before multipliers.
    pub flat: f32,
    /// Multiplicative bonus (1.0 = no change, 1.5 = +50%).
    pub multiplier: f32,
}

/// A block of stats with base values and a stack of modifiers.
///
/// Final stat values are computed as: (base + sum(flat)) * product(multipliers).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatBlock {
    base_values: HashMap<Stat, f32>,
    modifiers: Vec<StatModifier>,
}

impl StatBlock {
    /// Create a new stat block with default base values.
    pub fn new() -> Self {
        let mut base_values = HashMap::new();
        base_values.insert(Stat::MaxHealth, 100.0);
        base_values.insert(Stat::MaxEnergy, 50.0);
        base_values.insert(Stat::Armor, 0.0);
        base_values.insert(Stat::Damage, 10.0);
        base_values.insert(Stat::CritChance, 0.05);
        base_values.insert(Stat::CritMultiplier, 1.5);
        base_values.insert(Stat::MovementSpeed, 1.0);
        base_values.insert(Stat::AttackSpeed, 1.0);

        Self {
            base_values,
            modifiers: Vec::new(),
        }
    }

    /// Set the base value for a stat.
    pub fn set_base(&mut self, stat: Stat, value: f32) {
        self.base_values.insert(stat, value);
    }

    /// Add a modifier to the stack.
    pub fn add_modifier(&mut self, modifier: StatModifier) {
        self.modifiers.push(modifier);
    }

    /// Remove all modifiers from a given source.
    pub fn remove_modifiers_from(&mut self, source: &ModifierSource) {
        self.modifiers.retain(|m| &m.source != source);
    }

    /// Compute the final value of a stat after all modifiers.
    pub fn get(&self, stat: Stat) -> f32 {
        let base = self.base_values.get(&stat).copied().unwrap_or(0.0);

        let flat_sum: f32 = self
            .modifiers
            .iter()
            .filter(|m| m.stat == stat)
            .map(|m| m.flat)
            .sum();

        let mult_product: f32 = self
            .modifiers
            .iter()
            .filter(|m| m.stat == stat)
            .map(|m| m.multiplier)
            .product();

        (base + flat_sum) * mult_product
    }
}

impl Default for StatBlock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_values_returned_without_modifiers() {
        let block = StatBlock::new();
        assert!((block.get(Stat::MaxHealth) - 100.0).abs() < f32::EPSILON);
        assert!((block.get(Stat::Damage) - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn flat_modifier_adds_to_base() {
        let mut block = StatBlock::new();
        block.add_modifier(StatModifier {
            source: ModifierSource::Equipment(1),
            stat: Stat::Damage,
            flat: 5.0,
            multiplier: 1.0,
        });
        assert!((block.get(Stat::Damage) - 15.0).abs() < f32::EPSILON);
    }

    #[test]
    fn multiplier_scales_total() {
        let mut block = StatBlock::new();
        block.add_modifier(StatModifier {
            source: ModifierSource::Buff(1),
            stat: Stat::Damage,
            flat: 0.0,
            multiplier: 1.5,
        });
        assert!((block.get(Stat::Damage) - 15.0).abs() < f32::EPSILON);
    }

    #[test]
    fn remove_modifiers_by_source() {
        let mut block = StatBlock::new();
        let source = ModifierSource::Buff(99);
        block.add_modifier(StatModifier {
            source: source.clone(),
            stat: Stat::Armor,
            flat: 20.0,
            multiplier: 1.0,
        });
        assert!((block.get(Stat::Armor) - 20.0).abs() < f32::EPSILON);

        block.remove_modifiers_from(&source);
        assert!((block.get(Stat::Armor) - 0.0).abs() < f32::EPSILON);
    }
}
