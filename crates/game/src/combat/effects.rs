use serde::{Deserialize, Serialize};

/// Type of a buff/debuff effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectType {
    Buff,
    Debuff,
    DamageOverTime,
    HealOverTime,
}

/// A currently active effect ticking on an entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveEffect {
    pub effect_id: i32,
    pub effect_type: EffectType,
    pub source_entity_id: i32,
    pub target_entity_id: i32,
    pub remaining_secs: f32,
    pub tick_interval_secs: f32,
    pub time_until_next_tick: f32,
    pub value_per_tick: i32,
}

impl ActiveEffect {
    /// Create a new active effect.
    pub fn new(
        effect_id: i32,
        effect_type: EffectType,
        source_entity_id: i32,
        target_entity_id: i32,
        duration_secs: f32,
        tick_interval_secs: f32,
        value_per_tick: i32,
    ) -> Self {
        Self {
            effect_id,
            effect_type,
            source_entity_id,
            target_entity_id,
            remaining_secs: duration_secs,
            tick_interval_secs,
            time_until_next_tick: tick_interval_secs,
            value_per_tick,
        }
    }

    /// Returns `true` if the effect has expired.
    pub fn is_expired(&self) -> bool {
        self.remaining_secs <= 0.0
    }
}

/// Advance all active effects by `delta_secs`, returning indices of effects
/// that ticked this frame so the caller can apply damage/healing.
///
/// Expired effects are flagged but not removed (caller should sweep them).
pub fn tick_effects(effects: &mut [ActiveEffect], delta_secs: f32) -> Vec<usize> {
    let mut ticked = Vec::new();

    for (i, effect) in effects.iter_mut().enumerate() {
        if effect.is_expired() {
            continue;
        }

        effect.remaining_secs -= delta_secs;
        effect.time_until_next_tick -= delta_secs;

        if effect.time_until_next_tick <= 0.0 {
            effect.time_until_next_tick += effect.tick_interval_secs;
            ticked.push(i);
        }
    }

    ticked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effect_expires_after_duration() {
        let mut effect = ActiveEffect::new(1, EffectType::DamageOverTime, 1, 2, 5.0, 1.0, 10);
        assert!(!effect.is_expired());

        let mut effects = vec![effect.clone()];
        tick_effects(&mut effects, 5.5);
        assert!(effects[0].is_expired());
    }

    #[test]
    fn tick_fires_at_interval() {
        let effect = ActiveEffect::new(1, EffectType::HealOverTime, 1, 2, 10.0, 2.0, 5);
        let mut effects = vec![effect];

        let ticked = tick_effects(&mut effects, 2.0);
        assert_eq!(ticked.len(), 1);

        let ticked = tick_effects(&mut effects, 1.0);
        assert!(ticked.is_empty());
    }
}
