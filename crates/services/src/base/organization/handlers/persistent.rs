//! BaseApp-side handlers for persistent-organization (Team/Command) messages.
//!
//! Each public function here handles one `OrgPersistent*` `CellToBaseMsg` variant,
//! routed from `cell_dispatch::organization_dispatch`.
//!
//! # Invariants
//!
//! All handlers follow the same pattern:
//! 1. Gate on `OrgAuthority` availability (skip if no DB / no authority).
//! 2. Call the relevant `OrgAuthority` mutation (permission-checked, DB write-through).
//! 3. Fan out to online members via `broadcast_to_org` **after** the DB write commits
//!    and the in-memory state is updated. Never before.
//! 4. Send a direct acknowledgement to the acting player (success or failure).
//!
//! # Mutex discipline
//!
//! `OrgAuthority` uses `tokio::sync::Mutex` so the async mutation methods
//! (which do a DB write then an in-memory update) can hold the lock across
//! their internal `.await` points. The lock is always dropped before we start
//! any fan-out (which spawns independent tasks and must not hold the lock).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_game::social::guilds::{OrgRank, OrgType};
use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use crate::base::helpers::send_to_witness_reliable;
use crate::base::organization::authority::OrgAuthority;
use crate::base::organization::fanout::broadcast_to_org;
use crate::base::organization::wire::{
    build_on_member_joined_organization, build_on_member_left_organization,
    build_on_member_rank_changed_organization, build_on_organization_creation_result,
    build_on_organization_joined, build_on_organization_left, build_on_organization_motd_update,
};
use crate::base::ConnectedClientState;
use crate::mercury::{build_player_entity_method_packet, method_idx};

// ── EReasons (provisional) ───────────────────────────────────────────────────
//
// x64dbg has not yet confirmed the exact wire values; using these as placeholders.
// Open RE gap: trace `onOrganizationLeft` call sites in the 2009 binary to pin
// the correct reason-code enum.
const REASON_KICKED: u8 = 2;

// ── OrgCreation ───────────────────────────────────────────────────────────────

/// Handle `OrgPersistentCreate`.
///
/// DB write → in-memory update → send `onOrganizationCreationResult` [134] to actor.
/// On success also sends `onOrganizationJoined` [35] so the creator's UI shows
/// the org header.
///
/// No roster fanout is needed: the creator is the only member at creation time.
pub async fn handle_persistent_create(
    entity_id: u32,
    player_id: i32,
    player_name: String,
    org_type: u8,
    name: String,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    org_authority: &Arc<tokio::sync::Mutex<OrgAuthority>>,
    db_pool: &Arc<PgPool>,
) {
    let Some(org_type_enum) = OrgType::from_u8(org_type) else {
        tracing::warn!(
            entity_id,
            org_type,
            "persistent_create: invalid org_type byte"
        );
        send_creation_result(entity_id, 1, 0, transport, connected, entity_to_addr).await;
        return;
    };

    if !org_type_enum.is_persistent() {
        tracing::warn!(
            entity_id,
            ?org_type_enum,
            "persistent_create: org_type is not persistent (Squad is ephemeral)"
        );
        send_creation_result(entity_id, 1, 1, transport, connected, entity_to_addr).await;
        return;
    }

    let result = {
        let mut auth = org_authority.lock().await;
        auth.create_org(
            db_pool,
            org_type_enum,
            &name,
            player_id,
            player_name.clone(),
        )
        .await
    };

    match result {
        Ok(r) => {
            tracing::info!(
                entity_id,
                player_id,
                org_id = r.org_id,
                org_name = name,
                "persistent_create: org created"
            );
            // Success: send result then send joined.
            send_creation_result(entity_id, 0, 0, transport, connected, entity_to_addr).await;

            let joined_payload = build_on_organization_joined(
                r.org_id, org_type, 8,    // rank = Leader
                true, // is_new_member
            );
            send_to_entity(
                entity_id,
                method_idx::ON_ORGANIZATION_JOINED,
                joined_payload,
                transport,
                connected,
                entity_to_addr,
            )
            .await;
        }
        Err(e) => {
            tracing::warn!(entity_id, player_id, reason = %e, "persistent_create: failed");
            send_creation_result(entity_id, 1, 0, transport, connected, entity_to_addr).await;
        }
    }
}

// ── InviteAccepted ────────────────────────────────────────────────────────────

/// Handle `OrgPersistentInviteAccepted`.
///
/// DB write → in-memory update → fanout:
/// - `onMemberJoinedOrganization` [37] to all existing online members (not the new one)
/// - `onOrganizationJoined` [35] to the new member
pub async fn handle_persistent_invite_accepted(
    new_member_entity_id: u32,
    new_member_player_id: i32,
    new_member_name: String,
    actor_player_id: i32,
    org_id: i32,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    org_authority: &Arc<tokio::sync::Mutex<OrgAuthority>>,
    db_pool: &Arc<PgPool>,
) {
    let result = {
        let mut auth = org_authority.lock().await;
        auth.accept_invite(
            db_pool,
            org_id,
            actor_player_id,
            new_member_player_id,
            new_member_name.clone(),
        )
        .await
    };

    match result {
        Ok(()) => {
            tracing::info!(
                new_member_player_id,
                org_id,
                "persistent_invite_accepted: member added"
            );

            // Collect existing-member entity IDs (excludes the new member).
            // Lock dropped from result block above; re-acquire read-only here.
            let (existing_entity_ids, org_type) = {
                let auth = org_authority.lock().await;
                let eids = auth.online_member_entity_ids_except(org_id, new_member_player_id);
                let ot = auth.get_org(org_id).map(|o| o.org_type as u8).unwrap_or(0);
                (eids, ot)
            };

            // Fan out `onMemberJoinedOrganization` to existing members.
            let joined_payload = build_on_member_joined_organization(
                &new_member_name,
                new_member_player_id,
                org_id,
                1, // rank = Initiate
                false,
            );
            broadcast_to_org(
                existing_entity_ids,
                method_idx::ON_MEMBER_JOINED_ORGANIZATION,
                joined_payload,
                transport,
                connected,
                entity_to_addr,
            );

            // Send `onOrganizationJoined` to the new member (joins at Initiate rank).
            let joined_payload = build_on_organization_joined(
                org_id, org_type, 1,    // rank = Initiate
                true, // is_new_member
            );
            send_to_entity(
                new_member_entity_id,
                method_idx::ON_ORGANIZATION_JOINED,
                joined_payload,
                transport,
                connected,
                entity_to_addr,
            )
            .await;
        }
        Err(e) => {
            tracing::warn!(
                new_member_player_id,
                org_id,
                reason = %e,
                "persistent_invite_accepted: failed"
            );
        }
    }
}

// ── Kick ──────────────────────────────────────────────────────────────────────

/// Handle `OrgPersistentKick`.
///
/// Name-resolve target → DB write → in-memory update → fanout:
/// - `onMemberLeftOrganization` [39] to remaining online members
/// - `onOrganizationLeft` [36] to the kicked player (if online)
pub async fn handle_persistent_kick(
    actor_entity_id: u32,
    actor_player_id: i32,
    org_id: i32,
    target_player_name: String,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    org_authority: &Arc<tokio::sync::Mutex<OrgAuthority>>,
    db_pool: &Arc<PgPool>,
) {
    // Resolve target player_id from name.
    let Some((target_player_id, target_entity_id)) =
        resolve_player_by_name(&target_player_name, org_id, org_authority).await
    else {
        tracing::warn!(
            actor_entity_id,
            org_id,
            target = target_player_name,
            "persistent_kick: target not found"
        );
        return;
    };

    let result = {
        let mut auth = org_authority.lock().await;
        auth.kick_member(db_pool, org_id, actor_player_id, target_player_id)
            .await
    };

    match result {
        Ok(kicked_name) => {
            tracing::info!(
                actor_player_id,
                target_player_id,
                org_id,
                "persistent_kick: member kicked"
            );

            let remaining_entity_ids = {
                let auth = org_authority.lock().await;
                auth.online_member_entity_ids(org_id)
            };

            // Fan out `onMemberLeftOrganization` to remaining members.
            let left_payload = build_on_member_left_organization(
                target_player_id,
                REASON_KICKED,
                org_id,
                &kicked_name,
            );
            broadcast_to_org(
                remaining_entity_ids,
                method_idx::ON_MEMBER_LEFT_ORGANIZATION,
                left_payload,
                transport,
                connected,
                entity_to_addr,
            );

            // If the kicked player is online, send `onOrganizationLeft` to them.
            // Wire: UINT8 reason, INT32 org_id — matches the .def field order.
            if let Some(eid) = target_entity_id {
                let org_left_payload = build_on_organization_left(REASON_KICKED, org_id);
                send_to_entity(
                    eid,
                    method_idx::ON_ORGANIZATION_LEFT,
                    org_left_payload,
                    transport,
                    connected,
                    entity_to_addr,
                )
                .await;
            }
        }
        Err(e) => {
            tracing::warn!(
                actor_player_id,
                target_player_id,
                org_id,
                reason = %e,
                "persistent_kick: failed"
            );
        }
    }
}

// ── RankChange ────────────────────────────────────────────────────────────────

/// Handle `OrgPersistentRankChange`.
///
/// Name-resolve target → DB write → in-memory update →
/// fanout `onMemberRankChangedOrganization` [40] to all online members.
pub async fn handle_persistent_rank_change(
    actor_entity_id: u32,
    actor_player_id: i32,
    org_id: i32,
    target_player_name: String,
    new_rank: u8,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    org_authority: &Arc<tokio::sync::Mutex<OrgAuthority>>,
    db_pool: &Arc<PgPool>,
) {
    let Some(rank) = OrgRank::from_u8(new_rank) else {
        tracing::warn!(
            actor_entity_id,
            new_rank,
            "persistent_rank_change: invalid rank byte"
        );
        return;
    };

    let Some((target_player_id, _)) =
        resolve_player_by_name(&target_player_name, org_id, org_authority).await
    else {
        tracing::warn!(
            actor_entity_id,
            org_id,
            target = target_player_name,
            "persistent_rank_change: target not found"
        );
        return;
    };

    let result = {
        let mut auth = org_authority.lock().await;
        auth.change_rank(db_pool, org_id, actor_player_id, target_player_id, rank)
            .await
    };

    match result {
        Ok(target_name) => {
            tracing::info!(
                actor_player_id,
                target_player_id,
                org_id,
                new_rank,
                "persistent_rank_change: rank changed"
            );

            let all_entity_ids = {
                let auth = org_authority.lock().await;
                auth.online_member_entity_ids(org_id)
            };

            let payload = build_on_member_rank_changed_organization(
                target_player_id,
                new_rank,
                org_id,
                &target_name,
            );
            broadcast_to_org(
                all_entity_ids,
                method_idx::ON_MEMBER_RANK_CHANGED_ORGANIZATION,
                payload,
                transport,
                connected,
                entity_to_addr,
            );
        }
        Err(e) => {
            tracing::warn!(
                actor_player_id,
                target_player_id,
                org_id,
                reason = %e,
                "persistent_rank_change: failed"
            );
        }
    }
}

// ── MOTD ─────────────────────────────────────────────────────────────────────

/// Handle `OrgPersistentSetMotd`.
///
/// DB write → in-memory update →
/// fanout `onOrganizationMOTDUpdate` [45] to all online members.
pub async fn handle_persistent_set_motd(
    actor_entity_id: u32,
    actor_player_id: i32,
    org_id: i32,
    motd: String,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    org_authority: &Arc<tokio::sync::Mutex<OrgAuthority>>,
    db_pool: &Arc<PgPool>,
) {
    let result = {
        let mut auth = org_authority.lock().await;
        auth.set_motd(db_pool, org_id, actor_player_id, &motd).await
    };

    match result {
        Ok(()) => {
            tracing::info!(actor_entity_id, org_id, "persistent_set_motd: MOTD updated");

            let all_entity_ids = {
                let auth = org_authority.lock().await;
                auth.online_member_entity_ids(org_id)
            };

            let payload = build_on_organization_motd_update(org_id, &motd);
            broadcast_to_org(
                all_entity_ids,
                method_idx::ON_ORGANIZATION_MOTD_UPDATE,
                payload,
                transport,
                connected,
                entity_to_addr,
            );
        }
        Err(e) => {
            tracing::warn!(
                actor_entity_id,
                org_id,
                reason = %e,
                "persistent_set_motd: failed"
            );
        }
    }
}

// ── Shared helpers ────────────────────────────────────────────────────────────

/// Resolve a member's `(player_id, entity_id_if_online)` from their name
/// within a specific org.
async fn resolve_player_by_name(
    name: &str,
    org_id: i32,
    org_authority: &Arc<tokio::sync::Mutex<OrgAuthority>>,
) -> Option<(i32, Option<u32>)> {
    let auth = org_authority.lock().await;
    let org = auth.get_org(org_id)?;
    let (&pid, _) = org
        .members
        .iter()
        .find(|(_, m)| m.character_name.eq_ignore_ascii_case(name))?;
    let eid = auth.entity_id_of(pid);
    Some((pid, eid))
}

/// Send `onOrganizationCreationResult` [134] to a specific entity.
async fn send_creation_result(
    entity_id: u32,
    result: u8,
    ret_code: u8,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let payload = build_on_organization_creation_result(result, ret_code);
    send_to_entity(
        entity_id,
        method_idx::ON_ORGANIZATION_CREATION_RESULT,
        payload,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

/// Send an arbitrary client method to a single entity.
async fn send_to_entity(
    entity_id: u32,
    method_index: u16,
    payload: Vec<u8>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    send_to_witness_reliable(
        transport,
        connected,
        entity_to_addr,
        entity_id,
        |key, version, seq, acks| {
            build_player_entity_method_packet(
                key,
                seq,
                acks,
                entity_id,
                method_index,
                &payload,
                version,
            )
        },
    )
    .await;
}

// `build_organization_left` was removed. Use `wire::build_on_organization_left` directly,
// which encodes UINT8 reason then INT32 org_id — matching the .def field order:
//   onOrganizationLeft(aReason UINT8, aOrganizationId INT32)
