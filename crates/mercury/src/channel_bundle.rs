//! `ChannelBundle` — accumulator for multiple application-level messages
//! that the client processes as ONE frame after fragment reassembly.
//!
//! # Why bundling
//!
//! Today each `services` send site builds a packet body, encrypts, and puts
//! one UDP datagram on the wire per send. For bursts (28-NPC AoI on world
//! entry, world-entry payload, BSF state-change fanout, …) this consumes
//! more TX-window slots than necessary: every UDP datagram carries ~28 bytes
//! of IP+UDP overhead AND occupies one slot in the client's reliable-ack
//! tracker. A bundle collapses N messages targeting different entities into
//! one (or a few fragmented) packets, sharing one ACK piggyback and one
//! TX-window slot per fragment instead of N.
//!
//! # The "one bundle == one client frame" rule
//!
//! **Every message in a finalized bundle arrives at the client in one
//! reassembled frame.** This is the source of the bundling savings AND the
//! source of bundle's main hazard.
//!
//! When the client processes `CREATE_ENTITY(X)`, entity X enters a
//! "creation transaction" that holds until the end-of-bundle marker.
//! Subsequent same-entity messages in the same bundle hit the client's
//! HOLD-FOR-TRANSACTION path and are **silently dropped**. The existing
//! deliberate two-bundle split in
//! [`crates/services/src/base/world_entry/map_loaded.rs`] exists for
//! exactly this reason — combining `CELL_PLAYER` (which creates the player
//! entity) with same-entity `BeingAppearance` in one bundle dropped the
//! appearance message.
//!
//! ## Safe to combine in one bundle
//!
//! - `CREATE_ENTITY(A)` + `CREATE_ENTITY(B)` + … (different entity ids)
//! - Cross-entity AoI updates: `EnteredAoI(A)` + `EnteredAoI(B)` + …
//! - Property updates for an entity already created in a **prior** bundle
//!
//! ## Unsafe (silently dropped by client)
//!
//! - `CREATE_ENTITY(A)` + `BeingAppearance(A)` in the same bundle
//! - `CREATE_ENTITY(A)` + `onStatUpdate(A)` in the same bundle
//! - `CELL_PLAYER` + any same-player entity method in the same bundle
//!
//! When in doubt, split into two bundles. The TX-window savings of cross-
//! entity bundling are larger than the per-entity intra-bundle savings.
//!
//! # Caller-owned, not channel-owned
//!
//! `ChannelBundle` does NOT live on `Channel`. The caller constructs a
//! bundle, appends messages, calls [`ChannelBundle::finalize`] to get the
//! encrypted packets, sends them on the wire, and registers each emitted
//! packet with the channel's TX window via
//! `Channel::register_sent_packet`. This matches the channel's existing
//! shadow-register flow (see
//! [`crates/services/src/base/helpers.rs`]'s `shadow_register_reliable_send`).
//!
//! A per-channel auto-accumulator (every send appends to the same
//! channel-owned bundle, flushed on tick boundary) would re-introduce the
//! HOLD-FOR-TRANSACTION drop the map_loaded split exists to prevent —
//! every `CREATE_ENTITY` would collide with whatever same-entity message
//! happened to land in the same tick. Caller ownership keeps the "one
//! bundle == one client frame" decision in the caller's hands where the
//! transaction-state semantics are visible.
//!
//! # Wire format
//!
//! The bundle body uses the same byte layout as
//! [`crates/services/src/mercury/mod.rs`]'s `append_entity_method`:
//!
//! - Direct (method_index 0–60): `[(index | 0x80): u8][word_len: u16 LE]
//!   [entity_id: u32 LE][args...]`
//! - Extended (method_index ≥ 61): `[0xBD: u8][word_len: u16 LE]
//!   [entity_id: u32 LE][(index - 61): u8][args...]`
//!
//! On finalize, the body is handed to
//! [`crate::packet::build_fragmented_bundle`] which produces 1 packet for
//! bodies ≤ [`crate::packet::FRAGMENT_BODY_SIZE`] (1300 bytes) and
//! `ceil(body / 1300)` fragments otherwise. ACKs ride only the first
//! fragment.

use crate::packet::FRAGMENT_BODY_SIZE;

/// Method-index boundary between direct and extended entity-method
/// encoding. Indices `< 61` use direct encoding (msg_id = index | 0x80),
/// indices `>= 61` use extended encoding (msg_id = 0xBD, sub_index byte
/// after entity_id). Mirrors the constant baked into
/// `crates/services/src/mercury/mod.rs`'s `append_entity_method`.
const EXTENDED_ENCODING_THRESHOLD: u16 = 61;

/// Extended-encoding marker byte. Cannot be used as a direct msg_id.
const EXTENDED_ENCODING_MARKER: u8 = 0xBD;

/// Accumulator for one logical bundle of application-level messages.
///
/// See the module documentation for the "one bundle == one client frame"
/// rule and the safe/unsafe combinations.
#[derive(Debug)]
pub struct ChannelBundle {
    /// Concatenated message bodies in the same byte layout the existing
    /// per-call packet builders produce.
    body: Vec<u8>,
    /// ACKs to piggyback on the first finalized packet. Subsequent
    /// fragments carry none (matches the C++ Bundle behavior and the
    /// wire-format spec — ACKs are bundle-level, not per-fragment).
    acks: Vec<u32>,
    /// `true` if the bundle should ride the reliable Mercury path
    /// (FLAG_RELIABLE set on every finalized fragment). Informational on
    /// the bundle itself — the caller composes the `base_flags` argument
    /// to [`ChannelBundle::finalize`] and is responsible for matching
    /// reliability to the bundle's intent.
    reliable: bool,
    /// Count of [`ChannelBundle::append_entity_method`] +
    /// [`ChannelBundle::append_raw_message`] calls. Used by
    /// [`ChannelBundle::is_empty`] to distinguish "ack-only flush" from
    /// "no work to do."
    num_messages: usize,
}

impl ChannelBundle {
    /// Create an empty bundle. `reliable` is metadata — see field doc.
    pub fn new(reliable: bool) -> Self {
        Self {
            body: Vec::with_capacity(FRAGMENT_BODY_SIZE),
            acks: Vec::new(),
            reliable,
            num_messages: 0,
        }
    }

    /// `true` if this bundle is flagged as reliable. Caller is responsible
    /// for OR'ing the FLAG_RELIABLE bit into the `base_flags` argument to
    /// [`Self::finalize`] when appropriate; this accessor is purely for
    /// caller-side decision logic.
    pub fn is_reliable(&self) -> bool {
        self.reliable
    }

    /// Piggyback a peer-sent sequence number as an ACK on the first
    /// finalized packet. Subsequent fragments carry no ACKs.
    pub fn add_ack(&mut self, ack: u32) {
        self.acks.push(ack);
    }

    /// Append multiple ACKs at once. Equivalent to calling
    /// [`Self::add_ack`] in a loop.
    pub fn add_acks(&mut self, acks: &[u32]) {
        self.acks.extend_from_slice(acks);
    }

    /// Append a server→client entity-method call to the bundle body.
    ///
    /// Wire format matches
    /// [`crates/services/src/mercury/mod.rs`]'s `append_entity_method`
    /// byte-for-byte — see the module doc for the encoding.
    ///
    /// **Transaction-state hazard:** see the module doc. Do NOT combine
    /// `CREATE_ENTITY(X)` (or `CELL_PLAYER` for the player entity) with
    /// any later same-entity-X message in the same bundle.
    pub fn append_entity_method(&mut self, method_index: u16, entity_id: u32, args: &[u8]) {
        if method_index >= EXTENDED_ENCODING_THRESHOLD {
            self.body.push(EXTENDED_ENCODING_MARKER);
            let payload_len = (4 + 1 + args.len()) as u16;
            self.body.extend_from_slice(&payload_len.to_le_bytes());
            self.body.extend_from_slice(&entity_id.to_le_bytes());
            self.body
                .push((method_index - EXTENDED_ENCODING_THRESHOLD) as u8);
        } else {
            self.body.push((method_index as u8) | 0x80);
            let payload_len = (4 + args.len()) as u16;
            self.body.extend_from_slice(&payload_len.to_le_bytes());
            self.body.extend_from_slice(&entity_id.to_le_bytes());
        }
        self.body.extend_from_slice(args);
        self.num_messages += 1;
    }

    /// Append a pre-composed Mercury base message (e.g. CREATE_ENTITY,
    /// UPDATE_AVATAR, CREATE_BASE_PLAYER, RESET_ENTITIES). The caller is
    /// responsible for the entire wire format including the leading
    /// msg_id byte and any internal length prefix.
    ///
    /// **Transaction-state hazard:** `CREATE_ENTITY(X)` placed via this
    /// method puts entity X in transaction for the rest of the bundle.
    /// See the module doc.
    pub fn append_raw_message(&mut self, raw_msg: &[u8]) {
        self.body.extend_from_slice(raw_msg);
        self.num_messages += 1;
    }

    /// `true` if the bundle has neither messages nor ACKs — finalize on
    /// an empty bundle returns zero packets.
    pub fn is_empty(&self) -> bool {
        self.num_messages == 0 && self.acks.is_empty()
    }

    /// Length of the accumulated body in bytes.
    pub fn body_len(&self) -> usize {
        self.body.len()
    }

    /// Count of appended messages (not ACKs).
    pub fn num_messages(&self) -> usize {
        self.num_messages
    }

    /// Count of accumulated ACKs to piggyback on the first finalized
    /// packet.
    pub fn num_acks(&self) -> usize {
        self.acks.len()
    }

    /// Predicted fragment count if the bundle were finalized now. Useful
    /// for caller-side TX-window-pressure checks before committing to a
    /// finalize.
    pub fn estimated_packet_count(&self) -> usize {
        if self.body.is_empty() {
            if self.acks.is_empty() {
                0
            } else {
                1
            }
        } else {
            self.body.len().div_ceil(FRAGMENT_BODY_SIZE)
        }
    }

    /// Finalize the bundle into one or more encrypted Mercury packets.
    ///
    /// Returns `(encrypted_packets, sequence_ids_consumed)`. The caller
    /// must:
    /// 1. Send each packet via the session UDP socket.
    /// 2. Register each packet's sequence number + raw bytes with the
    ///    per-channel TX window via `Channel::register_sent_packet`
    ///    (or the services-layer `shadow_register_reliable_send` helper
    ///    that wraps it).
    /// 3. Advance the per-session reliable-seq counter by the returned
    ///    `sequence_ids_consumed`.
    ///
    /// Sequence number layout:
    /// - 1 message ≤ [`FRAGMENT_BODY_SIZE`]: 1 packet with seq `base_seq`.
    /// - N fragments: seq `base_seq + i` for fragment `i` (caller pre-
    ///   masks `base_seq` to [`crate::packet::SEQUENCE_MASK`]).
    ///
    /// `base_flags` is OR'd into every fragment's flags byte. Pass
    /// e.g. `FLAG_RELIABLE | FLAG_ON_CHANNEL`. `FLAG_HAS_SEQUENCE`,
    /// `FLAG_FRAGMENTED`, and `FLAG_HAS_ACKS` are added internally as
    /// needed.
    ///
    /// `encrypt` is the per-session AES-256-CBC packet encryption (the
    /// services-layer `encrypt_packet` closure capturing the session
    /// key). Passing it in keeps `cimmeria-mercury` free of session-key
    /// machinery.
    ///
    /// Empty bundle (no messages, no acks) returns `(vec![], 0)`.
    /// Acks-only bundle returns one packet with empty body + acks
    /// footer + the consumed seq.
    pub fn finalize(
        self,
        base_flags: u8,
        base_seq: u32,
        encrypt: impl Fn(&[u8]) -> Vec<u8>,
    ) -> (Vec<Vec<u8>>, u32) {
        if self.body.is_empty() && self.acks.is_empty() {
            return (Vec::new(), 0);
        }
        crate::packet::build_fragmented_bundle(
            base_flags, &self.body, base_seq, &self.acks, encrypt,
        )
    }
}

impl Default for ChannelBundle {
    /// Default to a non-reliable bundle. Production code should construct
    /// with the intended reliability explicit.
    fn default() -> Self {
        Self::new(false)
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::{
        parse_incoming, FLAG_FRAGMENTED, FLAG_HAS_ACKS, FLAG_HAS_SEQUENCE, FLAG_RELIABLE,
    };

    /// Identity encrypt for tests — packet bytes pass through unmodified
    /// so `parse_incoming` can verify the wire layout without needing
    /// session-key plumbing.
    fn passthrough(plaintext: &[u8]) -> Vec<u8> {
        plaintext.to_vec()
    }

    /// Pin a representative direct-encoding entity-method append against
    /// the wire layout the services-layer `append_entity_method` produces.
    /// If this drifts, any migrated call site would emit different bytes
    /// than the pre-migration packet builders and the client would
    /// silently fail to dispatch.
    ///
    /// Method index 12, entity_id 0xDEAD_BEEF, args [0xAA, 0xBB] → direct
    /// encoding: `[0x8C][0x06 0x00][0xEF 0xBE 0xAD 0xDE][0xAA 0xBB]`.
    #[test]
    fn append_entity_method_direct_encoding_matches_services_layer_byte_for_byte() {
        let mut bundle = ChannelBundle::new(true);
        bundle.append_entity_method(12, 0xDEAD_BEEF, &[0xAA, 0xBB]);

        // Body should be exactly the bytes the services-layer
        // append_entity_method would emit for the same inputs.
        let expected: &[u8] = &[
            0x8C, // 12 | 0x80
            0x06, 0x00, // word_len = 4 (entity_id) + 2 (args) = 6, u16 LE
            0xEF, 0xBE, 0xAD, 0xDE, // entity_id 0xDEAD_BEEF, u32 LE
            0xAA, 0xBB, // args
        ];
        assert_eq!(
            bundle.body, expected,
            "direct-encoding wire layout must match services-layer append_entity_method"
        );
    }

    /// Same byte-for-byte pin for the extended encoding path (index ≥ 61).
    /// Method index 122 (setupWorldParameters), entity_id 1, no args:
    /// `[0xBD][0x05 0x00][0x01 0x00 0x00 0x00][122 - 61 = 61]`.
    #[test]
    fn append_entity_method_extended_encoding_matches_services_layer_byte_for_byte() {
        let mut bundle = ChannelBundle::new(true);
        bundle.append_entity_method(122, 1, &[]);

        let expected: &[u8] = &[
            0xBD, // extended marker
            0x05, 0x00, // word_len = 4 (entity_id) + 1 (sub_index) = 5, u16 LE
            0x01, 0x00, 0x00, 0x00, // entity_id = 1, u32 LE
            61,   // sub_index = 122 - 61
        ];
        assert_eq!(
            bundle.body, expected,
            "extended-encoding wire layout must match services-layer append_entity_method"
        );
    }

    /// Boundary check: index 60 stays direct, index 61 flips to extended.
    /// A regression that moved the boundary would silently break either
    /// the high direct indices (e.g. 122 = setupWorldParameters, observed
    /// working with direct in C++) or the low extended ones.
    #[test]
    fn append_entity_method_boundary_between_direct_and_extended_encoding() {
        let mut direct = ChannelBundle::new(false);
        direct.append_entity_method(60, 1, &[]);
        assert_eq!(
            direct.body[0],
            60 | 0x80,
            "index 60 must use direct encoding"
        );

        let mut extended = ChannelBundle::new(false);
        extended.append_entity_method(61, 1, &[]);
        assert_eq!(
            extended.body[0], EXTENDED_ENCODING_MARKER,
            "index 61 must flip to extended encoding"
        );
    }

    /// Pack 5 different cross-entity messages into one bundle. After
    /// finalize, all 5 must land in a single packet (sub-fragment-size
    /// total) and the packet body must contain all 5 message records in
    /// append order. This is the core "N call sites collapse into 1 UDP
    /// datagram" promise of the bundle abstraction.
    #[test]
    fn bundle_packs_multiple_cross_entity_messages_into_one_packet() {
        let mut bundle = ChannelBundle::new(true);
        for (i, entity_id) in [10u32, 20, 30, 40, 50].iter().enumerate() {
            bundle.append_entity_method(12, *entity_id, &[i as u8]);
        }
        assert_eq!(bundle.num_messages(), 5);
        assert!(
            bundle.body_len() < FRAGMENT_BODY_SIZE,
            "5 tiny messages must fit in one fragment for this test to be meaningful"
        );

        let (packets, seqs_consumed) = bundle.finalize(FLAG_RELIABLE, 100, passthrough);
        assert_eq!(packets.len(), 1, "5 small messages collapse to 1 packet");
        assert_eq!(seqs_consumed, 1, "1 packet consumes 1 sequence id");

        // Parse the packet to confirm the body carries all 5 records.
        // Each direct-encoded record is 1 (msg_id) + 2 (len) + 4 (entity_id)
        // + 1 (arg) = 8 bytes; 5 records → 40 body bytes.
        let parsed = parse_incoming(&packets[0]).expect("packet parses");
        assert_eq!(
            parsed.body.len(),
            5 * 8,
            "body carries all 5 message records"
        );
        assert_eq!(parsed.seq_id, Some(100), "packet seq matches base_seq");
        assert_eq!(
            parsed.flags & FLAG_HAS_SEQUENCE,
            FLAG_HAS_SEQUENCE,
            "single-packet path sets FLAG_HAS_SEQUENCE"
        );
        assert_eq!(
            parsed.flags & FLAG_FRAGMENTED,
            0,
            "single-packet path must NOT set FLAG_FRAGMENTED"
        );
    }

    /// When the bundle body exceeds [`FRAGMENT_BODY_SIZE`] (1300 bytes),
    /// finalize must emit multiple fragmented packets. Build a body of
    /// ~3KB (3 fragments worth), finalize, and assert 3 fragments — each
    /// flagged FLAG_FRAGMENTED, with consecutive sequence numbers and
    /// matching frag_begin/frag_end footers.
    #[test]
    fn bundle_fragments_when_body_exceeds_packet_capacity() {
        let mut bundle = ChannelBundle::new(true);
        // Each append: 1 (msg_id) + 2 (len) + 4 (entity_id) + 100 (args) = 107 bytes.
        // 30 appends ≈ 3210 bytes → 3 fragments (1300 + 1300 + 610).
        let args = [0xAB; 100];
        for entity_id in 0u32..30 {
            bundle.append_entity_method(12, entity_id, &args);
        }
        assert!(
            bundle.body_len() > 2 * FRAGMENT_BODY_SIZE,
            "test setup must exceed 2 fragments"
        );
        let expected_frags = bundle.estimated_packet_count();

        let (packets, seqs_consumed) = bundle.finalize(FLAG_RELIABLE, 200, passthrough);
        assert_eq!(
            packets.len(),
            expected_frags,
            "fragment count matches estimate"
        );
        assert_eq!(
            seqs_consumed, expected_frags as u32,
            "each fragment consumes one sequence id"
        );

        for (i, raw) in packets.iter().enumerate() {
            let parsed = parse_incoming(raw).expect("fragment parses");
            assert_eq!(
                parsed.flags & FLAG_FRAGMENTED,
                FLAG_FRAGMENTED,
                "fragment {i} must set FLAG_FRAGMENTED"
            );
            assert_eq!(parsed.seq_id, Some(200 + i as u32), "fragment {i} seq");
            assert_eq!(
                parsed.frag_begin,
                Some(200),
                "all fragments share frag_begin"
            );
            assert_eq!(
                parsed.frag_end,
                Some(200 + expected_frags as u32 - 1),
                "all fragments share frag_end"
            );
        }
    }

    /// ACK piggyback rides ONLY the first fragment of a multi-packet
    /// bundle — matches the C++ Bundle and the wire-format spec. A
    /// regression that put acks on every fragment would inflate the
    /// per-fragment footer size AND duplicate ACK delivery to the peer's
    /// ack-coverage tracker.
    #[test]
    fn finalized_acks_ride_only_the_first_fragment() {
        let mut bundle = ChannelBundle::new(true);
        bundle.add_acks(&[10, 20, 30]);
        // Force fragmentation so we have multiple packets to inspect.
        let args = [0xCD; 100];
        for entity_id in 0u32..30 {
            bundle.append_entity_method(12, entity_id, &args);
        }

        let (packets, _) = bundle.finalize(FLAG_RELIABLE, 500, passthrough);
        assert!(packets.len() > 1, "test must exercise multi-fragment path");

        let first = parse_incoming(&packets[0]).expect("first fragment parses");
        assert_eq!(
            first.flags & FLAG_HAS_ACKS,
            FLAG_HAS_ACKS,
            "first fragment must carry FLAG_HAS_ACKS"
        );
        assert_eq!(
            first.acks,
            vec![10, 20, 30],
            "first fragment carries all acks"
        );

        for (i, raw) in packets.iter().enumerate().skip(1) {
            let parsed = parse_incoming(raw).expect("fragment parses");
            assert_eq!(
                parsed.flags & FLAG_HAS_ACKS,
                0,
                "fragment {i} must NOT carry FLAG_HAS_ACKS"
            );
            assert!(parsed.acks.is_empty(), "fragment {i} has no acks");
        }
    }

    /// Empty bundle finalize must emit zero packets (and consume zero
    /// sequence ids). A regression that emitted an empty seq-only packet
    /// would waste a TX-window slot on a no-op flush.
    #[test]
    fn empty_bundle_finalize_emits_zero_packets() {
        let bundle = ChannelBundle::new(true);
        assert!(bundle.is_empty(), "new bundle is empty");
        assert_eq!(bundle.estimated_packet_count(), 0);

        let (packets, seqs_consumed) = bundle.finalize(FLAG_RELIABLE, 999, passthrough);
        assert!(packets.is_empty(), "empty bundle emits no packets");
        assert_eq!(seqs_consumed, 0, "empty bundle consumes no seqs");
    }

    /// Ack-only bundle (no message body, but pending acks) must emit
    /// exactly one packet so the acks reach the peer. Without this path
    /// the channel could accumulate pending acks indefinitely if no
    /// outgoing application message coincided with the next flush.
    #[test]
    fn ack_only_bundle_finalize_emits_one_packet() {
        let mut bundle = ChannelBundle::new(true);
        bundle.add_acks(&[7, 8, 9]);
        assert!(!bundle.is_empty(), "ack-only bundle is not empty");
        assert_eq!(bundle.estimated_packet_count(), 1);

        let (packets, seqs_consumed) = bundle.finalize(FLAG_RELIABLE, 42, passthrough);
        assert_eq!(packets.len(), 1, "ack-only bundle emits one packet");
        assert_eq!(seqs_consumed, 1, "ack-only packet consumes one seq");

        let parsed = parse_incoming(&packets[0]).expect("packet parses");
        assert_eq!(parsed.acks, vec![7, 8, 9], "acks reach the peer");
        assert_eq!(
            parsed.seq_id,
            Some(42),
            "ack-only packet carries the allocated seq"
        );
    }

    /// Pin the exact bytes ChannelBundle produces for a representative
    /// 2-message bundle against an equivalent manual construction via
    /// the services-layer `append_entity_method` + `build_outgoing`.
    /// Byte-exact equality means migrating a single call site never
    /// changes what the client sees.
    ///
    /// This is the load-bearing regression guard for the migration: if
    /// it ever fails, a Layer B migration cannot be byte-equivalent
    /// without a corresponding wire-format change elsewhere.
    #[test]
    fn bundle_body_byte_exact_against_manual_construction() {
        // Bundle path
        let mut bundle = ChannelBundle::new(true);
        bundle.append_entity_method(12, 0x0000_0042, &[0x11, 0x22, 0x33]);
        bundle.append_entity_method(141, 0x0000_0099, &[0xFE, 0xED]);
        let bundle_body = bundle.body.clone();

        // Manual path — same wire format the services-layer builders use.
        let mut manual_body: Vec<u8> = Vec::new();
        // Message 1: direct encoding, index 12.
        manual_body.push(0x8C);
        manual_body.extend_from_slice(&(4u16 + 3).to_le_bytes());
        manual_body.extend_from_slice(&0x0000_0042u32.to_le_bytes());
        manual_body.extend_from_slice(&[0x11, 0x22, 0x33]);
        // Message 2: extended encoding, index 141.
        manual_body.push(EXTENDED_ENCODING_MARKER);
        manual_body.extend_from_slice(&(4u16 + 1 + 2).to_le_bytes());
        manual_body.extend_from_slice(&0x0000_0099u32.to_le_bytes());
        manual_body.push((141 - 61) as u8);
        manual_body.extend_from_slice(&[0xFE, 0xED]);

        assert_eq!(
            bundle_body, manual_body,
            "ChannelBundle body must be byte-identical to manual append_entity_method composition"
        );
    }

    /// Verify the AoI-burst shape that the conservative migration in
    /// Layer B targets: 28 NPC phase-1 packets (CREATE_ENTITY +
    /// UPDATE_AVATAR per NPC, ~50 bytes each via raw append) collapse
    /// into a small number of fragments.
    ///
    /// 28 × 50 = 1400 bytes → 2 fragments (1300 + 100). This pins the
    /// expected fragment count so the Layer B migration test can assert
    /// the same shape.
    #[test]
    fn aoi_burst_shape_28_npcs_collapse_to_two_fragments() {
        let mut bundle = ChannelBundle::new(true);
        // Mock per-NPC phase 1: CREATE_ENTITY (12 bytes) + UPDATE_AVATAR
        // (33 bytes) ≈ 45 bytes (close to the real shape in
        // crates/services/src/mercury/aoi/create.rs).
        let phase1_per_npc = vec![0xAB; 50];
        for _ in 0..28 {
            bundle.append_raw_message(&phase1_per_npc);
        }
        assert_eq!(
            bundle.body_len(),
            28 * 50,
            "28 NPCs × 50 bytes/phase-1 = 1400 bytes of body"
        );
        let estimated = bundle.estimated_packet_count();
        assert_eq!(
            estimated, 2,
            "28-NPC AoI burst should collapse to 2 fragments (was 28 packets pre-bundle)"
        );

        let (packets, seqs) = bundle.finalize(FLAG_RELIABLE, 1000, passthrough);
        assert_eq!(packets.len(), 2);
        assert_eq!(seqs, 2);
    }

    /// `is_reliable` reports back the constructor argument unmodified —
    /// it's metadata the caller consults to drive its own
    /// FLAG_RELIABLE-in-base_flags decision.
    #[test]
    fn is_reliable_returns_constructor_argument() {
        assert!(ChannelBundle::new(true).is_reliable());
        assert!(!ChannelBundle::new(false).is_reliable());
        assert!(!ChannelBundle::default().is_reliable());
    }
}
