//! SGWPlayer own exposed CellMethods (indices 67–108).

use tokio::sync::mpsc;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::stats::{HEALTH, FOCUS};
use crate::cell::combat;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

pub const CALL_FOR_AID: u16 = 67;
pub const USE_ABILITY: u16 = 68;
pub const USE_ABILITY_ON_GROUND: u16 = 69;
pub const RESPAWN: u16 = 70;
pub const UNSTUCK: u16 = 71;
pub const RESET_MY_ABILITIES: u16 = 72;
pub const WHO: u16 = 73;
pub const INTERACT: u16 = 74;
pub const DIALOG_BUTTON_CHOICE: u16 = 75;
pub const INITIAL_RESPONSE: u16 = 76;
pub const TRAIN_ABILITY: u16 = 77;
pub const PURCHASE_ITEMS: u16 = 78;
pub const SELL_ITEMS: u16 = 79;
pub const BUYBACK_ITEMS: u16 = 80;
pub const REPAIR_ITEMS: u16 = 81;
pub const RECHARGE_ITEMS: u16 = 82;
pub const SET_AUTO_CYCLE: u16 = 83;
pub const LOOT_ITEM: u16 = 84;
pub const TRIGGER_REGION: u16 = 85;
pub const REQUEST_RELOAD: u16 = 86;
pub const CHOSEN_REWARDS: u16 = 87;
pub const PET_INVOKE_ABILITY: u16 = 88;
pub const PET_ABILITY_TOGGLE: u16 = 89;
pub const PET_CHANGE_STANCE: u16 = 90;
pub const SET_RING_TRANSPORTER_DEST: u16 = 91;
pub const WORLD_INSTANCE_RESET: u16 = 92;
pub const UPDATE_SYSTEM_OPTIONS: u16 = 93;
pub const ORG_CREATION: u16 = 94;
pub const SPEND_APPLIED_SCIENCE_POINTS: u16 = 95;
pub const CRAFT: u16 = 96;
pub const RESEARCH: u16 = 97;
pub const REVERSE_ENGINEER: u16 = 98;
pub const ALLOYING: u16 = 99;
pub const RESPEC_CRAFTING: u16 = 100;
pub const CLIENT_CHALLENGE_RESPONSE: u16 = 101;
pub const SEND_DUEL_RESPONSE: u16 = 102;
pub const DUEL_FORFEIT: u16 = 103;
pub const TRADE_REQUEST: u16 = 104;
pub const TRADE_REQUEST_CANCEL: u16 = 105;
pub const TRADE_UPDATE_PROPOSAL: u16 = 106;
pub const TRADE_LOCK_STATE: u16 = 107;
pub const CANCEL_MOVIE: u16 = 108;

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) -> bool {
    match method_index {
        CALL_FOR_AID => {
            if args.len() >= 4 {
                let respawner_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, respawner_id, "callForAid");
                handle_respawn(entity_id, respawner_id, tx, space_mgr).await;
            }
            true
        }

        USE_ABILITY => {
            if args.len() >= 8 {
                let ability_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let target_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::debug!(entity_id, ability_id, target_id, "useAbility");
                crate::cell::abilities::handle_use_ability(entity_id, ability_id, target_id, tx, space_mgr).await;

                // Check if target NPC died -- fire entity_death for content chains
                // and set AI state to Dead so the NPC stops acting.
                if target_id > 0 {
                    let target_eid = target_id as u32;
                    let death_info = space_mgr.get_entity(target_eid).and_then(|target| {
                        let is_dead = target.stats.get(cimmeria_entity::stats::HEALTH)
                            .map_or(false, |s| s.cur <= 0);
                        if is_dead && !target.is_player {
                            Some((target.tag.clone(), target.is_player))
                        } else {
                            None
                        }
                    });

                    if let Some((tag, _is_player)) = death_info {
                        // Set AI state to Dead and clear threat
                        if let Some(target) = space_mgr.get_entity_mut(target_eid) {
                            target.ai_state = cimmeria_entity::cell_entity::AiState::Dead;
                            target.threat_list.clear();
                        }

                        // Fire entity_death for content engine (mission kill chains)
                        if let Some(tag) = tag {
                            let player_id = space_mgr.get_entity(entity_id)
                                .and_then(|e| e.player_id).unwrap_or(0);
                            crate::cell::content::fire_entity_death(
                                entity_id, player_id, &tag, engine, tx, space_mgr,
                            ).await;
                        }
                    }
                }
            }
            true
        }

        USE_ABILITY_ON_GROUND => {
            if args.len() >= 16 {
                let ability_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let x = f32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                let y = f32::from_le_bytes([args[8], args[9], args[10], args[11]]);
                let z = f32::from_le_bytes([args[12], args[13], args[14], args[15]]);
                tracing::debug!(entity_id, ability_id, x, y, z, "useAbilityOnGroundTarget");
                // TODO: Ground-target AoE ability handling
            }
            true
        }

        RESPAWN => {
            // Auto-respawn (timer expired): use -1 to mean "first respawner for my world"
            tracing::debug!(entity_id, "respawn (auto)");
            handle_respawn(entity_id, -1, tx, space_mgr).await;
            true
        }

        UNSTUCK => {
            tracing::info!(entity_id, "UNIMPLEMENTED: unstuck");
            true
        }

        RESET_MY_ABILITIES => {
            tracing::info!(entity_id, "UNIMPLEMENTED: resetMyAbilities");
            true
        }

        WHO => {
            // who(WSTRING name, WSTRING archetype, WSTRING alignment, WSTRING playerType)
            tracing::info!(entity_id, "UNIMPLEMENTED: who");
            true
        }

        INTERACT => {
            if args.len() >= 4 {
                let target_entity_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, target_entity_id, "interact");

                // ── Step 0: If target is a hostile NPC, set as combat target ──
                // Right-clicking a hostile NPC should start auto-attack, not open a dialog.
                // Faction 10 = hostile (NID guards, enemy mobs).
                {
                    let is_hostile = space_mgr.get_entity(target_entity_id as u32)
                        .map_or(false, |t| !t.is_player && t.faction == 10);
                    if is_hostile {
                        tracing::info!(entity_id, target_entity_id, "interact: targeting hostile NPC for combat");
                        // Send onTargetUpdate so client shows the target
                        let mut reply = Vec::with_capacity(4);
                        reply.extend_from_slice(&target_entity_id.to_le_bytes());
                        let _ = tx.send(CellToBaseMsg::EntityMethodCall {
                            entity_id,
                            method_index: 16, // onTargetUpdate
                            args: reply,
                        }).await;
                        // Also fire the player's default attack ability (Pistol Shot = 592)
                        // since the client auto-attack system may not trigger automatically.
                        crate::cell::abilities::handle_use_ability(
                            entity_id, 592, target_entity_id, tx, space_mgr,
                        ).await;
                        return true;
                    }
                }

                // ── Step 1: Fire interact_tag event ──
                // Python: self.first('entity.interact.tag::' + target.tag, ...)
                let mut handled = false;
                if let Some(target) = space_mgr.get_entity(target_entity_id as u32) {
                    let tag = target.tag.clone();
                    let template_name = target.npc_name.clone();
                    let player_id = space_mgr.get_entity(entity_id)
                        .and_then(|e| e.player_id).unwrap_or(0);

                    if let Some(ref tag) = tag {
                        handled = crate::cell::content::fire_interact_tag(
                            entity_id, player_id, tag, target_entity_id as u32,
                            engine, tx, space_mgr,
                        ).await;
                    }

                    // ── Step 2: Fire interact_template event ──
                    // Python: self.first('entity.interact.template::' + target.template.templateName, ...)
                    if !handled {
                        if let Some(ref name) = template_name {
                            handled = crate::cell::content::fire_interact_template(
                                entity_id, player_id, name, target_entity_id as u32,
                                engine, tx, space_mgr,
                            ).await;
                        }
                    }
                }

                // ── Step 3: Fall through to available_interactions / static interaction ──
                // Python: target.onInteract(self)
                if !handled {
                    let dialog_id = crate::cell::interactions::handle_interact(
                        entity_id, target_entity_id as u32, tx, space_mgr,
                    ).await;

                    if let Some(did) = dialog_id {
                        let player_id = space_mgr.get_entity(entity_id)
                            .and_then(|e| e.player_id)
                            .unwrap_or(0);
                        crate::cell::content::fire_dialog_open(
                            entity_id, player_id, did, engine, tx, space_mgr,
                        ).await;
                    } else {
                        // No interaction/dialog — if target is an NPC, set it as
                        // the player's target so the client enables auto-attack.
                        let is_hostile_npc = space_mgr.get_entity(target_entity_id as u32)
                            .map_or(false, |t| !t.is_player);
                        if is_hostile_npc {
                            tracing::debug!(entity_id, target_entity_id, "interact: targeting hostile NPC for combat");
                            let mut reply = Vec::with_capacity(4);
                            reply.extend_from_slice(&target_entity_id.to_le_bytes());
                            let _ = tx.send(CellToBaseMsg::EntityMethodCall {
                                entity_id,
                                method_index: 16, // onTargetUpdate
                                args: reply,
                            }).await;
                        }
                    }
                }
            }
            true
        }

        DIALOG_BUTTON_CHOICE => {
            if args.len() >= 8 {
                let dialog_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let button_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::info!(entity_id, dialog_id, button_id, "dialogButtonChoice");

                let player_id = space_mgr.get_entity(entity_id)
                    .and_then(|e| e.player_id).unwrap_or(0);
                // Fire with dialogId — matches Python: self.fire('dialog.choice::' + str(dialogId))
                crate::cell::content::fire_dialog_choice(
                    entity_id, player_id, dialog_id, engine, tx, space_mgr,
                ).await;
            }
            true
        }

        INITIAL_RESPONSE => {
            if args.len() >= 4 {
                let interaction_set_map_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, interaction_set_map_id, "initialResponse");

                crate::cell::interactions::handle_initial_response(
                    entity_id, interaction_set_map_id, engine, tx, space_mgr,
                ).await;
            }
            true
        }

        TRAIN_ABILITY => {
            if args.len() >= 4 {
                let ability_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, ability_id, "UNIMPLEMENTED: trainAbility");
            }
            true
        }

        PURCHASE_ITEMS => {
            // ARRAY<INT32> itemIndices, ARRAY<INT32> quantities
            tracing::info!(entity_id, "UNIMPLEMENTED: purchaseItems");
            true
        }

        SELL_ITEMS => {
            // ARRAY<INT32> itemIds, ARRAY<INT32> quantities
            tracing::info!(entity_id, "UNIMPLEMENTED: sellItems");
            true
        }

        BUYBACK_ITEMS => {
            // ARRAY<INT32> itemIds, ARRAY<INT32> quantities
            tracing::info!(entity_id, "UNIMPLEMENTED: buybackItems");
            true
        }

        REPAIR_ITEMS => {
            // ARRAY<INT32> itemIds
            tracing::info!(entity_id, "UNIMPLEMENTED: repairItems");
            true
        }

        RECHARGE_ITEMS => {
            // ARRAY<INT32> itemIds
            tracing::info!(entity_id, "UNIMPLEMENTED: rechargeItems");
            true
        }

        SET_AUTO_CYCLE => {
            if !args.is_empty() {
                let enabled = args[0] != 0;
                tracing::debug!(entity_id, enabled, "setAutoCycle");
                if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                    entity.abilities.auto_cycle = enabled;
                    if !enabled {
                        entity.abilities.auto_cycle_ability_id = None;
                    }
                }
            }
            true
        }

        LOOT_ITEM => {
            if args.len() >= 4 {
                let index = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                crate::cell::interactions::handle_loot_item(
                    entity_id, index, tx, space_mgr,
                ).await;
            }
            true
        }

        TRIGGER_REGION => {
            // triggerClientHintedGenericRegion(INT32 id, UINT8 bEntering, VECTOR3 position)
            if args.len() >= 17 {
                let region_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let b_entering = args[4] != 0;
                let _x = f32::from_le_bytes([args[5], args[6], args[7], args[8]]);
                let _y = f32::from_le_bytes([args[9], args[10], args[11], args[12]]);
                let _z = f32::from_le_bytes([args[13], args[14], args[15], args[16]]);

                // Look up the region tag from the auto-incrementing runtime ID.
                // The runtime ID is NOT the DB set_id or the number from the tag.
                let region_tag = space_mgr.get_region(region_id as u32)
                    .map(|r| r.tag.clone());

                if let Some(tag) = region_tag {
                    tracing::info!(entity_id, region_id, %tag, b_entering, "triggerClientHintedGenericRegion");

                    let player_id = space_mgr.get_entity(entity_id)
                        .and_then(|e| e.player_id).unwrap_or(0);

                    if b_entering {
                        crate::cell::content::fire_enter_region(
                            entity_id, player_id, &tag, engine, tx, space_mgr,
                        ).await;
                    } else {
                        crate::cell::content::fire_exit_region(
                            entity_id, player_id, &tag, engine, tx, space_mgr,
                        ).await;
                    }
                } else {
                    tracing::warn!(entity_id, region_id, "Unknown region ID in triggerClientHintedGenericRegion");
                }
            }
            true
        }

        REQUEST_RELOAD => {
            if !args.is_empty() {
                let reload_type = args[0];
                tracing::debug!(entity_id, reload_type, "requestReload");
                handle_reload(entity_id, tx, space_mgr).await;
            }
            true
        }

        CHOSEN_REWARDS => {
            // RewardChoices choices, INT32 missionId -- complex struct
            tracing::info!(entity_id, "UNIMPLEMENTED: chosenRewards");
            true
        }

        PET_INVOKE_ABILITY => {
            if args.len() >= 12 {
                let pet_entity_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let ability_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                let target_id = i32::from_le_bytes([args[8], args[9], args[10], args[11]]);
                tracing::info!(entity_id, pet_entity_id, ability_id, target_id, "UNIMPLEMENTED: petInvokeAbility");
            }
            true
        }

        PET_ABILITY_TOGGLE => {
            if args.len() >= 9 {
                let pet_entity_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let ability_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                let toggle = args[8] as i8;
                tracing::info!(entity_id, pet_entity_id, ability_id, toggle, "UNIMPLEMENTED: petAbilityToggle");
            }
            true
        }

        PET_CHANGE_STANCE => {
            if args.len() >= 5 {
                let pet_entity_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let stance = args[4] as i8;
                tracing::info!(entity_id, pet_entity_id, stance, "UNIMPLEMENTED: petChangeStance");
            }
            true
        }

        SET_RING_TRANSPORTER_DEST => {
            if args.len() >= 8 {
                let region_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let destination_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::info!(entity_id, region_id, destination_id, "UNIMPLEMENTED: setRingTransporterDestination");
            }
            true
        }

        WORLD_INSTANCE_RESET => {
            tracing::info!(entity_id, "UNIMPLEMENTED: onWorldInstanceReset");
            true
        }

        UPDATE_SYSTEM_OPTIONS => {
            // ARRAY<NameValuePair> options -- variable length
            tracing::info!(entity_id, "UNIMPLEMENTED: updateSystemOptions");
            true
        }

        ORG_CREATION => {
            // WSTRING organizationName -- variable length
            tracing::info!(entity_id, "UNIMPLEMENTED: onOrganizationCreation");
            true
        }

        SPEND_APPLIED_SCIENCE_POINTS => {
            if args.len() >= 4 {
                let discipline_seq_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, discipline_seq_id, "UNIMPLEMENTED: spendAppliedSciencePoints");
            }
            true
        }

        CRAFT => {
            // INT32 craftId, ARRAY<ItemID> items, INT32 quantity
            if args.len() >= 4 {
                let craft_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, craft_id, "UNIMPLEMENTED: craft");
            }
            true
        }

        RESEARCH => {
            // ItemID itemId, ARRAY<ItemID> kickers
            tracing::info!(entity_id, "UNIMPLEMENTED: research");
            true
        }

        REVERSE_ENGINEER => {
            // ItemID itemId
            tracing::info!(entity_id, "UNIMPLEMENTED: reverseEngineer");
            true
        }

        ALLOYING => {
            // INT32 craftId, ItemID currentTierItemId, ARRAY<ItemID> lowerTierItems
            if args.len() >= 4 {
                let craft_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, craft_id, "UNIMPLEMENTED: alloying");
            }
            true
        }

        RESPEC_CRAFTING => {
            tracing::info!(entity_id, "UNIMPLEMENTED: respecCrafting");
            true
        }

        CLIENT_CHALLENGE_RESPONSE => {
            // INT32, WSTRING, INT32, WSTRING, INT32, INT32, WSTRING -- anti-cheat
            if args.len() >= 4 {
                let challenge = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, challenge, "UNIMPLEMENTED: onClientChallengeResponse");
            }
            true
        }

        SEND_DUEL_RESPONSE => {
            if !args.is_empty() {
                let response = args[0] as i8;
                tracing::info!(entity_id, response, "UNIMPLEMENTED: sendDuelResponse");
            }
            true
        }

        DUEL_FORFEIT => {
            tracing::info!(entity_id, "UNIMPLEMENTED: duelForfeit");
            true
        }

        TRADE_REQUEST => {
            // INT32 entityId, LocalTradeProposal proposal
            if args.len() >= 4 {
                let target_entity_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, target_entity_id, "UNIMPLEMENTED: tradeRequest");
            }
            true
        }

        TRADE_REQUEST_CANCEL => {
            if args.len() >= 4 {
                let target_entity_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, target_entity_id, "UNIMPLEMENTED: tradeRequestCancel");
            }
            true
        }

        TRADE_UPDATE_PROPOSAL => {
            // INT32 entityId, LocalTradeProposal proposal
            if args.len() >= 4 {
                let target_entity_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, target_entity_id, "UNIMPLEMENTED: tradeUpdateProposal");
            }
            true
        }

        TRADE_LOCK_STATE => {
            if args.len() >= 9 {
                let local_version_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let remote_version_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                let lock_state = args[8] as i8;
                tracing::info!(entity_id, local_version_id, remote_version_id, lock_state, "UNIMPLEMENTED: tradeLockState");
            }
            true
        }

        CANCEL_MOVIE => {
            // WSTRING movieName -- variable length
            tracing::info!(entity_id, "UNIMPLEMENTED: cancelMovie");
            true
        }

        _ => false,
    }
}

// ── Respawn ──────────────────────────────────────────────────────────────────

/// Handle player respawn: close defeat UI, restore health/focus, clear all
/// state flags, teleport to spawn point, send updates.
///
/// Reference: `python/cell/SGWBeing.py:revive()` — restores pools to max,
/// clears combatantState dead flag, sends stat and state field updates.
/// Reference: `python/cell/SGWPlayer.py:1217` — self.client.onEndAidWait()
async fn handle_respawn(
    entity_id: u32,
    respawner_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let entity = match space_mgr.get_entity_mut(entity_id) {
        Some(e) => e,
        None => {
            tracing::warn!(entity_id, "respawn: entity not found");
            return;
        }
    };

    // Restore health and focus to max
    if let Some(health) = entity.stats.get_mut(HEALTH) {
        health.set_current(health.max);
    }
    if let Some(focus) = entity.stats.get_mut(FOCUS) {
        focus.set_current(focus.max);
    }

    // Serialize stat update (health + focus now dirty)
    let stat_update = entity.stats.serialize_dirty();
    entity.stats.clear_dirty();

    // Clear ALL state flags — not just BSF_Dead, but also BSF_InCombat,
    // BSF_MovementLock, etc. The client uses the state field to drive ragdoll
    // and other visual states; leaving BSF_InCombat set can prevent the death
    // animation from ending properly on respawn.
    entity.state_field = 0;

    // Clear all ability cooldowns
    entity.abilities.clear_all_cooldowns();

    tracing::info!(entity_id, "Player respawned, state_field=0");

    // Send onEndAidWait to close the Defeat Window.
    // Both callForAid and respawn paths need this.
    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index: crate::mercury::method_idx::ON_END_AID_WAIT,
        args: Vec::new(),
    }).await;

    // Send onStatUpdate (index 20) — restored health/focus
    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index: 20,
        args: stat_update,
    }).await;

    // Send Entity_Spawn (5000) kismet sequence to end ragdoll / play stand-up animation.
    // The death flow sends Entity_Death (5001) which calls InitRagdoll in kismet.
    // Entity_Spawn (5000) calls TermRagdoll — without this, the player stays ragdolled.
    {
        const EVENT_ENTITY_SPAWN: i32 = 5000;
        if let Some(&spawn_seq_id) = space_mgr.sequence_map.get(&(1025, EVENT_ENTITY_SPAWN)) {
            let mut seq_args = Vec::with_capacity(28);
            seq_args.extend_from_slice(&spawn_seq_id.to_le_bytes());       // KismetEventSetSeqID
            seq_args.extend_from_slice(&(entity_id as i32).to_le_bytes()); // SourceID
            seq_args.extend_from_slice(&(entity_id as i32).to_le_bytes()); // TargetID
            seq_args.push(1);                                               // PrimaryTarget
            seq_args.extend_from_slice(&0.0f32.to_le_bytes());             // ImpactTime
            seq_args.extend_from_slice(&0u32.to_le_bytes());               // NameValuePairs count
            seq_args.push(0);                                               // ViewType
            seq_args.extend_from_slice(&0i32.to_le_bytes());               // InstanceId
            let _ = tx.send(CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index: 1, // onSequence
                args: seq_args,
            }).await;
            tracing::debug!(entity_id, spawn_seq_id, "Sent Entity_Spawn sequence to end ragdoll");
        }
    }

    // Send onStateFieldUpdate (index 19) — fully cleared state
    let mut state_args = Vec::with_capacity(4);
    state_args.extend_from_slice(&0u32.to_le_bytes());
    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index: 19,
        args: state_args,
    }).await;

    // Teleport player to spawn point.
    let spawn_pos: [f32; 3] = resolve_respawn_position(respawner_id, entity_id, space_mgr);
    space_mgr.update_entity_position(entity_id, spawn_pos, [0, 0, 0], [0.0; 3]);

    // Use onClientMapLoad (method 117) to force a full map reload at the spawn
    // position. This resets the client's pawn state (including ragdoll) via a
    // loading screen transition. onPlayerTeleport (116) doesn't work while in
    // ragdoll because the pawn's physics mode overrides the position.
    // Wire: WSTRING areaName, WSTRING mapPath, INT32 WorldID, VECTOR3 Location, VECTOR3 Direction
    let world_name = space_mgr.get_entity_world_name(entity_id)
        .unwrap_or_else(|| "Castle_CellBlock".to_string());
    let mut ml_args = Vec::with_capacity(128);
    crate::mercury::write_wstring(&mut ml_args, &world_name);    // areaName
    crate::mercury::write_wstring(&mut ml_args, &world_name);    // mapPath
    ml_args.extend_from_slice(&0i32.to_le_bytes());              // WorldID (0 = current)
    ml_args.extend_from_slice(&spawn_pos[0].to_le_bytes());      // Location X
    ml_args.extend_from_slice(&spawn_pos[1].to_le_bytes());      // Location Y
    ml_args.extend_from_slice(&spawn_pos[2].to_le_bytes());      // Location Z
    ml_args.extend_from_slice(&0.0f32.to_le_bytes());            // Direction X
    ml_args.extend_from_slice(&0.0f32.to_le_bytes());            // Direction Y
    ml_args.extend_from_slice(&0.0f32.to_le_bytes());            // Direction Z
    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index: 117, // onClientMapLoad
        args: ml_args,
    }).await;
    tracing::info!(entity_id, ?spawn_pos, %world_name, "Sent onClientMapLoad for respawn");
}

/// Look up the respawn position for a given respawner_id.
///
/// If `respawner_id` matches a known respawner, use its position.
/// If `respawner_id` is -1 (auto-respawn), use the first respawner for
/// the player's current world. Falls back to a hardcoded default if no
/// respawners are defined for the world.
fn resolve_respawn_position(
    respawner_id: i32,
    entity_id: u32,
    space_mgr: &SpaceManager,
) -> [f32; 3] {
    const DEFAULT_POS: [f32; 3] = [-334.231, 73.472, -228.026];

    // Try exact match first (callForAid sends a specific respawner_id)
    if respawner_id > 0 {
        if let Some(resp) = space_mgr.respawners.iter().find(|r| r.respawner_id == respawner_id) {
            return resp.pos;
        }
        tracing::warn!(entity_id, respawner_id, "Respawner not found, falling back to world default");
    }

    // Auto-respawn or unknown ID: pick first respawner for the player's world
    if let Some(world_name) = space_mgr.get_entity_world_name(entity_id) {
        if let Some(resp) = space_mgr.respawners.iter().find(|r| r.world_name == world_name) {
            return resp.pos;
        }
    }

    // No respawners for this world — use hardcoded fallback
    tracing::debug!(entity_id, "No respawners for player's world, using default position");
    DEFAULT_POS
}

// ── Reload ────────────────────────────────────────────────────────────────────

/// Reload ability ID (from resources.abilities).
const ABILITY_RELOAD_WEAPON: i32 = 596;

/// Handle player weapon reload.
///
/// Launches ability 596 (Reload) which has a 2-second warmup, then
/// restores ammo to max. Sends onTimerUpdate for the cooldown/warmup,
/// onSequence for the reload animation, and onStatUpdate for ammo change.
///
/// Reference: `python/cell/SGWPlayer.py:1482-1498`
/// Reference: `python/cell/SGWBeing.py:863-874`
async fn handle_reload(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // Look up ability 596 (Reload) from DB before mutable borrow
    let reload_def = space_mgr.ability_defs.get(&ABILITY_RELOAD_WEAPON).cloned();
    let warmup = reload_def.as_ref().map_or(2.0f32, |d| d.warmup);
    let cooldown = reload_def.as_ref().map_or(1.0f32, |d| d.cooldown);
    let event_set_id = reload_def.as_ref().and_then(|d| d.event_set_id);

    let entity = match space_mgr.get_entity_mut(entity_id) {
        Some(e) => e,
        None => {
            tracing::warn!(entity_id, "requestReload: entity not found");
            return;
        }
    };

    if entity.current_ammo >= entity.max_ammo {
        tracing::debug!(entity_id, "requestReload: already at max ammo");
        return;
    }

    // Reset ammo to max (in the original, this happens after warmup;
    // we do it instantly since we don't have warmup timers yet)
    let old = entity.current_ammo;
    entity.current_ammo = entity.max_ammo;

    // Start ability cooldown
    let total_time = warmup + cooldown;
    entity.abilities.start_ability_cooldown(
        ABILITY_RELOAD_WEAPON,
        std::time::Duration::from_secs_f32(total_time),
    );

    tracing::info!(entity_id, old, new = entity.max_ammo, warmup, cooldown, "Weapon reloaded");

    // Send onTimerUpdate for reload warmup+cooldown
    let timer_args = cimmeria_entity::abilities::serialize_timer_update(
        ABILITY_RELOAD_WEAPON,
        cimmeria_entity::abilities::TIMER_ABILITY_COOLDOWN,
        entity_id as i32,
        total_time,
        0.0,
    );
    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index: 12, // onTimerUpdate
        args: timer_args,
    }).await;

    // Ensure combat state is set for reload animations
    {
        const BSF_IN_COMBAT: u32 = 1 << 3;
        const BSF_HOLSTER: u32 = 1 << 8;
        if let Some(e) = space_mgr.get_entity_mut(entity_id) {
            let old = e.state_field;
            e.state_field |= BSF_IN_COMBAT;
            e.state_field &= !BSF_HOLSTER;
            if e.state_field != old {
                let new_state = e.state_field;
                let _ = tx.send(CellToBaseMsg::EntityMethodCall {
                    entity_id,
                    method_index: 19, // onStateFieldUpdate
                    args: new_state.to_le_bytes().to_vec(),
                }).await;
            }
        }
    }

    // Send reload animation (onSequence) if event_set_id available.
    // Look up the correct sequence_id — the client expects sequence_id, not event_set_id.
    if let Some(esid) = event_set_id {
        use crate::cell::spawner::EVENT_ABILITY_END;

        if let Some(&seq_id) = space_mgr.sequence_map.get(&(esid, EVENT_ABILITY_END)) {
            let mut seq_args = Vec::with_capacity(28);
            seq_args.extend_from_slice(&seq_id.to_le_bytes());             // KismetEventSetSeqID (sequence_id)
            seq_args.extend_from_slice(&(entity_id as i32).to_le_bytes()); // SourceID
            seq_args.extend_from_slice(&(entity_id as i32).to_le_bytes()); // TargetID (self)
            seq_args.push(1);                                               // PrimaryTarget
            seq_args.extend_from_slice(&0.0f32.to_le_bytes());            // ImpactTime
            seq_args.extend_from_slice(&0u32.to_le_bytes());               // NameValuePairs count
            seq_args.push(0);                                               // ViewType
            seq_args.extend_from_slice(&0i32.to_le_bytes());               // InstanceId
            let _ = tx.send(CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index: 1, // onSequence
                args: seq_args,
            }).await;
        } else {
            tracing::debug!(entity_id, event_set_id = esid, "reload: no Ability_End sequence found");
        }
    }

    // Send onEntityProperty to update ammo display
    let mut args = Vec::with_capacity(8);
    args.extend_from_slice(&7i32.to_le_bytes()); // GENERICPROPERTY_AmmoTypeId = 7
    let ammo_type = space_mgr.get_entity(entity_id)
        .map_or(0, |e| e.ammo_type);
    args.extend_from_slice(&ammo_type.to_le_bytes());
    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index: 7, // onEntityProperty
        args,
    }).await;
}
