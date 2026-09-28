//! `DeployableRegistry`: every live deployable and the owner that placed it,
//! plus the ground point a deployable cast is waiting to use.
//!
//! The registry is the source of truth for "whose is this object": the
//! pulse tick reads the owner from here, never from anything a client sent,
//! and the owner check is the summon-time identity (account and character),
//! not the bare entity id, which is reused after a logout.

use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, Instant};

use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::PlayerIdentity;

/// Running totals for one deployable, reported when it despawns.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PulseTotals {
    /// Pulses that ran (including pulses that found no target).
    pub pulses: u32,
    /// Targets hit, summed over every pulse.
    pub hits: u32,
    /// HEALTH taken off targets, summed.
    pub health_damage: i64,
    /// FOCUS taken off targets, summed.
    pub focus_damage: i64,
    /// Targets a pulse killed.
    pub kills: u32,
}

/// One live deployable.
#[derive(Debug, Clone)]
pub struct DeployableState {
    /// The player that placed it. Damage, threat, XP and kill credit are
    /// all this entity's.
    pub owner: u32,
    /// The owner's identity when it placed the deployable. The owner check
    /// compares against this, so a player given a departed owner's entity
    /// id never inherits the object.
    pub owner_identity: PlayerIdentity,
    /// The ability that placed it.
    pub ability_id: i32,
    /// Its `entity_templates` row.
    pub template_id: i32,
    /// The effect each pulse applies (`resources.deployables.pulse_effect_id`).
    pub pulse_effect_id: i32,
    /// Hit radius around the deployable, metres.
    pub radius: f32,
    /// Time between pulses.
    pub pulse_interval: Duration,
    /// Pulses it fires before it despawns. Its lifetime is
    /// `pulses_total * pulse_interval`.
    pub pulses_total: u32,
    /// When it was placed.
    pub spawned_at: Instant,
    /// When the next pulse is due.
    pub next_pulse_at: Instant,
    /// What it has done so far.
    pub totals: PulseTotals,
}

impl DeployableState {
    /// True once every pulse has run.
    pub fn is_spent(&self) -> bool {
        self.totals.pulses >= self.pulses_total
    }
}

/// A validated ground point waiting for its cast to fire.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StagedPoint {
    /// The deployable ability the point was validated for.
    pub ability_id: i32,
    /// The point, already snapped to the floor where a navmesh covers it.
    pub point: Vector3,
}

/// Live deployables (keyed by entity id, so iteration runs oldest first:
/// NPC ids only grow) and staged ground points (keyed by caster).
#[derive(Debug, Default)]
pub struct DeployableRegistry {
    live: BTreeMap<u32, DeployableState>,
    staged: HashMap<u32, StagedPoint>,
}

impl DeployableRegistry {
    /// Record a newly placed deployable.
    pub fn register(&mut self, entity_id: u32, state: DeployableState) {
        self.live.insert(entity_id, state);
    }

    /// The deployable `entity_id`, if it is one.
    pub fn get(&self, entity_id: u32) -> Option<&DeployableState> {
        self.live.get(&entity_id)
    }

    /// Mutable access to the deployable `entity_id`.
    pub fn get_mut(&mut self, entity_id: u32) -> Option<&mut DeployableState> {
        self.live.get_mut(&entity_id)
    }

    /// Drop `entity_id` from the registry, returning what it held.
    pub fn forget(&mut self, entity_id: u32) -> Option<DeployableState> {
        self.live.remove(&entity_id)
    }

    /// Every live deployable id, oldest first.
    pub fn ids(&self) -> Vec<u32> {
        self.live.keys().copied().collect()
    }

    /// `owner`'s live deployables from `ability_id`, oldest first.
    pub fn of_owner(&self, owner: u32, ability_id: i32) -> Vec<u32> {
        self.live
            .iter()
            .filter(|(_, s)| s.owner == owner && s.ability_id == ability_id)
            .map(|(&id, _)| id)
            .collect()
    }

    /// Number of live deployables.
    pub fn len(&self) -> usize {
        self.live.len()
    }

    /// True when no deployable is live.
    pub fn is_empty(&self) -> bool {
        self.live.is_empty()
    }

    /// Hold `point` for `caster`'s next `ability_id` fire, replacing any
    /// point it already held.
    pub fn stage(&mut self, caster: u32, ability_id: i32, point: Vector3) {
        self.staged
            .insert(caster, StagedPoint { ability_id, point });
    }

    /// The point staged for `caster`, if it is for `ability_id`. Leaves it
    /// in place.
    pub fn staged_for(&self, caster: u32, ability_id: i32) -> Option<Vector3> {
        self.staged
            .get(&caster)
            .filter(|s| s.ability_id == ability_id)
            .map(|s| s.point)
    }

    /// Take the point staged for `caster` when it is for `ability_id`. A
    /// point for another ability is dropped, never used.
    pub fn take_staged(&mut self, caster: u32, ability_id: i32) -> Option<Vector3> {
        self.staged
            .remove(&caster)
            .filter(|s| s.ability_id == ability_id)
            .map(|s| s.point)
    }

    /// Drop whatever point `caster` has staged.
    pub fn clear_staged(&mut self, caster: u32) {
        self.staged.remove(&caster);
    }
}
