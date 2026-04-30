//! Entity create/destroy/connect/disconnect/update.
//!
//! These methods operate on `CellEntity` instances within a space. Player
//! entities are tracked in `SpaceInstance::players`; their disconnection
//! triggers AoI cleanup via the BaseService channel.

use cimmeria_common::{EntityId, SpaceId, Vector3};
use cimmeria_entity::cell_entity::CellEntity;

use super::super::messages::CellToBaseMsg;
use super::SpaceManager;

impl SpaceManager {
    /// Create a cell entity in the appropriate space.
    pub fn create_entity(
        &mut self,
        entity_id: u32,
        world_name: &str,
        position: [f32; 3],
        rotation: [f32; 3],
    ) -> Result<u32, String> {
        let space_id = self.find_or_create_space(world_name)?;

        let pos = Vector3::new(position[0], position[1], position[2]);
        let dir = Vector3::new(rotation[0], rotation[1], rotation[2]);

        let mut cell_entity = CellEntity::new(
            EntityId(entity_id as i32),
            SpaceId(space_id as i32),
            pos,
        );
        cell_entity.direction = dir;

        let space = self.spaces.get_mut(&space_id)
            .ok_or_else(|| format!("Space {space_id} disappeared"))?;

        space.space.add_entity(EntityId(entity_id as i32), &pos);
        space.entities.insert(entity_id, cell_entity);
        self.entity_space.insert(entity_id, space_id);

        tracing::debug!(entity_id, space_id, ?position, "Cell entity created");
        Ok(space_id)
    }

    /// Destroy a cell entity, removing it from its space.
    ///
    /// If the entity was in an instanced space and was the last player, the
    /// entire space instance is destroyed (all remaining NPCs removed).
    pub fn destroy_entity(&mut self, entity_id: u32) {
        if let Some(space_id) = self.entity_space.remove(&entity_id) {
            let mut should_destroy_space = false;

            if let Some(space) = self.spaces.get_mut(&space_id) {
                if let Some(cell_entity) = space.entities.remove(&entity_id) {
                    space.space.remove_entity(
                        EntityId(entity_id as i32),
                        &cell_entity.position,
                    );
                }
                space.players.remove(&entity_id);

                // Check if this was the last player in an instanced space
                if space.players.is_empty() {
                    let world_name = &space.world_name;
                    if self.worlds.get(world_name).map_or(false, |w| w.instanced) {
                        should_destroy_space = true;
                    }
                }
            }

            if should_destroy_space {
                self.destroy_space(space_id);
            }
        }
        tracing::debug!(entity_id, "Cell entity destroyed");
    }

    /// Mark an entity as having a client controller (player).
    pub fn connect_entity(&mut self, entity_id: u32) {
        if let Some(&space_id) = self.entity_space.get(&entity_id) {
            if let Some(space) = self.spaces.get_mut(&space_id) {
                space.players.insert(entity_id);
                if let Some(entity) = space.entities.get_mut(&entity_id) {
                    entity.is_player = true;
                    entity.class_id = 0x02; // SGWPlayer
                }
                tracing::debug!(entity_id, space_id, "Entity connected (player)");
            }
        }
    }

    /// Remove client controller and clean up AoI witnesses.
    pub async fn disconnect_entity(
        &mut self,
        entity_id: u32,
        tx: &tokio::sync::mpsc::Sender<CellToBaseMsg>,
    ) {
        if let Some(&space_id) = self.entity_space.get(&entity_id) {
            if let Some(space) = self.spaces.get_mut(&space_id) {
                space.players.remove(&entity_id);

                // Notify all entities that had this one in their AoI
                if let Some(cell_entity) = space.entities.get(&entity_id) {
                    let witnesses: Vec<u32> = cell_entity.witnesses
                        .iter()
                        .map(|eid| eid.0 as u32)
                        .collect();
                    for witness_id in witnesses {
                        let _ = tx.send(CellToBaseMsg::LeftAoI {
                            witness_id,
                            entity_id,
                        }).await;
                    }
                }

                // Remove this entity from all other entities' witness sets
                let eid = EntityId(entity_id as i32);
                for other in space.entities.values_mut() {
                    other.witnesses.remove(&eid);
                }
            }
        }

        // Then destroy the cell entity
        self.destroy_entity(entity_id);
        tracing::debug!(entity_id, "Entity disconnected and destroyed");
    }

    /// Update an entity's position from a client movement packet.
    pub fn update_entity_position(
        &mut self,
        entity_id: u32,
        position: [f32; 3],
        direction: [i8; 3],
        velocity: [f32; 3],
    ) {
        let space_id = match self.entity_space.get(&entity_id) {
            Some(&id) => id,
            None => return,
        };

        let space = match self.spaces.get_mut(&space_id) {
            Some(s) => s,
            None => return,
        };

        if let Some(cell_entity) = space.entities.get_mut(&entity_id) {
            let old_pos = cell_entity.position;
            let new_pos = Vector3::new(position[0], position[1], position[2]);

            cell_entity.position = new_pos;
            cell_entity.direction = Vector3::new(
                direction[0] as f32,
                direction[1] as f32,
                direction[2] as f32,
            );
            cell_entity.velocity = velocity;

            // Update the spatial grid
            space.space.grid.update_position(
                EntityId(entity_id as i32),
                &old_pos,
                &new_pos,
            );
        }
    }
}
