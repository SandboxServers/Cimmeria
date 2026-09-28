//! Deployables (Phase 0): the world-state half. A deployable is a
//! stationary object a player places with a "Deployable:" ability; it pulses
//! an effect around itself for a fixed lifetime, then goes. The first user is
//! 1012 "Deployable: Microwave Emitter". Design and evidence:
//! `docs/gameplay/deployables.md`.
//!
//! - [`registry`] holds `DeployableRegistry` (`SpaceManager::deployables`):
//!   every live deployable with its owner and schedule, and the ground point
//!   a deployable cast is waiting to use.
//! - [`spawn`] holds `SpaceManager::spawn_deployable`.
//! - [`teardown`] holds the per-tick verdict and `despawn_deployable`.
//!
//! The cast diversions and the pulse tick (which needs the damage pipeline)
//! are `cimmeria-cell-combat`'s `abilities::deployable`. The ability ->
//! template binding is `resources.deployables`
//! (`spawner::DeployableCatalog`, `SpaceManager::deployable_specs`).
//!
//! Log target `deployables.lifecycle`: INFO on spawn and despawn (with
//! `reason` and the damage totals), WARN on a failed spawn or despawn.

pub mod registry;
pub mod spawn;
pub mod teardown;

pub use registry::{DeployableRegistry, DeployableState, PulseTotals, StagedPoint};
pub use spawn::{pulse_radius, pulse_schedule, DeployableSpawnError, DEPLOYABLE_CLASS};
pub use teardown::{
    deployable_verdict, despawn_deployable, scrub_orphan, DeployableDespawnReason,
    DeployableVerdict,
};

#[cfg(test)]
mod tests;
