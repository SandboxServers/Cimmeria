//! Pets (issue #570): the world-state half. PT-01 "the pet exists": a pet
//! can be spawned for a player, is introduced to its owner with its lists,
//! and is torn down safely. No summon ability, commands or AI here; those
//! are PT-03, PT-04 and PT-05 (`docs/analysis/pets/work-packets.md`).
//!
//! - [`registry`] holds `PetRegistry` (owner <-> pet maps on
//!   `SpaceManager::pets`), the ownership guard `SpaceManager::owned_pet` and
//!   the kill-credit seam `SpaceManager::credit_recipient`.
//! - [`spawn`] holds `SpaceManager::spawn_pet_from_template`.
//! - [`create_on_client`] builds the owner-only lists replayed on AoI entry.
//! - [`teardown`] holds `despawn_pet`, `forget_owner` and the per-tick
//!   `pet_owner_sweep` (which also expires pet corpses, D-PT08).
//! - [`owner_hooks`] holds the PT-02 choke points every owner path calls:
//!   `on_owner_left` (despawn) and `on_owner_teleported` (move beside).
//!
//! Log target `pets.lifecycle`: INFO on summon and despawn (with `reason`),
//! WARN on a failed summon or despawn.

pub mod create_on_client;
pub mod owner_hooks;
pub mod registry;
pub mod spawn;
pub mod teardown;

pub use create_on_client::{pet_create_on_client_events, CLIENT_DEFAULT_STANCE};
pub use owner_hooks::{on_owner_left, on_owner_teleported, OwnerPath};
pub use registry::{PetRegistry, PetReject};
pub use spawn::{stance_mask_from_flags, PetSpawnError, PET_SPAWN_OFFSET};
pub use teardown::{
    despawn_pet, forget_owner, pet_owner_sweep, pet_owner_sweep_at, PetDespawnReason,
    PET_CORPSE_DESPAWN,
};

#[cfg(test)]
mod tests;
