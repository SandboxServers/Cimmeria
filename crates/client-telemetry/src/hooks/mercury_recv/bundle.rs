//! One bundle and the message loop over it.
//!
//! `Nub::processOrderedPacket` (`0x0157c820`, game thread) walks a packet
//! chain with a `Bundle::iterator`; a message is unpacked by
//! `Bundle::iterator::unpack` (`0x01579830`). The detours record the chain
//! when the loop starts ([`BundleTrace::new`]) and every message the loop
//! unpacks ([`BundleTrace::on_unpack`]); when the loop returns,
//! [`BundleTrace::finish`] names how it ended.
//!
//! Offsets here are **payload offsets**: the bytes of every packet after its
//! flags byte, concatenated, which is the stream a message boundary is
//! measured in. The rules are in the findings doc, "The bundle message
//! loop": a message header must lie wholly inside one packet, only a body may
//! straddle, and the loop stops (dropping every later message) on an unknown
//! message id or a header/body that does not fit.

use serde_json::json;

pub(crate) use super::iterator::{Element, IterState, Unpacked};
use super::{packet, Fields, MAX_CHAIN};
use crate::hooks::entity_trace::map::Mem;

/// Longest per-packet list put in an event.
const MAX_LIST: usize = 32;

/// `processOrderedPacket` results.
pub(crate) const RESULT_UNKNOWN_MESSAGE_ID: i32 = -5; // 0xfffffffb
pub(crate) const RESULT_CORRUPTED: i32 = -4; // 0xfffffffc

/// One packet of the chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PacketView {
    pub ptr: u32,
    /// Data length, flags byte included (payload is `len - 1`).
    pub len: usize,
    pub seq: u32,
}

impl PacketView {
    pub(crate) fn payload(&self) -> usize {
        self.len.saturating_sub(1)
    }
}

/// Read the chain starting at `head`, following `Packet+8`.
pub(crate) fn read_chain(mem: &dyn Mem, head: u32) -> Option<Vec<PacketView>> {
    let mut chain = Vec::new();
    let mut node = head;
    while node != 0 && chain.len() < MAX_CHAIN {
        chain.push(PacketView {
            ptr: node,
            len: mem.u32_at(node.wrapping_add(packet::LEN))? as usize,
            seq: mem.u32_at(node.wrapping_add(packet::SEQ))?,
        });
        node = mem.u32_at(node.wrapping_add(packet::NEXT))?;
    }
    (!chain.is_empty()).then_some(chain)
}

/// One message the loop unpacked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Message {
    /// Payload offset of its header.
    pub start: usize,
    pub msg_id: u8,
    pub header: usize,
    pub len: usize,
    /// Its body crosses a packet boundary.
    pub straddles: bool,
}

impl Message {
    pub(crate) fn end(&self) -> usize {
        self.start + self.header + self.len
    }
}

/// Why `unpack` refused a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnpackFault {
    /// `packetLen < cursor + header`: the header does not lie inside the
    /// packet. **The fragment-boundary rule.**
    HeaderDoesNotFit {
        header: usize,
        packet_len: u16,
        cursor: u16,
    },
    /// `expandLength` failed (an unhandled length width or overflow).
    LengthExpandFailed,
    /// The body straddles but no next packet exists.
    BodyRunsOut { body_end: usize, packet_len: u16 },
}

impl UnpackFault {
    pub(crate) fn reason(&self) -> &'static str {
        match self {
            UnpackFault::HeaderDoesNotFit { .. } => "header_does_not_fit_packet",
            UnpackFault::LengthExpandFailed => "length_expand_failed",
            UnpackFault::BodyRunsOut { .. } => "body_runs_out_of_packets",
        }
    }
}

/// How the loop ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Exit {
    /// The iterator reached the end of the chain.
    Clean,
    /// No handler for the next message id.
    UnknownMessageId,
    /// `unpack` refused a message, or `data()` ran out of packets.
    Corrupted,
    /// Some other non-zero result.
    Other,
}

impl Exit {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Exit::Clean => "clean_end",
            Exit::UnknownMessageId => "unknown_message_id",
            Exit::Corrupted => "corrupted_header",
            Exit::Other => "other",
        }
    }
}

/// Map `processOrderedPacket`'s result to how the loop ended.
pub(crate) fn exit_of(result: i32) -> Exit {
    match result {
        0 => Exit::Clean,
        RESULT_UNKNOWN_MESSAGE_ID => Exit::UnknownMessageId,
        RESULT_CORRUPTED => Exit::Corrupted,
        _ => Exit::Other,
    }
}

/// Everything recorded about one bundle's message loop.
#[derive(Debug, Clone)]
pub(crate) struct BundleTrace {
    chain: Vec<PacketView>,
    /// Payload bytes before each packet.
    prefix: Vec<usize>,
    /// Total payload bytes.
    pub total: usize,
    pub messages: u32,
    pub straddled: u32,
    pub first: Option<Message>,
    pub last: Option<Message>,
    /// Payload offset the last unpacked message ended at (0 if none).
    pub consumed: usize,
    pub fault: Option<(usize, UnpackFault)>,
}

impl BundleTrace {
    pub(crate) fn new(chain: Vec<PacketView>) -> Self {
        let mut prefix = Vec::with_capacity(chain.len());
        let mut total = 0usize;
        for p in &chain {
            prefix.push(total);
            total += p.payload();
        }
        Self {
            chain,
            prefix,
            total,
            messages: 0,
            straddled: 0,
            first: None,
            last: None,
            consumed: 0,
            fault: None,
        }
    }

    pub(crate) fn packets(&self) -> usize {
        self.chain.len()
    }

    /// `single_packet` or `assembled_fragments`.
    pub(crate) fn source(&self) -> &'static str {
        if self.chain.len() > 1 {
            "assembled_fragments"
        } else {
            "single_packet"
        }
    }

    /// Payload offsets where each packet after the first begins: the
    /// fragment boundaries in the message stream.
    pub(crate) fn boundaries(&self) -> Vec<usize> {
        self.prefix.iter().skip(1).copied().collect()
    }

    /// Chain index and payload offset of `cursor` in packet `ptr`.
    pub(crate) fn locate(&self, ptr: u32, cursor: u16) -> Option<(usize, usize)> {
        let idx = self.chain.iter().position(|p| p.ptr == ptr)?;
        Some((
            idx,
            self.prefix[idx] + usize::from(cursor).saturating_sub(1),
        ))
    }

    /// Where (packet pointer, cursor) the payload offset `abs` falls, for
    /// reading the id byte of the message that has no handler.
    pub(crate) fn position_of(&self, abs: usize) -> Option<(u32, u32)> {
        let idx = self.prefix.iter().rposition(|&start| start <= abs)?;
        let within = abs - self.prefix[idx];
        (within < self.chain[idx].payload()).then(|| {
            (
                self.chain[idx].ptr,
                packet::DATA + 1 + u32::try_from(within).unwrap_or(0),
            )
        })
    }

    /// Record one `unpack` call. `pre` is the iterator before the call,
    /// `post` after, `element` the message's interface element.
    pub(crate) fn on_unpack(&mut self, pre: IterState, post: Unpacked, element: Option<Element>) {
        let Some((_, start)) = self.locate(pre.packet, pre.cursor) else {
            return;
        };
        if post.flag == 0x20 {
            let fault = if post.decoded_len == u32::MAX {
                UnpackFault::LengthExpandFailed
            } else if let Some(h) = element
                .and_then(|e| e.header_len())
                .filter(|&h| usize::from(pre.packet_len) < usize::from(pre.cursor) + h)
            {
                UnpackFault::HeaderDoesNotFit {
                    header: h,
                    packet_len: pre.packet_len,
                    cursor: pre.cursor,
                }
            } else {
                UnpackFault::BodyRunsOut {
                    body_end: usize::from(post.body_offset) + post.len as usize,
                    packet_len: pre.packet_len,
                }
            };
            self.fault = Some((start, fault));
            return;
        }
        let header = usize::from(post.body_offset).saturating_sub(usize::from(pre.cursor));
        let len = post.len as usize;
        let straddles = usize::from(post.body_offset) + len > usize::from(pre.packet_len);
        let m = Message {
            start,
            msg_id: post.msg_id,
            header,
            len,
            straddles,
        };
        self.messages += 1;
        self.straddled += u32::from(straddles);
        self.consumed = m.end();
        self.first.get_or_insert(m);
        self.last = Some(m);
    }

    /// Bytes of the stream no message covers.
    pub(crate) fn unconsumed(&self) -> usize {
        self.total.saturating_sub(self.consumed)
    }

    /// The `client.mercury.bundle` start fields: what arrived.
    pub(crate) fn start_fields(&self) -> Fields {
        let mut f: Fields = vec![
            ("phase", json!("start")),
            ("source", json!(self.source())),
            ("packets", json!(self.packets())),
            ("total_bytes", json!(self.total)),
        ];
        self.push_seqs(&mut f);
        if self.chain.len() > 1 {
            f.push(("boundaries", json!(self.boundary_list())));
        }
        f
    }

    fn push_seqs(&self, f: &mut Fields) {
        if let (Some(a), Some(b)) = (self.chain.first(), self.chain.last()) {
            f.push(("seq_first", json!(a.seq)));
            f.push(("seq_last", json!(b.seq)));
        }
    }

    fn boundary_list(&self) -> Vec<usize> {
        self.boundaries().into_iter().take(MAX_LIST).collect()
    }

    /// The `client.mercury.bundle` end fields. `dispatched` is the nub's
    /// message counter delta, `next_id` the id byte at the point the loop
    /// stopped (read by the detour for an unknown message id).
    pub(crate) fn end_fields(
        &self,
        result: i32,
        dispatched: Option<u32>,
        aborted: Option<u32>,
        next_id: Option<u8>,
    ) -> Fields {
        let exit = exit_of(result);
        let mut f: Fields = vec![
            ("phase", json!("end")),
            ("source", json!(self.source())),
            ("packets", json!(self.packets())),
            ("total_bytes", json!(self.total)),
            ("messages", json!(self.messages)),
            ("consumed_bytes", json!(self.consumed)),
            ("unconsumed_bytes", json!(self.unconsumed())),
            ("straddled_messages", json!(self.straddled)),
            ("exit", json!(exit.name())),
            ("result", json!(format!("0x{:08x}", result as u32))),
        ];
        self.push_seqs(&mut f);
        if let Some(d) = dispatched {
            f.push(("dispatched", json!(d)));
        }
        // The client's own "loop stopped with messages left" counter
        // (`Nub+0x118`), an independent check on `exit`.
        if let Some(a) = aborted {
            f.push(("nub_aborted_delta", json!(a)));
        }
        if let Some(m) = self.first {
            f.push(("first_msg_id", json!(m.msg_id)));
        }
        if let Some(m) = self.last {
            f.push(("last_msg_id", json!(m.msg_id)));
            f.push(("last_msg_offset", json!(m.start)));
            f.push(("last_msg_len", json!(m.len)));
        }
        if !self.is_happy(result) || self.chain.len() > 1 {
            f.push(("boundaries", json!(self.boundary_list())));
        }
        if let Some((abs, fault)) = &self.fault {
            f.push(("fault", json!(fault.reason())));
            f.push(("abort_offset", json!(abs)));
            if let Some((idx, _)) = self.locate_offset(*abs) {
                f.push(("abort_packet_index", json!(idx)));
                f.push(("abort_packet_seq", json!(self.chain[idx].seq)));
            }
            match fault {
                UnpackFault::HeaderDoesNotFit {
                    header,
                    packet_len,
                    cursor,
                } => {
                    f.push(("header_len", json!(header)));
                    f.push(("packet_len", json!(packet_len)));
                    f.push(("abort_cursor", json!(cursor)));
                    // Bytes of the header that fit before the packet ends.
                    let fits = usize::from(*packet_len).saturating_sub(usize::from(*cursor));
                    f.push(("header_bytes_in_packet", json!(fits)));
                }
                UnpackFault::BodyRunsOut {
                    body_end,
                    packet_len,
                } => {
                    f.push(("body_end", json!(body_end)));
                    f.push(("packet_len", json!(packet_len)));
                }
                UnpackFault::LengthExpandFailed => {}
            }
        } else if exit != Exit::Clean {
            f.push(("abort_offset", json!(self.consumed)));
            if let Some(id) = next_id {
                f.push(("abort_msg_id", json!(id)));
            }
            if let Some((idx, _)) = self.locate_offset(self.consumed) {
                f.push(("abort_packet_index", json!(idx)));
                f.push(("abort_packet_seq", json!(self.chain[idx].seq)));
            }
        }
        f
    }

    /// Chain index (and the offset within it) holding payload offset `abs`.
    fn locate_offset(&self, abs: usize) -> Option<(usize, usize)> {
        let idx = self.prefix.iter().rposition(|&start| start <= abs)?;
        Some((idx, abs - self.prefix[idx]))
    }

    /// The loop ran to the end of the chain and consumed all of it.
    pub(crate) fn is_happy(&self, result: i32) -> bool {
        exit_of(result) == Exit::Clean && self.unconsumed() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::entity_trace::map::fake::FakeMem;

    fn chain(lens: &[usize]) -> Vec<PacketView> {
        lens.iter()
            .enumerate()
            .map(|(i, &l)| PacketView {
                ptr: 0x1000 + 0x100 * i as u32,
                len: l + 1,
                seq: 70 + i as u32,
            })
            .collect()
    }

    fn iter_at(c: &[PacketView], idx: usize, cursor: u16) -> IterState {
        IterState {
            packet: c[idx].ptr,
            packet_len: c[idx].len as u16,
            cursor,
        }
    }

    fn ok(id: u8, body_off: u16, len: u32) -> Unpacked {
        Unpacked {
            msg_id: id,
            flag: 0,
            body_offset: body_off,
            len,
            decoded_len: len,
        }
    }

    const WORD: Element = Element { style: 1, param: 2 };

    #[test]
    fn the_chain_is_read_through_next_links() {
        let mut m = FakeMem::default();
        for (i, seq) in [70u32, 71].iter().enumerate() {
            let p = 0x1000 + 0x100 * i as u32;
            m.set(p + packet::LEN, 1401);
            m.set(p + packet::SEQ, *seq);
            m.set(p + packet::NEXT, if i == 0 { 0x1100 } else { 0 });
        }
        let c = read_chain(&m, 0x1000).unwrap();
        assert_eq!(c.len(), 2);
        assert_eq!(c[1].seq, 71);
        assert_eq!(read_chain(&m, 0), None);
    }

    /// Message offsets are measured in the concatenated payload, so a
    /// message in the second packet lands after the first packet's bytes.
    #[test]
    fn message_offsets_span_packets() {
        let c = chain(&[1400, 1400, 500]);
        let mut t = BundleTrace::new(c.clone());
        assert_eq!(t.total, 3300);
        assert_eq!(t.boundaries(), vec![1400, 2800]);
        // Header at cursor 1 of packet 1 (payload offset 1400), 3-byte
        // header, 100-byte body.
        t.on_unpack(iter_at(&c, 1, 1), ok(0x80, 4, 100), Some(WORD));
        let m = t.last.unwrap();
        assert_eq!(
            (m.start, m.header, m.len, m.straddles),
            (1400, 3, 100, false)
        );
        assert_eq!(t.consumed, 1503);
    }

    #[test]
    fn a_body_that_crosses_the_packet_end_is_counted_as_straddling() {
        let c = chain(&[1400, 1400]);
        let mut t = BundleTrace::new(c.clone());
        // Header at cursor 1300, body from 1303 for 200 bytes: ends at
        // 1503 > packet_len 1401.
        t.on_unpack(iter_at(&c, 0, 1300), ok(0x81, 1303, 200), Some(WORD));
        assert_eq!(t.straddled, 1);
        assert_eq!(t.consumed, 1299 + 3 + 200);
    }

    /// The finding: a WORD_LENGTH header that a fragment ends inside aborts
    /// the loop; the event names the fragment boundary, the header length
    /// and how many header bytes made it into the packet.
    #[test]
    fn a_header_split_by_a_fragment_boundary_is_named() {
        let c = chain(&[1400, 1400]);
        let mut t = BundleTrace::new(c.clone());
        t.on_unpack(iter_at(&c, 0, 1), ok(0x80, 4, 1295), Some(WORD));
        // Next header at cursor 1299 of a packet whose len is 1401: 2 of 3
        // header bytes fit... make it 1400: only two bytes left.
        let mut pre = iter_at(&c, 0, 1400);
        pre.packet_len = 1401;
        let post = Unpacked {
            msg_id: 0x80,
            flag: 0x20,
            body_offset: 0,
            len: 0,
            decoded_len: 0,
        };
        t.on_unpack(pre, post, Some(WORD));
        let (abs, fault) = t.fault.unwrap();
        assert_eq!(abs, 1399);
        assert!(matches!(
            fault,
            UnpackFault::HeaderDoesNotFit {
                header: 3,
                packet_len: 1401,
                cursor: 1400
            }
        ));
        let f = t.end_fields(RESULT_CORRUPTED, Some(1), Some(1), None);
        let get = |k: &str| f.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone());
        assert_eq!(get("exit"), Some(json!("corrupted_header")));
        assert_eq!(get("fault"), Some(json!("header_does_not_fit_packet")));
        assert_eq!(get("header_bytes_in_packet"), Some(json!(1)));
        assert_eq!(get("abort_packet_index"), Some(json!(0)));
        assert_eq!(get("messages"), Some(json!(1)));
        assert_eq!(get("boundaries"), Some(json!([1400])));
    }

    #[test]
    fn a_clean_bundle_is_happy_only_if_every_byte_was_consumed() {
        let c = chain(&[100]);
        let mut t = BundleTrace::new(c.clone());
        t.on_unpack(iter_at(&c, 0, 1), ok(0x80, 4, 97), Some(WORD));
        assert!(t.is_happy(0));
        let mut short = BundleTrace::new(chain(&[100]));
        short.on_unpack(iter_at(&c, 0, 1), ok(0x80, 4, 50), Some(WORD));
        assert!(!short.is_happy(0), "50 bytes left over is not clean");
        assert!(!t.is_happy(RESULT_UNKNOWN_MESSAGE_ID));
    }

    #[test]
    fn an_unknown_message_id_reports_where_the_loop_stopped() {
        let c = chain(&[1400, 1400]);
        let mut t = BundleTrace::new(c.clone());
        t.on_unpack(iter_at(&c, 0, 1), ok(0x80, 4, 1396), Some(WORD));
        assert_eq!(t.consumed, 1399);
        let f = t.end_fields(RESULT_UNKNOWN_MESSAGE_ID, Some(1), Some(1), Some(0xEE));
        let get = |k: &str| f.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone());
        assert_eq!(get("exit"), Some(json!("unknown_message_id")));
        assert_eq!(get("abort_msg_id"), Some(json!(0xEE)));
        assert_eq!(get("abort_offset"), Some(json!(1399)));
        assert_eq!(get("abort_packet_index"), Some(json!(0)));
        assert_eq!(get("unconsumed_bytes"), Some(json!(1401)));
        // The id byte lives in packet 0, at payload offset 1399: data
        // offset 1400.
        assert_eq!(t.position_of(1399), Some((0x1000, packet::DATA + 1400)));
    }

    #[test]
    fn the_start_event_names_the_source_and_the_group_seqs() {
        let t = BundleTrace::new(chain(&[1400; 15]));
        let f = t.start_fields();
        let get = |k: &str| f.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone());
        assert_eq!(get("source"), Some(json!("assembled_fragments")));
        assert_eq!(get("packets"), Some(json!(15)));
        assert_eq!(get("total_bytes"), Some(json!(21000)));
        assert_eq!(get("seq_first"), Some(json!(70)));
        assert_eq!(get("seq_last"), Some(json!(84)));
        let single = BundleTrace::new(chain(&[10]));
        assert_eq!(single.source(), "single_packet");
    }
}
