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
        // Iterate by id (not `values_mut`) so the per-player body can re-borrow
        // `self.spaces` through the shared helper.
        let space_ids: Vec<u32> = self.spaces.keys().copied().collect();
        for space_id in space_ids {
            let player_ids: Vec<u32> = match self.spaces.get(&space_id) {
                Some(s) if !s.players.is_empty() => s.players.iter().copied().collect(),
                _ => continue,
            };
            for player_id in player_ids {
                self.compute_player_aoi(space_id, player_id, &mut events);
            }
        }
        events
    }

    /// Compute AoI changes for a **single** player immediately, without waiting
    /// for the next AoI tick.
    ///
    /// Called from the `ConnectEntity` handler so a newly-connected player is
    /// introduced to every entity already in range right away. Without this,
    /// the `tokio::select!` in the cell loop can run an AoI tick *before*
    /// `connect_entity` adds the player to `space.players` — that tick hits the
    /// `space.players.is_empty()` guard in [`Self::compute_aoi_changes`] and
    /// skips the whole space, leaving NPCs that were spawned during instance
    /// creation (e.g. the Castle_CellBlock stasis-room corpses) un-introduced
    /// until a later tick or a relog. Updating the witness set here makes the
    /// follow-up ticks idempotent (no double `EnteredAoI`).
    pub fn compute_aoi_changes_for_player(&mut self, player_id: u32) -> Vec<CellToBaseMsg> {
        let mut events = Vec::new();
        if let Some(space_id) = self.entity_space.get(&player_id).copied() {
            self.compute_player_aoi(space_id, player_id, &mut events);
        }
        events
    }

    /// Per-player AoI diff: pushes `EnteredAoI` / `LeftAoI` / `EntityMoved`
    /// events for `player_id` in `space_id` onto `events`, then refreshes the
    /// player's witness set. Shared by [`Self::compute_aoi_changes`] (the tick)
    /// and [`Self::compute_aoi_changes_for_player`] (the connect path).
    fn compute_player_aoi(
        &mut self,
        space_id: u32,
        player_id: u32,
        events: &mut Vec<CellToBaseMsg>,
    ) {
        let Some(space) = self.spaces.get_mut(&space_id) else {
            return;
        };
        {
            let (player_pos, aoi_radius, player_interactions) = match space.entities.get(&player_id)
            {
                Some(e) => (e.position, e.aoi_radius, e.available_interactions.clone()),
                None => return,
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
                None => return,
            };

            // Entered AoI: in current but not in previous
            for &eid in &current_aoi {
                if !previous_aoi.contains(&eid) {
                    if let Some(other) = space.entities.get(&eid) {
                        // Stable AoI-enter log — `aoi.entity_enter` event
                        // name lets SigNoz answer "did entity X enter
                        // player Y's view?" without grepping for the
                        // EnteredAoI message text.
                        tracing::debug!(
                            target: "aoi.entity_enter",
                            witness_id = player_id,
                            entity_id = eid,
                            space_id = space.space_id,
                            is_player = other.is_player,
                            "AoI: entity entered witness view"
                        );
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
                            direction: [other.direction.x, other.direction.y, other.direction.z],
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
                                    let merged =
                                        base | entries.iter().fold(0i64, |acc, &(_, _, f)| acc | f);
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
                                        method_index: crate::mercury::method_idx::INTERACTION_TYPE,
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
                    tracing::debug!(
                        target: "aoi.entity_leave",
                        witness_id = player_id,
                        entity_id = eid,
                        space_id = space.space_id,
                        "AoI: entity left witness view"
                    );
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
                            direction: [other.direction.x, other.direction.y, other.direction.z],
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
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Entity ids introduced (`EnteredAoI`) to `witness` in this event batch.
    fn entered_for(events: &[CellToBaseMsg], witness: u32) -> Vec<u32> {
        events
            .iter()
            .filter_map(|e| match e {
                CellToBaseMsg::EnteredAoI {
                    witness_id,
                    entity_id,
                    ..
                } if *witness_id == witness => Some(*entity_id),
                _ => None,
            })
            .collect()
    }

    /// Regression guard for the `ConnectEntity` ↔ AoI-tick race: an AoI tick
    /// that runs before `connect_entity` must NOT introduce anything (the
    /// player isn't in `space.players` yet), and the connect-path AoI compute
    /// must introduce every in-range NPC immediately — with no double-introduce
    /// on the following tick.
    #[test]
    fn connect_introduces_aoi_without_waiting_for_tick() {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="-100" MaxX="100" MinY="-100" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();

        // Player created but NOT connected (mirrors the window after
        // CreateEntity but before ConnectEntity is processed).
        let player = 1u32;
        mgr.create_entity(player, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        // Two near-colocated corpses in range (the stasis-room shape).
        let body1 = mgr.allocate_npc_id();
        mgr.spawn_npc(body1, "Agnos", [5.0, 0.0, 5.0], [0.0; 3])
            .unwrap();
        let body2 = mgr.allocate_npc_id();
        mgr.spawn_npc(body2, "Agnos", [6.0, 0.0, 6.0], [0.0; 3])
            .unwrap();

        // 1. The racing tick: player not yet in space.players → space skipped.
        let pre = mgr.compute_aoi_changes();
        assert!(
            entered_for(&pre, player).is_empty(),
            "no introductions before connect (space.players empty)"
        );

        // 2. Connect → immediate introduction of both bodies (the fix).
        mgr.connect_entity(player);
        let on_connect = mgr.compute_aoi_changes_for_player(player);
        let mut entered = entered_for(&on_connect, player);
        entered.sort_unstable();
        assert!(
            entered.contains(&body1) && entered.contains(&body2),
            "both bodies must be introduced on connect, got {entered:?}"
        );

        // 3. Idempotent: the next full tick must NOT re-introduce them.
        let next = mgr.compute_aoi_changes();
        assert!(
            entered_for(&next, player).is_empty(),
            "no double-introduction on the following tick"
        );
    }
}
