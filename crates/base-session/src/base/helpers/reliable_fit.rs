//! Reliable sends that never put a datagram on the wire larger than the
//! client can receive.
//!
//! The client's Mercury socket reader (`FUN_0158a200`) gives `recvfrom` a
//! [`PACKET_MAX_SIZE`] (1472-byte) buffer. A larger datagram fails with
//! `WSAEMSGSIZE` before Mercury sees it, and every retransmit resends the
//! same cached bytes, so one oversized reliable packet wedges the session's
//! reliable stream for good (`mercury.tx_hole`). See
//! `docs/protocol/mercury-wire-format.md` § Reliable datagram size budget.
//!
//! A single-packet send hands the helper a closure that frames and encrypts
//! one packet. [`send_reliable_to_addr`] calls it once without ACKs to
//! measure it, before any sequence number is reserved:
//!
//! * **It fits.** The helper reserves one sequence number, takes only the
//!   ACKs that still fit beside that body, and builds the packet for real.
//! * **It does not fit.** The body alone is too big for one datagram (a
//!   humanoid NPC's `createOnClient` cascade in a dense area is the known
//!   case). The helper decrypts the measured packet, takes its body, and
//!   sends it through [`send_bundle_reliable_to_addr`], which reserves one
//!   contiguous sequence number per fragment and cuts on message boundaries.
//!   The client reassembles the fragments into the same single frame the one
//!   packet would have been, so ordering and the "one bundle == one client
//!   frame" rule are unchanged.
//!
//! Measuring first is what makes fragmenting possible: fragments need
//! contiguous sequence numbers, and once one number is reserved another task
//! may reserve the next.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::channel_bundle::ChannelBundle;
use cimmeria_mercury::consts::PACKET_MAX_SIZE;
use cimmeria_mercury::encryption::{EncryptionVersion, MercuryEncryption};
use cimmeria_mercury::packet::{
    Bytes, FLAG_FRAGMENTED, FLAG_HAS_REQUESTS, FLAG_ON_CHANNEL, FLAG_RELIABLE, SEQUENCE_MASK,
};
use cimmeria_mercury::transport::Transport;

use super::reliable_send::{shadow_register_reliable_send_with_details, ReliableSendDetails};
use super::{BundleSendOutcome, ConnectedClientState, WitnessSendOutcome};

/// Sequence number the measuring build uses. Any value frames to the same
/// size; the packet is never sent.
const PROBE_SEQ: u32 = 1;

/// Send one reliable packet to the session at `addr`, fragmenting it when its
/// body cannot fit one datagram (module doc).
///
/// `build_packet(key, version, seq, acks)` frames and encrypts the packet. It
/// is called twice, once to measure and once to send, so it must be a pure
/// function of its arguments. `log_id` is the entity the log lines name
/// (the witness, or the session's player); `kind` is the `send_kind` the
/// `mercury.reliable_send` and `mercury.tx_hole` lines carry.
///
/// Returns [`WitnessSendOutcome::Sent`] with the first sequence number used
/// and the encrypted length (for a fragmented send, the body length), or
/// [`WitnessSendOutcome::ClientDisconnected`] /
/// [`WitnessSendOutcome::SendError`]; never `AddrUnresolved`.
pub async fn send_reliable_to_addr<F>(
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
    log_id: u32,
    kind: &'static str,
    build_packet: F,
) -> WitnessSendOutcome
where
    F: Fn(&[u8; 32], EncryptionVersion, u32, &[u32]) -> Vec<u8>,
{
    let session = connected
        .lock()
        .ok()
        .and_then(|clients| clients.get(&addr).map(|c| (c.key, c.enc_version)));
    let Some((key, version)) = session else {
        log_client_disconnected(log_id, addr);
        return WitnessSendOutcome::ClientDisconnected;
    };

    let probe = build_packet(&key, version, PROBE_SEQ, &[]);
    let plaintext_bound = if probe.len() > PACKET_MAX_SIZE {
        match reframe_body(&probe, &key, version) {
            Some(body) => {
                return send_oversize_as_bundle(
                    transport,
                    connected,
                    addr,
                    log_id,
                    kind,
                    probe.len(),
                    &body,
                )
                .await;
            }
            // The packet could not be taken apart (a builder that does not
            // produce a plain one-packet frame). Send it as built: the
            // `mercury.reliable_send` WARN names it.
            None => {
                tracing::error!(
                    target: "mercury.reliable_send",
                    event = "oversize_unframeable",
                    peer = %addr,
                    log_id, // nt:id-only the rare-path line; reliable_send names the session
                    send_kind = kind,
                    wire_len = probe.len(),
                    packet_max_size = PACKET_MAX_SIZE,
                    "reliable packet is larger than the client buffer and could not be fragmented"
                );
                // No ACK fits beside a body this size.
                PACKET_MAX_SIZE
            }
        }
    } else {
        cimmeria_mercury::packet::max_plaintext_len(probe.len(), version)
    };

    let reserved = {
        let clients = connected.lock().unwrap();
        clients.get(&addr).map(|c| {
            let seq = c.next_seq.fetch_add(1, Ordering::Relaxed) & SEQUENCE_MASK;
            let acks: Vec<u32> = cimmeria_mercury::packet::take_acks_for_plaintext(
                &mut c.pending_acks.lock().unwrap(),
                plaintext_bound,
                c.enc_version,
            );
            (c.key, c.enc_version, seq, acks)
        })
    };
    let Some((key, version, seq, acks)) = reserved else {
        log_client_disconnected(log_id, addr);
        return WitnessSendOutcome::ClientDisconnected;
    };
    let packet = build_packet(&key, version, seq, &acks);
    let bytes = packet.len();
    if let Err(e) = transport.send_to(&packet, addr).await {
        tracing::warn!(
            witness_id = log_id,
            witness_name = crate::base::session_identity::identity_for_addr(connected, addr)
                .player_name,
            %addr,
            send_kind = kind,
            "AoI reliable: failed to send packet: {e}"
        );
        return WitnessSendOutcome::SendError;
    }
    // Register the encrypted bytes with the per-session Channel so
    // the retransmit driver in tick_sync re-sends on RTO expiry.
    shadow_register_reliable_send_with_details(
        connected,
        addr,
        seq,
        Bytes::copy_from_slice(&packet),
        ReliableSendDetails {
            kind,
            fragment: None,
            message_count: None,
            // The closure encrypts the packet; its message is not seen here.
            first_message: None,
        },
    );
    WitnessSendOutcome::Sent { addr, seq, bytes }
}

fn log_client_disconnected(log_id: u32, addr: SocketAddr) {
    tracing::debug!(
        witness_id = log_id, // nt:id-only the session left mid-send, so nothing names it
        %addr,
        reason = "client_disconnected",
        "AoI reliable: client disconnected mid-send -- packet dropped"
    );
}

/// The body of a measured packet, if it is a plain one-packet frame whose
/// body can be refragmented without losing anything (no request offset, not
/// already a fragment).
fn reframe_body(packet: &[u8], key: &[u8; 32], version: EncryptionVersion) -> Option<Vec<u8>> {
    let plaintext = MercuryEncryption::from_session_key_versioned(*key, version)
        .decrypt(packet)
        .ok()?;
    let parsed = cimmeria_mercury::packet::parse_incoming(&plaintext).ok()?;
    let unsupported = FLAG_HAS_REQUESTS | FLAG_FRAGMENTED;
    (parsed.flags & unsupported == 0 && !parsed.body.is_empty()).then(|| parsed.body.to_vec())
}

async fn send_oversize_as_bundle(
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
    log_id: u32,
    kind: &'static str,
    wire_len: usize,
    body: &[u8],
) -> WitnessSendOutcome {
    let names = cimmeria_mercury::packet::first_message_head(body).map(|head| {
        connected.lock().map_or_else(
            |_| cimmeria_mercury::channel::MessageNames::default(),
            |clients| super::reliable_send::name_stalled_message(&clients, &head),
        )
    });
    let names = names.unwrap_or_default();
    // Rare (a body over one datagram) and the reason a stream did not wedge:
    // INFO, so it is in SigNoz without a DEBUG filter.
    tracing::info!(
        target: "mercury.reliable_send",
        event = "oversize_fragmented",
        peer = %addr,
        witness_id = log_id,
        witness_name = crate::base::session_identity::identity_for_addr(connected, addr)
            .player_name,
        send_kind = kind,
        wire_len,
        packet_max_size = PACKET_MAX_SIZE,
        body_len = body.len(),
        fragments = cimmeria_mercury::packet::fragment_count(body),
        msg_id = ?names.msg_id,
        msg_name = names.msg_name,
        method_name = names.method_name,
        "reliable packet is larger than the client buffer; sending it as a fragmented bundle"
    );
    let mut bundle = ChannelBundle::new(true);
    bundle.append_raw_message(body);
    match send_bundle_reliable_to_addr(transport, connected, addr, log_id, bundle, kind).await {
        BundleSendOutcome::Sent {
            addr,
            base_seq,
            bytes,
            ..
        } => WitnessSendOutcome::Sent {
            addr,
            seq: base_seq,
            bytes,
        },
        BundleSendOutcome::ClientDisconnected => WitnessSendOutcome::ClientDisconnected,
        BundleSendOutcome::AddrUnresolved
        | BundleSendOutcome::Empty
        | BundleSendOutcome::SendError => WitnessSendOutcome::SendError,
    }
}

/// Send a [`ChannelBundle`] reliably to the session at `addr`: the body of
/// [`super::send_bundle_to_witness_reliable`] once the witness is resolved,
/// and the fragmenting path of [`send_reliable_to_addr`].
///
/// See [`super::send_bundle_to_witness_reliable`] for the sequence
/// reservation contract. `log_id` is the entity the log lines name; `kind`
/// is the `send_kind` each registered fragment carries.
pub(super) async fn send_bundle_reliable_to_addr(
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
    witness_id: u32,
    mut bundle: ChannelBundle,
    kind: &'static str,
) -> BundleSendOutcome {
    let send_data = {
        let clients = connected.lock().unwrap();
        let c = match clients.get(&addr) {
            Some(c) => c,
            None => {
                tracing::debug!(
                    witness_id, // nt:id-only the session left mid-send, so nothing names it
                    %addr,
                    reason = "client_disconnected",
                    "AoI bundle: client disconnected mid-send -- bundle dropped"
                );
                return BundleSendOutcome::ClientDisconnected;
            }
        };

        // Drain pending ACKs into the bundle so they ride the first
        // finalized packet. Done under the same lock window as the seq
        // reservation so a concurrent ACK-pumping send doesn't race.
        let drained_acks: Vec<u32> = cimmeria_mercury::packet::take_piggyback_acks(
            &mut c.pending_acks.lock().unwrap(),
            c.enc_version,
        );
        bundle.add_acks(&drained_acks);

        // Now that ACKs are in, estimated_packet_count reflects the true
        // emit count (empty body + empty acks → 0; empty body + acks → 1;
        // otherwise ceil(body / FRAGMENT_BODY_SIZE)).
        let packet_count = bundle.estimated_packet_count();
        if packet_count == 0 {
            return BundleSendOutcome::Empty;
        }

        // Atomically reserve `packet_count` consecutive sequence numbers.
        // Mask the base to the 28-bit Mercury space; per-fragment seqs
        // (base+1, base+2, ...) inherit the contiguous reservation and are
        // re-masked by build_fragmented_bundle internally.
        let base_seq = c.next_seq.fetch_add(packet_count as u32, Ordering::Relaxed) & SEQUENCE_MASK;
        let key = c.key;
        let version = c.enc_version;
        // For the flush line (Rule 6). Interning is a read-locked hash hit
        // once the name has been seen, taken in this existing lock window.
        let witness_name = cimmeria_entity::name_intern::intern_opt(c.player_name.as_deref());
        (key, version, base_seq, packet_count, witness_name)
    };
    let (key, version, base_seq, packet_count, witness_name) = send_data;

    let num_messages = bundle.num_messages();
    let body_len = bundle.body_len();
    // Fragment shape for the flush event: body bytes per packet and how many
    // cuts were moved off a message header (the client aborts a bundle whose
    // header straddles two packets).
    let plan = bundle.fragment_plan();
    let packet_bytes = format!("{:?}", plan.packet_sizes());
    let header_guarded_cuts = plan.header_guarded_cuts;

    // Finalize through the session encrypt closure. Use FLAG_RELIABLE +
    // FLAG_ON_CHANNEL as base flags — the bundle adds FLAG_HAS_SEQUENCE,
    // FLAG_FRAGMENTED, FLAG_HAS_ACKS internally as needed per fragment.
    let base_flags = FLAG_RELIABLE | FLAG_ON_CHANNEL;
    let (packets, seqs_consumed) = bundle.finalize(base_flags, base_seq, |plaintext| {
        crate::mercury::encrypt_packet(plaintext, &key, version)
    });

    debug_assert_eq!(
        seqs_consumed as usize, packet_count,
        "estimated_packet_count contract violated — seq reservation overshoots finalize"
    );

    tracing::info!(
        %addr,
        witness_id,
        witness_name,
        messages = num_messages,
        body_bytes = body_len,
        packets = packets.len(),
        fragmented = packets.len() > 1,
        packet_bytes = %packet_bytes,
        header_guarded_cuts,
        base_seq,
        send_kind = kind,
        "AoI bundle: flushed {num_messages} messages in {} packet(s)",
        packets.len()
    );

    for (i, pkt) in packets.iter().enumerate() {
        let frag_seq = base_seq.wrapping_add(i as u32) & SEQUENCE_MASK;
        if let Err(e) = transport.send_to(pkt, addr).await {
            // Abort the rest of the bundle on the first send failure.
            // Continuing would push the trailing fragments onto the wire
            // with no chance of client-side reassembly (the failed
            // fragment's seq is already a gap in the reliable stream and
            // the bundle's frag_begin/frag_end footers expect every
            // fragment in [base_seq..base_seq+packet_count) to arrive).
            // The retransmit driver in tick_sync re-sends the registered
            // fragments [0..i); the unsent fragments [i..packet_count)
            // remain a permanent gap until the inactivity timer reaps
            // the channel — an outcome no worse than continuing, with
            // less wasted bandwidth.
            tracing::error!(
                witness_id,
                witness_name,
                %addr,
                frag_seq,
                fragment = i + 1,
                total = packets.len(),
                already_sent = i,
                "AoI bundle: failed to send fragment; aborting remainder of bundle. \
                 Reliable seq stream now has gaps at [{}..{}); channel will be reaped \
                 on inactivity timeout: {e}",
                frag_seq,
                base_seq.wrapping_add(packet_count as u32) & SEQUENCE_MASK,
            );
            return BundleSendOutcome::SendError;
        }
        shadow_register_reliable_send_with_details(
            connected,
            addr,
            frag_seq,
            Bytes::copy_from_slice(pkt),
            ReliableSendDetails {
                kind,
                fragment: (packets.len() > 1).then_some((i + 1, packets.len())),
                message_count: Some(num_messages),
                // A later fragment starts inside the stream; the plan's walk
                // recorded the message it starts in, for `mercury.tx_hole`.
                first_message: plan.heads.get(i).copied().flatten().filter(|_| i > 0),
            },
        );
    }

    BundleSendOutcome::Sent {
        addr,
        base_seq,
        packets: packets.len(),
        bytes: body_len,
    }
}
