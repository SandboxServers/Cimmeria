//! BaseApp-side handlers for organization messages.
//!
//! - `mod.rs` (this file) — Squad (ephemeral) handlers.
//! - `persistent` — Team/Command (persistent) handlers (Phase 3).
//!
//! Every `OrgSquad*` / `OrgPersistent*` variant that arrives via `CellToBaseMsg`
//! is routed here from `cell_dispatch::organization_dispatch`.  Handlers fan out
//! S→C client methods to the affected entity IDs using `send_to_witness_reliable`
//! (the same pattern as `base::contact_list::handlers`).
//!
//! # Fan-out convention
//! Fan-out is fire-and-forget: we `tokio::spawn` independent sends per
//! recipient so a slow client does not stall the entire dispatch loop.  The
//! individual `send_to_witness_reliable` call is already non-blocking.
//!
//! # OrgType::Squad = 0
//! All wire messages use `org_type = 0` (Squad).

pub mod persistent;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;

use crate::base::helpers::send_to_witness_reliable;
use crate::base::ConnectedClientState;
use crate::cell::social::squad_manager::RosterEntry;
use crate::mercury::{build_player_entity_method_packet, method_idx};

use super::wire::{
    build_on_member_joined_organization, build_on_member_left_organization,
    build_on_organization_joined, build_on_organization_left, build_on_organization_roster_info,
    build_on_squad_loot_type,
};

/// OrgType::Squad wire value.
const ORG_TYPE_SQUAD: u8 = 0;

/// Handle `OrgSquadAccepted` — fan out join notifications.
///
/// Sends:
/// - `onMemberJoinedOrganization` [37] to each existing member
/// - `onOrganizationJoined` [35] to the new member
/// - `onOrganizationRosterInfo` [38] to the new member
pub async fn handle_squad_accepted(
    org_id: i32,
    new_member_entity_id: u32,
    new_member_player_id: i32,
    new_member_name: String,
    new_member_rank: u8,
    existing_member_entity_ids: Vec<u32>,
    roster: Vec<RosterEntry>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    // 1. Notify existing members: onMemberJoinedOrganization [37]
    let joined_args = build_on_member_joined_organization(
        &new_member_name,
        new_member_player_id,
        org_id,
        new_member_rank,
        true,
    );
    for &member_eid in &existing_member_entity_ids {
        let args = joined_args.clone();
        let transport = Arc::clone(transport);
        let connected = Arc::clone(connected);
        let entity_to_addr = Arc::clone(entity_to_addr);
        tokio::spawn(async move {
            send_to_witness_reliable(
                &transport,
                &connected,
                &entity_to_addr,
                member_eid,
                |key, version, seq, acks| {
                    build_player_entity_method_packet(
                        key,
                        seq,
                        acks,
                        member_eid,
                        method_idx::ON_MEMBER_JOINED_ORGANIZATION,
                        &args,
                        version,
                    )
                },
            )
            .await;
        });
    }

    // 2. Notify new member: onOrganizationJoined [35]
    let joined_self_args =
        build_on_organization_joined(org_id, ORG_TYPE_SQUAD, new_member_rank, true);
    {
        let args = joined_self_args;
        let transport = Arc::clone(transport);
        let connected = Arc::clone(connected);
        let entity_to_addr = Arc::clone(entity_to_addr);
        let eid = new_member_entity_id;
        tokio::spawn(async move {
            send_to_witness_reliable(
                &transport,
                &connected,
                &entity_to_addr,
                eid,
                |key, version, seq, acks| {
                    build_player_entity_method_packet(
                        key,
                        seq,
                        acks,
                        eid,
                        method_idx::ON_ORGANIZATION_JOINED,
                        &args,
                        version,
                    )
                },
            )
            .await;
        });
    }

    // 3. Send full roster to new member: onOrganizationRosterInfo [38]
    let roster_args = build_on_organization_roster_info(org_id, &roster);
    {
        let args = roster_args;
        let transport = Arc::clone(transport);
        let connected = Arc::clone(connected);
        let entity_to_addr = Arc::clone(entity_to_addr);
        let eid = new_member_entity_id;
        tokio::spawn(async move {
            send_to_witness_reliable(
                &transport,
                &connected,
                &entity_to_addr,
                eid,
                |key, version, seq, acks| {
                    build_player_entity_method_packet(
                        key,
                        seq,
                        acks,
                        eid,
                        method_idx::ON_ORGANIZATION_ROSTER_INFO,
                        &args,
                        version,
                    )
                },
            )
            .await;
        });
    }
}

/// Handle `OrgSquadMemberLeft` — fan out departure notifications.
///
/// Sends:
/// - `onOrganizationLeft` [36] to the departing entity
/// - `onMemberLeftOrganization` [39] to all remaining members
pub async fn handle_squad_member_left(
    org_id: i32,
    leaving_entity_id: u32,
    leaving_player_id: i32,
    leaving_name: String,
    reason: u8,
    remaining_member_entity_ids: Vec<u32>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    // 1. Tell the departing player: onOrganizationLeft [36]
    {
        let args = build_on_organization_left(reason, org_id);
        let transport = Arc::clone(transport);
        let connected = Arc::clone(connected);
        let entity_to_addr = Arc::clone(entity_to_addr);
        let eid = leaving_entity_id;
        tokio::spawn(async move {
            send_to_witness_reliable(
                &transport,
                &connected,
                &entity_to_addr,
                eid,
                |key, version, seq, acks| {
                    build_player_entity_method_packet(
                        key,
                        seq,
                        acks,
                        eid,
                        method_idx::ON_ORGANIZATION_LEFT,
                        &args,
                        version,
                    )
                },
            )
            .await;
        });
    }

    // 2. Tell remaining members: onMemberLeftOrganization [39]
    let left_args =
        build_on_member_left_organization(leaving_player_id, reason, org_id, &leaving_name);
    for &member_eid in &remaining_member_entity_ids {
        let args = left_args.clone();
        let transport = Arc::clone(transport);
        let connected = Arc::clone(connected);
        let entity_to_addr = Arc::clone(entity_to_addr);
        tokio::spawn(async move {
            send_to_witness_reliable(
                &transport,
                &connected,
                &entity_to_addr,
                member_eid,
                |key, version, seq, acks| {
                    build_player_entity_method_packet(
                        key,
                        seq,
                        acks,
                        member_eid,
                        method_idx::ON_MEMBER_LEFT_ORGANIZATION,
                        &args,
                        version,
                    )
                },
            )
            .await;
        });
    }
}

/// Handle `OrgSquadLootMode` — fan out `onSquadLootType` [51] to all members.
pub async fn handle_squad_loot_mode(
    org_id: i32,
    loot_mode: i32,
    member_entity_ids: Vec<u32>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let args = build_on_squad_loot_type(org_id, loot_mode);
    for &eid in &member_entity_ids {
        let args = args.clone();
        let transport = Arc::clone(transport);
        let connected = Arc::clone(connected);
        let entity_to_addr = Arc::clone(entity_to_addr);
        tokio::spawn(async move {
            send_to_witness_reliable(
                &transport,
                &connected,
                &entity_to_addr,
                eid,
                |key, version, seq, acks| {
                    build_player_entity_method_packet(
                        key,
                        seq,
                        acks,
                        eid,
                        method_idx::ON_SQUAD_LOOT_TYPE,
                        &args,
                        version,
                    )
                },
            )
            .await;
        });
    }
}

/// Handle `OrgSquadMinimapPing` — fan out `receivedMinimapPing` to squad members.
///
/// `receivedMinimapPing` is a client cell method (server-distributed).
///
/// # TODO: fanout deferred — method index unconfirmed
///
/// The `receivedMinimapPing` method index is blocked on x64dbg confirmation.
/// Do NOT assign a speculative index — sending on the wrong index will silently
/// invoke an unrelated client method. Once the index is confirmed via x64dbg
/// tracing the OrganizationMember interface dispatch table, replace this stub
/// with a `send_to_witness_reliable` fan-out loop over `member_entity_ids`,
/// matching the pattern in `handle_squad_loot_mode` and update
/// `docs/protocol/client-method-dispatch-table.md` with the confirmed value.
///
/// Until then this handler logs the event so the server is not silent on ping.
pub async fn handle_squad_minimap_ping(
    org_id: i32,
    sender_entity_id: u32,
    location: [f32; 3],
    member_entity_ids: Vec<u32>,
) {
    // TODO: wire fanout once receivedMinimapPing method index is confirmed via x64dbg.
    tracing::debug!(
        org_id,
        sender_entity_id,
        x = location[0],
        y = location[1],
        z = location[2],
        member_count = member_entity_ids.len(),
        "Squad minimap ping (fanout deferred — receivedMinimapPing method index unconfirmed)"
    );
}
