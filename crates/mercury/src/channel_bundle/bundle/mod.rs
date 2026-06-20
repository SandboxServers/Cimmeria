//! The [`ChannelBundle`] accumulator type and its finalize-to-packets path.
//!
//! See the [crate-level module doc][super] for the "one bundle == one client
//! frame" rule and the safe/unsafe combinations. Tests for this module live
//! in the [`tests`] child so they can reach `ChannelBundle`'s private `body`
//! buffer for byte-exact wire assertions.

use crate::packet::FRAGMENT_BODY_SIZE;

use super::idbase::EXTENDED_ENCODING_MARKER;

#[cfg(test)]
mod tests;

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
    /// `idbase` is the per-entity-type sub-slot threshold for the target
    /// entity — see [`idbase_from_exposed_method_count`]. For methods
    /// targeting SGWPlayer pass [`IDBASE_SGW_PLAYER`] (`61`). For entities
    /// with ≤62 exposed methods, pass `62`. The threshold is **not** a
    /// global constant; encoding with the wrong idbase produces a wire
    /// byte the client decodes as a different method.
    ///
    /// Wire format matches
    /// [`crates/services/src/mercury/mod.rs`]'s `append_entity_method`
    /// byte-for-byte — see the module doc for the encoding.
    ///
    /// **Transaction-state hazard:** see the module doc. Do NOT combine
    /// `CREATE_ENTITY(X)` (or `CELL_PLAYER` for the player entity) with
    /// any later same-entity-X message in the same bundle.
    ///
    /// **Field-width contract:** panics on inputs the Mercury wire format
    /// cannot represent:
    /// - `method_index >= idbase + 256` (extended sub-index byte
    ///   overflow — for SGWPlayer's `idbase = 61` that's `61 + 255 = 316`)
    /// - `args.len()` such that the per-message length field would exceed
    ///   `u16::MAX` (~65 KB body)
    ///
    /// A panic is preferable to a silent narrowing cast: the latter would
    /// emit a packet with a corrupt method/length field that the client
    /// parses incorrectly, producing a hard-to-diagnose downstream bug.
    ///
    /// [`idbase_from_exposed_method_count`]: super::idbase_from_exposed_method_count
    /// [`IDBASE_SGW_PLAYER`]: super::IDBASE_SGW_PLAYER
    pub fn append_entity_method(
        &mut self,
        method_index: u16,
        idbase: u8,
        entity_id: u32,
        args: &[u8],
    ) {
        let threshold = u16::from(idbase);
        if method_index >= threshold {
            let sub_index = u8::try_from(method_index - threshold).expect(
                "method_index exceeds Mercury extended-encoding range (idbase + 255 = max)",
            );
            let payload_len = u16::try_from(4 + 1 + args.len())
                .expect("entity-method payload exceeds Mercury u16 length field (~65 KB max)");
            self.body.push(EXTENDED_ENCODING_MARKER);
            self.body.extend_from_slice(&payload_len.to_le_bytes());
            self.body.extend_from_slice(&entity_id.to_le_bytes());
            self.body.push(sub_index);
        } else {
            let payload_len = u16::try_from(4 + args.len())
                .expect("entity-method payload exceeds Mercury u16 length field (~65 KB max)");
            // Safe: method_index < idbase <= 62 < u8::MAX, so `as u8`
            // cannot truncate. The high bit is then set via `| 0x80` as
            // the direct-encoding marker.
            self.body.push((method_index as u8) | 0x80);
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
    ///
    /// **Debug-only well-formedness check:** an empty `raw_msg` is a
    /// caller bug — the bundle would silently swallow the append (no msg
    /// id, no length, no bytes). In debug builds a `debug_assert!` fires;
    /// release builds tolerate the no-op append and bump `num_messages`
    /// as if a real message had been written, which the caller can detect
    /// downstream via [`Self::body_len`]. This avoids paying the branch
    /// cost in release builds where the caller is trusted code paths
    /// (the body composers in `services::mercury`).
    pub fn append_raw_message(&mut self, raw_msg: &[u8]) {
        debug_assert!(
            !raw_msg.is_empty(),
            "append_raw_message: empty raw_msg silently swallowed — caller bug"
        );
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
