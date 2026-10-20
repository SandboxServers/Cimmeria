//! Reliable sends that never put a datagram on the wire larger than the
//! client can receive.
//! The client's Mercury socket reader (`FUN_0158a200`) has a 1472-byte
//! receive buffer. An oversized datagram fails before Mercury sees it, then
//! cached retransmits wedge the reliable stream (`mercury.tx_hole`). See
//! `docs/protocol/mercury-wire-format.md` § Reliable datagram size budget.
//! [`send_reliable_to_addr`] measures an encrypted packet without ACKs before
//! reserving sequence numbers. A fitting packet gets ACKs within the remaining
//! budget. An oversized body is decrypted and sent as a fragmented bundle
//! with contiguous sequence numbers and unchanged client frame ordering.

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

use super::bundle_manifest::{log_bundle_manifest, BundleManifestEntry, ManifestSend};
use super::entry_guard::superseded;
use super::reliable_send::{
    shadow_register_reliable_send_with_details, ReliableRegistrationOutcome, ReliableSendDetails,
};
use super::session_bound::RecipientSnapshot;
use super::{
    origin_still_current, send_with_origin, BundleSendOutcome, ConnectedClientState,
    WitnessSendOutcome,
};

/// Sequence number the measuring build uses. Any value frames to the same
/// size; the packet is never sent.
const PROBE_SEQ: u32 = 1;

/// Send a reliable packet, fragmenting when the body exceeds one datagram.
/// `build_packet` must be pure; only its measuring call logs. `kind` labels
/// reliable-send and TX-hole telemetry. A successful outcome reports its first
/// sequence number, byte count and datagram count.
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
    send_reliable_to_addr_owned(
        transport,
        connected,
        addr,
        log_id,
        kind,
        build_packet,
        None,
        None,
    )
    .await
}

/// Send only while the connection still belongs to the character in
/// `expected` (entity, player id and Base connection instance). The check
/// repeats in every lock window that reserves a seq, takes ACKs or registers
/// the datagram, so a relaunch on the same address cannot receive old content.
pub(super) async fn send_reliable_to_addr_matching<F>(
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
    log_id: u32,
    kind: &'static str,
    expected: Option<RecipientSnapshot>,
    build_packet: F,
) -> WitnessSendOutcome
where
    F: Fn(&[u8; 32], EncryptionVersion, u32, &[u32]) -> Vec<u8>,
{
    send_reliable_to_addr_owned(
        transport,
        connected,
        addr,
        log_id,
        kind,
        build_packet,
        None,
        expected,
    )
    .await
}

/// Send content only if its original session still owns the peer before
/// any sequence number or ACK is reserved, including on the fragment path.
pub async fn send_reliable_to_addr_for_owner<F>(
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
    log_id: u32,
    kind: &'static str,
    build_packet: F,
    expected_seq: &Arc<std::sync::atomic::AtomicU32>,
) -> WitnessSendOutcome
where
    F: Fn(&[u8; 32], EncryptionVersion, u32, &[u32]) -> Vec<u8>,
{
    send_reliable_to_addr_owned(
        transport,
        connected,
        addr,
        log_id,
        kind,
        build_packet,
        Some(expected_seq),
        None,
    )
    .await
}

async fn send_reliable_to_addr_owned<F>(
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
    log_id: u32,
    kind: &'static str,
    build_packet: F,
    expected_seq: Option<&Arc<std::sync::atomic::AtomicU32>>,
    expected: Option<RecipientSnapshot>,
) -> WitnessSendOutcome
where
    F: Fn(&[u8; 32], EncryptionVersion, u32, &[u32]) -> Vec<u8>,
{
    let session = connected.lock().ok().and_then(|clients| {
        clients
            .get(&addr)
            .filter(|c| expected_seq.is_none_or(|expected| Arc::ptr_eq(&c.next_seq, expected)))
            .filter(|c| !superseded(addr, c))
            .filter(|c| owner_matches(expected, c))
            .map(|c| (c.key, c.enc_version, Arc::clone(&c.next_seq)))
    });
    let Some((key, version, session_seq)) = session else {
        super::log_client_disconnected(log_id, addr);
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
                    &session_seq,
                    expected,
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

    // The probe already read the key and version; the real build uses the
    // same pair, so it frames exactly as the probe measured.
    let reserved = {
        let clients = connected.lock().unwrap();
        clients
            .get(&addr)
            .filter(|c| Arc::ptr_eq(&c.next_seq, &session_seq))
            .filter(|c| !superseded(addr, c))
            .filter(|c| owner_matches(expected, c))
            .map(|c| {
                let _seq_gate = c.reliable_seq_gate.lock().unwrap();
                let raw_seq = c.next_seq.fetch_add(1, Ordering::Relaxed);
                let acks: Vec<u32> = cimmeria_mercury::packet::take_acks_for_plaintext(
                    &mut c.pending_acks.lock().unwrap(),
                    plaintext_bound,
                    version,
                );
                (raw_seq, acks, c.wire_send_origin())
            })
    };
    let Some((raw_seq, acks, origin)) = reserved else {
        super::log_client_disconnected(log_id, addr);
        return WitnessSendOutcome::ClientDisconnected;
    };
    let seq = raw_seq & SEQUENCE_MASK;
    let mut packet = silent_build(&build_packet, &key, version, seq, &acks);
    if packet.len() > PACKET_MAX_SIZE && !acks.is_empty() {
        // The ACKs did not fit after all (a builder whose body grows with its
        // ACK list). The probe proved the packet fits without them: put them
        // back for the next send and rebuild at the same seq.
        super::requeue_acks(connected, addr, &session_seq, &acks);
        tracing::warn!(
            target: "mercury.reliable_send",
            event = "ack_overflow_rebuilt",
            peer = %addr,
            witness_id = log_id,
            witness_name = crate::base::session_identity::identity_for_addr(connected, addr)
                .player_name,
            send_kind = kind,
            seq,
            wire_len = packet.len(),
            acks = acks.len(),
            packet_max_size = PACKET_MAX_SIZE,
            "reliable packet with ACKs is larger than the client buffer; rebuilt without them"
        );
        packet = silent_build(&build_packet, &key, version, seq, &[]);
    }
    if packet.len() > PACKET_MAX_SIZE {
        // Still too big: the builder did not frame as it measured. Hand the
        // reserved seq back if nobody has taken the next one, and fragment.
        if let Some(body) = reframe_body(&packet, &key, version) {
            let released = connected.lock().ok().is_some_and(|clients| {
                clients
                    .get(&addr)
                    .filter(|c| Arc::ptr_eq(&c.next_seq, &session_seq))
                    .is_some_and(|c| {
                        let Ok(_seq_gate) = c.reliable_seq_gate.lock() else {
                            return false;
                        };
                        c.next_seq
                            .compare_exchange(
                                raw_seq.wrapping_add(1),
                                raw_seq,
                                Ordering::Relaxed,
                                Ordering::Relaxed,
                            )
                            .is_ok()
                    })
            });
            if released {
                return send_oversize_as_bundle(
                    transport,
                    connected,
                    addr,
                    log_id,
                    kind,
                    packet.len(),
                    &body,
                    &session_seq,
                    expected,
                )
                .await;
            }
        }
        // The seq is taken and must be filled: send as built. The
        // `mercury.reliable_send` WARN below names the packet.
        tracing::error!(
            target: "mercury.reliable_send",
            event = "oversize_after_reserve",
            peer = %addr,
            witness_id = log_id,
            witness_name = crate::base::session_identity::identity_for_addr(connected, addr)
                .player_name,
            send_kind = kind,
            seq,
            wire_len = packet.len(),
            packet_max_size = PACKET_MAX_SIZE,
            "reliable packet grew past the client buffer after its seq was reserved"
        );
    }
    let bytes = packet.len();
    if let Err(e) = send_with_origin(transport, &packet, addr, origin.as_ref()).await {
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
    if !origin_still_current(connected, addr, origin.as_ref()) {
        return WitnessSendOutcome::Sent {
            addr,
            seq,
            bytes,
            packets: 1,
        };
    }
    let registration = shadow_register_reliable_send_with_details(
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
        Some((&session_seq, origin.as_ref())),
    );
    if expected.is_some() {
        match registration {
            ReliableRegistrationOutcome::Registered => {}
            ReliableRegistrationOutcome::SessionChanged => {
                return WitnessSendOutcome::ClientDisconnected;
            }
            ReliableRegistrationOutcome::Rejected => return WitnessSendOutcome::SendError,
        }
    }
    WitnessSendOutcome::Sent {
        addr,
        seq,
        bytes,
        packets: 1,
    }
}

/// Build the packet with tracing switched off on this thread. The measuring
/// build already emitted whatever the builder logs (an appearance-missing
/// WARN, say); a second copy per send would be noise.
fn silent_build<F>(
    build_packet: &F,
    key: &[u8; 32],
    version: EncryptionVersion,
    seq: u32,
    acks: &[u32],
) -> Vec<u8>
where
    F: Fn(&[u8; 32], EncryptionVersion, u32, &[u32]) -> Vec<u8>,
{
    tracing::dispatcher::with_default(&tracing::Dispatch::none(), || {
        build_packet(key, version, seq, acks)
    })
}

fn owner_matches(expected: Option<RecipientSnapshot>, state: &ConnectedClientState) -> bool {
    expected.is_none_or(|snapshot| snapshot.matches(state))
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
    session_seq: &Arc<std::sync::atomic::AtomicU32>,
    expected: Option<RecipientSnapshot>,
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
    // One raw body, not a flush: no per-message manifest.
    match send_bundle_reliable_to_addr_owned(
        transport,
        connected,
        addr,
        log_id,
        bundle,
        kind,
        &[],
        Some(session_seq),
        expected,
    )
    .await
    {
        BundleSendOutcome::Sent {
            addr,
            base_seq,
            packets,
            wire_bytes,
            ..
        } => WitnessSendOutcome::Sent {
            addr,
            seq: base_seq,
            bytes: wire_bytes,
            packets,
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
/// is the `send_kind` each registered fragment carries. `manifest` is the
/// per-message list an `aoi_flush_*` bundle logs on `aoi.bundle_manifest`
/// (TG-AOI-09); empty for every other source.
pub(super) async fn send_bundle_reliable_to_addr(
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
    witness_id: u32,
    bundle: ChannelBundle,
    kind: &'static str,
    manifest: &[BundleManifestEntry],
) -> BundleSendOutcome {
    send_bundle_reliable_to_addr_owned(
        transport, connected, addr, witness_id, bundle, kind, manifest, None, None,
    )
    .await
}

async fn send_bundle_reliable_to_addr_owned(
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
    witness_id: u32,
    mut bundle: ChannelBundle,
    kind: &'static str,
    manifest: &[BundleManifestEntry],
    expected_seq: Option<&Arc<std::sync::atomic::AtomicU32>>,
    expected: Option<RecipientSnapshot>,
) -> BundleSendOutcome {
    let send_data = {
        let clients = connected.lock().unwrap();
        let c = match clients
            .get(&addr)
            .filter(|c| expected_seq.is_none_or(|expected| Arc::ptr_eq(&c.next_seq, expected)))
            .filter(|c| owner_matches(expected, c))
        {
            // Replaced by a new entry: nothing drains, nothing is reserved.
            Some(c) if superseded(addr, c) => return BundleSendOutcome::ClientDisconnected,
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
        let _seq_gate = c.reliable_seq_gate.lock().unwrap();
        let base_seq = c.next_seq.fetch_add(packet_count as u32, Ordering::Relaxed) & SEQUENCE_MASK;
        let key = c.key;
        let version = c.enc_version;
        // For the flush line (Rule 6). Interning is a read-locked hash hit
        // once the name has been seen, taken in this existing lock window.
        let witness_name = cimmeria_entity::name_intern::intern_opt(c.player_name.as_deref());
        (
            key,
            version,
            base_seq,
            packet_count,
            witness_name,
            c.wire_send_origin(),
            Arc::clone(&c.next_seq),
        )
    };
    let (key, version, base_seq, packet_count, witness_name, origin, session_seq) = send_data;

    let num_messages = bundle.num_messages();
    let body_len = bundle.body_len();
    // Fragment size and message-header cuts for the flush event.
    let plan = bundle.fragment_plan();
    let packet_bytes = format!("{:?}", plan.packet_sizes());
    let header_guarded_cuts = plan.header_guarded_cuts;

    // The bundle adds sequence, fragment and ACK flags per packet.
    let base_flags = FLAG_RELIABLE | FLAG_ON_CHANNEL;
    let (packets, seqs_consumed) = bundle.finalize(base_flags, base_seq, |plaintext| {
        crate::mercury::encrypt_packet(plaintext, &key, version)
    });

    debug_assert_eq!(
        seqs_consumed as usize, packet_count,
        "estimated_packet_count contract violated — seq reservation overshoots finalize"
    );

    // Multi-packet flushes use INFO; routine single packets use DEBUG.
    if packets.len() > 1 {
        tracing::info!(
            %addr,
            witness_id,
            witness_name,
            messages = num_messages,
            body_bytes = body_len,
            packets = packets.len(),
            fragmented = true,
            packet_bytes = %packet_bytes,
            header_guarded_cuts,
            base_seq,
            send_kind = kind,
            "AoI bundle: flushed {num_messages} messages in {} packet(s)",
            packets.len()
        );
    } else {
        tracing::debug!(
            %addr,
            witness_id,
            witness_name,
            messages = num_messages,
            body_bytes = body_len,
            packets = packets.len(),
            fragmented = false,
            packet_bytes = %packet_bytes,
            header_guarded_cuts,
            base_seq,
            send_kind = kind,
            "AoI bundle: flushed {num_messages} messages in {} packet(s)",
            packets.len()
        );
    }
    // Name message offsets for client `unpack_fault` correlation.
    log_bundle_manifest(&ManifestSend {
        addr,
        witness_id,
        witness_name,
        kind,
        base_seq,
        messages: num_messages,
        packets: packets.len(),
        entries: manifest,
    });

    for (i, pkt) in packets.iter().enumerate() {
        if i > 0 && !origin_still_current(connected, addr, origin.as_ref()) {
            return BundleSendOutcome::SendError;
        }
        let frag_seq = base_seq.wrapping_add(i as u32) & SEQUENCE_MASK;
        if let Err(e) = send_with_origin(transport, pkt, addr, origin.as_ref()).await {
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
        if !origin_still_current(connected, addr, origin.as_ref()) {
            return BundleSendOutcome::SendError;
        }
        let registration = shadow_register_reliable_send_with_details(
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
            Some((&session_seq, origin.as_ref())),
        );
        if expected.is_some() {
            match registration {
                ReliableRegistrationOutcome::Registered => {}
                ReliableRegistrationOutcome::SessionChanged => {
                    return BundleSendOutcome::ClientDisconnected;
                }
                ReliableRegistrationOutcome::Rejected => return BundleSendOutcome::SendError,
            }
        }
    }

    BundleSendOutcome::Sent {
        addr,
        base_seq,
        packets: packets.len(),
        bytes: body_len,
        wire_bytes: packets.iter().map(Vec::len).sum(),
    }
}
