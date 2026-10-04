//! Per-action dispatch: [`execute_one`] runs one resolved action against the
//! game state. Each match arm forwards to its family's sibling module; the
//! single-arm actions with no shared helpers (PlaySequence, StartMinigame,
//! SystemMessage, SendMessage, GrantXP, SetActiveSlot, TriggerChain, the
//! fallback) are handled inline here.

use std::collections::HashMap;

use tokio::sync::mpsc;

use cimmeria_content_engine::actions::Action;

use super::{
    ability_granter, bark, black_market, counter, dialog, inventory, loot, mail, mission, spawn,
    stargate, stats, transport, world,
};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Dispatch a single resolved action against the game state. Shared by
/// `execute_actions`'s immediate (`delay_ms == 0`) path and
/// `deferred_content_action_tick`'s drain of elapsed `delay_ms > 0`
/// entries — the match arms below are identical either way; only *when*
/// this function runs differs.
pub(super) async fn execute_one(
    chain_id: i64,
    action: Action,
    entity_id: u32,
    player_id: i32,
    params: &HashMap<String, serde_json::Value>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &cimmeria_content_engine::chain::ChainEngine,
) {
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
            dialog::display(dialog_id, entity_id, chain_id, params, tx, space_mgr).await;
        }
        Action::PlaySequence { sequence_id } => {
            tracing::info!(
                entity_id,
                entity_name = space_mgr.entity_names(entity_id).entity_name,
                sequence_id,
                sequence_name = cimmeria_names::book().sequence(sequence_id),
                chain_id,
                chain_name = cimmeria_names::book().chain(chain_id),
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
                    entity_name = space_mgr.entity_names(entity_id).entity_name,
                    sequence_id,
                    sequence_name = cimmeria_names::book().sequence(sequence_id),
                    chain_id,
                    chain_name = cimmeria_names::book().chain(chain_id),
                    "PlaySequence: cell→base send failed -- kismet sequence will not play: {e}"
                );
            }
        }
        Action::AdvanceStep {
            mission_id,
            step_id,
        } => {
            mission::advance_step(
                mission_id, step_id, entity_id, player_id, chain_id, tx, space_mgr, engine,
            )
            .await;
        }
        Action::AddDialogSet {
            dialog_set_id,
            slot,
            mission_id: _,
        } => {
            dialog::add_dialog_set(dialog_set_id, slot, entity_id, chain_id, tx, space_mgr).await;
        }
        Action::RemoveDialogSet {
            dialog_set_id,
            slot,
        } => {
            dialog::remove_dialog_set(dialog_set_id, slot, entity_id, chain_id, tx, space_mgr)
                .await;
        }
        Action::RemoveItem { item_id, count } => {
            inventory::remove(
                item_id, count, entity_id, player_id, chain_id, params, tx, space_mgr,
            )
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
            difficulty,
            on_victory_chains,
        } => {
            tracing::info!(
                entity_id,
                entity_name = space_mgr.entity_names(entity_id).entity_name,
                %minigame_type,
                difficulty,
                ?on_victory_chains,
                chain_id,
                chain_name = cimmeria_names::book().chain(chain_id),
                "Content: starting minigame"
            );
            if let Err(e) = tx
                .send(CellToBaseMsg::StartMinigame {
                    entity_id,
                    player_id,
                    game_name: minigame_type.clone(),
                    // Range-checked 1-5 at load time (loader/action.rs);
                    // the seed default is 1.
                    difficulty,
                    on_victory_chains: on_victory_chains.clone(),
                })
                .await
            {
                // drop here means the minigame never
                // launches but the player click already fired —
                // chain stalls with no signal.
                tracing::warn!(
                    entity_id,
                    entity_name = space_mgr.entity_names(entity_id).entity_name,
                    %minigame_type,
                    chain_id,
                    chain_name = cimmeria_names::book().chain(chain_id),
                    "StartMinigame: cell→base send failed -- minigame will not launch: {e}"
                );
            }
        }
        Action::SetAggression {
            entity_tag,
            level: agg_level,
        } => {
            world::set_aggression(entity_tag, agg_level, entity_id, chain_id, tx, space_mgr).await;
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
            use_player,
        } => {
            world::set_follow_target(
                entity_tag, target_tag, use_player, entity_id, chain_id, space_mgr,
            );
        }
        Action::SetNpcAiState { entity_tag, state } => {
            world::set_npc_ai_state(entity_tag, state, entity_id, chain_id, space_mgr);
        }
        // `destroy_entity` is the older spelling of `despawn_entity` and
        // routes identically — same `despawn_by_tag`, same `LeftAoI` fan-out
        // before the destroy. The two arms differ only in the `verb` they
        // log. (The pass-through wrapper this used to call was deleted in
        // the PR #662 review; the routing history lives on `despawn_by_tag`.)
        Action::DestroyTaggedEntity { entity_tag } => {
            spawn::despawn_by_tag(
                entity_tag,
                entity_id,
                chain_id,
                "destroy_entity",
                tx,
                space_mgr,
            )
            .await;
        }
        Action::DespawnEntity { entity_tag } => {
            spawn::despawn_by_tag(
                entity_tag,
                entity_id,
                chain_id,
                "despawn_entity",
                tx,
                space_mgr,
            )
            .await;
        }
        Action::SpawnEntity {
            template_id,
            position,
            heading,
            tag,
            is_stationary,
            aggression,
            allow_shared,
        } => {
            spawn::spawn_entity(
                template_id,
                position,
                heading,
                tag,
                is_stationary,
                aggression,
                allow_shared,
                entity_id,
                chain_id,
                space_mgr,
            )
            .await;
        }
        Action::TriggerTransporter { region_id } => {
            transport::trigger_transporter(region_id, entity_id, chain_id, tx, space_mgr, engine)
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
            // TODO: wire format unknown. Method 28 with a string id garbled
            // chat and froze clients; needs RE (onErrorCode or a UI method?).
            tracing::info!(
                entity_id,
                entity_name = space_mgr.entity_names(entity_id).entity_name,
                message_id, // nt:id-only an unresolved string id; its wire format and table are unknown
                chain_id,
                chain_name = cimmeria_names::book().chain(chain_id),
                "Content: system message (stub — correct wire format TBD)"
            );
        }
        Action::NpcBark {
            screen_id,
            speaker,
            channel,
        } => {
            bark::npc_bark(
                screen_id, &speaker, channel, entity_id, chain_id, tx, space_mgr,
            )
            .await;
        }
        Action::AbandonMission { mission_id } => {
            mission::abandon(
                mission_id, entity_id, player_id, chain_id, tx, space_mgr, engine,
            )
            .await;
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
                player_id,
                chain_id,
                tx,
                space_mgr,
                engine,
            )
            .await;
        }
        Action::SendMessage { channel, message } => {
            tracing::info!(
                entity_id,
                entity_name = space_mgr.entity_names(entity_id).entity_name,
                %channel,
                %message,
                chain_id,
                chain_name = cimmeria_names::book().chain(chain_id),
                "Content: sending message"
            );
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
            world::generate_threat(entity_tag, threat_level, entity_id, chain_id, tx, space_mgr)
                .await;
        }
        Action::SetVisible {
            entity_tag,
            visible,
        } => {
            world::set_visible(entity_tag, visible, entity_id, chain_id, tx, space_mgr).await;
        }
        Action::GrantXP { amount } => {
            if amount == 0 {
                tracing::warn!(
                    entity_id,
                    entity_name = space_mgr.entity_names(entity_id).entity_name,
                    chain_id,
                    chain_name = cimmeria_names::book().chain(chain_id),
                    "GrantXP: action resolved with amount 0 -- seed row is missing \
                     its `amount` param; no XP awarded"
                );
                return;
            }
            tracing::info!(
                entity_id,
                entity_name = space_mgr.entity_names(entity_id).entity_name,
                xp = amount,
                chain_id,
                chain_name = cimmeria_names::book().chain(chain_id),
                "Content: granting XP"
            );
            // Same round-trip mob-kill XP and GM `gmGiveXp` use — base
            // owns the XP/level write and the client notifications.
            // `gm_feedback_to: None`: a chain grant is gameplay, not a
            // GM action, so it must not emit a GM feedback line.
            if let Err(e) = tx
                .send(CellToBaseMsg::GrantXP {
                    entity_id,
                    xp_amount: amount,
                    gm_feedback_to: None,
                })
                .await
            {
                tracing::error!(
                    entity_id,
                    entity_name = space_mgr.entity_names(entity_id).entity_name,
                    xp = amount,
                    chain_id,
                    chain_name = cimmeria_names::book().chain(chain_id),
                    error = %e,
                    "GrantXP: cell→base send failed -- player silently loses the chain's XP reward"
                );
            }
        }
        Action::GmAbilityBulk { change } => {
            ability_granter::run(change, entity_id, chain_id, params, tx, space_mgr).await;
        }
        Action::OpenBlackMarket => {
            black_market::open(entity_id, chain_id, params, tx, space_mgr).await;
        }
        action @ Action::OpenLoot { .. } => {
            loot::open_loot(
                action, entity_id, player_id, chain_id, params, tx, space_mgr,
            )
            .await;
        }
        Action::GrantStargateAddress { stargate_id } => {
            stargate::grant_stargate_address(
                stargate_id,
                entity_id,
                player_id,
                chain_id,
                tx,
                space_mgr,
            )
            .await;
        }
        action @ Action::SendSystemMail { .. } => {
            mail::send_system_mail(action, entity_id, player_id, chain_id, tx, space_mgr).await;
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
            world::move_waypoint(entity_tag, destination, entity_id, chain_id, tx, space_mgr).await;
        }
        Action::SetActiveSlot { bag_id, slot } => {
            tracing::info!(
                entity_id,
                entity_name = space_mgr.entity_names(entity_id).entity_name,
                bag_id, // nt:id-only a bandolier bag slot index, not a seed row
                slot,
                chain_id,
                chain_name = cimmeria_names::book().chain(chain_id),
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
                    entity_name = space_mgr.entity_names(entity_id).entity_name,
                    bag_id, // nt:id-only a bandolier bag slot index, not a seed row
                    slot,
                    chain_id,
                    chain_name = cimmeria_names::book().chain(chain_id),
                    "SetActiveSlot: cell→base send failed -- active slot not synced: {e}"
                );
            }
        }
        Action::LaunchAbility {
            ability_id,
            entity_tag,
        } => {
            // `entity_tag: None` means "the entity that fired the
            // chain" — for a `player_loaded` trigger that is the
            // player themselves, which is exactly the self-target the
            // combat pipeline's friendly-fire gate would reject.
            let target_id = match entity_tag.as_deref() {
                None => Some(entity_id),
                Some(tag) => space_mgr.find_entity_by_tag(entity_id, tag),
            };
            match target_id {
                Some(target_id) => {
                    super::super::effect_apply::apply_ability_effects(
                        ability_id, target_id, entity_id, chain_id, tx, space_mgr,
                    )
                    .await;
                }
                None => {
                    // Tagged NPC isn't spawned (or is in another
                    // space). Same shape as the other tag-resolving
                    // world actions: skip, don't fall back to self —
                    // a debuff meant for an NPC must never land on
                    // the player.
                    tracing::warn!(
                        entity_id,
                        entity_name = space_mgr.entity_names(entity_id).entity_name,
                        ability_id,
                        ability_name = cimmeria_names::book().ability(ability_id),
                        entity_tag = ?entity_tag,
                        chain_id,
                        chain_name = cimmeria_names::book().chain(chain_id),
                        "LaunchAbility: no entity matched the tag -- ability not launched"
                    );
                }
            }
        }
        Action::ApplyEffect {
            effect_id,
            duration_secs: _,
        } => {
            // `duration_secs` is hardcoded `None` by the action
            // loader and the effect's own `pulse_count` /
            // `pulse_duration` already carry the duration, so there
            // is nothing to honour here yet.
            //
            // This arm is correct but currently unreachable: the only
            // seeded `apply_effect` row is on an `effect`-scoped
            // chain, and no `effect_*` trigger is dispatched anywhere
            // in the cell service. It fires as soon as that
            // dispatch lands.
            super::super::effect_apply::apply_effect(
                effect_id, entity_id, entity_id, chain_id, tx, space_mgr,
            )
            .await;
        }
        Action::TriggerChain {
            chain_id: target_chain_id,
        } => {
            tracing::debug!(
                entity_id,
                entity_name = space_mgr.entity_names(entity_id).entity_name,
                target_chain_id,
                target_chain_name = cimmeria_names::book().chain(target_chain_id),
                chain_id,
                chain_name = cimmeria_names::book().chain(chain_id),
                "Content: trigger chain (caller must re-dispatch)"
            );
        }
        other => {
            tracing::debug!(
                entity_id,
                entity_name = space_mgr.entity_names(entity_id).entity_name,
                chain_id,
                chain_name = cimmeria_names::book().chain(chain_id),
                action = ?other,
                "Content: unhandled action"
            );
        }
    }
}
