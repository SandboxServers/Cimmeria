//! Test model of the SGW client's bundle iterator, for asserting that a
//! server-built bundle survives the client's message loop.
//!
//! Ports the framing rules of `Bundle::iterator::unpack`
//! (`ghidra://SGW.exe@0x01579830`) and `Bundle::iterator::data`
//! (`0x01579a50`), driven by `Nub::processOrderedPacket` (`0x0157c820`):
//!
//! - a message header (id byte, plus the length field of a variable-length
//!   message) must lie inside ONE packet; a header that runs past the end of
//!   its packet aborts the bundle (`Error unpacking header length`, then
//!   "Discarding bundle due to corrupted header"), so every later message is
//!   lost;
//! - a message BODY may continue into following packets;
//! - fragments are taken in sequence order.
//!
//! Available under `cfg(test)` and the `test-support` feature.

use crate::packet::{parse_incoming, server_message_framing, ServerMessageFraming};

/// Outcome of running the client model over a bundle's packets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientUnpack {
    /// Messages the client dispatches, in order: `(msg_id, payload)`.
    pub messages: Vec<(u8, Vec<u8>)>,
    /// Why the loop stopped early, `None` when the whole bundle was consumed.
    pub abort: Option<ClientAbort>,
}

/// Reasons the client's message loop stops before the end of a bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientAbort {
    /// The header of the message starting at `offset` in packet `packet`
    /// does not fit in that packet (`Error unpacking header length`).
    HeaderStraddlesPackets { packet: usize, offset: usize },
    /// A message id outside the client's interface table.
    UnknownMessage { msg_id: u8 },
    /// A message body longer than the rest of the bundle.
    TruncatedBody { msg_id: u8 },
}

/// Run the client model over packet bodies in order (one entry per fragment).
pub fn unpack_packet_bodies(bodies: &[&[u8]]) -> ClientUnpack {
    let mut messages = Vec::new();
    let mut pkt = 0usize;
    let mut off = 0usize;
    loop {
        // Advance past exhausted packets; done when none is left.
        while pkt < bodies.len() && off >= bodies[pkt].len() {
            pkt += 1;
            off = 0;
        }
        if pkt >= bodies.len() {
            return ClientUnpack {
                messages,
                abort: None,
            };
        }
        let cur = bodies[pkt];
        let msg_id = cur[off];
        let Some(framing) = server_message_framing(msg_id) else {
            return ClientUnpack {
                messages,
                abort: Some(ClientAbort::UnknownMessage { msg_id }),
            };
        };
        let header_len = match framing {
            ServerMessageFraming::Constant(_) => 1,
            ServerMessageFraming::Word => 3,
        };
        if off + header_len > cur.len() {
            return ClientUnpack {
                messages,
                abort: Some(ClientAbort::HeaderStraddlesPackets {
                    packet: pkt,
                    offset: off,
                }),
            };
        }
        let len = match framing {
            ServerMessageFraming::Constant(n) => n,
            ServerMessageFraming::Word => u16::from_le_bytes([cur[off + 1], cur[off + 2]]) as usize,
        };
        off += header_len;
        // Copy the body, continuing into following packets.
        let mut payload = Vec::with_capacity(len);
        while payload.len() < len {
            while pkt < bodies.len() && off >= bodies[pkt].len() {
                pkt += 1;
                off = 0;
            }
            if pkt >= bodies.len() {
                return ClientUnpack {
                    messages,
                    abort: Some(ClientAbort::TruncatedBody { msg_id }),
                };
            }
            let take = (len - payload.len()).min(bodies[pkt].len() - off);
            payload.extend_from_slice(&bodies[pkt][off..off + take]);
            off += take;
        }
        messages.push((msg_id, payload));
    }
}

/// Run the client model over decrypted wire packets of one bundle. Packets
/// are parsed, put in sequence order, and their bodies fed to
/// [`unpack_packet_bodies`]. Panics on an unparsable packet (test helper).
pub fn unpack_plaintext_packets(packets: &[Vec<u8>]) -> ClientUnpack {
    let mut parsed: Vec<_> = packets
        .iter()
        .map(|p| parse_incoming(p).expect("packet must parse"))
        .collect();
    parsed.sort_by_key(|p| p.seq_id.unwrap_or(0));
    let bodies: Vec<&[u8]> = parsed.iter().map(|p| p.body.as_ref()).collect();
    unpack_packet_bodies(&bodies)
}

/// Run the client model over a whole reliable stream of decrypted packets:
/// packets are put in sequence order (the client's reorder window), a
/// non-fragment packet is one bundle, and consecutive fragments sharing a
/// `frag_end` form one bundle that completes at its last fragment. Returns
/// one [`ClientUnpack`] per bundle, in delivery order. Panics on an
/// unparsable packet or an incomplete fragment group (test helper).
pub fn unpack_reliable_stream(packets: &[Vec<u8>]) -> Vec<ClientUnpack> {
    let mut parsed: Vec<_> = packets
        .iter()
        .map(|p| parse_incoming(p).expect("packet must parse"))
        .collect();
    parsed.sort_by_key(|p| p.seq_id.unwrap_or(0));

    let mut out = Vec::new();
    let mut group: Vec<&[u8]> = Vec::new();
    let mut group_end: Option<u32> = None;
    for p in &parsed {
        match (p.frag_begin, p.frag_end) {
            (Some(_), Some(end)) => {
                if let Some(open_end) = group_end {
                    assert_eq!(
                        open_end, end,
                        "second fragment group opened before the first completed"
                    );
                }
                group_end = Some(end);
                group.push(p.body.as_ref());
                if p.seq_id == Some(end) {
                    out.push(unpack_packet_bodies(&group));
                    group.clear();
                    group_end = None;
                }
            }
            _ => out.push(unpack_packet_bodies(&[p.body.as_ref()])),
        }
    }
    assert!(group.is_empty(), "fragment group never completed");
    out
}
