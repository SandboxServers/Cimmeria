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
use cimmeria_mercury::transport::Transport;

use crate::mercury::{
    build_enter_world_body, build_player_entity_method_packet, encrypt_packet, method_idx,
    WorldEntryInfo, BASEMSG_CREATE_BASE_PLAYER, REPLY_FLAGS, SGWPLAYER_CLASS_ID,
};

use super::super::ConnectedClientState;

/// Build the burst-body bytes: `CREATE_BASE_PLAYER` header + `enter_world_body`.
///
/// Pure function so the wire layout is unit-testable without spinning a
/// transport. The byte ordering here is load-bearing — the client's
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
                     // Reanchor passes None for `load`: the appearance/tint replay is sent
                     // as separate packets after the burst (see `build_reanchor_packets`),
                     // matching the original wire shape.
    body.extend_from_slice(&build_enter_world_body(info, None));
    body
}

/// Build the encrypted packets a Reanchor sends.
///
/// Returns either 1 packet (burst only, when cached appearance/tint is
/// missing) or 3 packets (burst + BeingAppearance + onEntityTint).
/// Pure function — no I/O, no shared state — so the count, sequencing,
/// and ack attachment are unit-testable.
///
/// Invariants pinned by tests in this module:
/// - Acks are attached to the burst packet only; replay packets pass `&[]`.
/// - Sequence IDs are consecutive: `base_seq`, `base_seq + 1`, `base_seq + 2`.
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
) -> Vec<Vec<u8>> {
    let burst = build_reanchor_burst_body(entity_id, info);
    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let burst_plaintext = build_outgoing(flags, &burst, Some(base_seq), acks, None);
    let mut packets = vec![encrypt_packet(&burst_plaintext, key)];

    // Both must be cached. A partial replay would re-create the pawn with
    // body geometry but no skin tint (or vice versa), worse than no replay.
    if let (Some(appearance), Some(tint)) = (appearance_args, tint_args) {
        packets.push(build_player_entity_method_packet(
            key,
            base_seq + 1,
            &[],
            entity_id,
            method_idx::BEING_APPEARANCE,
            appearance,
        ));
        packets.push(build_player_entity_method_packet(
            key,
            base_seq + 2,
            &[],
            entity_id,
            method_idx::ON_ENTITY_TINT,
            tint,
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

    // Reserve seq IDs up front so build_reanchor_packets can hand back a
    // self-contained list. 1 for burst, 2 more for the replay if both
    // cache slots are populated.
    let has_replay = appearance_args.is_some() && tint_args.is_some();
    let total_seqs = if has_replay { 3 } else { 1 };

    let acks: Vec<u32> = {
        let mut pending = pending_acks_arc.lock().unwrap();
        pending.drain(..).collect()
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

    if has_replay {
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
            build_enter_world_body(&info, None).as_slice(),
            "tail must equal build_enter_world_body(info, None) verbatim — Reanchor and gate-travel must stay in lockstep on space/viewport/position"
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

    const TEST_KEY: [u8; 32] = [0x42; 32];

    /// Cached appearance + tint → 3 packets (burst + BeingAppearance + onEntityTint).
    #[test]
    fn build_reanchor_packets_emits_three_when_both_cached() {
        let info = sample_info(0x1234);
        let appearance = vec![0xAA, 0xBB];
        let tint = vec![0xCC, 0xDD];

        let pkts = build_reanchor_packets(
            &TEST_KEY,
            100,
            0x1234,
            &info,
            Some(&appearance),
            Some(&tint),
            &[],
        );

        assert_eq!(
            pkts.len(),
            3,
            "with cached appearance + tint, must return 3 packets"
        );
    }

    /// No cache → 1 packet (burst only). The pawn will render blank, but
    /// emitting a partial replay would be worse — body without tint or
    /// vice versa flickers visibly.
    #[test]
    fn build_reanchor_packets_emits_one_when_neither_cached() {
        let info = sample_info(0x1234);
        let pkts = build_reanchor_packets(&TEST_KEY, 100, 0x1234, &info, None, None, &[]);

        assert_eq!(
            pkts.len(),
            1,
            "without cache, must return only the burst packet"
        );
    }

    /// Partial cache → 1 packet. Pinning the "all-or-nothing" rule for
    /// the replay so a future bug where only one of appearance/tint is
    /// populated doesn't silently emit a half-replay.
    #[test]
    fn build_reanchor_packets_emits_one_when_only_one_side_cached() {
        let info = sample_info(0x1234);
        let appearance = vec![0xAA];
        let tint = vec![0xBB];

        let only_appearance =
            build_reanchor_packets(&TEST_KEY, 100, 0x1234, &info, Some(&appearance), None, &[]);
        let only_tint =
            build_reanchor_packets(&TEST_KEY, 100, 0x1234, &info, None, Some(&tint), &[]);

        assert_eq!(
            only_appearance.len(),
            1,
            "appearance-only cache must not emit appearance packet alone"
        );
        assert_eq!(
            only_tint.len(),
            1,
            "tint-only cache must not emit tint packet alone"
        );
    }

    /// Acks attach to the burst packet only. Replay packets must use
    /// empty acks — duplicating acks would re-acknowledge already-acked
    /// sequence IDs and (depending on the protocol layer) could be
    /// rejected as malformed by the client.
    #[test]
    fn build_reanchor_packets_attaches_acks_to_burst_only() {
        let info = sample_info(0x1234);
        let appearance = vec![0xAA];
        let tint = vec![0xBB];

        let no_acks = build_reanchor_packets(
            &TEST_KEY,
            100,
            0x1234,
            &info,
            Some(&appearance),
            Some(&tint),
            &[],
        );
        let with_acks = build_reanchor_packets(
            &TEST_KEY,
            100,
            0x1234,
            &info,
            Some(&appearance),
            Some(&tint),
            &[42, 43],
        );

        assert_ne!(
            no_acks[0], with_acks[0],
            "adding acks must change the burst packet"
        );
        assert_eq!(
            no_acks[1], with_acks[1],
            "BeingAppearance packet must not include acks (was identical with vs without acks)"
        );
        assert_eq!(
            no_acks[2], with_acks[2],
            "onEntityTint packet must not include acks (was identical with vs without acks)"
        );
    }

    /// Sequence IDs are consecutive: base, base+1, base+2. Verified by
    /// reconstructing what each replay packet would look like with an
    /// explicit seq ID and asserting equality. This catches any future
    /// "simplification" that hardcodes the same seq for all packets, or
    /// that increments by the wrong stride.
    #[test]
    fn build_reanchor_packets_uses_consecutive_seqs() {
        use crate::mercury::{build_player_entity_method_packet, method_idx};

        let info = sample_info(0x1234);
        let appearance = vec![0xAA, 0xBB];
        let tint = vec![0xCC, 0xDD];
        let base_seq = 500u32;

        let pkts = build_reanchor_packets(
            &TEST_KEY,
            base_seq,
            0x1234,
            &info,
            Some(&appearance),
            Some(&tint),
            &[],
        );

        let expected_appearance = build_player_entity_method_packet(
            &TEST_KEY,
            base_seq + 1,
            &[],
            0x1234,
            method_idx::BEING_APPEARANCE,
            &appearance,
        );
        let expected_tint = build_player_entity_method_packet(
            &TEST_KEY,
            base_seq + 2,
            &[],
            0x1234,
            method_idx::ON_ENTITY_TINT,
            &tint,
        );

        assert_eq!(
            pkts[1], expected_appearance,
            "packet[1] must be BeingAppearance with seq=base_seq+1"
        );
        assert_eq!(
            pkts[2], expected_tint,
            "packet[2] must be onEntityTint with seq=base_seq+2"
        );
    }

    /// Domain C (fan-out byte test): `handle_reanchor_player` for a client with
    /// no cached appearance/tint emits exactly **one** packet — the reanchor
    /// burst — to the owner's own addr, byte-exact, with **zero** witness
    /// fan-out (reanchor is owner-only). Catches a regression that fans the
    /// owner-only burst out to witnesses, or that emits a half-replay when no
    /// cache is present.
    #[tokio::test]
    async fn reanchor_emits_single_burst_to_owner_only() {
        use crate::test_support::{test_default_connected_client_state, TestTransport};

        let transport = Arc::new(TestTransport::new());
        let dyn_transport: Arc<dyn Transport> = transport.clone();

        let entity_id = 0x4321u32;
        let space_id = 0x0001_0042u32;
        let position = [10.0f32, 20.0, 30.0];
        let rotation = [0.5f32, 1.5, 2.5];
        let addr: SocketAddr = "127.0.0.1:40200".parse().unwrap();

        // Default state has no cached appearance/tint → burst-only (1 packet).
        let entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>> =
            Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));
        let connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> =
            Arc::new(Mutex::new(HashMap::from([(
                addr,
                test_default_connected_client_state(),
            )])));

        handle_reanchor_player(
            entity_id,
            space_id,
            position,
            rotation,
            &dyn_transport,
            &connected,
            &entity_to_addr,
        )
        .await
        .expect("reanchor must succeed");

        let sent = transport.drain();
        assert_eq!(
            sent.len(),
            1,
            "burst-only (no cache) ⇒ exactly one packet, no witness fan-out"
        );
        assert_eq!(
            sent[0].0, addr,
            "reanchor burst goes to the owner's own addr"
        );

        // base_seq = 0 (default next_seq), no acks, all-zero key, no replay.
        let info = WorldEntryInfo {
            player_entity_id: entity_id,
            space_id,
            pos: position,
            rot: rotation,
            world_name: String::new(),
            class_id: SGWPLAYER_CLASS_ID,
            world_stargates: Vec::new(),
        };
        let expected = build_reanchor_packets(&[0u8; 32], 0, entity_id, &info, None, None, &[]);
        assert_eq!(expected.len(), 1, "test premise: burst-only");
        assert_eq!(sent[0].1, expected[0], "reanchor burst wire bytes (seq 0)");
    }
}
