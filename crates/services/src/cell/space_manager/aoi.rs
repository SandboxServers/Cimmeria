//! Area-of-Interest computation.
//!
//! Each tick, we walk every player in every space and diff their previous
//! witness set against the current set of nearby entities. The resulting
//! `EnteredAoI` / `LeftAoI` / `EntityMoved` events are forwarded to the
//! BaseService for client dispatch.

use std::collections::HashSet;

use cimmeria_common::EntityId;

use super::super::messages::CellToBaseMsg;
use super::SpaceManager;

impl SpaceManager {
    /// Compute AoI changes for all players across all spaces.
    ///
    /// Returns a list of `CellToBaseMsg` events: `EnteredAoI`, `LeftAoI`, `EntityMoved`.
    pub fn compute_aoi_changes(&mut self) -> Vec<CellToBaseMsg> {
        let mut events = Vec::new();

        for space in self.spaces.values_mut() {
            if space.players.is_empty() {
                continue;
            }

            // Collect player IDs to iterate (can't borrow space mutably while iterating)
            let player_ids: Vec<u32> = space.players.iter().copied().collect();

            for &player_id in &player_ids {
                let (player_pos, aoi_radius, player_interactions) =
                    match space.entities.get(&player_id) {
                        Some(e) => (e.position, e.aoi_radius, e.available_interactions.clone()),
                        None => continue,
                    };

                // Query the grid for nearby entities
                let candidates = space.space.get_entities_in_range(&player_pos, aoi_radius);

                // Filter to actual AoI: all entities in range (players + NPCs)
                let mut current_aoi: HashSet<u32> = HashSet::new();
                for candidate_eid in &candidates {
                    let cid = candidate_eid.0 as u32;
                    if cid == player_id {
                        continue; // skip self
                    }
                    // Exact distance check
                    if let Some(other) = space.entities.get(&cid) {
                        let dist_sq = player_pos.distance_squared_to(&other.position);
                        if dist_sq <= aoi_radius * aoi_radius {
                            current_aoi.insert(cid);
                        }
                    }
                }

                // Get previous witness set
                let previous_aoi: HashSet<u32> = match space.entities.get(&player_id) {
                    Some(e) => e.witnesses.iter().map(|eid| eid.0 as u32).collect(),
                    None => continue,
                };

                // Entered AoI: in current but not in previous
                for &eid in &current_aoi {
                    if !previous_aoi.contains(&eid) {
                        if let Some(other) = space.entities.get(&eid) {
                            let npc_data = if !other.is_player {
                                Some(super::super::messages::NpcAoIData {
                                    name_id: other.name_id,
                                    faction: other.faction,
                                    alignment: other.alignment,
                                    entity_flags: other.entity_flags,
                                    // Send the BASE interaction type in the cascade (not merged).
                                    // Dynamic per-player flags are sent as a separate
                                    // InteractionType update below, matching the C++ server's
                                    // createOnClient(base) → dynamicUpdate(merged) flow.
                                    interaction_type: other.interaction_type_flags,
                                    speaker_id: other.speaker_id,
                                    event_set_id: other.event_set_id,
                                    static_mesh: other.static_mesh.clone(),
                                    body_set: other.body_set.clone(),
                                    components: other.components.clone(),
                                })
                            } else {
                                None
                            };
                            events.push(CellToBaseMsg::EnteredAoI {
                                witness_id: player_id,
                                entity_id: eid,
                                space_id: space.space_id,
                                class_id: other.class_id,
                                position: [other.position.x, other.position.y, other.position.z],
                                direction: [
                                    other.direction.x,
                                    other.direction.y,
                                    other.direction.z,
                                ],
                                level: other.level,
                                npc_data,
                            });

                            // ── dynamicUpdate: standalone InteractionType update ──
                            //
                            // In the C++ server, createOnClient() sends InteractionType
                            // with the entity's BASE flags (often 0). Then dynamicUpdate()
                            // fires and sends InteractionType with the MERGED per-player
                            // flags as a separate message. The client treats this as a
                            // state change that enables right-click interaction.
                            //
                            // Reference: src/cellapp/base_client.cpp:455-458
                            if other.has_dynamic_properties {
                                if let Some(tmpl_id) = other.template_id {
                                    if let Some(entries) = player_interactions.get(&tmpl_id) {
                                        let base = other.interaction_type_flags;
                                        let merged = base
                                            | entries.iter().fold(0i64, |acc, &(_, _, f)| acc | f);
                                        if merged != base {
                                            tracing::info!(
                                                player_id,
                                                entity_id = eid,
                                                template_id = tmpl_id,
                                                base,
                                                merged,
                                                "AoI: dynamicUpdate InteractionType (base→merged)"
                                            );
                                        }
                                        events.push(CellToBaseMsg::WitnessEntityMethod {
                                            witness_id: player_id,
                                            entity_id: eid,
                                            method_index:
                                                crate::mercury::method_idx::INTERACTION_TYPE,
                                            args: (merged as u64).to_le_bytes().to_vec(),
                                        });

                                        // Register the entity as interactable on the client.
                                        // GameBeing::isInteractable() checks player+0x16c;
                                        // onDuelEntitiesRemove (method 152) adds to this set.
                                        // Must arrive AFTER CREATE_ENTITY so the client can
                                        // find the entity and refresh its interaction state.
                                        events.push(CellToBaseMsg::EntityMethodCall {
                                            entity_id: player_id,
                                            method_index: 152, // onDuelEntitiesRemove
                                            args: (eid as i32).to_le_bytes().to_vec(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }

                // Left AoI: in previous but not in current
                for &eid in &previous_aoi {
                    if !current_aoi.contains(&eid) {
                        events.push(CellToBaseMsg::LeftAoI {
                            witness_id: player_id,
                            entity_id: eid,
                        });
                    }
                }

                // Entity moved: in both, send position updates to this witness
                // (BaseApp can diff to skip no-ops if position unchanged)
                for &eid in &current_aoi {
                    if previous_aoi.contains(&eid) {
                        if let Some(other) = space.entities.get(&eid) {
                            events.push(CellToBaseMsg::EntityMoved {
                                witness_id: player_id,
                                entity_id: eid,
                                space_id: space.space_id,
                                position: [other.position.x, other.position.y, other.position.z],
                                direction: [
                                    other.direction.x,
                                    other.direction.y,
                                    other.direction.z,
                                ],
                                velocity: other.velocity,
                            });
                        }
                    }
                }

                // Update the witness set
                if let Some(entity) = space.entities.get_mut(&player_id) {
                    entity.witnesses = current_aoi.iter().map(|&id| EntityId(id as i32)).collect();
                }
            }
        }

        events
    }
}
