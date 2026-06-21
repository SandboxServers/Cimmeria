//! OrganizationMember interface exposed CellMethods (indices 8–19).
//!
//! Phase 2 implements the Squad (ephemeral) state machine:
//! - CM 8  organizationInviteResponse — accept/decline a pending squad invite
//! - CM 9  organizationLeave          — leave the squad; auto-promote leader
//! - CM 10 BroadcastMinimapPing       — fan out minimap ping to squad members
//! - CM 18 squadSetLootMode           — set + fan out the squad loot mode
//!
//! All other CMs (Team/Command persistence, PvP, MOTD, notes, rank, cash)
//! are Phase 3+; their arms still parse and log so the wire is not silent.
//!
//! References:
//! - docs/reverse-engineering/findings/organization-wire-formats.md
//! - docs/reverse-engineering/findings/organization-restoration.md

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::social::squad_manager::{e_reasons, RosterEntry};
use crate::cell::space_manager::SpaceManager;

// ── Cell method indices (C→S, OrganizationMember interface) ──────────────────
pub const INVITE_RESPONSE: u16 = 8;
pub const LEAVE: u16 = 9;
pub const BROADCAST_MINIMAP_PING: u16 = 10;
pub const STRIKE_TEAM_RESPONSE: u16 = 11;
pub const PVP_LEAVE_RESPONSE: u16 = 12;
pub const MOTD: u16 = 13;
pub const NOTE: u16 = 14;
pub const OFFICER_NOTE: u16 = 15;
pub const SET_RANK_PERMISSIONS: u16 = 16;
pub const SET_RANK_NAME: u16 = 17;
pub const SQUAD_SET_LOOT_MODE: u16 = 18;
pub const TRANSFER_CASH: u16 = 19;

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    match method_index {
        INVITE_RESPONSE => {
            handle_invite_response(entity_id, args, tx, space_mgr).await;
            true
        }
        LEAVE => {
            handle_leave(entity_id, args, tx, space_mgr).await;
            true
        }
        BROADCAST_MINIMAP_PING => {
            handle_minimap_ping(entity_id, args, tx, space_mgr).await;
            true
        }
        STRIKE_TEAM_RESPONSE => {
            if args.len() >= 5 {
                let org_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let response = args[4];
                tracing::info!(
                    entity_id,
                    org_id,
                    response,
                    "UNIMPLEMENTED: strikeTeamResponse"
                );
            }
            true
        }
        PVP_LEAVE_RESPONSE => {
            if args.len() >= 5 {
                let org_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let response = args[4];
                tracing::info!(
                    entity_id,
                    org_id,
                    response,
                    "UNIMPLEMENTED: pvpOrganizationLeaveResponse"
                );
            }
            true
        }
        MOTD => {
            if args.len() >= 4 {
                let org_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, org_id, "UNIMPLEMENTED: organizationMOTD");
            }
            true
        }
        NOTE => {
            if args.len() >= 4 {
                let org_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, org_id, "UNIMPLEMENTED: organizationNote");
            }
            true
        }
        OFFICER_NOTE => {
            if args.len() >= 4 {
                let org_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, org_id, "UNIMPLEMENTED: organizationOfficerNote");
            }
            true
        }
        SET_RANK_PERMISSIONS => {
            if args.len() >= 12 {
                let org_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let rank = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                let permissions = i32::from_le_bytes([args[8], args[9], args[10], args[11]]);
                tracing::info!(
                    entity_id,
                    org_id,
                    rank,
                    permissions,
                    "UNIMPLEMENTED: organizationSetRankPermissions"
                );
            }
            true
        }
        SET_RANK_NAME => {
            // Bug noted in org-restoration.md: stub dropped the trailing WSTRING.
            // Fixed: we parse it but still log UNIMPLEMENTED until Phase 3.
            if args.len() >= 8 {
                let org_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let rank = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                let name = crate::mercury::read_wstring(args, 8)
                    .map(|(s, _)| s)
                    .unwrap_or_default();
                tracing::info!(
                    entity_id,
                    org_id,
                    rank,
                    name,
                    "UNIMPLEMENTED: organizationSetRankName"
                );
            }
            true
        }
        SQUAD_SET_LOOT_MODE => {
            handle_squad_set_loot_mode(entity_id, args, tx, space_mgr).await;
            true
        }
        TRANSFER_CASH => {
            if args.len() >= 8 {
                let org_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let cash = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::info!(
                    entity_id,
                    org_id,
                    cash,
                    "UNIMPLEMENTED: organizationTransferCash"
                );
            }
            true
        }
        _ => false,
    }
}

// ── CM 8: organizationInviteResponse ─────────────────────────────────────────

/// Wire: INT32 request_id, UINT8 response (non-zero = accept).
async fn handle_invite_response(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    if args.len() < 5 {
        tracing::warn!(
            entity_id,
            "organizationInviteResponse: too short ({} bytes)",
            args.len()
        );
        return;
    }
    let request_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
    let accepted = args[4] != 0;

    if !accepted {
        space_mgr.squads.cancel_invite(request_id);
        tracing::debug!(entity_id, request_id, "Squad invite declined");
        return;
    }

    // Resolve the accepting entity's live state from the space.
    let (target_player_id, target_name) = match space_mgr.get_entity(entity_id) {
        Some(e) => (
            e.player_id.unwrap_or(0),
            e.character_name.clone().unwrap_or_default(),
        ),
        None => {
            tracing::warn!(
                entity_id,
                request_id,
                "organizationInviteResponse: entity not found in space"
            );
            space_mgr.squads.cancel_invite(request_id);
            return;
        }
    };

    // Resolve inviter's player_id for squad creation (new-squad path).
    let inviter_player_id = space_mgr
        .squads
        .get_invite(request_id)
        .and_then(|inv| space_mgr.get_entity(inv.inviter_entity_id))
        .and_then(|e| e.player_id)
        .unwrap_or(0);

    match space_mgr.squads.accept_invite(
        request_id,
        entity_id,
        target_player_id,
        target_name.clone(),
        inviter_player_id,
    ) {
        Ok(result) => {
            // Build full roster snapshot for onOrganizationRosterInfo.
            let roster: Vec<RosterEntry> = space_mgr.squads.build_roster(result.org_id, |eid| {
                space_mgr.get_entity(eid).map_or((1, 0), |e| {
                    (
                        e.level.min(255) as u8,
                        e.archetype_id.unwrap_or(0).min(255) as u8,
                    )
                })
            });

            let _ = tx
                .send(CellToBaseMsg::OrgSquadAccepted {
                    org_id: result.org_id,
                    new_member_entity_id: result.new_member_entity_id,
                    new_member_player_id: result.new_member_player_id,
                    new_member_name: result.new_member_name,
                    new_member_rank: result.new_member_rank,
                    existing_member_entity_ids: result.existing_member_entity_ids,
                    roster,
                })
                .await;

            tracing::info!(
                entity_id,
                request_id,
                org_id = result.org_id,
                "Squad invite accepted"
            );
        }
        Err(reason) => {
            tracing::warn!(entity_id, request_id, reason, "Squad invite accept failed");
        }
    }
}

// ── CM 9: organizationLeave ───────────────────────────────────────────────────

/// Wire: INT32 org_id.
async fn handle_leave(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    if args.len() < 4 {
        tracing::warn!(entity_id, "organizationLeave: too short");
        return;
    }
    // org_id from the wire — validate it matches the entity's actual squad.
    let wire_org_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);

    // Only squads are cell-side; Team/Command are Phase 3.
    if space_mgr.squads.squad_of(entity_id) != Some(wire_org_id) {
        tracing::debug!(
            entity_id,
            wire_org_id,
            "organizationLeave: not in this squad (Phase 3 org or stale client?)"
        );
        return;
    }

    let (player_id, name) = match space_mgr.get_entity(entity_id) {
        Some(e) => (
            e.player_id.unwrap_or(0),
            e.character_name.clone().unwrap_or_default(),
        ),
        None => {
            tracing::warn!(entity_id, "organizationLeave: entity not found");
            return;
        }
    };

    match space_mgr
        .squads
        .remove_member(entity_id, player_id, name.clone())
    {
        Ok(res) => {
            let _ = tx
                .send(CellToBaseMsg::OrgSquadMemberLeft {
                    org_id: res.org_id,
                    leaving_entity_id: entity_id,
                    leaving_player_id: player_id,
                    leaving_name: name,
                    reason: e_reasons::LEFT,
                    remaining_member_entity_ids: res.remaining_entity_ids,
                })
                .await;
            tracing::info!(entity_id, org_id = res.org_id, "Player left squad");
        }
        Err(reason) => {
            tracing::warn!(entity_id, reason, "organizationLeave: remove_member failed");
        }
    }
}

// ── CM 10: BroadcastMinimapPing ───────────────────────────────────────────────

/// Wire: INT32 org_id, VECTOR3 location (3×f32 LE).
async fn handle_minimap_ping(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    if args.len() < 16 {
        tracing::warn!(
            entity_id,
            "BroadcastMinimapPing: too short ({} bytes)",
            args.len()
        );
        return;
    }
    let org_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
    let x = f32::from_le_bytes([args[4], args[5], args[6], args[7]]);
    let y = f32::from_le_bytes([args[8], args[9], args[10], args[11]]);
    let z = f32::from_le_bytes([args[12], args[13], args[14], args[15]]);

    // Validate the sender is actually in this squad.
    if space_mgr.squads.squad_of(entity_id) != Some(org_id) {
        tracing::debug!(
            entity_id,
            org_id,
            "BroadcastMinimapPing: sender not in squad"
        );
        return;
    }

    let member_ids = match space_mgr.squads.get_squad(org_id) {
        Some(s) => s.member_entity_ids(),
        None => return,
    };

    let _ = tx
        .send(CellToBaseMsg::OrgSquadMinimapPing {
            org_id,
            sender_entity_id: entity_id,
            location: [x, y, z],
            member_entity_ids: member_ids,
        })
        .await;

    tracing::debug!(entity_id, org_id, x, y, z, "Squad minimap ping");
}

// ── CM 18: squadSetLootMode ───────────────────────────────────────────────────

/// Wire: INT32 loot_mode.
async fn handle_squad_set_loot_mode(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    if args.len() < 4 {
        tracing::warn!(entity_id, "squadSetLootMode: too short");
        return;
    }
    let loot_mode = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);

    match space_mgr.squads.set_loot_mode(entity_id, loot_mode) {
        Some((org_id, member_ids)) => {
            let _ = tx
                .send(CellToBaseMsg::OrgSquadLootMode {
                    org_id,
                    loot_mode,
                    member_entity_ids: member_ids,
                })
                .await;
            tracing::debug!(entity_id, org_id, loot_mode, "Squad loot mode updated");
        }
        None => {
            tracing::debug!(entity_id, loot_mode, "squadSetLootMode: not in a squad");
        }
    }
}
