use serde::{Deserialize, Serialize};

/// Damage types matching the EDamageType enum from the database schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DamageType {
    Physical,
    Energy,
    Fire,
    Ice,
    Poison,
}

/// A single damage event flowing through the combat pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamageEvent {
    pub source_entity_id: i32,
    pub target_entity_id: i32,
    pub ability_id: Option<i32>,
    pub damage_type: DamageType,
    pub base_amount: i32,
    pub is_critical: bool,
    pub final_amount: i32,
}

impl DamageEvent {
    /// Create a new damage event before mitigation.
    pub fn new(
        source_entity_id: i32,
        target_entity_id: i32,
        damage_type: DamageType,
        base_amount: i32,
    ) -> Self {
        Self {
            source_entity_id,
            target_entity_id,
            ability_id: None,
            damage_type,
            base_amount,
            is_critical: false,
            final_amount: base_amount,
        }
    }
}

/// Run a damage event through the full combat pipeline: crit check, armor
/// mitigation, and resist calculation.
///
/// Returns the finalized event with `final_amount` set.
pub fn calculate_damage(mut event: DamageEvent, target_armor: i32, crit_chance: f32, crit_mult: f32) -> DamageEvent {
    // Crit roll
    let roll: f32 = rand::random();
    if roll < crit_chance {
        event.is_critical = true;
        event.base_amount = (event.base_amount as f32 * crit_mult) as i32;
    }

    // Armor mitigation (flat reduction, minimum 1)
    event.final_amount = (event.base_amount - target_armor).max(1);

    tracing::trace!(
        source = event.source_entity_id,
        target = event.target_entity_id,
        base = event.base_amount,
        armor = target_armor,
        crit = event.is_critical,
        final_dmg = event.final_amount,
        "Damage calculated"
    );

    event
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn damage_event_minimum_one() {
        let event = DamageEvent::new(1, 2, DamageType::Physical, 5);
        let result = calculate_damage(event, 100, 0.0, 1.0);
        assert_eq!(result.final_amount, 1);
    }

    #[test]
    fn damage_event_no_armor() {
        let event = DamageEvent::new(1, 2, DamageType::Energy, 50);
        let result = calculate_damage(event, 0, 0.0, 1.0);
        assert_eq!(result.final_amount, 50);
    }
}
