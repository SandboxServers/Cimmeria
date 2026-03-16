//! Client→server cell method dispatch for the SGWPlayer entity type.
//!
//! When the client calls an exposed CellMethod, the BaseApp forwards it as a
//! [`CellMethodCall`](super::messages::BaseToCellMsg::CellMethodCall) message.
//! This module maps the flattened method index to a named handler.
//!
//! ## Flattened EXPOSED CellMethod index ordering
//!
//! The client encodes cell method calls as `msg_id = index | 0x80` (direct, 0-60)
//! or `msg_id = 0xBD` with sub-index (extended, >= 61). The index is the
//! flattened position across the entity type hierarchy, counting only `<Exposed/>`
//! methods in CellMethods sections.
//!
//! ### SGWBeing interface CellMethods (exposed only)
//! | Index | Method | Args |
//! |-------|--------|------|
//! | 0 | setTargetID | INT32 targetId |
//! | 1 | setMovementType | UINT8 movementType |
//!
//! ### SGWAbilityManager interface CellMethods (exposed only)
//! | 2 | toggleCombatDebug | (none) |
//! | 3 | toggleCombatVerboseDebug | (none) |
//! | 4 | confirmationResponse | INT32 effectId, UINT8 accepted |
//!
//! ### SGWCombatant interface CellMethods (exposed only)
//! | 5 | setCrouched | INT8 enabled |
//! | 6 | toggleHealDebug | (none) |
//! | 7 | requestHolsterWeapon | INT8 holster |
//!
//! ### SGWPlayer interface + own CellMethods (exposed only)
//! Indices 8+ follow the interface traversal order then SGWPlayer's own methods.
//! (Full mapping TBD as handlers are implemented.)

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::stats::{HEALTH, FOCUS};

use super::combat;
use super::messages::CellToBaseMsg;
use super::space_manager::SpaceManager;

// ── Flattened EXPOSED CellMethod indices ────────────────────────────────────

/// SGWBeing: setTargetID(INT32)
pub const CM_SET_TARGET_ID: u16 = 0;
/// SGWBeing: setMovementType(UINT8)
pub const CM_SET_MOVEMENT_TYPE: u16 = 1;
/// SGWAbilityManager: toggleCombatDebug()
pub const CM_TOGGLE_COMBAT_DEBUG: u16 = 2;
/// SGWAbilityManager: toggleCombatVerboseDebug()
pub const CM_TOGGLE_COMBAT_VERBOSE_DEBUG: u16 = 3;
/// SGWAbilityManager: confirmationResponse(INT32, UINT8)
pub const CM_CONFIRMATION_RESPONSE: u16 = 4;
/// SGWCombatant: setCrouched(INT8)
pub const CM_SET_CROUCHED: u16 = 5;
/// SGWCombatant: toggleHealDebug()
pub const CM_TOGGLE_HEAL_DEBUG: u16 = 6;
/// SGWCombatant: requestHolsterWeapon(INT8)
pub const CM_REQUEST_HOLSTER_WEAPON: u16 = 7;

// OrganizationMember: indices 8–19 (12 exposed)
/// First OrganizationMember exposed CellMethod index.
pub const CM_ORG_FIRST: u16 = 8;
/// Last OrganizationMember exposed CellMethod index.
pub const CM_ORG_LAST: u16 = 19;
// MinigamePlayer: indices 20–34 (15 exposed)
/// First MinigamePlayer exposed CellMethod index.
pub const CM_MINIGAME_FIRST: u16 = 20;
/// Last MinigamePlayer exposed CellMethod index.
pub const CM_MINIGAME_LAST: u16 = 34;
// GateTravel: index 35 (1 exposed: onDialGate)
/// GateTravel: onDialGate(INT32 TargetAddressId, INT32 SourceAddressId)
pub const CM_ON_DIAL_GATE: u16 = 35;
// SGWInventoryManager: indices 36–42 (7 exposed)
/// SGWInventoryManager: removeItem(ItemID, INT16 quantity)
pub const CM_REMOVE_ITEM: u16 = 36;
/// SGWInventoryManager: listItems()
pub const CM_LIST_ITEMS: u16 = 37;
/// SGWInventoryManager: moveItem(INT32 itemId, INT32 targetBag, INT32 targetSlot, INT32 quantity)
pub const CM_MOVE_ITEM: u16 = 38;
/// SGWInventoryManager: useItem(INT32 itemId, INT32 targetId)
pub const CM_USE_ITEM: u16 = 39;
/// SGWInventoryManager: repairItemRequest(INT32 itemId, FLOAT repairRatio)
pub const CM_REPAIR_ITEM: u16 = 40;
/// SGWInventoryManager: requestActiveSlotChange(INT32 bagId, INT32 slotId)
pub const CM_REQUEST_ACTIVE_SLOT_CHANGE: u16 = 41;
/// SGWInventoryManager: requestAmmoChange(INT32 itemId, INT32 ammoType)
pub const CM_REQUEST_AMMO_CHANGE: u16 = 42;
// SGWMailManager: indices 43–51 (9 exposed)
/// SGWMailManager: requestMailHeaders(UINT8)
pub const CM_REQUEST_MAIL_HEADERS: u16 = 43;
/// SGWMailManager: sendMailMessage(INT32, ARRAY<WSTRING>, WSTRING, WSTRING, INT32, UINT8, INT32, INT32)
pub const CM_SEND_MAIL_MESSAGE: u16 = 44;
/// SGWMailManager: archiveMailMessage(INT32)
pub const CM_ARCHIVE_MAIL_MESSAGE: u16 = 45;
/// SGWMailManager: deleteMailMessage(INT32)
pub const CM_DELETE_MAIL_MESSAGE: u16 = 46;
/// SGWMailManager: returnMailMessage(INT32)
pub const CM_RETURN_MAIL_MESSAGE: u16 = 47;
/// SGWMailManager: requestMailBody(INT32)
pub const CM_REQUEST_MAIL_BODY: u16 = 48;
/// SGWMailManager: takeCashFromMailMessage(INT32)
pub const CM_TAKE_CASH_FROM_MAIL: u16 = 49;
/// SGWMailManager: takeItemFromMailMessage(INT32, INT32, INT32)
pub const CM_TAKE_ITEM_FROM_MAIL: u16 = 50;
/// SGWMailManager: payCODForMailMessage(INT32)
pub const CM_PAY_COD_FOR_MAIL: u16 = 51;
// Missionary: indices 52–54 (3 exposed)
// SGWPoller: 0 exposed
// ContactListManager: indices 55–60 (6 exposed)
/// ContactListManager: contactListCreate(WSTRING, UINT32)
pub const CM_CONTACT_LIST_CREATE: u16 = 55;
/// ContactListManager: contactListDelete(INT32)
pub const CM_CONTACT_LIST_DELETE: u16 = 56;
/// ContactListManager: contactListRename(INT32, WSTRING)
pub const CM_CONTACT_LIST_RENAME: u16 = 57;
/// ContactListManager: contactListFlagsUpdate(INT32, UINT32)
pub const CM_CONTACT_LIST_FLAGS_UPDATE: u16 = 58;
/// ContactListManager: contactListAddMembers(INT32, ARRAY<WSTRING>)
pub const CM_CONTACT_LIST_ADD_MEMBERS: u16 = 59;
/// ContactListManager: contactListRemoveMembers(INT32, ARRAY<WSTRING>)
pub const CM_CONTACT_LIST_REMOVE_MEMBERS: u16 = 60;
// SGWBlackMarketManager: indices 61–66 (6 exposed)
/// First SGWBlackMarketManager exposed CellMethod index.
pub const CM_BLACK_MARKET_FIRST: u16 = 61;
/// Last SGWBlackMarketManager exposed CellMethod index.
pub const CM_BLACK_MARKET_LAST: u16 = 66;
// ClientCache: 0 exposed

// Missionary: indices 52–54 (3 exposed)
/// Missionary: abandonMission(INT32)
pub const CM_ABANDON_MISSION: u16 = 52;
/// Missionary: shareMission(INT32)
pub const CM_SHARE_MISSION: u16 = 53;
/// Missionary: shareMissionResponse(INT8)
pub const CM_SHARE_MISSION_RESPONSE: u16 = 54;

// SGWPlayer own CellMethods (exposed only, starting at 67)
/// SGWPlayer: callForAid()
pub const CM_CALL_FOR_AID: u16 = 67;
/// SGWPlayer: useAbility(INT32 AbilityID, INT32 TargetID)
pub const CM_USE_ABILITY: u16 = 68;
/// SGWPlayer: useAbilityOnGroundTarget(INT32 AbilityID, FLOAT x, FLOAT y, FLOAT z)
pub const CM_USE_ABILITY_ON_GROUND: u16 = 69;
/// SGWPlayer: respawn()
pub const CM_RESPAWN: u16 = 70;
/// SGWPlayer: interact(INT32 entityId)
pub const CM_INTERACT: u16 = 74;
/// SGWPlayer: dialogButtonChoice(INT32 dialogId, INT32 buttonId)
pub const CM_DIALOG_BUTTON_CHOICE: u16 = 75;
/// SGWPlayer: initialResponse(INT32 dialogSetMapId)
pub const CM_INITIAL_RESPONSE: u16 = 76;
/// SGWPlayer: trainAbility(INT32 AbilityID)
pub const CM_TRAIN_ABILITY: u16 = 77;
/// SGWPlayer: purchaseItems(ARRAY<{INT32, INT32}>)
pub const CM_PURCHASE_ITEMS: u16 = 78;
/// SGWPlayer: sellItems(ARRAY<{INT32, INT32}>)
pub const CM_SELL_ITEMS: u16 = 79;
/// SGWPlayer: buybackItems(ARRAY<{INT32, INT32}>)
pub const CM_BUYBACK_ITEMS: u16 = 80;
/// SGWPlayer: repairItems(ARRAY<INT32>)
pub const CM_REPAIR_ITEMS: u16 = 81;
/// SGWPlayer: rechargeItems(ARRAY<INT32>)
pub const CM_RECHARGE_ITEMS: u16 = 82;
// lootItem index TBD — needs exact position counting from .def
/// SGWPlayer: setAutoCycle(INT8 enabled)
pub const CM_SET_AUTO_CYCLE: u16 = 83;

// ── Dispatch ────────────────────────────────────────────────────────────────

/// Dispatch a client→server cell method call to the appropriate handler.
pub async fn dispatch_cell_method(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
    ability_registry: &std::sync::Arc<cimmeria_entity::abilities::AbilityRegistry>,
    loot_cache: &std::sync::Arc<super::loot::LootCache>,
    vendor_cache: &super::vendor::VendorCache,
    effect_mgr: &mut super::effects::EffectManager,
) {
    match method_index {
        CM_SET_TARGET_ID => {
            if args.len() >= 4 {
                let target_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::debug!(entity_id, target_id, "setTargetID");
                if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                    entity.target_id = if target_id > 0 { Some(target_id) } else { None };
                }
                // Notify witnesses via onTargetUpdate (method_index 11)
                notify_witnesses(entity_id, 11, args.to_vec(), tx, space_mgr).await;
            }
        }

        CM_SET_MOVEMENT_TYPE => {
            if !args.is_empty() {
                let movement_type = args[0];
                tracing::debug!(entity_id, movement_type, "setMovementType");
                if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                    entity.movement_type = movement_type;
                }
                // Notify witnesses via onMovementTypeUpdate (method_index 15)
                notify_witnesses(entity_id, 15, args.to_vec(), tx, space_mgr).await;
            }
        }

        CM_SET_CROUCHED => {
            if !args.is_empty() {
                let enabled = args[0] != 0;
                tracing::debug!(entity_id, enabled, "setCrouched");
                if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                    entity.is_crouched = enabled;
                }
                // Notify witnesses via onCrouchToggled (method_index 16)
                notify_witnesses(entity_id, 16, args.to_vec(), tx, space_mgr).await;
            }
        }

        CM_REQUEST_HOLSTER_WEAPON => {
            if !args.is_empty() {
                let holster = args[0] != 0;
                tracing::debug!(entity_id, holster, "requestHolsterWeapon");
                if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                    entity.is_holstered = holster;
                }
                // Notify witnesses via onHolsterWeapon (method_index 18)
                notify_witnesses(entity_id, 18, args.to_vec(), tx, space_mgr).await;
            }
        }

        CM_CONFIRMATION_RESPONSE => {
            if args.len() >= 5 {
                let effect_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let accepted = args[4] != 0;
                tracing::debug!(entity_id, effect_id, accepted, "confirmationResponse");
                // Effect confirmations are acknowledged — no further action needed
                // until the effect confirmation system is fully implemented.
            }
        }

        CM_TOGGLE_COMBAT_DEBUG | CM_TOGGLE_COMBAT_VERBOSE_DEBUG | CM_TOGGLE_HEAL_DEBUG => {
            tracing::debug!(entity_id, method_index, "Debug toggle (no-op)");
        }

        CM_USE_ABILITY => {
            if args.len() >= 8 {
                let ability_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let target_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::debug!(entity_id, ability_id, target_id, "useAbility");
                super::abilities::handle_use_ability(entity_id, ability_id, target_id, tx, space_mgr, ability_registry, loot_cache, effect_mgr).await;
            }
        }

        CM_USE_ABILITY_ON_GROUND => {
            if args.len() >= 16 {
                let ability_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let x = f32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                let y = f32::from_le_bytes([args[8], args[9], args[10], args[11]]);
                let z = f32::from_le_bytes([args[12], args[13], args[14], args[15]]);
                tracing::debug!(entity_id, ability_id, x, y, z, "useAbilityOnGroundTarget");

                // Look up ability for range/AoE radius
                let aoe_radius = ability_registry.get_ability(ability_id)
                    .map_or(5.0f32, |a| a.max_range as f32);

                // Find all hostile entities within AoE radius of the target point
                let target_pos = cimmeria_common::Vector3::new(x, y, z);
                let nearby = space_mgr.get_entities_near_point(entity_id, &target_pos, aoe_radius);

                tracing::debug!(entity_id, ability_id, targets = nearby.len(), aoe_radius, "AoE targets found");

                // Apply ability to each target
                for target_id in nearby {
                    super::abilities::handle_use_ability(
                        entity_id, ability_id, target_id as i32,
                        tx, space_mgr, ability_registry, loot_cache, effect_mgr,
                    ).await;
                }
            }
        }

        CM_SET_AUTO_CYCLE => {
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
        }

        CM_DIALOG_BUTTON_CHOICE => {
            if args.len() >= 8 {
                let dialog_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let button_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::debug!(entity_id, dialog_id, button_id, "dialogButtonChoice");

                // Fire content engine event for this dialog button press
                let player_id = space_mgr.get_entity(entity_id)
                    .and_then(|e| e.player_id)
                    .unwrap_or(0);
                super::content::fire_dialog_open(
                    entity_id, player_id, dialog_id, engine, tx, space_mgr,
                ).await;
            }
        }

        CM_INITIAL_RESPONSE => {
            if args.len() >= 4 {
                let dialog_set_map_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::debug!(entity_id, dialog_set_map_id, "initialResponse");

                // Fire content engine event
                let player_id = space_mgr.get_entity(entity_id)
                    .and_then(|e| e.player_id)
                    .unwrap_or(0);
                super::content::fire_dialog_open(
                    entity_id, player_id, dialog_set_map_id, engine, tx, space_mgr,
                ).await;
            }
        }

        CM_TRAIN_ABILITY => {
            if args.len() >= 4 {
                let ability_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::debug!(entity_id, ability_id, "trainAbility");

                // Look up ability for training cost
                let cost = ability_registry.get_ability(ability_id)
                    .map_or(1i32, |a| a.training_cost as i32);

                if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                    // Check if entity already knows this ability
                    if entity.abilities.has_ability(ability_id) {
                        tracing::debug!(entity_id, ability_id, "trainAbility: already known");
                    } else {
                        // Add ability to known set
                        entity.abilities.add_ability(ability_id);
                        tracing::info!(entity_id, ability_id, cost, "Ability trained");

                        // Send onKnownAbilitiesUpdate to client
                        let known_args = entity.abilities.serialize_known_abilities();
                        let _ = tx.send(CellToBaseMsg::EntityMethodCall {
                            entity_id,
                            method_index: 13, // onKnownAbilitiesUpdate
                            args: known_args,
                        }).await;

                        // Deduct training points via BaseApp
                        // Send onEntityProperty(GENERICPROPERTY_TrainingPoints, -cost)
                        let mut tp_args = Vec::with_capacity(8);
                        tp_args.extend_from_slice(&1i32.to_le_bytes()); // GENERICPROPERTY_TrainingPoints
                        tp_args.extend_from_slice(&(-cost).to_le_bytes());
                        let _ = tx.send(CellToBaseMsg::EntityMethodCall {
                            entity_id,
                            method_index: 95, // onEntityProperty (delta mode)
                            args: tp_args,
                        }).await;
                    }
                }
            }
        }

        CM_PURCHASE_ITEMS => {
            handle_purchase_items(entity_id, args, tx, space_mgr, vendor_cache).await;
        }

        CM_SELL_ITEMS => {
            handle_sell_items(entity_id, args, tx, space_mgr, vendor_cache).await;
        }

        CM_BUYBACK_ITEMS => {
            // Buyback uses the same wire format as purchase: ARRAY of {designId, quantity}
            // For now, treat identically to purchase — the buyback list is populated
            // client-side from items previously sold in this session.
            handle_purchase_items(entity_id, args, tx, space_mgr, vendor_cache).await;
        }

        CM_REPAIR_ITEMS | CM_RECHARGE_ITEMS => {
            tracing::debug!(entity_id, method_index, "Store repair/recharge (stub)");
        }

        CM_RESPAWN => {
            tracing::debug!(entity_id, "respawn");
            handle_respawn(entity_id, tx, space_mgr).await;
        }

        CM_ABANDON_MISSION => {
            if args.len() >= 4 {
                let mission_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::debug!(entity_id, mission_id, "abandonMission");
                super::missions::abandon_mission(entity_id, mission_id, tx, space_mgr).await;
            }
        }

        CM_SHARE_MISSION | CM_SHARE_MISSION_RESPONSE => {
            tracing::debug!(entity_id, method_index, "Mission sharing (not implemented)");
        }

        CM_REQUEST_MAIL_HEADERS => {
            let b_archive = if !args.is_empty() { args[0] } else { 0 };
            tracing::debug!(entity_id, b_archive, "requestMailHeaders");
            super::mail::handle_request_mail_headers(entity_id, b_archive, tx).await;
        }

        CM_REQUEST_MAIL_BODY => {
            if args.len() >= 4 {
                let mail_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::debug!(entity_id, mail_id, "requestMailBody");
                super::mail::handle_request_mail_body(entity_id, mail_id, tx).await;
            }
        }

        CM_DELETE_MAIL_MESSAGE => {
            if args.len() >= 4 {
                let mail_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::debug!(entity_id, mail_id, "deleteMailMessage");
                super::mail::handle_delete_mail(entity_id, mail_id, tx).await;
            }
        }

        CM_ARCHIVE_MAIL_MESSAGE => {
            if args.len() >= 4 {
                let mail_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::debug!(entity_id, mail_id, "archiveMailMessage");
                super::mail::handle_archive_mail(entity_id, mail_id, tx).await;
            }
        }

        CM_SEND_MAIL_MESSAGE => {
            // sendMailMessage(INT32 flags, ARRAY<WSTRING> recipients, WSTRING subject,
            //                 WSTRING body, INT32 cash, UINT8 bCOD, INT32 itemId, INT32 itemQuantity)
            if args.len() >= 8 {
                let _recipient_flags = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                // Parse recipients array: u32 count + N × WSTRING
                let mut offset = 4;
                if offset + 4 > args.len() { return; }
                let recipient_count = u32::from_le_bytes([args[offset], args[offset+1], args[offset+2], args[offset+3]]) as usize;
                offset += 4;
                let mut recipients = Vec::with_capacity(recipient_count);
                for _ in 0..recipient_count {
                    if offset + 4 > args.len() { break; }
                    let str_len = u32::from_le_bytes([args[offset], args[offset+1], args[offset+2], args[offset+3]]) as usize;
                    offset += 4;
                    if offset + str_len * 2 > args.len() { break; }
                    let utf16: Vec<u16> = (0..str_len).map(|i| {
                        u16::from_le_bytes([args[offset + i*2], args[offset + i*2 + 1]])
                    }).collect();
                    recipients.push(String::from_utf16_lossy(&utf16));
                    offset += str_len * 2;
                }
                // Parse subject (WSTRING)
                let (subject, new_offset) = parse_wstring(args, offset);
                offset = new_offset;
                // Parse body (WSTRING)
                let (body, new_offset) = parse_wstring(args, offset);
                offset = new_offset;
                // Parse remaining fields
                let cash = if offset + 4 <= args.len() {
                    let v = i32::from_le_bytes([args[offset], args[offset+1], args[offset+2], args[offset+3]]);
                    offset += 4; v
                } else { 0 };
                let is_cod = if offset < args.len() {
                    let v = args[offset] != 0;
                    offset += 1; v
                } else { false };
                let item_id = if offset + 4 <= args.len() {
                    let v = i32::from_le_bytes([args[offset], args[offset+1], args[offset+2], args[offset+3]]);
                    offset += 4; v
                } else { 0 };
                let item_quantity = if offset + 4 <= args.len() {
                    i32::from_le_bytes([args[offset], args[offset+1], args[offset+2], args[offset+3]])
                } else { 0 };

                let sender_name = space_mgr.get_entity(entity_id)
                    .and_then(|e| e.npc_name.clone())
                    .unwrap_or_else(|| "Unknown".to_string());

                tracing::info!(entity_id, ?recipients, %subject, cash, "sendMailMessage");

                // Forward to BaseApp for DB persistence
                let _ = tx.send(CellToBaseMsg::MailRequest {
                    entity_id,
                    op: super::messages::MailOp::Send {
                        sender_name,
                        recipients,
                        subject,
                        body,
                        cash,
                        is_cod,
                        item_id,
                        item_quantity,
                    },
                }).await;
            }
        }

        CM_RETURN_MAIL_MESSAGE |
        CM_TAKE_CASH_FROM_MAIL | CM_TAKE_ITEM_FROM_MAIL | CM_PAY_COD_FOR_MAIL => {
            tracing::debug!(entity_id, method_index, "Mail operation (not yet implemented)");
        }

        CM_REMOVE_ITEM => {
            if args.len() >= 6 {
                let item_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let quantity = i16::from_le_bytes([args[4], args[5]]);
                tracing::debug!(entity_id, item_id, quantity, "removeItem");
                // Forward to BaseApp for DB removal + send onRemoveItem to client
                let player_id = space_mgr.get_entity(entity_id)
                    .and_then(|e| e.player_id)
                    .unwrap_or(0);
                let _ = tx.send(CellToBaseMsg::GrantItem {
                    entity_id,
                    player_id,
                    item_id,
                    container_id: 0, // BaseApp will resolve
                    count: -(quantity as i32), // negative = removal
                }).await;
                // Send onRemoveItem(ARRAY<INT32>) to client
                let mut rm_args = Vec::with_capacity(8);
                rm_args.extend_from_slice(&1u32.to_le_bytes()); // array count = 1
                rm_args.extend_from_slice(&item_id.to_le_bytes());
                let _ = tx.send(CellToBaseMsg::EntityMethodCall {
                    entity_id,
                    method_index: 73, // onRemoveItem
                    args: rm_args,
                }).await;
            }
        }

        CM_LIST_ITEMS => {
            tracing::debug!(entity_id, "listItems");
            // Request full inventory resend from BaseApp
            // For now, this is a no-op — inventory is sent during world entry
        }

        CM_MOVE_ITEM => {
            if args.len() >= 16 {
                let item_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let target_bag = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                let target_slot = i32::from_le_bytes([args[8], args[9], args[10], args[11]]);
                let quantity = i32::from_le_bytes([args[12], args[13], args[14], args[15]]);
                tracing::debug!(entity_id, item_id, target_bag, target_slot, quantity, "moveItem");
                // Acknowledge the move by sending onUpdateItem with new bag/slot
                // The client handles visual updates; we persist via BaseApp
                let mut update_args = Vec::with_capacity(20);
                update_args.extend_from_slice(&1u32.to_le_bytes()); // array count = 1
                update_args.extend_from_slice(&item_id.to_le_bytes());       // instanceId
                update_args.extend_from_slice(&item_id.to_le_bytes());       // typeId
                update_args.extend_from_slice(&(quantity as i16).to_le_bytes()); // stackSize (i16)
                update_args.extend_from_slice(&target_slot.to_le_bytes());   // slotId
                update_args.extend_from_slice(&target_bag.to_le_bytes());    // containerId
                let _ = tx.send(CellToBaseMsg::EntityMethodCall {
                    entity_id,
                    method_index: 72, // onUpdateItem
                    args: update_args,
                }).await;
            }
        }

        CM_USE_ITEM => {
            if args.len() >= 8 {
                let item_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let target_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::debug!(entity_id, item_id, target_id, "useItem");
                // For consumables, remove the item and apply its effect
                // For equippables, the client handles visuals; we just ack
                // Send onRefreshItem to acknowledge (method_index 74)
                let mut refresh_args = Vec::with_capacity(4);
                refresh_args.extend_from_slice(&item_id.to_le_bytes());
                let _ = tx.send(CellToBaseMsg::EntityMethodCall {
                    entity_id,
                    method_index: 74, // onRefreshItem
                    args: refresh_args,
                }).await;
            }
        }

        CM_REPAIR_ITEM => {
            if args.len() >= 8 {
                let item_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let repair_ratio = f32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::debug!(entity_id, item_id, repair_ratio, "repairItemRequest");
                // Acknowledge repair — restore durability to max * repair_ratio
                let mut refresh_args = Vec::with_capacity(4);
                refresh_args.extend_from_slice(&item_id.to_le_bytes());
                let _ = tx.send(CellToBaseMsg::EntityMethodCall {
                    entity_id,
                    method_index: 74, // onRefreshItem
                    args: refresh_args,
                }).await;
            }
        }

        CM_REQUEST_ACTIVE_SLOT_CHANGE => {
            if args.len() >= 8 {
                let bag_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let slot_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::debug!(entity_id, bag_id, slot_id, "requestActiveSlotChange");
                // Send onActiveSlotUpdate to client (acknowledge the change)
                let _ = tx.send(CellToBaseMsg::EntityMethodCall {
                    entity_id,
                    method_index: 71, // onActiveSlotUpdate
                    args: args.to_vec(),
                }).await;
            }
        }

        CM_REQUEST_AMMO_CHANGE => {
            if args.len() >= 8 {
                let item_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let ammo_type = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::debug!(entity_id, item_id, ammo_type, "requestAmmoChange");
                // TODO: Validate ammo type is known, update weapon, send onUpdateItem
            }
        }

        CM_CONTACT_LIST_CREATE | CM_CONTACT_LIST_DELETE | CM_CONTACT_LIST_RENAME |
        CM_CONTACT_LIST_FLAGS_UPDATE | CM_CONTACT_LIST_ADD_MEMBERS |
        CM_CONTACT_LIST_REMOVE_MEMBERS => {
            tracing::debug!(entity_id, method_index, "Contact list operation (stub — no persistence)");
        }

        idx if (CM_ORG_FIRST..=CM_ORG_LAST).contains(&idx) => {
            tracing::debug!(entity_id, method_index, "Organization operation (stub)");
        }

        idx if (CM_MINIGAME_FIRST..=CM_MINIGAME_LAST).contains(&idx) => {
            tracing::debug!(entity_id, method_index, "Minigame operation (stub)");
        }

        idx if (CM_BLACK_MARKET_FIRST..=CM_BLACK_MARKET_LAST).contains(&idx) => {
            tracing::debug!(entity_id, method_index, "Black market operation (stub)");
        }

        CM_ON_DIAL_GATE => {
            if args.len() >= 8 {
                let target_address_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let source_address_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::debug!(entity_id, target_address_id, source_address_id, "onDialGate");
                super::gate_travel::handle_dial_gate(
                    entity_id, target_address_id, source_address_id, tx, space_mgr,
                ).await;
            }
        }

        CM_INTERACT => {
            if args.len() >= 4 {
                let target_entity_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::debug!(entity_id, target_entity_id, "interact");
                let dialog_id = super::interactions::handle_interact(
                    entity_id, target_entity_id as u32, tx, space_mgr, vendor_cache,
                ).await;

                // Fire content engine event if a dialog was opened
                if let Some(did) = dialog_id {
                    // Get player_id from entity (0 fallback since DB persist uses entity lookup)
                    let player_id = space_mgr.get_entity(entity_id)
                        .and_then(|e| e.player_id)
                        .unwrap_or(0);
                    super::content::fire_dialog_open(
                        entity_id, player_id, did, engine, tx, space_mgr,
                    ).await;
                }
            }
        }

        _ => {
            tracing::trace!(
                entity_id,
                method_index,
                args_len = args.len(),
                "Unhandled cell method call"
            );
        }
    }

    // Suppress unused variable warnings for parameters used in future handlers
    let _ = (tx, space_mgr);
}

/// Return a human-readable name for a cell method index, if known.
pub fn cell_method_name(index: u16) -> &'static str {
    match index {
        CM_SET_TARGET_ID => "setTargetID",
        CM_SET_MOVEMENT_TYPE => "setMovementType",
        CM_TOGGLE_COMBAT_DEBUG => "toggleCombatDebug",
        CM_TOGGLE_COMBAT_VERBOSE_DEBUG => "toggleCombatVerboseDebug",
        CM_CONFIRMATION_RESPONSE => "confirmationResponse",
        CM_SET_CROUCHED => "setCrouched",
        CM_TOGGLE_HEAL_DEBUG => "toggleHealDebug",
        CM_REQUEST_HOLSTER_WEAPON => "requestHolsterWeapon",
        CM_REQUEST_MAIL_HEADERS => "requestMailHeaders",
        CM_SEND_MAIL_MESSAGE => "sendMailMessage",
        CM_ARCHIVE_MAIL_MESSAGE => "archiveMailMessage",
        CM_DELETE_MAIL_MESSAGE => "deleteMailMessage",
        CM_RETURN_MAIL_MESSAGE => "returnMailMessage",
        CM_REQUEST_MAIL_BODY => "requestMailBody",
        CM_TAKE_CASH_FROM_MAIL => "takeCashFromMailMessage",
        CM_TAKE_ITEM_FROM_MAIL => "takeItemFromMailMessage",
        CM_PAY_COD_FOR_MAIL => "payCODForMailMessage",
        CM_CONTACT_LIST_CREATE => "contactListCreate",
        CM_CONTACT_LIST_DELETE => "contactListDelete",
        CM_CONTACT_LIST_RENAME => "contactListRename",
        CM_CONTACT_LIST_FLAGS_UPDATE => "contactListFlagsUpdate",
        CM_CONTACT_LIST_ADD_MEMBERS => "contactListAddMembers",
        CM_CONTACT_LIST_REMOVE_MEMBERS => "contactListRemoveMembers",
        CM_REMOVE_ITEM => "removeItem",
        CM_LIST_ITEMS => "listItems",
        CM_MOVE_ITEM => "moveItem",
        CM_USE_ITEM => "useItem",
        CM_REPAIR_ITEM => "repairItemRequest",
        CM_REQUEST_ACTIVE_SLOT_CHANGE => "requestActiveSlotChange",
        CM_REQUEST_AMMO_CHANGE => "requestAmmoChange",
        CM_DIALOG_BUTTON_CHOICE => "dialogButtonChoice",
        CM_INITIAL_RESPONSE => "initialResponse",
        CM_TRAIN_ABILITY => "trainAbility",
        CM_PURCHASE_ITEMS => "purchaseItems",
        CM_SELL_ITEMS => "sellItems",
        CM_BUYBACK_ITEMS => "buybackItems",
        CM_REPAIR_ITEMS => "repairItems",
        CM_RECHARGE_ITEMS => "rechargeItems",
        CM_ON_DIAL_GATE => "onDialGate",
        CM_ABANDON_MISSION => "abandonMission",
        CM_SHARE_MISSION => "shareMission",
        CM_SHARE_MISSION_RESPONSE => "shareMissionResponse",
        CM_CALL_FOR_AID => "callForAid",
        CM_USE_ABILITY => "useAbility",
        CM_USE_ABILITY_ON_GROUND => "useAbilityOnGroundTarget",
        CM_RESPAWN => "respawn",
        CM_INTERACT => "interact",
        CM_SET_AUTO_CYCLE => "setAutoCycle",
        _ => "unknown",
    }
}

// ── Witness Notification ──────────────────────────────────────────────────────

/// Send a client method call to all entities that have `entity_id` in their AoI.
///
/// Used for state changes (target, movement type, crouch, holster) that need
/// to be relayed to nearby players so they can update their view of this entity.
async fn notify_witnesses(
    entity_id: u32,
    method_index: u16,
    args: Vec<u8>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let witnesses: Vec<u32> = match space_mgr.get_entity(entity_id) {
        Some(e) => e.witnesses.iter().map(|eid| eid.0 as u32).collect(),
        None => return,
    };

    for witness_id in witnesses {
        let _ = tx.send(CellToBaseMsg::EntityMethodCall {
            entity_id: witness_id,
            method_index,
            args: args.clone(),
        }).await;
    }
}

// ── Respawn ──────────────────────────────────────────────────────────────────

/// Handle player respawn: restore health/focus, clear dead state, send updates.
///
/// Reference: `python/cell/SGWBeing.py:revive()` — restores pools to max,
/// clears combatantState dead flag, sends stat and state field updates.
async fn handle_respawn(
    entity_id: u32,
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

    // Clear dead state
    let mut state_field = 0u32;
    combat::clear_dead_state(&mut state_field);

    // Clear all ability cooldowns
    entity.abilities.clear_all_cooldowns();

    tracing::info!(entity_id, "Player respawned");

    // Send onStatUpdate (index 20) — restored health/focus
    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index: 20,
        args: stat_update,
    }).await;

    // Send onStateFieldUpdate (index 19) — cleared dead flag
    let mut state_args = Vec::with_capacity(4);
    state_args.extend_from_slice(&state_field.to_le_bytes());
    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index: 19,
        args: state_args,
    }).await;
}

// ── Store Transactions ───────────────────────────────────────────────────────

/// Parse a WSTRING from the wire at the given offset.
///
/// Returns the decoded string and the new offset past the WSTRING data.
fn parse_wstring(args: &[u8], mut offset: usize) -> (String, usize) {
    if offset + 4 > args.len() {
        return (String::new(), offset);
    }
    let char_count = u32::from_le_bytes([args[offset], args[offset+1], args[offset+2], args[offset+3]]) as usize;
    offset += 4;
    if offset + char_count * 2 > args.len() {
        return (String::new(), offset);
    }
    let utf16: Vec<u16> = (0..char_count).map(|i| {
        u16::from_le_bytes([args[offset + i*2], args[offset + i*2 + 1]])
    }).collect();
    offset += char_count * 2;
    (String::from_utf16_lossy(&utf16), offset)
}

/// Parse an ARRAY of {INT32, INT32} pairs from the wire.
///
/// Wire format: `count:u32, [field_a:i32, field_b:i32] * count`
fn parse_i32_pair_array(args: &[u8]) -> Vec<(i32, i32)> {
    if args.len() < 4 {
        return Vec::new();
    }
    let count = u32::from_le_bytes([args[0], args[1], args[2], args[3]]) as usize;
    let mut offset = 4;
    let mut pairs = Vec::with_capacity(count);
    for _ in 0..count {
        if offset + 8 > args.len() {
            break;
        }
        let a = i32::from_le_bytes([args[offset], args[offset+1], args[offset+2], args[offset+3]]);
        let b = i32::from_le_bytes([args[offset+4], args[offset+5], args[offset+6], args[offset+7]]);
        pairs.push((a, b));
        offset += 8;
    }
    pairs
}

/// Send `onCashChanged(INT32 newBalance)` to the client.
///
/// TODO: The cell doesn't track the player's naquadah balance, so we send
/// a delta here. The BaseApp should ideally maintain the authoritative balance
/// and send the absolute value. For now this gets the wire message flowing.
async fn send_cash_changed(
    entity_id: u32,
    delta: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index: 75, // onCashChanged
        args: delta.to_le_bytes().to_vec(),
    }).await;
}

/// Handle `purchaseItems(ARRAY<{INT32 designId, INT32 quantity}>)`.
///
/// For each item in the request:
/// 1. Look up the unit price from the vendor cache
/// 2. Grant the item via CellToBaseMsg::GrantItem
/// 3. After all items, send onCashChanged with the total cost (negative delta)
///
/// Reference: `python/cell/SGWPlayer.py` purchaseItems
async fn handle_purchase_items(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
    vendor_cache: &super::vendor::VendorCache,
) {
    let items = parse_i32_pair_array(args);
    if items.is_empty() {
        return;
    }

    let player_id = space_mgr.get_entity(entity_id)
        .and_then(|e| e.player_id)
        .unwrap_or(0);

    let mut total_cost: i64 = 0;

    for (design_id, quantity) in &items {
        let quantity = (*quantity).max(1);

        // Look up price from any cached vendor's buy list
        let unit_price = vendor_cache.find_buy_price(*design_id).await.unwrap_or(0);
        total_cost += unit_price as i64 * quantity as i64;

        tracing::debug!(
            entity_id, design_id, quantity, unit_price,
            "purchaseItems: granting item"
        );

        // Grant each item to the player
        let _ = tx.send(CellToBaseMsg::GrantItem {
            entity_id,
            player_id,
            item_id: *design_id,
            container_id: 0, // BaseApp resolves the target container
            count: quantity,
        }).await;
    }

    // Notify the client about the cash change (negative = spent)
    let cost_i32 = (total_cost.min(i32::MAX as i64)) as i32;
    send_cash_changed(entity_id, -cost_i32, tx).await;

    tracing::info!(
        entity_id, item_count = items.len(), total_cost,
        "Purchase complete"
    );
}

/// Handle `sellItems(ARRAY<{INT32 instanceId, INT32 quantity}>)`.
///
/// For each item in the request:
/// 1. Send onRemoveItem to remove it from the client's inventory
/// 2. After all items, send onCashChanged with the sell value (positive delta)
///
/// Reference: `python/cell/SGWPlayer.py` sellItems
async fn handle_sell_items(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
    vendor_cache: &super::vendor::VendorCache,
) {
    let items = parse_i32_pair_array(args);
    if items.is_empty() {
        return;
    }

    let player_id = space_mgr.get_entity(entity_id)
        .and_then(|e| e.player_id)
        .unwrap_or(0);

    let mut total_value: i64 = 0;
    let mut remove_ids: Vec<i32> = Vec::with_capacity(items.len());

    for (instance_id, quantity) in &items {
        let quantity = (*quantity).max(1);

        // Sell price: check vendor sell list, fall back to a flat default.
        // The instance_id is an inventory row ID, not a design_id, so we can't
        // easily look up the exact item type here without DB access. Use a
        // placeholder sell value until BaseApp-side inventory validation is wired.
        // TODO: Look up the item's design_id from inventory to get the real price.
        let sell_price = vendor_cache.find_sell_price(*instance_id).await.unwrap_or(1);
        total_value += sell_price as i64 * quantity as i64;

        tracing::debug!(
            entity_id, instance_id, quantity, sell_price,
            "sellItems: removing item"
        );

        remove_ids.push(*instance_id);

        // Tell BaseApp to remove from DB (negative count = removal)
        let _ = tx.send(CellToBaseMsg::GrantItem {
            entity_id,
            player_id,
            item_id: *instance_id,
            container_id: 0,
            count: -(quantity),
        }).await;
    }

    // Send onRemoveItem(ARRAY<INT32>) to the client — batch all removed IDs
    if !remove_ids.is_empty() {
        let mut rm_args = Vec::with_capacity(4 + remove_ids.len() * 4);
        rm_args.extend_from_slice(&(remove_ids.len() as u32).to_le_bytes());
        for id in &remove_ids {
            rm_args.extend_from_slice(&id.to_le_bytes());
        }
        let _ = tx.send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: 73, // onRemoveItem
            args: rm_args,
        }).await;
    }

    // Notify the client about the cash gained (positive = earned)
    let value_i32 = (total_value.min(i32::MAX as i64)) as i32;
    send_cash_changed(entity_id, value_i32, tx).await;

    tracing::info!(
        entity_id, item_count = items.len(), total_value,
        "Sell complete"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_method_name_known() {
        assert_eq!(cell_method_name(CM_SET_TARGET_ID), "setTargetID");
        assert_eq!(cell_method_name(CM_SET_CROUCHED), "setCrouched");
        assert_eq!(cell_method_name(CM_REQUEST_HOLSTER_WEAPON), "requestHolsterWeapon");
    }

    #[test]
    fn cell_method_name_unknown() {
        assert_eq!(cell_method_name(255), "unknown");
    }

    #[test]
    fn indices_are_sequential() {
        // SGWBeing exposed CellMethods come first
        assert_eq!(CM_SET_TARGET_ID, 0);
        assert_eq!(CM_SET_MOVEMENT_TYPE, 1);
        // Then SGWAbilityManager
        assert_eq!(CM_TOGGLE_COMBAT_DEBUG, 2);
        assert_eq!(CM_TOGGLE_COMBAT_VERBOSE_DEBUG, 3);
        assert_eq!(CM_CONFIRMATION_RESPONSE, 4);
        // Then SGWCombatant
        assert_eq!(CM_SET_CROUCHED, 5);
        assert_eq!(CM_TOGGLE_HEAL_DEBUG, 6);
        assert_eq!(CM_REQUEST_HOLSTER_WEAPON, 7);
    }

    #[test]
    fn parse_i32_pair_array_empty() {
        assert!(parse_i32_pair_array(&[]).is_empty());
        // count = 0
        assert!(parse_i32_pair_array(&[0, 0, 0, 0]).is_empty());
    }

    #[test]
    fn parse_i32_pair_array_single() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&1u32.to_le_bytes()); // count = 1
        buf.extend_from_slice(&100i32.to_le_bytes()); // designId = 100
        buf.extend_from_slice(&3i32.to_le_bytes());   // quantity = 3

        let pairs = parse_i32_pair_array(&buf);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0], (100, 3));
    }

    #[test]
    fn parse_i32_pair_array_multiple() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&2u32.to_le_bytes()); // count = 2
        buf.extend_from_slice(&10i32.to_le_bytes()); // pair 0: a=10
        buf.extend_from_slice(&1i32.to_le_bytes());  //         b=1
        buf.extend_from_slice(&20i32.to_le_bytes()); // pair 1: a=20
        buf.extend_from_slice(&5i32.to_le_bytes());  //         b=5

        let pairs = parse_i32_pair_array(&buf);
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0], (10, 1));
        assert_eq!(pairs[1], (20, 5));
    }

    #[test]
    fn parse_i32_pair_array_truncated() {
        // count says 2 but only 1 pair of data present
        let mut buf = Vec::new();
        buf.extend_from_slice(&2u32.to_le_bytes());
        buf.extend_from_slice(&42i32.to_le_bytes());
        buf.extend_from_slice(&7i32.to_le_bytes());
        // missing second pair

        let pairs = parse_i32_pair_array(&buf);
        assert_eq!(pairs.len(), 1); // gracefully stops at available data
        assert_eq!(pairs[0], (42, 7));
    }
}
