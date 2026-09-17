//! Entity create/destroy/connect/disconnect/update.
//!
//! These methods operate on `CellEntity` instances within a space. Player
//! entities are tracked in `SpaceInstance::players`; their disconnection
//! triggers AoI cleanup via the BaseService channel.

use cimmeria_common::{EntityId, SpaceId, Vector3};
use cimmeria_entity::cell_entity::{CellEntity, PlayerIdentity};

use super::super::messages::CellToBaseMsg;
use super::SpaceManager;

/// Outcome of [`SpaceManager::despawn_npc`].
///
/// Every variant is a *real* outcome the caller must report to the GM
/// verbatim — an "accepted" despawn request is not the same as a completed
/// one, and the dot-console `.despawn` handler is required to propagate the
/// difference (see `docs/analysis/legacy-command-parity/README.md`'s
/// Architecture Guardrails).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use = "a despawn outcome must be reported to the calling GM, not discarded"]
pub enum DespawnOutcome {
    /// The entity was removed from its space. `witnesses_notified` is the
    /// number of observing players that were sent `LeftAoI` for it.
    Despawned { witnesses_notified: usize },
    /// No such entity in any loaded space (already despawned, or never here).
    NotFound,
    /// The entity is a player. `despawn_npc` is an NPC/spawnable-only
    /// primitive and refuses players structurally — destroying a connected
    /// player's cell entity out from under its session would strand the
    /// client with no avatar and no disconnect handshake.
    RefusedPlayer,
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
        self.insert_entity_into_space(entity_id, space_id, position, rotation)
    }

    /// Create a cell entity in one *specific, already-loaded* space instance.
    ///
    /// This is the cross-instance transfer entry point (GM `.goto <player>`):
    /// [`Self::create_entity`] resolves by world name, and for an instanced
    /// world `find_or_create_space` always allocates a BRAND NEW space — so
    /// it can never be used to join somebody else's existing instance.
    ///
    /// Fails if `space_id` isn't loaded; the caller decides whether to fall
    /// back to by-world-name resolution (it should — an entity in no space at
    /// all is worse than an entity in the wrong instance of the right world).
    pub fn create_entity_in_space(
        &mut self,
        entity_id: u32,
        space_id: u32,
        position: [f32; 3],
        rotation: [f32; 3],
    ) -> Result<u32, String> {
        if !self.spaces.contains_key(&space_id) {
            return Err(format!("Space {space_id} is not loaded"));
        }
        self.insert_entity_into_space(entity_id, space_id, position, rotation)
    }

    /// Shared tail of the create paths: build the `CellEntity`, index it in
    /// the space's spatial grid and entity map, and bind `entity_space`.
    fn insert_entity_into_space(
        &mut self,
        entity_id: u32,
        space_id: u32,
        position: [f32; 3],
        rotation: [f32; 3],
    ) -> Result<u32, String> {
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

        // No identity fields here by design: the `CellEntity` was constructed
        // one statement ago and is not stamped until the caller
        // (`base_messages::lifecycle::handle_create_entity`) applies the
        // identity from `BaseToCellMsg::CreateEntity`. That handler emits the
        // identity-bearing "CreateEntity" line for this same event — this one
        // is the spatial-grid insert, keyed by entity/space only.
        tracing::debug!(entity_id, space_id, ?position, "Cell entity created");
        Ok(space_id)
    }

    /// Destroy a cell entity, removing it from its space.
    ///
    /// If the entity was in an instanced space and was the last player, the
    /// entire space instance is destroyed (all remaining NPCs removed).
    pub fn destroy_entity(&mut self, entity_id: u32) {
        // Snapshot the identity while the entity still exists — it is removed
        // from its space below, and this is the last chance to attribute the
        // teardown to an account. `entity_id` alone is not enough here of all
        // places: the id is released for reuse the moment this returns.
        let id = self.player_identity(entity_id);
        // GM-only session buffers are keyed by entity_id; drop them so a
        // destroyed (and possibly later reused) id can't inherit stale pending
        // authoring SQL or the autosave-spawn flag.
        self.authoring_changes.remove(&entity_id);
        self.autosave_spawns.remove(&entity_id);
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
        tracing::debug!(
            entity_id,
            account_id = id.account_id,
            player_id = id.player_id,
            "Cell entity destroyed"
        );
    }

    /// Despawn a **non-player** entity: tell every player currently
    /// witnessing it that it left, scrub it out of every witness set, then
    /// destroy it.
    ///
    /// This is the runtime half of the legacy `Resource.despawnEntity`
    /// (`Atrea.destroyCellEntity(target.entityId)`) — purely ephemeral, it
    /// never touches `resources.spawnlist`. Deleting the persistent row is a
    /// separate operation (`.delspawn`).
    ///
    /// # Why this is not just `destroy_entity`
    ///
    /// [`Self::destroy_entity`] removes the entity from `space.entities` and
    /// the spatial grid but leaves it sitting in every observer's
    /// `witnesses` set. The next AoI tick *would* eventually notice and emit
    /// `LeftAoI`, but only for players the tick happens to visit, only after
    /// up to a full tick of the client still rendering a corpse-less ghost,
    /// and only while the observer is still in the space. Fanning the
    /// `LeftAoI` out here — the same shape [`Self::disconnect_entity`] uses
    /// for a leaving player — makes the removal immediate and makes the
    /// *count* of notified observers an assertable result rather than a
    /// timing accident. Because the witness sets are scrubbed in the same
    /// pass, the following AoI tick does not emit a duplicate `LeftAoI`.
    ///
    /// # NPC-only
    ///
    /// A player target is refused with [`DespawnOutcome::RefusedPlayer`] and
    /// nothing is mutated. The check lives here, in the primitive, rather
    /// than only in the console command's registry `Target` — the legacy
    /// registration used `SGWSpawnableEntity`, and `SGWPlayer` derives from
    /// it (`SGWPlayer(SGWBeing(SGWSpawnableEntity))` in
    /// `deprecated/python/cell/`), so the legacy command would happily
    /// destroy a logged-in player's cell entity. Correcting that is D02
    /// ("correct legacy bugs rather than reproducing them"), and a guard that
    /// only exists in the command table would be one registry edit away from
    /// regressing.
    pub async fn despawn_npc(
        &mut self,
        entity_id: u32,
        tx: &tokio::sync::mpsc::Sender<CellToBaseMsg>,
    ) -> DespawnOutcome {
        let Some(&space_id) = self.entity_space.get(&entity_id) else {
            return DespawnOutcome::NotFound;
        };
        let Some(space) = self.spaces.get(&space_id) else {
            return DespawnOutcome::NotFound;
        };
        let Some(entity) = space.entities.get(&entity_id) else {
            return DespawnOutcome::NotFound;
        };

        // Two independent player signals, both checked: `is_player` is the
        // flag every other call site discriminates on, and `space.players`
        // is the client-controller registry the AoI tick iterates. They are
        // set together by `connect_entity`, but reading both means a future
        // change that forgets one still cannot route a player in here.
        if entity.is_player || space.players.contains(&entity_id) {
            tracing::warn!(
                target: "console.despawn",
                entity_id,
                space_id,
                is_player = entity.is_player,
                in_players_set = space.players.contains(&entity_id),
                "despawn refused: target is a player — despawn_npc is NPC/spawnable-only"
            );
            return DespawnOutcome::RefusedPlayer;
        }

        // Observers = players in this space (other than the target) whose
        // witness set contains it. NPCs never receive LeftAoI, so scanning
        // `space.players` keeps this O(P) rather than O(E) — same rationale
        // as `disconnect_entity`.
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

        let mut witnesses_notified = 0usize;
        for witness_id in observers {
            match tx
                .send(CellToBaseMsg::LeftAoI {
                    witness_id,
                    entity_id,
                })
                .await
            {
                Ok(()) => witnesses_notified += 1,
                Err(e) => {
                    // Report what actually reached a witness, not what we
                    // intended to send — the caller turns this count into GM
                    // feedback.
                    tracing::warn!(
                        target: "console.despawn",
                        witness_id, entity_id, error = %e,
                        "LeftAoI send to base failed during despawn"
                    );
                }
            }
        }

        // Scrub the dead id out of every witness set (players and NPCs
        // alike) so the next AoI tick has nothing left to diff.
        if let Some(space) = self.spaces.get_mut(&space_id) {
            for other in space.entities.values_mut() {
                other.witnesses.remove(&target);
            }
        }

        self.destroy_entity(entity_id);
        DespawnOutcome::Despawned { witnesses_notified }
    }

    /// Mark an entity as having a client controller (player).
    pub fn connect_entity(&mut self, entity_id: u32) {
        if let Some(&space_id) = self.entity_space.get(&entity_id) {
            if let Some(space) = self.spaces.get_mut(&space_id) {
                space.players.insert(entity_id);
                // Identity read from the entity we already have borrowed, so
                // the log below needs no second lookup.
                let id = match space.entities.get_mut(&entity_id) {
                    Some(entity) => {
                        entity.is_player = true;
                        entity.class_id = 0x02; // SGWPlayer
                        entity.identity()
                    }
                    None => PlayerIdentity::UNKNOWN,
                };
                tracing::debug!(
                    entity_id,
                    account_id = id.account_id,
                    player_id = id.player_id,
                    space_id,
                    "Entity connected (player)"
                );
            }
        }
    }

    /// Remove client controller and clean up AoI witnesses.
    pub async fn disconnect_entity(
        &mut self,
        entity_id: u32,
        tx: &tokio::sync::mpsc::Sender<CellToBaseMsg>,
    ) {
        // Snapshot identity up front: `destroy_entity` below removes the
        // entity, so the closing log can no longer resolve it.
        let id = self.player_identity(entity_id);
        // Drop GM-only session buffers (keyed by entity_id) on disconnect so
        // pending authoring SQL / the autosave-spawn flag don't outlive the
        // session. Same rationale as `destroy_entity`.
        self.authoring_changes.remove(&entity_id);
        self.autosave_spawns.remove(&entity_id);
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
                            witness_id, entity_id,
                            account_id = id.account_id,
                            player_id = id.player_id,
                            error = %e,
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
        tracing::debug!(
            entity_id,
            account_id = id.account_id,
            player_id = id.player_id,
            "Entity disconnected and destroyed"
        );
    }

    /// Update an entity's position from a client movement packet.
    ///
    /// `direction` is written **unconditionally**. A caller that is moving an
    /// entity without also re-facing it — every server-authoritative teleport
    /// — wants [`Self::update_position_preserving_facing`] instead; passing
    /// `[0, 0, 0]` here silently snaps the entity's facing to north.
    pub fn update_entity_position(
        &mut self,
        entity_id: u32,
        position: [f32; 3],
        direction: [i8; 3],
        velocity: [f32; 3],
    ) {
        let facing = Vector3::new(
            direction[0] as f32,
            direction[1] as f32,
            direction[2] as f32,
        );
        self.write_position(entity_id, position, Some(facing), velocity);
    }

    /// Move an entity without touching its facing — the position-only write
    /// every server-authoritative teleport needs (GM travel, snap-back
    /// recovery, console placement).
    ///
    /// [`Self::update_entity_position`] takes a `[i8; 3]` direction and writes
    /// it unconditionally, so a teleport that has no new facing to supply had
    /// to pass `[0, 0, 0]` and then hand-restore the captured `direction`
    /// afterwards. That workaround was duplicated across every GM travel call
    /// site and simply missing from the native `gm*` handlers, which zeroed
    /// facing on every teleport. This preserves facing **by construction**:
    /// `direction` is never written, so there is nothing to forget to restore.
    pub fn update_position_preserving_facing(
        &mut self,
        entity_id: u32,
        position: [f32; 3],
        velocity: [f32; 3],
    ) {
        self.write_position(entity_id, position, None, velocity);
    }

    /// Shared tail of the two position writers: move the entity in
    /// `space.entities` and keep the AoI spatial grid's index in sync.
    /// `direction: None` leaves the entity's facing untouched.
    fn write_position(
        &mut self,
        entity_id: u32,
        position: [f32; 3],
        direction: Option<Vector3>,
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
            if let Some(facing) = direction {
                cell_entity.direction = facing;
            }
            cell_entity.velocity = velocity;

            // Update the spatial grid
            space
                .space
                .grid
                .update_position(EntityId(entity_id as i32), &old_pos, &new_pos);
        }
    }
}
