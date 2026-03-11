use serde::{Deserialize, Serialize};

/// State of an ability that has been loaded from the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbilityInstance {
    pub ability_id: i32,
    pub name: String,
    pub cooldown_secs: f32,
    pub remaining_cooldown: f32,
    pub energy_cost: i32,
    pub base_damage: i32,
    pub range: f32,
}

impl AbilityInstance {
    /// Create a new ability instance with no cooldown active.
    pub fn new(ability_id: i32, name: String, cooldown_secs: f32, energy_cost: i32, base_damage: i32, range: f32) -> Self {
        Self {
            ability_id,
            name,
            cooldown_secs,
            remaining_cooldown: 0.0,
            energy_cost,
            base_damage,
            range,
        }
    }

    /// Returns `true` if this ability is off cooldown and ready to use.
    pub fn is_ready(&self) -> bool {
        self.remaining_cooldown <= 0.0
    }

    /// Advance the cooldown timer by `delta_secs`.
    pub fn tick_cooldown(&mut self, delta_secs: f32) {
        if self.remaining_cooldown > 0.0 {
            self.remaining_cooldown = (self.remaining_cooldown - delta_secs).max(0.0);
        }
    }

    /// Put this ability on cooldown.
    pub fn start_cooldown(&mut self) {
        self.remaining_cooldown = self.cooldown_secs;
    }
}

/// Attempt to execute an ability from `source` against `target`.
///
/// Validates cooldown, energy, and range before firing.
pub fn execute_ability(
    _ability: &mut AbilityInstance,
    _source_entity_id: i32,
    _target_entity_id: i32,
    _distance: f32,
    _source_energy: i32,
) -> AbilityResult {
    todo!("execute_ability - full ability execution pipeline")
}

/// Outcome of an ability execution attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbilityResult {
    Success,
    OnCooldown,
    NotEnoughEnergy,
    OutOfRange,
    InvalidTarget,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_ability_is_ready() {
        let a = AbilityInstance::new(1, "Zat Blast".to_string(), 3.0, 10, 25, 15.0);
        assert!(a.is_ready());
    }

    #[test]
    fn cooldown_ticks_down() {
        let mut a = AbilityInstance::new(1, "Zat Blast".to_string(), 3.0, 10, 25, 15.0);
        a.start_cooldown();
        assert!(!a.is_ready());
        a.tick_cooldown(2.0);
        assert!(!a.is_ready());
        a.tick_cooldown(1.5);
        assert!(a.is_ready());
    }
}
