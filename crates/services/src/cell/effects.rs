//! Duration-based effect (buff/debuff) system for the CellService.
//!
//! Abilities can apply timed stat modifiers to entities. Each active effect
//! has a duration and a set of stat changes that are applied on activation
//! and removed on expiration.
//!
//! Reference: `python/cell/AbilityManager.py:applyDurationEffect()`

use std::collections::HashMap;
use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use cimmeria_entity::abilities::{
    TIMER_DURATION_EFFECT,
    serialize_timer_update,
};

use super::messages::CellToBaseMsg;
use super::space_manager::SpaceManager;

// ── Effect tracking ─────────────────────────────────────────────────────────

/// A single stat modification applied by a duration effect.
#[derive(Debug, Clone)]
pub struct StatModifier {
    pub stat_id: i32,
    pub delta: i32,
}

/// An active duration effect on an entity.
#[derive(Debug, Clone)]
pub struct ActiveEffect {
    /// Unique effect instance ID.
    pub effect_id: i32,
    /// Ability that applied this effect.
    pub ability_id: i32,
    /// Source entity that applied the effect.
    pub source_entity_id: u32,
    /// When the effect expires.
    pub expires_at: Instant,
    /// Total duration (for client timer display).
    pub total_duration: Duration,
    /// Stat modifiers applied while this effect is active.
    pub modifiers: Vec<StatModifier>,
}

/// Per-entity effect tracker.
///
/// Stored in the CellService's effect manager (not on CellEntity, since
/// effects need to be ticked globally).
#[derive(Debug, Default)]
pub struct EntityEffects {
    pub effects: Vec<ActiveEffect>,
}

impl EntityEffects {
    pub fn add_effect(&mut self, effect: ActiveEffect) {
        self.effects.push(effect);
    }

    /// Remove expired effects and return the stat deltas to reverse.
    pub fn expire_effects(&mut self) -> Vec<(i32, Vec<StatModifier>)> {
        let now = Instant::now();
        let mut expired = Vec::new();

        self.effects.retain(|effect| {
            if now >= effect.expires_at {
                expired.push((effect.effect_id, effect.modifiers.clone()));
                false
            } else {
                true
            }
        });

        expired
    }

    pub fn has_effects(&self) -> bool {
        !self.effects.is_empty()
    }
}

// ── Global effect manager ───────────────────────────────────────────────────

/// Manages all active duration effects across all entities.
pub struct EffectManager {
    /// entity_id → active effects
    entity_effects: HashMap<u32, EntityEffects>,
    /// Next unique effect instance ID.
    next_effect_id: i32,
}

impl EffectManager {
    pub fn new() -> Self {
        Self {
            entity_effects: HashMap::new(),
            next_effect_id: 10000, // Start high to avoid collision with ability effect IDs
        }
    }

    /// Apply a duration effect to an entity.
    ///
    /// Applies the stat modifiers immediately and schedules expiration.
    /// Sends `onTimerUpdate` to the client to show the buff/debuff timer.
    pub async fn apply_effect(
        &mut self,
        target_entity_id: u32,
        source_entity_id: u32,
        ability_id: i32,
        modifiers: Vec<StatModifier>,
        duration_secs: f32,
        tx: &mpsc::Sender<CellToBaseMsg>,
        space_mgr: &mut SpaceManager,
    ) {
        let effect_id = self.next_effect_id;
        self.next_effect_id += 1;

        // Apply stat modifiers to the target entity
        if let Some(entity) = space_mgr.get_entity_mut(target_entity_id) {
            for modifier in &modifiers {
                if let Some(stat) = entity.stats.get_mut(modifier.stat_id) {
                    stat.change(modifier.delta);
                }
            }

            // Send stat update to client
            let stat_update = entity.stats.serialize_dirty();
            entity.stats.clear_dirty();

            if !stat_update.is_empty() {
                let _ = tx.send(CellToBaseMsg::EntityMethodCall {
                    entity_id: target_entity_id,
                    method_index: 20, // onStatUpdate
                    args: stat_update,
                }).await;
            }
        }

        // Send timer to client
        let timer_args = serialize_timer_update(
            effect_id,
            TIMER_DURATION_EFFECT,
            source_entity_id as i32,
            duration_secs,
            0.0, // expireTime (absolute game time — TODO)
        );
        let _ = tx.send(CellToBaseMsg::EntityMethodCall {
            entity_id: target_entity_id,
            method_index: 12, // onTimerUpdate
            args: timer_args,
        }).await;

        // Store the active effect
        let active = ActiveEffect {
            effect_id,
            ability_id,
            source_entity_id,
            expires_at: Instant::now() + Duration::from_secs_f32(duration_secs),
            total_duration: Duration::from_secs_f32(duration_secs),
            modifiers,
        };

        self.entity_effects
            .entry(target_entity_id)
            .or_default()
            .add_effect(active);

        tracing::debug!(
            target = target_entity_id,
            source = source_entity_id,
            ability_id,
            effect_id,
            duration_secs,
            "Duration effect applied"
        );
    }

    /// Process all expired effects — reverses stat modifiers and sends updates.
    ///
    /// Called once per tick from the cell loop.
    pub async fn tick(
        &mut self,
        tx: &mpsc::Sender<CellToBaseMsg>,
        space_mgr: &mut SpaceManager,
    ) {
        let mut to_remove = Vec::new();

        for (&entity_id, entity_effects) in self.entity_effects.iter_mut() {
            let expired = entity_effects.expire_effects();

            for (effect_id, modifiers) in expired {
                // Reverse stat modifiers
                if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                    for modifier in &modifiers {
                        if let Some(stat) = entity.stats.get_mut(modifier.stat_id) {
                            stat.change(-modifier.delta); // Reverse the modifier
                        }
                    }

                    let stat_update = entity.stats.serialize_dirty();
                    entity.stats.clear_dirty();

                    if !stat_update.is_empty() {
                        let _ = tx.send(CellToBaseMsg::EntityMethodCall {
                            entity_id,
                            method_index: 20, // onStatUpdate
                            args: stat_update,
                        }).await;
                    }
                }

                tracing::debug!(entity_id, effect_id, "Duration effect expired — stats reversed");
            }

            if !entity_effects.has_effects() {
                to_remove.push(entity_id);
            }
        }

        // Clean up entities with no active effects
        for entity_id in to_remove {
            self.entity_effects.remove(&entity_id);
        }
    }

    /// Remove all effects from an entity (e.g., on death or disconnect).
    pub fn clear_entity_effects(&mut self, entity_id: u32) {
        self.entity_effects.remove(&entity_id);
    }

    /// Number of entities with active effects.
    pub fn active_entity_count(&self) -> usize {
        self.entity_effects.len()
    }

    /// Total number of active effects across all entities.
    pub fn total_effect_count(&self) -> usize {
        self.entity_effects.values().map(|e| e.effects.len()).sum()
    }
}

impl Default for EffectManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_effects_expire() {
        let mut effects = EntityEffects::default();

        // Add an effect that expires immediately
        effects.add_effect(ActiveEffect {
            effect_id: 1,
            ability_id: 100,
            source_entity_id: 1,
            expires_at: Instant::now() - Duration::from_secs(1), // Already expired
            total_duration: Duration::from_secs(5),
            modifiers: vec![StatModifier { stat_id: 10, delta: 50 }],
        });

        // Add an effect that hasn't expired
        effects.add_effect(ActiveEffect {
            effect_id: 2,
            ability_id: 200,
            source_entity_id: 1,
            expires_at: Instant::now() + Duration::from_secs(60),
            total_duration: Duration::from_secs(60),
            modifiers: vec![StatModifier { stat_id: 20, delta: -10 }],
        });

        let expired = effects.expire_effects();
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].0, 1); // effect_id 1 expired
        assert_eq!(effects.effects.len(), 1); // effect 2 still active
    }

    #[test]
    fn effect_manager_basics() {
        let mut mgr = EffectManager::new();
        assert_eq!(mgr.active_entity_count(), 0);
        assert_eq!(mgr.total_effect_count(), 0);

        mgr.entity_effects.entry(1).or_default().add_effect(ActiveEffect {
            effect_id: 1,
            ability_id: 100,
            source_entity_id: 2,
            expires_at: Instant::now() + Duration::from_secs(60),
            total_duration: Duration::from_secs(60),
            modifiers: vec![],
        });

        assert_eq!(mgr.active_entity_count(), 1);
        assert_eq!(mgr.total_effect_count(), 1);

        mgr.clear_entity_effects(1);
        assert_eq!(mgr.active_entity_count(), 0);
    }
}
