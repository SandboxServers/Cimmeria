//! Re-anchor the local pawn without `RESET_ENTITIES`.
//!
//! Triggered by `CellToBaseMsg::ReanchorPlayer`. Sends two packets to
//! the client to rebuild just the player's pawn while leaving every
//! other client-side entity (and all kismet sequence state) untouched:
//!
//! Packet 1 — pawn recreate burst:
//! 1. `BASEMSG_CREATE_BASE_PLAYER` (0x05) — destroys the existing pawn
//!    actor (carrying its ragdoll physics state) and instantiates a
//!    fresh standing one. Same primitive used on initial login.
//! 2. `BASEMSG_SPACE_VIEWPORT_INFO` (0x08)
//! 3. `BASEMSG_CREATE_CELL_PLAYER` (0x06)
//! 4. `BASEMSG_FORCED_POSITION` (0x31) — snaps the new pawn to spawn.
//!
//! Packet 2 — property replay (separate bundle, after the entity's
//! creation transaction settles):
//! 5. `BeingAppearance` entity method — restores bodyset + components.
//! 6. `onEntityTint` entity method — restores skin color.
//!
//! Both replay args are pulled from `ConnectedClientState`'s
//! `cached_appearance_args` / `cached_tint_args` (populated during
//! initial world entry in `map_loaded.rs`). Packet 2 mirrors what
//! `handle_cancel_movie` does after the first-login cinematic.
//!
//! Why split: the client treats `CREATE_CELL_PLAYER` as the start of a
//! creation transaction. Entity methods sent in the same bundle are
//! held / dropped (see comment in `map_loaded.rs:74-81`). Sending the
//! property replay in a separate bundle ensures it lands after the
//! transaction settles.
//!
//! No `RESET_ENTITIES`, no `onClientMapLoad`, no terrain reload.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::channel_bundle::IDBASE_SGW_PLAYER;
use cimmeria_mercury::encryption::EncryptionVersion;
use cimmeria_mercury::packet::{
    build_fragmented_bundle, build_outgoing, fragment_count, FLAG_HAS_ACKS, SEQUENCE_MASK,
};
use cimmeria_mercury::transport::Transport;

use crate::mercury::{
    append_entity_method, build_enter_world_body, build_player_entity_method_packet,
    encrypt_packet, method_idx, WorldEntryInfo, BASEMSG_CREATE_BASE_PLAYER, REPLY_FLAGS,
    SGWPLAYER_CLASS_ID,
};

use super::super::session_identity;
use super::super::ConnectedClientState;
use super::space_registry::world_for_space;

/// Build the burst-body bytes: `CREATE_BASE_PLAYER` header + `enter_world_body`.
///
/// Pure function so the wire layout is unit-testable without spinning a
/// transport. The byte ordering here is load-bearing — the client's
/// `createBasePlayer` handler reads `entity_id u32` then `class_id u16`
/// (yes, u16 — the trailing `propertyCount` byte gets folded into the
/// class read; see the in-tree `phases::build_create_player` for the
/// reference layout this mirrors).
///
/// The class byte is `info.class_id`, the class the client created the
/// player with at login, as `phases::build_create_player` writes it.
fn build_reanchor_burst_body(entity_id: u32, info: &WorldEntryInfo) -> Vec<u8> {
    let mut body = Vec::with_capacity(128);
    body.push(BASEMSG_CREATE_BASE_PLAYER);
    body.extend_from_slice(&6u16.to_le_bytes());
    body.extend_from_slice(&entity_id.to_le_bytes());
    body.push(info.class_id);
    body.push(0x00); // propertyCount = 0
                     // Reanchor passes None for `load`: the appearance/tint replay is sent
                     // as separate packets after the burst (see `build_reanchor_packets`),
                     // matching the original wire shape.
    body.extend_from_slice(&build_enter_world_body(info, None));
    body
}

/// The `BeingAppearance` replay message, unframed.
fn appearance_replay_body(entity_id: u32, appearance_args: &[u8]) -> Vec<u8> {
    let mut body = Vec::with_capacity(16 + appearance_args.len());
    append_entity_method(
        &mut body,
        method_idx::BEING_APPEARANCE,
        IDBASE_SGW_PLAYER,
        entity_id,
        appearance_args,
    );
    body
}

/// Reliable sequence numbers [`build_reanchor_packets`] uses: the burst, and
/// with a full cache the appearance (one per fragment) and the tint.
fn reanchor_seq_count(
    entity_id: u32,
    appearance_args: Option<&[u8]>,
    tint_args: Option<&[u8]>,
) -> u32 {
    match (appearance_args, tint_args) {
        (Some(appearance), Some(_)) => {
            let body = appearance_replay_body(entity_id, appearance);
            2 + fragment_count(&body) as u32
        }
        _ => 1,
    }
}

/// Build the encrypted packets a Reanchor sends.
///
/// Returns either 1 packet (burst only, when cached appearance/tint is
/// missing) or 3 packets (burst + BeingAppearance + onEntityTint), more when
/// an appearance past 1300 body bytes is fragmented
/// ([`reanchor_seq_count`] gives the count).
/// Pure function — no I/O, no shared state — so the count, sequencing,
/// and ack attachment are unit-testable.
///
/// Invariants pinned by tests in this module:
/// - Acks are attached to the burst packet only; replay packets pass `&[]`.
/// - Sequence IDs are consecutive from `base_seq`, one per packet.
/// - Partial cache (only one of appearance/tint cached) is treated as
///   "no cache" — we never emit a half-replay.
fn build_reanchor_packets(
    key: &[u8; 32],
    base_seq: u32,
    entity_id: u32,
    info: &WorldEntryInfo,
    appearance_args: Option<&[u8]>,
    tint_args: Option<&[u8]>,
    acks: &[u32],
    version: EncryptionVersion,
) -> Vec<Vec<u8>> {
    let burst = build_reanchor_burst_body(entity_id, info);
    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let burst_plaintext = build_outgoing(flags, &burst, Some(base_seq), acks, None);
    let mut packets = vec![encrypt_packet(&burst_plaintext, key, version)];

    // Both must be cached. A partial replay would re-create the pawn with
    // body geometry but no skin tint (or vice versa), worse than no replay.
    if let (Some(appearance), Some(tint)) = (appearance_args, tint_args) {
        // The appearance is data-sized: cut like any bundle so no datagram
        // outgrows the client's 1472-byte buffer (one packet, unchanged,
        // up to 1300 body bytes).
        let appearance_seq = base_seq.wrapping_add(1) & SEQUENCE_MASK;
        let (appearance_packets, appearance_seqs) = build_fragmented_bundle(
            REPLY_FLAGS,
            &appearance_replay_body(entity_id, appearance),
            appearance_seq,
            &[],
            |plaintext| encrypt_packet(plaintext, key, version),
        );
        packets.extend(appearance_packets);
        packets.push(build_player_entity_method_packet(
            key,
            appearance_seq.wrapping_add(appearance_seqs) & SEQUENCE_MASK,
            &[],
            entity_id,
            method_idx::ON_ENTITY_TINT,
            tint,
            version,
        ));
    }

    packets
}

/// Handle a re-anchor request from CellService.
///
/// `level = "info"` — reanchor is low-frequency (respawn, gate travel
/// within-world) and high-signal. Operators look at this span to
/// answer "did the reanchor broadcast everything the client needs to
/// rebuild its model?" A trace that shows reanchor with no child
/// inventory/mission/appearance broadcasts is the signature of the
/// post-respawn inventory-desync bug class.
#[tracing::instrument(
    name = "world_entry.reanchor_player",
    level = "info",
    skip_all,
    fields(entity_id, space_id, x = position[0], y = position[1], z = position[2]),
)]
pub(crate) async fn handle_reanchor_player(
    entity_id: u32,
    space_id: u32,
    position: [f32; 3],
    rotation: [f32; 3],
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = entity_to_addr
        .lock()
        .unwrap()
        .get(&entity_id)
        .copied()
        .ok_or("Reanchor: no client addr for entity")?;

    let (key, enc_version, pending_acks_arc, next_seq, appearance_args, tint_args, class_id, id) = {
        let clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        let c = clients
            .get(&addr)
            .ok_or("Reanchor: client state not found")?;
        (
            c.key,
            c.enc_version,
            Arc::clone(&c.pending_acks),
            Arc::clone(&c.next_seq),
            c.cached_appearance_args.clone(),
            c.cached_tint_args.clone(),
            c.player_class_id,
            session_identity::session_identity(c),
        )
    };

    // The class the client created the player with at login. A GM logs in
    // as SGWGmPlayer (0x03); re-creating it as SGWPlayer (0x02) demoted the
    // GM on every respawn. The old "0x03 shifts method indices" reason was
    // disproved (play_character.rs: SGWGmPlayer only appends methods).
    let class_id = class_id.unwrap_or_else(|| {
        tracing::warn!(
            entity_id,
            entity_name = id.player_name,
            account_id = id.account_id,
            account_name = id.account_name,
            player_id = id.player_id,
            player_name = id.player_name,
            %addr,
            reason = "login_class_unknown",
            "Reanchor: no login class cached for this session; re-creating as SGWPlayer"
        );
        SGWPLAYER_CLASS_ID
    });
    let info = WorldEntryInfo {
        player_entity_id: entity_id,
        space_id,
        pos: position,
        rot: rotation,
        world_name: String::new(),
        class_id,
        world_stargates: Vec::new(),
    };

    // Reserve seq IDs up front so build_reanchor_packets can hand back a
    // self-contained list. 1 for burst, 2 more for the replay if both
    // cache slots are populated.
    let has_replay = appearance_args.is_some() && tint_args.is_some();
    let total_seqs =
        reanchor_seq_count(entity_id, appearance_args.as_deref(), tint_args.as_deref());

    let acks: Vec<u32> = {
        let mut pending = pending_acks_arc.lock().unwrap();
        cimmeria_mercury::packet::take_piggyback_acks(&mut pending, enc_version)
    };
    let base_seq =
        next_seq.fetch_add(total_seqs, Ordering::Relaxed) & cimmeria_mercury::packet::SEQUENCE_MASK;

    let packets = build_reanchor_packets(
        &key,
        base_seq,
        entity_id,
        &info,
        appearance_args.as_deref(),
        tint_args.as_deref(),
        &acks,
        enc_version,
    );

    for (i, pkt) in packets.iter().enumerate() {
        transport.send_to(pkt, addr).await?;
        // Reanchor burst is state-change traffic — register each packet
        // with the Channel for retransmit on loss. Derived seqs are masked
        // to the 28-bit space so the contiguous range stays in-band even
        // when `base_seq + i` would otherwise cross `NULL_SEQUENCE`.
        let pkt_seq = base_seq.wrapping_add(i as u32) & cimmeria_mercury::packet::SEQUENCE_MASK;
        super::super::helpers::shadow_register_reliable_send(
            connected,
            addr,
            pkt_seq,
            cimmeria_mercury::packet::Bytes::copy_from_slice(pkt),
        );
    }

    // Same journal as the cell side: a reanchor with no `world_enter` after it
    // means the cell never re-sent regions / missions to the recreated pawn.
    crate::cell::player_journal::note(
        entity_id,
        crate::cell::player_journal::kinds::REANCHOR,
        format!("space={space_id} replay={has_replay}"),
    );
    if has_replay {
        tracing::info!(
            entity_id,
            entity_name = id.player_name,
            account_id = id.account_id,
            account_name = id.account_name,
            player_id = id.player_id,
            player_name = id.player_name,
            %addr,
            space_id,
            world = world_for_space(space_id),
            ?position,
            class_id,
            class_name = cimmeria_wire::names::class_name(class_id),
            resent = "create_base_player,being_appearance,entity_tint",
            // The cell queues the rest right behind this burst
            // (`cell::respawn`: `send_client_hinted_regions`, the inventory
            // snapshot, and `resync::resync_after_pawn_recreate`).
            resent_by_cell = "generic_regions,inventory,level,state_field,stats,base_stats,\
                              archetype,ability_tree,known_abilities,active_slot,missions",
            "Reanchor: sent CREATE_BASE_PLAYER burst + BeingAppearance + onEntityTint (no RESET_ENTITIES)"
        );
    } else {
        tracing::warn!(
            entity_id,
            entity_name = id.player_name,
            account_id = id.account_id,
            account_name = id.account_name,
            player_id = id.player_id,
            player_name = id.player_name,
            %addr,
            "Reanchor: cached appearance/tint missing — sent burst only, pawn may render blank"
        );
    }

    Ok(())
}

#[cfg(test)]
#[path = "reanchor_player_tests.rs"]
mod tests;
