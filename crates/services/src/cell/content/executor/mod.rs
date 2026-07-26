//! Action execution — dispatches resolved content engine actions against the
//! game state (missions, items, dialogs, interactions, etc.).
//!
//! Each match arm forwards to a per-family handler in a sibling module:
//!
//! - [`mission`]   — accept/advance/complete/abandon, advance step, complete objective
//! - [`inventory`] — grant/remove items, bandolier seeding
//! - [`dialog`]    — display, add/remove dialog set, add dialog
//! - [`stats`]     — `Action::ChangeStat`
//! - [`world`]     — interaction-type/visibility/destroy/move/threat/aggression
//! - [`counter`]   — increment/reset
//! - [`transport`] — teleport, ring transporter
//!
//! Single-arm actions with no shared helpers (PlaySequence, StartMinigame,
//! SystemMessage, SendMessage, SetActiveSlot, TriggerChain, fallback) stay
//! inline in the match below.

use tokio::sync::mpsc;

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ResolvedActions;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

mod counter;
mod dialog;
mod inventory;
mod mission;
mod stats;
mod transport;
mod world;

#[cfg(test)]
mod tests;

// Re-export `item_container` so the parent module's test suite (which
// imports `super::executor::item_container`) keeps working without
// touching the call site. Only the parent's `#[cfg(test)]` block reads
// it through this path, so gate the re-export on `cfg(test)` to keep
// the unused-imports lint happy on release builds.
#[cfg(test)]
pub(super) use inventory::item_container;

/// Execute resolved actions from the content engine against the game state.
///
/// `level = "info"` because chain firings are low-rate and high-signal
/// — every "player did X, missions did Y" sequence shows up as one
/// span containing the action vector. The `actions_len` field gives a
/// quick "how complex is this chain?" view in SigNoz.
#[tracing::instrument(
    name = "content.execute_actions",
    level = "info",
    skip_all,
    fields(entity_id, player_id, actions_len = resolved.actions.len()),
)]
pub(super) async fn execute_actions(
    resolved: ResolvedActions,
    entity_id: u32,
    player_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &cimmeria_content_engine::chain::ChainEngine,
) {
    // Destructure so the inner loop can move `actions` while later
    // action branches still read trigger-time `params`. `Action::RemoveItem`
    // looks up `instance_id` here to consume the exact stack the
    // player clicked on `useItem`.
    let ResolvedActions { actions, params } = resolved;
    for (chain_id, action) in actions {
        match action {
            Action::AcceptMission { mission_id } | Action::AdvanceMission { mission_id } => {
                mission::accept_or_advance(
                    mission_id, entity_id, player_id, chain_id, tx, space_mgr, engine,
                )
                .await;
            }
            Action::CompleteMission { mission_id } => {
                mission::complete(
                    mission_id, entity_id, player_id, chain_id, tx, space_mgr, engine,
                )
                .await;
            }
            Action::GrantItem {
                item_id,
                count,
                container_id,
            } => {
                inventory::grant(
                    item_id,
                    count,
                    container_id,
                    entity_id,
                    player_id,
                    chain_id,
                    tx,
                    space_mgr,
                )
                .await;
            }
            Action::DisplayDialog { dialog_id }
            | Action::StartDialog {
                dialog_set_id: dialog_id,
            } => {
                dialog::display(dialog_id, entity_id, chain_id, &params, tx, space_mgr).await;
            }
            Action::PlaySequence { sequence_id } => {
                tracing::info!(
                    entity_id,
                    sequence_id,
                    chain_id,
                    "Content: playing sequence"
                );
                let mut args = Vec::with_capacity(26);
                args.extend_from_slice(&sequence_id.to_le_bytes()); // KismetEventSetSeqID
                args.extend_from_slice(&(entity_id as i32).to_le_bytes()); // SourceID
                args.extend_from_slice(&(entity_id as i32).to_le_bytes()); // TargetID
                args.push(1); // PrimaryTarget = true
                args.extend_from_slice(&0.0f32.to_le_bytes()); // ImpactTime
                args.extend_from_slice(&0u32.to_le_bytes()); // NameValuePairs count = 0
                args.push(0); // ViewType = 0
                args.extend_from_slice(&0i32.to_le_bytes()); // InstanceId
                if let Err(e) = tx
                    .send(CellToBaseMsg::EntityMethodCall {
                        entity_id,
                        method_index: crate::mercury::method_idx::ON_SEQUENCE,
                        args,
                    })
                    .await
                {
                    // cell→base channel drop swallows the
                    // cinematic — player misses the visual cue for the
                    // chain action. warn! so a missing kismet correlates
                    // with a log line.
                    tracing::warn!(
                        entity_id,
                        sequence_id,
                        chain_id,
                        "PlaySequence: cell→base send failed -- kismet sequence will not play: {e}"
                    );
                }
            }
            Action::AdvanceStep {
                mission_id,
                step_id,
            } => {
                mission::advance_step(
                    mission_id, step_id, entity_id, player_id, chain_id, tx, space_mgr,
                )
                .await;
            }
            Action::AddDialogSet {
                dialog_set_id,
                slot,
                mission_id: _,
            } => {
                dialog::add_dialog_set(dialog_set_id, slot, entity_id, chain_id, tx, space_mgr)
                    .await;
            }
            Action::RemoveDialogSet {
                dialog_set_id,
                slot,
            } => {
                dialog::remove_dialog_set(dialog_set_id, slot, entity_id, chain_id, tx, space_mgr)
                    .await;
            }
            Action::RemoveItem { item_id, count } => {
                inventory::remove(item_id, count, entity_id, player_id, chain_id, &params, tx)
                    .await;
            }
            Action::ChangeStat {
                stat_id,
                min,
                max,
                set_to_max,
                amount,
                use_ammo_stat,
            } => {
                stats::change_stat(
                    stat_id,
                    min,
                    max,
                    set_to_max,
                    amount,
                    use_ammo_stat,
                    entity_id,
                    chain_id,
                    tx,
                    space_mgr,
                )
                .await;
            }
            Action::SetInteractionType {
                entity_tag,
                operation,
                mask,
            } => {
                world::set_interaction_type(
                    entity_tag, operation, mask, entity_id, chain_id, tx, space_mgr,
                )
                .await;
            }
            Action::StartMinigame {
                minigame_type,
                on_victory_chains,
            } => {
                tracing::info!(entity_id, %minigame_type, ?on_victory_chains, chain_id, "Content: starting minigame");
                if let Err(e) = tx
                    .send(CellToBaseMsg::StartMinigame {
                        entity_id,
                        player_id,
                        game_name: minigame_type.clone(),
                        difficulty: 1, // TODO: parse from chain params when difficulty field is added
                        on_victory_chains: on_victory_chains.clone(),
                    })
                    .await
                {
                    // drop here means the minigame never
                    // launches but the player click already fired —
                    // chain stalls with no signal.
                    tracing::warn!(
                        entity_id,
                        %minigame_type,
                        chain_id,
                        "StartMinigame: cell→base send failed -- minigame will not launch: {e}"
                    );
                }
            }
            Action::SetAggression {
                entity_tag,
                level: agg_level,
            } => {
                world::set_aggression(entity_tag, agg_level, entity_id, chain_id, space_mgr);
            }
            Action::SetNpcPoi {
                entity_tag,
                x,
                y,
                z,
            } => {
                world::set_npc_poi(entity_tag, x, y, z, entity_id, chain_id, space_mgr);
            }
            Action::SetFollowTarget {
                entity_tag,
                target_tag,
            } => {
                world::set_follow_target(entity_tag, target_tag, entity_id, chain_id, space_mgr);
            }
            Action::SetNpcAiState { entity_tag, state } => {
                world::set_npc_ai_state(entity_tag, state, entity_id, chain_id, space_mgr);
            }
            Action::DestroyTaggedEntity { entity_tag } => {
                world::destroy_tagged_entity(entity_tag, entity_id, chain_id, space_mgr);
            }
            Action::TriggerTransporter { region_id } => {
                transport::trigger_transporter(
                    region_id, entity_id, chain_id, tx, space_mgr, engine,
                )
                .await;
            }
            Action::Teleport { space_id, position } => {
                transport::teleport(space_id, position, entity_id, chain_id, tx, space_mgr).await;
            }
            Action::CrossWorldTeleport {
                world_name,
                position,
            } => {
                transport::cross_world_teleport(
                    world_name, position, entity_id, chain_id, tx, space_mgr,
                )
                .await;
            }
            Action::SystemMessage { message_id } => {
                // TODO: Wire format for system messages is unknown. The previous
                // implementation incorrectly used onPlayerCommunication (method 28)
                // which caused garbled chat spam ("[] says") and client freezes.
                // Needs RE to find the correct client method for localized string
                // ID display (possibly onErrorCode or a UI-specific method).
                tracing::info!(
                    entity_id,
                    message_id,
                    chain_id,
                    "Content: system message (stub — correct wire format TBD)"
                );
            }
            Action::AbandonMission { mission_id } => {
                mission::abandon(mission_id, entity_id, chain_id, tx, space_mgr).await;
            }
            Action::IncrementCounter {
                counter_name,
                amount,
            } => {
                counter::increment(
                    counter_name,
                    amount,
                    entity_id,
                    player_id,
                    chain_id,
                    space_mgr,
                );
            }
            Action::ResetCounter { counter_name } => {
                counter::reset(counter_name, entity_id, player_id, chain_id, space_mgr);
            }
            Action::CompleteObjective {
                mission_id,
                objective_id,
            } => {
                mission::complete_objective(
                    mission_id,
                    objective_id,
                    entity_id,
                    chain_id,
                    tx,
                    space_mgr,
                )
                .await;
            }
            Action::SendMessage { channel, message } => {
                tracing::info!(entity_id, %channel, %message, chain_id, "Content: sending message");
            }
            Action::AddDialog {
                dialog_set_id,
                entity_template,
                mission_id: _,
            } => {
                dialog::add_dialog(
                    dialog_set_id,
                    entity_template,
                    entity_id,
                    chain_id,
                    tx,
                    space_mgr,
                )
                .await;
            }
            Action::GenerateThreat {
                entity_tag,
                threat_level,
            } => {
                world::generate_threat(
                    entity_tag,
                    threat_level,
                    entity_id,
                    chain_id,
                    tx,
                    space_mgr,
                )
                .await;
            }
            Action::SetVisible {
                entity_tag,
                visible,
            } => {
                world::set_visible(entity_tag, visible, entity_id, chain_id, tx, space_mgr).await;
            }
            Action::MoveEntity {
                entity_tag,
                destination,
                world,
                use_player,
            } => {
                world::move_entity(
                    entity_tag,
                    destination,
                    world,
                    use_player,
                    entity_id,
                    chain_id,
                    tx,
                    space_mgr,
                )
                .await;
            }
            Action::MoveWaypoint {
                entity_tag,
                destination,
                speed: _,
            } => {
                world::move_waypoint(entity_tag, destination, entity_id, chain_id, space_mgr);
            }
            Action::SetActiveSlot { bag_id, slot } => {
                tracing::info!(
                    entity_id,
                    bag_id,
                    slot,
                    chain_id,
                    "Content: set active slot"
                );
                // Send onActiveSlotUpdate(bagId, slotId) — slotId is 1-indexed on wire
                let mut args = Vec::with_capacity(8);
                args.extend_from_slice(&bag_id.to_le_bytes());
                args.extend_from_slice(&(slot + 1).to_le_bytes()); // 1-indexed
                if let Err(e) = tx
                    .send(CellToBaseMsg::EntityMethodCall {
                        entity_id,
                        method_index: crate::mercury::method_idx::ON_ACTIVE_SLOT_UPDATE,
                        args,
                    })
                    .await
                {
                    // same shape as PlaySequence/StartMinigame;
                    // a dropped active-slot update leaves the client showing
                    // the wrong bandolier slot until the next equip toggle.
                    tracing::warn!(
                        entity_id,
                        bag_id,
                        slot,
                        chain_id,
                        "SetActiveSlot: cell→base send failed -- active slot not synced: {e}"
                    );
                }
            }
            Action::TriggerChain {
                chain_id: target_chain_id,
            } => {
                tracing::debug!(
                    entity_id,
                    target_chain_id,
                    chain_id,
                    "Content: trigger chain (caller must re-dispatch)"
                );
            }
            other => {
                tracing::debug!(entity_id, chain_id, action = ?other, "Content: unhandled action");
            }
        }
    }
}
