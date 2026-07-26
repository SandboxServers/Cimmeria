//! World-mutation action handlers: interaction-type flags, visibility,
//! destruction, waypoint movement, aggression, threat generation.
//!
//! These all locate a target entity by tag and either flip a flag or push
//! a state change.

use tokio::sync::mpsc;

use super::transport;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

#[cfg(test)]
mod tests;

/// `Action::SetInteractionType` — flip an interaction-type bit on the
/// tagged entity (add / remove / set), broadcasting the new flags to
/// every witness via `WitnessEntityMethod`.
pub(super) async fn set_interaction_type(
    entity_tag: String,
    operation: String,
    mask: i64,
    entity_id: u32,
    chain_id: i64,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    if let Some(target_id) = space_mgr.find_entity_by_tag(entity_id, &entity_tag) {
        let new_flags = if let Some(target) = space_mgr.get_entity_mut(target_id) {
            let old = target.interaction_type_flags;
            match operation.as_str() {
                "add" | "|" => target.interaction_type_flags |= mask,
                "remove" | "~" => target.interaction_type_flags &= !mask,
                "set" => target.interaction_type_flags = mask,
                _ => tracing::warn!(%operation, "Unknown interaction type operation"),
            }
            tracing::debug!(
                entity_id, %entity_tag, target_id, %operation, mask,
                old, new = target.interaction_type_flags, chain_id,
                "Content: set interaction type"
            );
            Some(target.interaction_type_flags)
        } else {
            None
        };

        if let Some(flags) = new_flags {
            let target_is_player = space_mgr.get_entity(target_id).is_some_and(|e| e.is_player);
            let witnesses = space_mgr.get_witnesses_of(target_id);
            for witness_id in witnesses {
                let _ = tx
                    .send(CellToBaseMsg::WitnessEntityMethod {
                        witness_id,
                        entity_id: target_id,
                        method_index: crate::mercury::method_idx::INTERACTION_TYPE,
                        args: (flags as u64).to_le_bytes().to_vec(),
                        entity_is_player: target_is_player,
                    })
                    .await;
            }
        }
    } else {
        tracing::debug!(entity_id, %entity_tag, chain_id, "Content: entity tag not found for SetInteractionType");
    }
}

/// `Action::SetAggression` — set the tagged NPC's behavior-aggression
/// level (`0` = passive, `≥1` = hostile-on-sight). The AI idle tick reads
/// this directly off the entity (no property-bag lookup) and seeds threat
/// on opposing-faction witnesses when `aggression > 0`.
///
/// The Python flow uses `setAggression` for the *durable behavior bit*
/// and a separate `threatGenerated` for the *initial threat seed* — see
/// `python/cell/missions/Castle_CellBlock/FindAmbernol.py:99-103`. Chain
/// 1032 follows the same pattern: this action sets the behavior, then a
/// `generate_threat` action focuses the NPC on the player who triggered
/// the chain. Without that explicit seed the drone would aggro on the
/// next idle tick anyway, but the seed delivers the correct frame
/// ordering (drone faces the player immediately, not 2s later).
pub(super) fn set_aggression(
    entity_tag: String,
    agg_level: i32,
    entity_id: u32,
    chain_id: i64,
    space_mgr: &mut SpaceManager,
) {
    if let Some(target_id) = space_mgr.find_entity_by_tag(entity_id, &entity_tag) {
        tracing::debug!(entity_id, %entity_tag, target_id, agg_level, chain_id, "Content: set aggression");
        if let Some(target) = space_mgr.get_entity_mut(target_id) {
            target.aggression = agg_level;
        }
    }
}

/// `Action::SetNpcPoi` — push the tagged NPC into `AiState::Investigating`
/// with the given POI. The NPC pathfinds to the POI on the next AI tick,
/// dwells `investigate_dwell_secs` (5s default), then returns to Idle.
///
/// Threat preemption: a damaged NPC mid-investigate transitions to
/// Fighting via the standard threat path; the POI persists on the
/// entity but isn't re-routed back to after the fight ends (content
/// authors fire a fresh `SetNpcPoi` if needed).
pub(super) fn set_npc_poi(
    entity_tag: String,
    x: f32,
    y: f32,
    z: f32,
    entity_id: u32,
    chain_id: i64,
    space_mgr: &mut SpaceManager,
) {
    use cimmeria_entity::cell_entity::AiState;
    if let Some(target_id) = space_mgr.find_entity_by_tag(entity_id, &entity_tag) {
        tracing::info!(
            entity_id, %entity_tag, target_id, x, y, z, chain_id,
            "Content: set NPC POI (Investigating)"
        );
        if let Some(target) = space_mgr.get_entity_mut(target_id) {
            target.poi = Some(cimmeria_common::Vector3::new(x, y, z));
            target.ai_state = AiState::Investigating;
            // Clear in-flight nav so the investigate handler can
            // pathfind to the POI from the current position rather
            // than continuing toward a stale patrol/wander waypoint.
            target.nav_path.clear();
        }
    } else {
        tracing::debug!(entity_id, %entity_tag, chain_id, "Content: entity tag not found for SetNpcPoi");
    }
}

/// `Action::SetFollowTarget` — set or clear the follow target for a
/// tagged NPC. When `target_tag` resolves, the NPC transitions to
/// `AiState::Follow` and maintains the distance band defined by
/// `follow_min/max_distance`. When `target_tag` is None (or doesn't
/// resolve), the follow state clears and the NPC returns to Idle.
pub(super) fn set_follow_target(
    entity_tag: String,
    target_tag: Option<String>,
    entity_id: u32,
    chain_id: i64,
    space_mgr: &mut SpaceManager,
) {
    use cimmeria_entity::cell_entity::AiState;
    let Some(npc_id) = space_mgr.find_entity_by_tag(entity_id, &entity_tag) else {
        tracing::debug!(entity_id, %entity_tag, chain_id, "Content: entity tag not found for SetFollowTarget");
        return;
    };
    let resolved_target = match &target_tag {
        Some(tag) => space_mgr.find_entity_by_tag(entity_id, tag),
        None => None,
    };
    tracing::info!(
        entity_id, %entity_tag, npc_id, ?target_tag, ?resolved_target, chain_id,
        "Content: set follow target"
    );
    if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
        npc.follow_target_id = resolved_target;
        // Transition to Follow if a target landed; otherwise drop to
        // Idle and clear any in-flight nav so the AI tick re-routes
        // cleanly.
        if resolved_target.is_some() {
            npc.ai_state = AiState::Follow;
        } else {
            npc.ai_state = AiState::Idle;
        }
        npc.nav_path.clear();
    }
}

/// `Action::SetNpcAiState` — push a tagged NPC into a content-reachable
/// terminal / scripted AI state. See [`Action::SetNpcAiState`] for the
/// admitted states and the rationale for the subset.
pub(super) fn set_npc_ai_state(
    entity_tag: String,
    state: cimmeria_content_engine::actions::NpcAiStateAction,
    entity_id: u32,
    chain_id: i64,
    space_mgr: &mut SpaceManager,
) {
    use cimmeria_content_engine::actions::NpcAiStateAction;
    use cimmeria_entity::cell_entity::AiState;
    let Some(target_id) = space_mgr.find_entity_by_tag(entity_id, &entity_tag) else {
        tracing::debug!(entity_id, %entity_tag, chain_id, "Content: entity tag not found for SetNpcAiState");
        return;
    };
    let new_state = match state {
        NpcAiStateAction::Idle => AiState::Idle,
        NpcAiStateAction::Despawning => AiState::Despawning,
        NpcAiStateAction::Submit => AiState::Submit,
        NpcAiStateAction::Error => AiState::Error,
    };
    tracing::info!(
        entity_id, %entity_tag, target_id, ?state, ?new_state, chain_id,
        "Content: set NPC AI state"
    );
    if let Some(npc) = space_mgr.get_entity_mut(target_id) {
        npc.ai_state = new_state;
        // Clear in-flight nav so the new-state handler can re-route.
        npc.nav_path.clear();
    }
}

/// `Action::DestroyTaggedEntity` — remove the tagged entity from the
/// space. Witnesses get the destroy on the next AoI sweep.
pub(super) fn destroy_tagged_entity(
    entity_tag: String,
    entity_id: u32,
    chain_id: i64,
    space_mgr: &mut SpaceManager,
) {
    if let Some(target_id) = space_mgr.find_entity_by_tag(entity_id, &entity_tag) {
        tracing::info!(entity_id, %entity_tag, target_id, chain_id, "Content: destroying tagged entity");
        space_mgr.destroy_entity(target_id);
    } else {
        tracing::debug!(entity_id, %entity_tag, chain_id, "Content: entity tag not found for DestroyTaggedEntity");
    }
}

/// `Action::GenerateThreat` — push the player's threat level on the tagged
/// NPC. If a state-flag transition lands (NPC enters combat), broadcast the
/// new state to the originating player so the in-combat HUD flips.
pub(super) async fn generate_threat(
    entity_tag: Option<String>,
    threat_level: i32,
    entity_id: u32,
    chain_id: i64,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // Generate threat on the NPC (found by tag) from the player.
    // If no entity_tag, the threat is on the player entity itself (ignored by combat).
    if let Some(tag) = &entity_tag {
        if let Some(target_id) = space_mgr.find_entity_by_tag(entity_id, tag) {
            tracing::info!(
                entity_id, %tag, target_id, threat_level, chain_id,
                "Content: generate threat on NPC from player"
            );
            if let Some(new_state) = crate::cell::combat::generate_threat(
                space_mgr,
                entity_id, // attacker = the player
                target_id, // target = the NPC
                threat_level as f32,
            ) {
                // Player just entered combat. `enter_player_combat`
                // (inside `combat::generate_threat`) flipped
                // `weapon_holstered = false` via
                // `sync_holster_to_combat(true)`. We must broadcast
                // BOTH:
                //
                //   - `BeingAppearance` refresh, so the client's
                //     cached `ComponentList` picks up the now-drawn
                //     weapon mesh. Without this the client keeps
                //     rendering the holstered/empty-hand mesh while
                //     the server thinks the weapon is drawn — fire
                //     animations play against empty hands and the
                //     in-combat pose shows no weapon. Pre-fix
                //     symptom from chain 1032 (Ambernol pickup
                //     triggers drone aggro): "fists go into combat
                //     position, player shoots without a weapon,
                //     fists holster when aggro drops."
                //
                //   - `onStateFieldUpdate`, so the in-combat HUD /
                //     targeting cursor / state-bit-derived UI flips.
                //
                // **Order matters**: appearance BEFORE state field.
                // Both flow through the same client-side state-machine
                // entry point (`FUN_00e7b4c0`) but only the appearance
                // path triggers the socket re-attach (`FUN_00e7b7c0`)
                // that writes the weapon-category byte. If
                // `BSF_InCombat` flips first, the unholster animation
                // starts before the weapon mesh is attached — hand
                // reaches for the holster, grabs air, mesh snaps in
                // mid-animation (the "splinch" documented in
                // `apply_damage_to_target` and reproduced here for the
                // chain-driven aggro path).
                //
                // This mirrors the existing belt-and-braces in
                // `damage_apply::apply_damage_to_target` and the
                // appearance-only broadcast in
                // `npc_ai::npc_ai_idle_auto_aggro` (which intentionally
                // suppresses the state field to avoid the "ghost
                // combat HUD" carve-out). Three callers of
                // `combat::generate_threat`; this is the third to
                // gain the appearance refresh.
                crate::cell::abilities::request_appearance_refresh(entity_id, tx, space_mgr).await;
                crate::cell::abilities::send_entity_method(
                    entity_id,
                    crate::mercury::method_idx::ON_STATE_FIELD_UPDATE,
                    new_state.to_le_bytes().to_vec(),
                    tx,
                    space_mgr,
                )
                .await;
            }
        }
    } else {
        tracing::debug!(
            entity_id,
            threat_level,
            chain_id,
            "Content: generate threat (no target tag, skipped)"
        );
    }
}

/// `Action::SetVisible` — emit a per-target `onVisible(0|1)` to flip the
/// client-side visibility bit on the tagged entity.
pub(super) async fn set_visible(
    entity_tag: String,
    visible: bool,
    entity_id: u32,
    chain_id: i64,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    if let Some(target_id) = space_mgr.find_entity_by_tag(entity_id, &entity_tag) {
        tracing::debug!(entity_id, %entity_tag, target_id, visible, chain_id, "Content: set visible");
        let vis_byte: u8 = if visible { 1 } else { 0 };
        let _ = tx
            .send(CellToBaseMsg::EntityMethodCall {
                entity_id: target_id,
                method_index: crate::mercury::method_idx::ON_VISIBLE,
                args: vec![vis_byte],
            })
            .await;
    }
}

/// `Action::MoveEntity` — reposition either the acting player or a
/// tagged NPC. One seed verb, two very different mechanisms:
///
/// - **`use_player: true`** (seed rows 3007 / 3028, both with
///   `target_key` NULL) moves the *player* who fired the chain. That has
///   to go through [`transport::teleport`], which owns the spatial-grid
///   update, the `note_authorized_teleport` validator reseed, the
///   `CellToBaseMsg::TeleportPlayer` forced-position snap and the
///   prev-position anti-camera-snap. `update_entity_position` alone
///   moves the server's idea of the player and nothing the client sees.
/// - **`entity_tag`** (rows 3005 / 3009 / 3011) repositions an NPC, which
///   is exactly [`move_waypoint`]'s job.
///
/// `use_player` wins when both are set — the seed never does that, but
/// "move the player" is the more specific instruction.
///
/// The `world` param is a cross-world guard, not a destination selector.
/// All five seeded rows name the world they are already on, so it
/// normally resolves to the same-world path; a genuine mismatch on the
/// player path routes to [`transport::cross_world_teleport`] instead of
/// silently dropping the avatar at those coordinates on the wrong map.
/// A mismatch on the NPC path is refused: NPCs have no gate-travel
/// equivalent, and snapping one to coordinates in a world it isn't in
/// would place it somewhere arbitrary.
pub(super) async fn move_entity(
    entity_tag: Option<String>,
    destination: [f32; 3],
    world: Option<String>,
    use_player: Option<bool>,
    entity_id: u32,
    chain_id: i64,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    if use_player == Some(true) {
        let current_world = space_mgr.get_entity_world_name(entity_id);
        // Only a *known* mismatch counts. An unresolvable current world
        // (entity already gone) falls through to the same-world path,
        // which fail-softs on the missing entity rather than firing a
        // gate travel for a player the cell can't see.
        let cross_world = match (world.as_deref(), current_world.as_deref()) {
            (Some(want), Some(cur)) => want != cur,
            _ => false,
        };
        if cross_world {
            // `world` is Some in this branch by construction.
            let target_world = world.unwrap_or_default();
            tracing::info!(
                entity_id, %target_world, ?destination, chain_id,
                "Content: move_entity crosses worlds -- routing through gate travel"
            );
            transport::cross_world_teleport(
                target_world,
                destination,
                entity_id,
                chain_id,
                tx,
                space_mgr,
            )
            .await;
            return;
        }
        // Same-world player move. `transport::teleport` treats its
        // `space_id` as the *destination* space and warns on a mismatch,
        // so pass the player's current space to keep it on the
        // same-space path. `0` is its "unspecified" sentinel and is what
        // a missing entity degrades to.
        let space_id = space_mgr
            .get_entity(entity_id)
            .map(|e| e.space_id.0)
            .unwrap_or(0);
        transport::teleport(space_id, destination, entity_id, chain_id, tx, space_mgr).await;
        return;
    }

    let Some(entity_tag) = entity_tag else {
        tracing::warn!(
            entity_id,
            ?destination,
            chain_id,
            "MoveEntity: row has neither use_player nor target_key -- nothing moved"
        );
        return;
    };

    let Some(target_id) = space_mgr.find_entity_by_tag(entity_id, &entity_tag) else {
        tracing::warn!(
            entity_id, %entity_tag, ?destination, chain_id,
            "MoveEntity: no entity matched tag in the source entity's space -- NPC reposition skipped"
        );
        return;
    };

    if let (Some(want), Some(cur)) = (
        world.as_deref(),
        space_mgr.get_entity_world_name(target_id).as_deref(),
    ) {
        if want != cur {
            tracing::warn!(
                entity_id, %entity_tag, target_id, want_world = %want, current_world = %cur,
                chain_id,
                "MoveEntity: cross-world NPC move is not supported -- NPC reposition skipped"
            );
            return;
        }
    }

    move_waypoint(entity_tag, destination, entity_id, chain_id, space_mgr);
}

/// `Action::MoveWaypoint` — snap the tagged entity to a new position.
/// No yaw/orientation change; chains call `update_entity_position` directly.
pub(super) fn move_waypoint(
    entity_tag: String,
    destination: [f32; 3],
    entity_id: u32,
    chain_id: i64,
    space_mgr: &mut SpaceManager,
) {
    if let Some(target_id) = space_mgr.find_entity_by_tag(entity_id, &entity_tag) {
        tracing::debug!(entity_id, %entity_tag, target_id, ?destination, chain_id, "Content: move waypoint");
        space_mgr.update_entity_position(target_id, destination, [0, 0, 0], [0.0; 3]);
        // Authorized server move: reseed the movement-validator clock for
        // the moved entity (harmless for NPC targets — they never pass
        // through the client-position validator).
        space_mgr.note_authorized_teleport(target_id);
    }
}
