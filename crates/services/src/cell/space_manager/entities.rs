//! Entity create/destroy/connect/disconnect/update.
//!
//! These methods operate on `CellEntity` instances within a space. Player
//! entities are tracked in `SpaceInstance::players`; their disconnection
//! triggers AoI cleanup via the BaseService channel.

use std::time::Instant;

use cimmeria_common::{EntityId, SpaceId, Vector3};
use cimmeria_entity::cell_entity::CellEntity;
use cimmeria_entity::movement_validation::{MovementReject, MovementValidator, SpaceBounds};

use super::super::messages::CellToBaseMsg;
use super::SpaceManager;

/// Outcome of `SpaceManager::apply_client_position_update`.
///
/// `Accepted` means the cell entity's position has been advanced; the
/// caller does not need to do anything else. `Rejected` means the
/// validator refused the proposed position and the cell entity is
/// unchanged — the caller must emit `CellToBaseMsg::TeleportPlayer` so
/// the offending client snaps back to `last_valid`. `EntityMissing` is
/// the cell-side equivalent of "address not found" — log and drop.
#[derive(Debug)]
pub enum ClientMoveOutcome {
    /// Position passed validation and was written. Carries the new
    /// position so callers that want to log it can do so without
    /// re-querying the entity.
    Accepted { position: [f32; 3] },
    /// Position failed validation. Carries the last-valid position and
    /// the space id needed to compose the `BASEMSG_FORCED_POSITION`
    /// snap-back message.
    Rejected {
        reason: MovementReject,
        last_valid: [f32; 3],
        space_id: u32,
        /// The bounds the proposed position was tested against. Carried
        /// out so the caller's structured log can include `bounds_min`
        /// and `bounds_max` per the negative-logging convention.
        bounds: SpaceBounds,
    },
    /// The entity is not currently in any space — likely a stale
    /// inbound packet that arrived after destroy / disconnect. The
    /// caller can safely no-op; the original `update_entity_position`
    /// silently dropped the same shape.
    EntityMissing,
}

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

        let mut cell_entity =
            CellEntity::new(EntityId(entity_id as i32), SpaceId(space_id as i32), pos);
        cell_entity.direction = dir;

        let space = self
            .spaces
            .get_mut(&space_id)
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
                    space
                        .space
                        .remove_entity(EntityId(entity_id as i32), &cell_entity.position);
                }
                space.players.remove(&entity_id);

                // Check if this was the last player in an instanced space
                if space.players.is_empty() {
                    let world_name = &space.world_name;
                    if self.worlds.get(world_name).is_some_and(|w| w.instanced) {
                        should_destroy_space = true;
                    }
                }
            }

            if should_destroy_space {
                self.destroy_space(space_id);
            }
        }
        // Release the entity's movement-validator clock so it can't leak
        // or carry a stale speed sample across `entity_id` reuse.
        self.movement_validator.forget(entity_id);
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

                // Notify every player that had this one in its AoI -- i.e. each
                // player whose `witnesses` set contains the disconnecting id.
                // `cell_entity.witnesses` holds the entities IT sees (wrong
                // direction); iterate `space.players` and check inbound
                // membership instead. NPCs don't receive LeftAoI, so we skip
                // scanning them and keep disconnect O(P) rather than O(E).
                let target = EntityId(entity_id as i32);
                let observers: Vec<u32> = space
                    .players
                    .iter()
                    .copied()
                    .filter(|other_id| {
                        *other_id != entity_id
                            && space
                                .entities
                                .get(other_id)
                                .is_some_and(|other| other.witnesses.contains(&target))
                    })
                    .collect();
                for witness_id in observers {
                    if let Err(e) = tx
                        .send(CellToBaseMsg::LeftAoI {
                            witness_id,
                            entity_id,
                        })
                        .await
                    {
                        tracing::warn!(
                            witness_id, entity_id, error = %e,
                            "LeftAoI send to base failed during disconnect"
                        );
                    }
                }

                // Remove this entity from all other entities' witness sets
                for other in space.entities.values_mut() {
                    other.witnesses.remove(&target);
                }
            }
        }

        // Then destroy the cell entity
        self.destroy_entity(entity_id);
        tracing::debug!(entity_id, "Entity disconnected and destroyed");
    }

    /// Apply a client-authoritative position update through the
    /// movement validator.
    ///
    /// This is the **only** seam that should be called from the inbound
    /// `BaseToCellMsg::EntityMove` handler — every other position
    /// mutation in the cell is server-authoritative (ring transport,
    /// respawn, content-engine teleport, NPC movement) and goes through
    /// the unchecked [`Self::update_entity_position`] directly.
    ///
    /// On accept, the call is equivalent to `update_entity_position`.
    /// On reject, the cell entity is left untouched so the next AoI
    /// tick rebroadcasts the last-valid position to witnesses; the
    /// caller emits `CellToBaseMsg::TeleportPlayer` so the offending
    /// client receives `BASEMSG_FORCED_POSITION` and snaps its own
    /// avatar back to the last-valid position.
    ///
    /// All four validation layers run here, in cheapest-first order:
    /// bounds (catches NaN / infinity / absurd coordinates and the
    /// Z-floor-clip), navmesh containment (off-walkable-polygon), then
    /// the stateful speed/teleport kinematics. Bounds and navmesh both
    /// hard-reject; speed is warn-only (logged + counted, still
    /// accepted); teleport hard-rejects. The `spaceId` cross-check
    /// lives in the `EntityMove` handler, where the client-claimed space
    /// id is in hand.
    ///
    /// Production callers use this 4-arg form (server `Instant::now()`);
    /// the time-injected [`Self::apply_client_position_update_at`] backs
    /// it so the speed/teleport layer is deterministic under test.
    pub fn apply_client_position_update(
        &mut self,
        entity_id: u32,
        position: [f32; 3],
        direction: [i8; 3],
        velocity: [f32; 3],
    ) -> ClientMoveOutcome {
        self.apply_client_position_update_at(
            Instant::now(),
            entity_id,
            position,
            direction,
            velocity,
        )
    }

    /// Time-injected core of [`Self::apply_client_position_update`]. `now`
    /// is the server's monotonic processing time, threaded in so unit
    /// tests can drive the speed/teleport layer with controlled deltas.
    pub fn apply_client_position_update_at(
        &mut self,
        now: Instant,
        entity_id: u32,
        position: [f32; 3],
        direction: [i8; 3],
        velocity: [f32; 3],
    ) -> ClientMoveOutcome {
        let space_id = match self.entity_space.get(&entity_id) {
            Some(&id) => id,
            None => return ClientMoveOutcome::EntityMissing,
        };

        // Source bounds from the active space's navmesh if present; fall
        // back to the generous default for spaces without a loaded
        // navmesh (most non-Castle zones today). The fallback is wider
        // than any legitimate world by an order of magnitude — see
        // `SpaceBounds::FALLBACK`.
        let (bounds, last_valid) = {
            let space = match self.spaces.get(&space_id) {
                Some(s) => s,
                None => return ClientMoveOutcome::EntityMissing,
            };
            let bounds = match &space.navmesh {
                Some(nav) => SpaceBounds::new(nav.bmin, nav.bmax),
                None => SpaceBounds::FALLBACK,
            };
            let last_valid = match space.entities.get(&entity_id) {
                Some(e) => [e.position.x, e.position.y, e.position.z],
                None => return ClientMoveOutcome::EntityMissing,
            };
            (bounds, last_valid)
        };

        let proposed = Vector3::new(position[0], position[1], position[2]);

        // Advance the per-entity processing clock up front and recover the
        // previous sample, *before* any layer can short-circuit. This is
        // what stops an attacker from spamming cheaply-rejected
        // (out-of-bounds / off-navmesh) packets to inflate `dt`, then
        // slipping one large jump past the teleport gate at an
        // artificially low implied speed. Every processed packet advances
        // the clock by exactly one tick regardless of which layer rejects.
        let prev_sample = self.movement_validator.touch_clock(entity_id, now);

        // Layer 1 — bounds (also the Z-axis floor-clip / NaN / infinity gate).
        if let Err(reason) = self.movement_validator.check_bounds(proposed, &bounds) {
            return ClientMoveOutcome::Rejected {
                reason,
                last_valid,
                space_id,
                bounds,
            };
        }

        // Layer 4 — navmesh containment. `is_position_valid` fails open
        // for spaces with no loaded navmesh, so this is a no-op there and
        // the bounds AABB remains the only spatial gate. Checked before
        // kinematics so an off-mesh point is reported as `OffNavmesh`
        // regardless of how far it is from the last position.
        if !self.is_position_valid(entity_id, &proposed) {
            return ClientMoveOutcome::Rejected {
                reason: MovementReject::OffNavmesh,
                last_valid,
                space_id,
                bounds,
            };
        }

        // Layers 2+3 — speed (warn-only) + teleport (hard reject). Measured
        // against the entity's current authoritative position.
        let last_pos = Vector3::new(last_valid[0], last_valid[1], last_valid[2]);
        let kin = self.movement_validator.check_kinematics(
            now,
            prev_sample,
            last_pos,
            proposed,
            MovementValidator::DEFAULT_TOP_SPEED,
        );
        if let Some(reason) = kin.reject {
            return ClientMoveOutcome::Rejected {
                reason,
                last_valid,
                space_id,
                bounds,
            };
        }
        if let Some(sample) = kin.speed_warn {
            // Warn-only: the move is accepted. Surface the full
            // (distance, dt, implied_speed) triple so the SigNoz
            // tolerance-calibration pipeline can compute the legitimate
            // p99.9 before the speed layer is ever promoted to snap-back.
            tracing::warn!(
                target: "movement.validation",
                entity_id,
                space_id,
                client_x = position[0],
                client_y = position[1],
                client_z = position[2],
                distance = sample.distance,
                dt_secs = sample.dt_secs,
                implied_speed = sample.implied_speed,
                top_speed = sample.top_speed,
                ratio = sample.implied_speed / sample.top_speed,
                reason = "speed",
                "movement.speed_warning: client move exceeded speed tolerance \
                 (warn-only — accepted; calibrate before enforcing)"
            );
            cimmeria_observability::counter!(
                "movement_validation_warns_total",
                "reason" => "speed",
            );
        }

        self.update_entity_position(entity_id, position, direction, velocity);
        ClientMoveOutcome::Accepted { position }
    }

    /// Reseed an entity's movement-validator clock after a server-
    /// authoritative position write (ring transport, respawn, gate
    /// arrival, content-engine teleport, GM travel). Suppresses a
    /// spurious speed warn on the first post-teleport client packet; see
    /// [`MovementValidator::note_authorized_teleport`].
    pub fn note_authorized_teleport(&mut self, entity_id: u32) {
        self.movement_validator
            .note_authorized_teleport(entity_id, Instant::now());
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
            space
                .space
                .grid
                .update_position(EntityId(entity_id as i32), &old_pos, &new_pos);
        }
    }
}
