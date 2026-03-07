use serde::{Deserialize, Serialize};

/// Shared stats for any "being" entity (player, mob, NPC).
///
/// Corresponds to the SGWBeing base class in the Python scripting layer,
/// which provides health, armor, and level to all living entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeingStats {
    pub health: i32,
    pub max_health: i32,
    pub armor: i32,
    pub level: u32,
}

impl BeingStats {
    /// Create a new `BeingStats` at full health with the given max HP and level.
    pub fn new(max_health: i32, armor: i32, level: u32) -> Self {
        Self {
            health: max_health,
            max_health,
            armor,
            level,
        }
    }

    /// Returns `true` if the being is still alive (health > 0).
    pub fn is_alive(&self) -> bool {
        self.health > 0
    }

    /// Apply damage after armor mitigation. Returns the actual damage dealt.
    ///
    /// Armor reduces incoming damage by a flat amount (minimum 1 damage if
    /// the being is alive and raw damage is positive).
    pub fn take_damage(&mut self, amount: i32) -> i32 {
        if amount <= 0 || !self.is_alive() {
            return 0;
        }

        let mitigated = (amount - self.armor).max(1);
        self.health = (self.health - mitigated).max(0);

        tracing::trace!(
            raw_damage = amount,
            armor = self.armor,
            actual_damage = mitigated,
            remaining_hp = self.health,
            "Damage applied"
        );

        mitigated
    }

    /// Heal the being, clamping at max health. Returns the actual amount healed.
    pub fn heal(&mut self, amount: i32) -> i32 {
        if amount <= 0 {
            return 0;
        }

        let before = self.health;
        self.health = (self.health + amount).min(self.max_health);
        self.health - before
    }

    /// Current health as a percentage of max health (0.0 to 1.0).
    pub fn health_percentage(&self) -> f32 {
        if self.max_health <= 0 {
            return 0.0;
        }
        self.health as f32 / self.max_health as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_being_at_full_health() {
        let b = BeingStats::new(100, 10, 5);
        assert_eq!(b.health, 100);
        assert_eq!(b.max_health, 100);
        assert!(b.is_alive());
    }

    #[test]
    fn take_damage_applies_armor() {
        let mut b = BeingStats::new(100, 10, 5);
        let dealt = b.take_damage(25);
        assert_eq!(dealt, 15); // 25 - 10 armor
        assert_eq!(b.health, 85);
    }

    #[test]
    fn take_damage_minimum_one() {
        let mut b = BeingStats::new(100, 50, 5);
        let dealt = b.take_damage(5); // 5 - 50 = -45, clamped to 1
        assert_eq!(dealt, 1);
        assert_eq!(b.health, 99);
    }

    #[test]
    fn take_damage_clamps_at_zero() {
        let mut b = BeingStats::new(10, 0, 1);
        b.take_damage(100);
        assert_eq!(b.health, 0);
        assert!(!b.is_alive());
    }

    #[test]
    fn heal_clamps_at_max() {
        let mut b = BeingStats::new(100, 0, 1);
        b.take_damage(30);
        let healed = b.heal(50);
        assert_eq!(healed, 30);
        assert_eq!(b.health, 100);
    }

    #[test]
    fn health_percentage() {
        let mut b = BeingStats::new(200, 0, 1);
        b.take_damage(100);
        assert!((b.health_percentage() - 0.5).abs() < f32::EPSILON);
    }
}
