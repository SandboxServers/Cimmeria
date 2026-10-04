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

use cimmeria_mercury::encryption::EncryptionVersion;
use cimmeria_mercury::packet::{build_outgoing, FLAG_HAS_ACKS};
use cimmeria_mercury::transport::Transport;

use crate::mercury::{
    build_enter_world_body, build_player_entity_method_packet, encrypt_packet, method_idx,
    WorldEntryInfo, BASEMSG_CREATE_BASE_PLAYER, REPLY_FLAGS, SGWPLAYER_CLASS_ID,
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
    version: EncryptionVersion,
) -> Vec<Vec<u8>> {
    let burst = build_reanchor_burst_body(entity_id, info);
    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let burst_plaintext = build_outgoing(flags, &burst, Some(base_seq), acks, None);
    let mut packets = vec![encrypt_packet(&burst_plaintext, key, version)];

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
            version,
        ));
        packets.push(build_player_entity_method_packet(
            key,
            base_seq + 2,
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
    let total_seqs = if has_replay { 3 } else { 1 };

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
mod tests {
    use super::*;
    use crate::mercury::SGWGMPLAYER_CLASS_ID;

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
            body[7], info.class_id,
            "class_id byte must be the info's (login) class"
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

    /// A GM's reanchor re-creates SGWGmPlayer (0x03), the class it logged in
    /// with. Hard-coding SGWPlayer (0x02) demoted a GM on every respawn; the
    /// "0x03 shifts method indices" reason for it was disproved
    /// (`play_character.rs`).
    #[test]
    fn build_reanchor_burst_body_writes_the_login_class() {
        let info = WorldEntryInfo {
            class_id: SGWGMPLAYER_CLASS_ID,
            ..sample_info(99)
        };
        let body = build_reanchor_burst_body(99, &info);
        assert_eq!(
            body[7], SGWGMPLAYER_CLASS_ID,
            "the burst must carry the login class, not a hard-coded SGWPlayer"
        );
    }

    /// Fan-out byte test: a GM session (login class 0x03 cached on the
    /// connected state) gets a reanchor burst that re-creates SGWGmPlayer,
    /// byte for byte. Fails on the old handler, which always sent 0x02.
    #[tokio::test]
    async fn reanchor_keeps_the_gm_class_for_a_gm_session() {
        use crate::test_support::{test_default_connected_client_state, TestTransport};

        let transport = Arc::new(TestTransport::new());
        let dyn_transport: Arc<dyn Transport> = transport.clone();
        let entity_id = 0x4322u32;
        let space_id = 0x0001_0042u32;
        let position = [1.0f32, 2.0, 3.0];
        let addr: SocketAddr = "127.0.0.1:40201".parse().unwrap();

        let mut gm = test_default_connected_client_state();
        gm.player_class_id = Some(SGWGMPLAYER_CLASS_ID);
        let entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>> =
            Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));
        let connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> =
            Arc::new(Mutex::new(HashMap::from([(addr, gm)])));

        handle_reanchor_player(
            entity_id,
            space_id,
            position,
            [0.0; 3],
            &dyn_transport,
            &connected,
            &entity_to_addr,
        )
        .await
        .expect("reanchor must succeed");

        let sent = transport.drain();
        assert_eq!(sent.len(), 1, "burst only (no cached appearance)");
        let info = WorldEntryInfo {
            player_entity_id: entity_id,
            space_id,
            pos: position,
            rot: [0.0; 3],
            world_name: String::new(),
            class_id: SGWGMPLAYER_CLASS_ID,
            world_stargates: Vec::new(),
        };
        // Built by hand, not through `build_reanchor_burst_body`, so the
        // expectation cannot inherit a class byte the handler got wrong.
        let mut body = vec![BASEMSG_CREATE_BASE_PLAYER];
        body.extend_from_slice(&6u16.to_le_bytes());
        body.extend_from_slice(&entity_id.to_le_bytes());
        body.push(SGWGMPLAYER_CLASS_ID);
        body.push(0x00);
        body.extend_from_slice(&build_enter_world_body(&info, None));
        let plaintext = build_outgoing(REPLY_FLAGS, &body, Some(0), &[], None);
        let expected = encrypt_packet(&plaintext, &[0u8; 32], EncryptionVersion::V1);
        assert_eq!(
            sent[0].1, expected,
            "a GM's reanchor must re-create SGWGmPlayer (0x03), the login class"
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
            EncryptionVersion::V1,
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
        let pkts = build_reanchor_packets(
            &TEST_KEY,
            100,
            0x1234,
            &info,
            None,
            None,
            &[],
            EncryptionVersion::V1,
        );

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

        let only_appearance = build_reanchor_packets(
            &TEST_KEY,
            100,
            0x1234,
            &info,
            Some(&appearance),
            None,
            &[],
            EncryptionVersion::V1,
        );
        let only_tint = build_reanchor_packets(
            &TEST_KEY,
            100,
            0x1234,
            &info,
            None,
            Some(&tint),
            &[],
            EncryptionVersion::V1,
        );

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
            EncryptionVersion::V1,
        );
        let with_acks = build_reanchor_packets(
            &TEST_KEY,
            100,
            0x1234,
            &info,
            Some(&appearance),
            Some(&tint),
            &[42, 43],
            EncryptionVersion::V1,
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
            EncryptionVersion::V1,
        );

        let expected_appearance = build_player_entity_method_packet(
            &TEST_KEY,
            base_seq + 1,
            &[],
            0x1234,
            method_idx::BEING_APPEARANCE,
            &appearance,
            EncryptionVersion::V1,
        );
        let expected_tint = build_player_entity_method_packet(
            &TEST_KEY,
            base_seq + 2,
            &[],
            0x1234,
            method_idx::ON_ENTITY_TINT,
            &tint,
            EncryptionVersion::V1,
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
        let expected = build_reanchor_packets(
            &[0u8; 32],
            0,
            entity_id,
            &info,
            None,
            None,
            &[],
            EncryptionVersion::V1,
        );
        assert_eq!(expected.len(), 1, "test premise: burst-only");
        assert_eq!(sent[0].1, expected[0], "reanchor burst wire bytes (seq 0)");
    }
}
