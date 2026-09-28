//! `spawn_deployable`: put an owned, stationary deployable at a ground
//! point.
//!
//! Built on the cached `entity_templates` prototype, like pets and content
//! spawns, then reshaped before the entity exists:
//!
//! - wire class `SGWBeing` (0x01) whatever the template says. A being is
//!   left out of every AoE and cone candidate list (they scan SGWMob only),
//!   never gets an AI fight pass, and `generate_threat` refuses it, so no
//!   player, pet or mob can target or attack it (Phase 0 keeps it out of
//!   NPC-versus-NPC combat, #1009). It is deliberately not an `SGWPet`: the
//!   owner's client would bind a pet into its pet bar.
//! - the owner's faction, so the #444 gate refuses a player's shot at it;
//! - stationary, no loot, respawn, patrol, wander, cover or tag.
//!
//! The lifetime and pulse cadence come from the spec's lifetime effect
//! (`pulse_count` x `pulse_duration`), the hit radius from the pulse
//! effect's `Radius` NVP, else the client's AE radius for its `tcm_param1`
//! tier. Nothing is sent
//! from here: the next AoI tick introduces the object like any NPC.

use std::time::{Duration, Instant};

use cimmeria_common::Vector3;
use cimmeria_entity::abilities::{ae_radius_metres, EffectDef};

use super::super::space_manager::SpaceManager;
use super::super::spawner::DeployableSpec;
use super::registry::{DeployableState, PulseTotals};

/// `SGWBeing`'s wire class, the class every deployable is spawned as.
pub const DEPLOYABLE_CLASS: &str = "being";

/// Why a deployable could not be placed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DeployableSpawnError {
    /// The owner is in no loaded space.
    #[error("owner {0} is not in any space")]
    OwnerNotFound(u32),
    /// Only players place deployables.
    #[error("entity {0} is not a player")]
    OwnerNotPlayer(u32),
    /// No `entity_templates` row with this id in the startup cache.
    #[error("template {0} not in the entity_templates cache")]
    UnknownTemplate(i32),
    /// The lifetime effect is missing or does not pulse.
    #[error("lifetime effect {0} is missing or has no pulses")]
    BadLifetimeEffect(i32),
    /// The pulse effect is missing.
    #[error("pulse effect {0} is missing")]
    MissingPulseEffect(i32),
    /// The underlying NPC spawn failed.
    #[error("spawn failed: {0}")]
    Spawn(String),
}

impl DeployableSpawnError {
    /// Stable `reason` value for logs.
    pub fn reason(&self) -> &'static str {
        match self {
            Self::OwnerNotFound(_) => "owner_not_found",
            Self::OwnerNotPlayer(_) => "owner_not_player",
            Self::UnknownTemplate(_) => "unknown_template",
            Self::BadLifetimeEffect(_) => "bad_lifetime_effect",
            Self::MissingPulseEffect(_) => "missing_pulse_effect",
            Self::Spawn(_) => "spawn_failed",
        }
    }
}

/// The timing a lifetime effect gives a deployable: `(pulses, interval)`.
/// `None` for an effect that does not pulse on a positive interval.
pub fn pulse_schedule(lifetime: &EffectDef) -> Option<(u32, Duration)> {
    if lifetime.pulse_count < 1
        || !lifetime.pulse_duration.is_finite()
        || lifetime.pulse_duration <= 0.0
    {
        return None;
    }
    Some((
        lifetime.pulse_count as u32,
        Duration::from_secs_f32(lifetime.pulse_duration),
    ))
}

/// The hit radius, metres, of a pulse effect: its `Radius` NVP when that is
/// positive, else the client's AE radius for its `tcm_param1` tier
/// ("Medium" = 1000 UE3 units = 10 m, `ae_radius_metres`). A tier the
/// client does not know falls back to the cone tiers (which warn).
pub fn pulse_radius(pulse: &EffectDef) -> f32 {
    let r = pulse.param_f32("Radius");
    if r > 0.0 {
        r
    } else {
        ae_radius_metres(&pulse.tcm_param1)
            .unwrap_or_else(|| EffectDef::tcm_range_meters(&pulse.tcm_param1))
    }
}

impl SpaceManager {
    /// Place a deployable for `owner` from `spec` at `point`, facing
    /// `heading` (yaw, radians), and register it. Its first pulse is due one
    /// interval after `now`. Returns its entity id.
    pub fn spawn_deployable(
        &mut self,
        owner: u32,
        spec: DeployableSpec,
        point: Vector3,
        heading: f32,
        now: Instant,
    ) -> Result<u32, DeployableSpawnError> {
        let result = self.spawn_deployable_inner(owner, spec, point, heading, now);
        let id = self.player_identity(owner);
        match &result {
            Ok(entity_id) => {
                let state = self.deployables.get(*entity_id);
                tracing::info!(
                    target: "deployables.lifecycle",
                    decision_outcome = "spawned",
                    event = "spawned",
                    entity_id = *entity_id,
                    deployable_id = *entity_id,
                    owner_id = owner,
                    account_id = id.account_id,
                    player_id = id.player_id,
                    ability_id = spec.ability_id,
                    template_id = spec.template_id,
                    space_id = self.get_entity_space_id(*entity_id),
                    x = point.x,
                    y = point.y,
                    z = point.z,
                    radius = state.map(|s| s.radius),
                    pulses_total = state.map(|s| s.pulses_total),
                    pulse_interval_secs = state.map(|s| s.pulse_interval.as_secs_f32()),
                    "deployable placed"
                );
            }
            // Every reason is a caller or seed error: the cast validated the
            // owner, and a deployables row names a seeded template and
            // effects. So WARN.
            Err(e) => {
                tracing::warn!(
                    target: "deployables.lifecycle",
                    decision_outcome = "spawn_failed",
                    event = "spawn_failed",
                    entity_id = owner,
                    owner_id = owner,
                    account_id = id.account_id,
                    player_id = id.player_id,
                    ability_id = spec.ability_id,
                    template_id = spec.template_id,
                    reason = e.reason(),
                    error = %e,
                    "deployable could not be placed"
                );
            }
        }
        result
    }

    fn spawn_deployable_inner(
        &mut self,
        owner: u32,
        spec: DeployableSpec,
        point: Vector3,
        heading: f32,
        now: Instant,
    ) -> Result<u32, DeployableSpawnError> {
        let space_id = self
            .get_entity_space_id(owner)
            .ok_or(DeployableSpawnError::OwnerNotFound(owner))?;
        let owner_entity = self
            .get_entity(owner)
            .ok_or(DeployableSpawnError::OwnerNotFound(owner))?;
        if !owner_entity.is_player {
            return Err(DeployableSpawnError::OwnerNotPlayer(owner));
        }
        let owner_faction = owner_entity.faction;
        let owner_identity = owner_entity.identity();
        let (pulses_total, pulse_interval) = self
            .effect_defs
            .get(&spec.lifetime_effect_id)
            .and_then(pulse_schedule)
            .ok_or(DeployableSpawnError::BadLifetimeEffect(
                spec.lifetime_effect_id,
            ))?;
        let radius = self
            .effect_defs
            .get(&spec.pulse_effect_id)
            .map(pulse_radius)
            .ok_or(DeployableSpawnError::MissingPulseEffect(
                spec.pulse_effect_id,
            ))?;
        let world_name = self
            .spaces
            .get(&space_id)
            .map(|s| s.world_name.clone())
            .ok_or(DeployableSpawnError::OwnerNotFound(owner))?;
        let mut record = self
            .spawn_templates
            .get(&spec.template_id)
            .cloned()
            .ok_or(DeployableSpawnError::UnknownTemplate(spec.template_id))?;

        record.world_name = world_name;
        record.x = point.x;
        record.y = point.y;
        record.z = point.z;
        record.heading = heading;
        record.class = DEPLOYABLE_CLASS.to_string();
        record.faction = Some(i32::from(owner_faction));
        record.aggression_override = None;
        record.spawn_id = -1;
        record.tag = None;
        record.loot_table_id = None;
        record.respawn_secs = None;
        // Stationary: never paths, and keeps the Y the cast validated
        // (`grounded_spawn_position` leaves a stationary spawn alone).
        record.is_stationary = true;
        record.use_cover = Some(false);
        record.patrol_path.clear();
        record.wander_radius = 0.0;

        let entity_id = self.allocate_npc_id();
        self.spawn_npc_from_record_in_space(entity_id, &record, space_id)
            .map_err(DeployableSpawnError::Spawn)?;
        self.deployables.register(
            entity_id,
            DeployableState {
                owner,
                owner_identity,
                ability_id: spec.ability_id,
                template_id: spec.template_id,
                pulse_effect_id: spec.pulse_effect_id,
                radius,
                pulse_interval,
                pulses_total,
                spawned_at: now,
                next_pulse_at: now + pulse_interval,
                totals: PulseTotals::default(),
            },
        );
        Ok(entity_id)
    }
}
