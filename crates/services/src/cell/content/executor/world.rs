//! World-mutation action handlers: interaction-type flags, visibility,
//! destruction, waypoint movement, aggression, threat generation.
//!
//! These all locate a target entity by tag and either flip a flag or push
//! a state change.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

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
            let witnesses = space_mgr.get_witnesses_of(target_id);
            for witness_id in witnesses {
                let _ = tx
                    .send(CellToBaseMsg::WitnessEntityMethod {
                        witness_id,
                        entity_id: target_id,
                        method_index: crate::mercury::method_idx::INTERACTION_TYPE,
                        args: (flags as u64).to_le_bytes().to_vec(),
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
    }
}
