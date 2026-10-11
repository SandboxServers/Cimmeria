//! The `unpack` request-misparse and fault detector: `client.mercury.request_misparse`
//! and `client.mercury.unpack_fault`, read by `unpack_detour` in
//! [`super::mercury_recv`] around the original call (game thread). The rule
//! and its evidence are in
//! `docs/reverse-engineering/findings/client-mercury-receive-path.md`.

use super::mercury_recv::{read_element, read_iter};
use crate::hooks::emit::emit;
use crate::hooks::entity_trace::map::{LiveMem, Mem};
use crate::hooks::mercury_recv::{after_fault, packet, wire, Fields};

/// The iterator fields `unpack` decides with, read before the call.
///
/// The client's `Bundle::iterator` copy constructor (`0x01578e90`) never
/// initializes `+0x14`, the next-request offset, so it holds stack residue.
/// When it equals the cursor, `unpack` treats an ordinary message as a
/// request: it reads a 4-byte reply id and a 2-byte next-request offset
/// out of the message body and misparses everything after it.
#[derive(Clone, Copy)]
pub(super) struct IterEntry {
    packet: u32,
    packet_len: u16,
    cursor: u16,
    next_request: u16,
    /// The packet's flags byte (payload byte 0); `0x01` = has requests.
    packet_flags: u8,
}

impl IterEntry {
    /// Fault-safe reads (`ReadProcessMemory`): a bad pointer is a `None`,
    /// never an access violation inside the client.
    pub(super) fn read(it: u32) -> Option<Self> {
        let raw = LiveMem.bytes_at(it, 0x16)?;
        let u16_at = |o: usize| u16::from_le_bytes([raw[o], raw[o + 1]]);
        let packet = u32::from_le_bytes(raw[0..4].try_into().ok()?);
        let packet_flags = LiveMem.bytes_at(packet.wrapping_add(packet::DATA), 1)?[0];
        Some(Self {
            packet,
            packet_len: u16_at(4),
            cursor: u16_at(6),
            next_request: u16_at(0x14),
            packet_flags,
        })
    }

    /// Takes the request path although the packet carries no requests.
    fn is_request_misparse(&self) -> bool {
        self.next_request == self.cursor && self.packet_flags & 0x01 == 0
    }

    /// Report a request misparse and any unpack fault, with the client's
    /// own post-call numbers and the message's raw bytes.
    pub(super) fn report(&self, it: u32, element: u32) {
        let post = read_iter(it);
        let failed = post.map(|p| p.unpacked().flag == 0x20).unwrap_or(false);
        let misparse = self.is_request_misparse();
        if !misparse && !failed {
            return;
        }
        let left = usize::from(self.packet_len).saturating_sub(usize::from(self.cursor));
        let bytes = LiveMem
            .bytes_at(
                self.packet
                    .wrapping_add(packet::DATA)
                    .wrapping_add(u32::from(self.cursor)),
                left.min(24),
            )
            .map(|b| b.iter().map(|x| format!("{x:02x}")).collect::<String>());
        let elem = read_element(element);
        let mut fields: Fields = vec![
            ("cursor", serde_json::json!(self.cursor)),
            ("packet_len", serde_json::json!(self.packet_len)),
            ("next_request_offset", serde_json::json!(self.next_request)),
            ("packet_flags", serde_json::json!(self.packet_flags)),
            (
                "request_path",
                serde_json::json!(self.next_request == self.cursor),
            ),
            ("request_misparse", serde_json::json!(misparse)),
            ("failed", serde_json::json!(failed)),
            ("bytes", serde_json::json!(bytes)),
            ("elem_style", serde_json::json!(elem.map(|e| e.style))),
            ("elem_param", serde_json::json!(elem.map(|e| e.param))),
        ];
        if let Some(p) = post {
            let u = p.unpacked();
            fields.push(("msg_id", serde_json::json!(u.msg_id)));
            fields.push(("decoded_len", serde_json::json!(u.decoded_len)));
        }
        // Join keys: the packet's sequence number and, if the filter
        // recorded it, the encrypted datagram's fingerprint.
        let seq = LiveMem.u32_at(self.packet.wrapping_add(packet::SEQ));
        fields.extend(wire::join_fields(self.packet, seq));
        // TG-CLI-02: the BigWorld messages this fault causes are raised on
        // this (game) thread before the next bundle; tag them.
        if failed {
            after_fault::mark_fault(seq);
        }
        let target = if misparse {
            "client.mercury.request_misparse"
        } else {
            "client.mercury.unpack_fault"
        };
        emit(target, "warn", fields);
    }
}
