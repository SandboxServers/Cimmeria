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

use cimmeria_mercury::packet::{build_outgoing, FLAG_HAS_ACKS};
use tokio::net::UdpSocket;

use crate::mercury::{
    build_enter_world_body, build_entity_method_packet, encrypt_packet, method_idx, WorldEntryInfo,
    BASEMSG_CREATE_BASE_PLAYER, REPLY_FLAGS, SGWPLAYER_CLASS_ID,
};

use super::super::ConnectedClientState;

/// Build the burst-body bytes: `CREATE_BASE_PLAYER` header + `enter_world_body`.
///
/// Pure function so the wire layout is unit-testable without spinning a
/// socket. The byte ordering here is load-bearing — the client's
/// `createBasePlayer` handler reads `entity_id u32` then `class_id u16`
/// (yes, u16 — the trailing `propertyCount` byte gets folded into the
/// class read; see the in-tree `phases::build_create_player` for the
/// reference layout this mirrors).
fn build_reanchor_burst_body(entity_id: u32, info: &WorldEntryInfo) -> Vec<u8> {
    let mut body = Vec::with_capacity(128);
    body.push(BASEMSG_CREATE_BASE_PLAYER);
    body.extend_from_slice(&6u16.to_le_bytes());
    body.extend_from_slice(&entity_id.to_le_bytes());
    body.push(SGWPLAYER_CLASS_ID);
    body.push(0x00); // propertyCount = 0
    body.extend_from_slice(&build_enter_world_body(info));
    body
}

/// Handle a re-anchor request from CellService.
pub(crate) async fn handle_reanchor_player(
    entity_id: u32,
    space_id: u32,
    position: [f32; 3],
    rotation: [f32; 3],
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = entity_to_addr
        .lock()
        .unwrap()
        .get(&entity_id)
        .copied()
        .ok_or("Reanchor: no client addr for entity")?;

    let (key, pending_acks_arc, next_seq, appearance_args, tint_args) = {
        let clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        let c = clients
            .get(&addr)
            .ok_or("Reanchor: client state not found")?;
        (
            c.key,
            Arc::clone(&c.pending_acks),
            Arc::clone(&c.next_seq),
            c.cached_appearance_args.clone(),
            c.cached_tint_args.clone(),
        )
    };

    // Always SGWPlayer (0x02). gate_travel uses the same convention — the
    // explicit note there is that SGWGmPlayer (0x03) shifts method indices.
    let info = WorldEntryInfo {
        player_entity_id: entity_id,
        space_id,
        pos: position,
        rot: rotation,
        world_name: String::new(),
        class_id: SGWPLAYER_CLASS_ID,
        world_stargates: Vec::new(),
    };

    let burst = build_reanchor_burst_body(entity_id, &info);

    // Reserve 1 seq for burst + 1 for appearance + 1 for tint (if cached).
    let replay_count = match (&appearance_args, &tint_args) {
        (Some(_), Some(_)) => 2,
        _ => 0,
    };
    let total_seqs = 1 + replay_count;

    let acks: Vec<u32> = {
        let mut pending = pending_acks_arc.lock().unwrap();
        pending.drain(..).collect()
    };
    let base_seq = next_seq.fetch_add(total_seqs, Ordering::Relaxed);

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &burst, Some(base_seq), &acks, None);
    let pkt = encrypt_packet(&plaintext, &key);
    socket.send_to(&pkt, addr).await?;

    // Packet 2: property replay (BeingAppearance + onEntityTint), separate
    // bundle so the client's creation-transaction window settles first.
    if let (Some(appearance), Some(tint)) = (appearance_args, tint_args) {
        let appearance_pkt = build_entity_method_packet(
            &key,
            base_seq + 1,
            &[],
            entity_id,
            method_idx::BEING_APPEARANCE,
            &appearance,
        );
        socket.send_to(&appearance_pkt, addr).await?;

        let tint_pkt = build_entity_method_packet(
            &key,
            base_seq + 2,
            &[],
            entity_id,
            method_idx::ON_ENTITY_TINT,
            &tint,
        );
        socket.send_to(&tint_pkt, addr).await?;

        tracing::info!(
            entity_id, %addr, space_id, ?position,
            "Reanchor: sent CREATE_BASE_PLAYER burst + BeingAppearance + onEntityTint (no RESET_ENTITIES)"
        );
    } else {
        tracing::warn!(
            entity_id, %addr,
            "Reanchor: cached appearance/tint missing — sent burst only, pawn may render blank"
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_info(entity_id: u32) -> WorldEntryInfo {
        WorldEntryInfo {
            player_entity_id: entity_id,
            space_id: 0x0001_0042,
            pos: [10.0, 20.0, 30.0],
            rot: [0.5, 1.5, 2.5],
            world_name: String::new(),
            class_id: SGWPLAYER_CLASS_ID,
            world_stargates: Vec::new(),
        }
    }

    /// Pin the wire layout of the CREATE_BASE_PLAYER + enter_world_body burst.
    ///
    /// This is the load-bearing un-ragdoll primitive. If anyone refactors
    /// `build_enter_world_body`, changes the `BASEMSG_CREATE_BASE_PLAYER`
    /// constant, or "cleans up" the `propertyCount = 0` byte, this test
    /// catches it before the pawn-recreate hook silently breaks.
    #[test]
    fn build_reanchor_burst_body_pins_create_base_player_then_enter_world_body() {
        let entity_id: u32 = 0x1234_5678;
        let info = sample_info(entity_id);

        let body = build_reanchor_burst_body(entity_id, &info);

        // CREATE_BASE_PLAYER header: [0x05][len=6 LE][entity_id LE][class=0x02][propCount=0]
        assert_eq!(
            body[0], BASEMSG_CREATE_BASE_PLAYER,
            "first byte must be CREATE_BASE_PLAYER (0x05)"
        );
        assert_eq!(
            &body[1..3],
            &6u16.to_le_bytes(),
            "length field must be 6 (entity_id 4 + class 1 + propCount 1)"
        );
        assert_eq!(
            &body[3..7],
            &entity_id.to_le_bytes(),
            "entity_id must be little-endian u32"
        );
        assert_eq!(
            body[7], SGWPLAYER_CLASS_ID,
            "class_id must be SGWPlayer (0x02) — SGWGmPlayer (0x03) shifts method indices"
        );
        assert_eq!(body[8], 0x00, "propertyCount byte must be 0");

        // Tail must be byte-identical to build_enter_world_body so any future
        // refactor of that function (Y/Z swap fixes, viewport id changes,
        // forced-position flags) flows through Reanchor unchanged.
        assert_eq!(
            &body[9..],
            build_enter_world_body(&info).as_slice(),
            "tail must equal build_enter_world_body(info) verbatim — Reanchor and gate-travel must stay in lockstep on space/viewport/position"
        );
    }

    /// Guard against accidental class_id changes leaking from auth context.
    /// Even if a future caller plumbs `access_level` through, Reanchor must
    /// continue to send SGWPlayer (0x02) — SGWGmPlayer (0x03) shifts method
    /// indices and corrupts the client's entity table.
    #[test]
    fn build_reanchor_burst_body_always_uses_sgwplayer_class() {
        let info = WorldEntryInfo {
            class_id: 0x03, // pretend caller asked for SGWGmPlayer
            ..sample_info(99)
        };
        let body = build_reanchor_burst_body(99, &info);
        assert_eq!(
            body[7], SGWPLAYER_CLASS_ID,
            "class_id in the burst must hard-code SGWPlayer regardless of caller-supplied class"
        );
    }
}
