//! `spawn_pet_from_template`: put an owned pet into its owner's space.
//!
//! Built on the same cached `entity_templates` prototype as
//! `spawn_npc_from_template` (A-21), then reshaped into a pet before the
//! entity exists, so every derived field (health from level, cover use,
//! wire class) comes out right from the one spawn routine:
//!
//! - wire class `SGWPet` (0x05) and `ENTITYFLAG_Pet`;
//! - the owner's faction (D-PT06), which keeps the #444 gate refusing owner
//!   friendly fire and keeps the Idle scan from aggroing players;
//! - the owner's level (D-PT02), unless the template sets
//!   `ENTITYFLAG_NoPetLeveling`;
//! - no loot, no respawn, no patrol, no wander, no cover, no tag: a pet is
//!   not a placed NPC and must not behave like one;
//! - stance Defensive, and the template's ability set as the pet bar.
//!
//! Nothing is sent from here. The next AoI tick introduces the pet like any
//! NPC, and `create_on_client` adds the owner-only lists to that intro, so
//! the lists can never race ahead of the CREATE_ENTITY.

use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::{PetStance, PetState, ALL_STANCES_MASK};

use super::super::space_manager::SpaceManager;
use crate::mercury::SGWPET_CLASS_ID;
use cimmeria_wire::cell::client_methods::pet::{
    ENTITYFLAG_NO_AGGRESSIVE, ENTITYFLAG_NO_DEFENSIVE, ENTITYFLAG_NO_PASSIVE,
    ENTITYFLAG_NO_PET_LEVELING, ENTITYFLAG_PET,
};

/// How far behind its owner a pet appears, in world units.
pub const PET_SPAWN_OFFSET: f32 = 2.0;

/// Why a pet could not be spawned.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PetSpawnError {
    /// The owner is in no loaded space.
    #[error("owner {0} is not in any space")]
    OwnerNotFound(u32),
    /// Only players own pets.
    #[error("entity {0} is not a player")]
    OwnerNotPlayer(u32),
    /// No `entity_templates` row with this id in the startup cache.
    #[error("template {0} not in the entity_templates cache")]
    UnknownTemplate(i32),
    /// The underlying NPC spawn failed.
    #[error("spawn failed: {0}")]
    Spawn(String),
}

impl PetSpawnError {
    /// Stable `reason` value for logs.
    pub fn reason(&self) -> &'static str {
        match self {
            Self::OwnerNotFound(_) => "owner_not_found",
            Self::OwnerNotPlayer(_) => "owner_not_player",
            Self::UnknownTemplate(_) => "unknown_template",
            Self::Spawn(_) => "spawn_failed",
        }
    }
}

/// The stances a template allows, as a [`PetState::stance_mask`]: every
/// stance whose `ENTITYFLAG_No*` bit is clear. Inferred from the flag names
/// (code map §0); no client evidence ties the flags to the list.
pub fn stance_mask_from_flags(entity_flags: u64) -> u8 {
    let mut mask = ALL_STANCES_MASK;
    for (flag, stance) in [
        (ENTITYFLAG_NO_PASSIVE, PetStance::Passive),
        (ENTITYFLAG_NO_DEFENSIVE, PetStance::Defensive),
        (ENTITYFLAG_NO_AGGRESSIVE, PetStance::Aggressive),
    ] {
        if entity_flags & flag != 0 {
            mask &= !stance.mask_bit();
        }
    }
    mask
}

/// Where a pet appears: [`PET_SPAWN_OFFSET`] behind its owner, facing the
/// same way. `direction` is `[pitch, yaw, roll]` radians, and a heading of
/// `yaw` faces `(sin yaw, cos yaw)` in x/z.
fn beside_owner(position: Vector3, direction: Vector3) -> [f32; 3] {
    let yaw = direction.y;
    [
        position.x - yaw.sin() * PET_SPAWN_OFFSET,
        position.y,
        position.z - yaw.cos() * PET_SPAWN_OFFSET,
    ]
}

impl SpaceManager {
    /// Spawn a pet from `template_id` beside `owner`, owned by it, and
    /// register it. Returns the pet's entity id.
    ///
    /// `summon_ability_id` is the ability that summoned it (`0` for a GM or
    /// test spawn); it is recorded on the pet only.
    pub fn spawn_pet_from_template(
        &mut self,
        owner: u32,
        template_id: i32,
        summon_ability_id: i32,
    ) -> Result<u32, PetSpawnError> {
        let result = self.spawn_pet_inner(owner, template_id, summon_ability_id);
        // The owner is alive here on both arms; its identity is also kept on
        // the registry for teardown logs, which may run after it is gone.
        let id = self.player_identity(owner);
        match &result {
            Ok(pet_id) => {
                self.pets.note_owner_identity(owner, id);
                tracing::info!(
                    target: "pets.lifecycle",
                    decision_outcome = "summoned",
                    event = "summoned",
                    entity_id = *pet_id,
                    pet_id = *pet_id,
                    owner_id = owner,
                    account_id = id.account_id,
                    player_id = id.player_id,
                    template_id,
                    ability_id = summon_ability_id,
                    space_id = self.get_entity_space_id(*pet_id),
                    "pet summoned"
                );
            }
            // Not client-triggerable at will: every reason is a caller or
            // seed error (a summon names a seeded template for a live
            // player), so WARN.
            Err(e) => {
                tracing::warn!(
                    target: "pets.lifecycle",
                    decision_outcome = "summon_failed",
                    event = "summon_failed",
                    entity_id = owner,
                    owner_id = owner,
                    account_id = id.account_id,
                    player_id = id.player_id,
                    template_id,
                    ability_id = summon_ability_id,
                    reason = e.reason(),
                    error = %e,
                    "pet summon failed"
                );
            }
        }
        result
    }

    fn spawn_pet_inner(
        &mut self,
        owner: u32,
        template_id: i32,
        summon_ability_id: i32,
    ) -> Result<u32, PetSpawnError> {
        let space_id = self
            .get_entity_space_id(owner)
            .ok_or(PetSpawnError::OwnerNotFound(owner))?;
        let owner_entity = self
            .get_entity(owner)
            .ok_or(PetSpawnError::OwnerNotFound(owner))?;
        if !owner_entity.is_player {
            return Err(PetSpawnError::OwnerNotPlayer(owner));
        }
        let (owner_pos, owner_dir, owner_level, owner_faction) = (
            owner_entity.position,
            owner_entity.direction,
            owner_entity.level,
            owner_entity.faction,
        );
        let world_name = self
            .spaces
            .get(&space_id)
            .map(|s| s.world_name.clone())
            .ok_or(PetSpawnError::OwnerNotFound(owner))?;
        let mut record = self
            .spawn_templates
            .get(&template_id)
            .cloned()
            .ok_or(PetSpawnError::UnknownTemplate(template_id))?;

        let position = beside_owner(owner_pos, owner_dir);
        record.world_name = world_name;
        record.x = position[0];
        record.y = position[1];
        record.z = position[2];
        record.heading = owner_dir.y;
        // A pet is built as class 'pet' whatever the template row says, so a
        // template authored as 'mob' still becomes a GamePet on the client.
        record.class = "pet".to_string();
        record.flags |= ENTITYFLAG_PET as i64;
        record.faction = Some(i32::from(owner_faction));
        record.aggression_override = None;
        if record.flags as u64 & ENTITYFLAG_NO_PET_LEVELING == 0 {
            record.level = Some(owner_level as i32);
        }
        record.spawn_id = -1;
        record.tag = None;
        record.loot_table_id = None;
        record.respawn_secs = None;
        record.is_stationary = false;
        record.use_cover = Some(false);
        record.patrol_path.clear();
        record.wander_radius = 0.0;

        let pet_id = self.allocate_npc_id();
        self.spawn_npc_from_record_in_space(pet_id, &record, space_id)
            .map_err(PetSpawnError::Spawn)?;

        let stance_mask = stance_mask_from_flags(record.flags as u64);
        if let Some(e) = self.get_entity_mut(pet_id) {
            debug_assert_eq!(e.class_id, SGWPET_CLASS_ID);
            e.pet = Some(Box::new(PetState::new(
                owner,
                record.ability_ids.clone(),
                stance_mask,
                summon_ability_id,
            )));
        }
        self.pets.register(owner, pet_id);
        Ok(pet_id)
    }
}
