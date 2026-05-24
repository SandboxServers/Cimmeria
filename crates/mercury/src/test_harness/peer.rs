//! One end of a paired Mercury session.
//!
//! Owns a [`Channel`], a real loopback `UdpSocket`, an injected
//! [`TestClock`](super::TestClock), and (optionally) a
//! [`MercuryEncryption`] context. Spawns a recv pump that reads from the
//! socket, decrypts, parses, feeds inbound packets into the channel, and
//! makes reassembled bundles available via [`Self::recv_n_bundles`].
//!
//! Per-direction [`NetworkPolicy`] is held by the
//! [`LoopbackSession`](super::LoopbackSession) and consulted by each
//! peer's outbound pump.

use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bytes::Bytes;
use tokio::net::UdpSocket;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

use crate::channel::Channel;
use crate::encryption::MercuryEncryption;
use crate::packet::{
    build_fragmented_bundle, build_outgoing, parse_incoming, FLAG_HAS_ACKS, FLAG_HAS_SEQUENCE,
    FLAG_ON_CHANNEL, FLAG_RELIABLE, FRAGMENT_BODY_SIZE,
};

use super::clock::TestClock;
use super::policy::{Direction, NetworkPolicy};

/// Punch list returned by [`LoopbackPeer::tick`].
///
/// Captures what the tick pass *would have caused this peer to emit*.
/// The bytes are already on the wire (subject to policy) by the time
/// `tick` returns — these fields are observation hooks for assertions.
#[derive(Debug, Default, Clone)]
pub struct TickActions {
    /// Encrypted datagrams the retransmit scan re-sent on this tick.
    pub retransmits: Vec<Bytes>,
    /// Encrypted keepalive datagrams emitted by this tick (zero or one).
    pub keepalives: Vec<Bytes>,
}

/// Outcome of one `send_with_policy` policy-evaluation pass. Split out
/// of `send_with_policy` so the lock-holding decision logic stays in a
/// non-async function (clearer scoping, easier to reason about).
///
/// Duplication semantics: `extra_copies` applies **only to the
/// current send** (the packet passed into `decide_send_policy`).
/// Packets in `flushed_held` come from a previous send that was
/// buffered by the reorder policy and already had their own
/// per-send policy decisions made at the time they were buffered;
/// re-applying duplication to them would compound the cycle.
struct SendDecision {
    /// True if the packet must not go on the wire (any drop policy fired).
    drop_this: bool,
    /// Latency to sleep before any send.
    latency: std::time::Duration,
    /// Extra copies beyond the primary send for the **current** packet
    /// only (from `duplicate_every` + `duplicate_next_count` summed).
    extra_copies: u32,
    /// The current send's payload. `None` when the policy buffered it
    /// (reorder hold) — nothing goes on the wire this call.
    current_packet: Option<Bytes>,
    /// Held packets being flushed alongside the current send. Each
    /// emitted exactly once, **never duplicated**. Order is wire
    /// arrival order at the peer (pair-mode: this is the previously
    /// held packet shipped after the current; buffer-mode: the
    /// remainder of the reversed buffer).
    flushed_held: Vec<Bytes>,
}

/// One end of a paired Mercury session.
pub struct LoopbackPeer {
    /// Mercury protocol state.
    pub channel: Arc<Mutex<Channel>>,
    /// Bound loopback socket — the harness binds `127.0.0.1:0` so the
    /// OS assigns an ephemeral port. Parallel test runs cannot collide.
    pub socket: Arc<UdpSocket>,
    /// This peer's bound address (post-`bind`).
    pub addr: SocketAddr,
    /// The paired peer's bound address — where outbound packets are
    /// `send_to`'d. Set when the session is constructed.
    pub peer_addr: SocketAddr,
    /// Pluggable clock; the same `Arc<TestClock>` is the `Clock` the
    /// inner `Channel` reads from. Test code advances this directly.
    pub clock: Arc<TestClock>,
    /// Optional encryption context. When `Some`, every outbound bundle is
    /// AES-256-CBC + HMAC-MD5 encrypted before going on the wire and
    /// every inbound datagram is decrypted before parse.
    pub encryption: Option<Arc<MercuryEncryption>>,
    /// Direction tag for outbound-policy lookups.
    direction: Direction,
    /// Shared policy with the paired peer.
    policy: Arc<Mutex<NetworkPolicy>>,
    /// Reassembled inbound bundles awaiting test-side consumption.
    inbox: Arc<Mutex<VecDeque<Bytes>>>,
    /// Wakes any [`Self::recv_n_bundles`] waiter when `inbox` grows.
    inbox_notify: Arc<Notify>,
    /// Sequence numbers acked-but-not-yet-piggybacked on an outbound
    /// packet. Drained on the next reliable send.
    pending_acks: Arc<Mutex<Vec<u32>>>,
    /// Recv pump handle. Aborted on `shutdown`.
    recv_task: JoinHandle<()>,
}

impl LoopbackPeer {
    /// Take ownership of a pre-bound loopback socket, install `clock`
    /// into a new `Channel`, optionally attach `encryption`, and spawn
    /// the recv pump. `peer_addr` is the address the paired peer is
    /// bound to — used as the `send_to` target.
    ///
    /// Sockets are pre-bound (rather than bound here) so the session
    /// can pair two peers safely: binding A, querying `local_addr()`,
    /// dropping, and rebinding A would race the OS's ephemeral-port
    /// reuse against any other process. Passing the live socket through
    /// keeps the address stable.
    pub async fn from_socket(
        socket: UdpSocket,
        peer_addr: SocketAddr,
        clock: Arc<TestClock>,
        encryption: Option<Arc<MercuryEncryption>>,
        direction: Direction,
        policy: Arc<Mutex<NetworkPolicy>>,
    ) -> std::io::Result<Self> {
        let addr = socket.local_addr()?;
        let socket = Arc::new(socket);
        let channel = Arc::new(Mutex::new(Channel::with_clock(peer_addr, clock.clone())));

        let inbox = Arc::new(Mutex::new(VecDeque::new()));
        let inbox_notify = Arc::new(Notify::new());
        let pending_acks = Arc::new(Mutex::new(Vec::new()));

        let recv_task = spawn_recv_pump(
            socket.clone(),
            channel.clone(),
            encryption.clone(),
            inbox.clone(),
            inbox_notify.clone(),
            pending_acks.clone(),
        );

        Ok(Self {
            channel,
            socket,
            addr,
            peer_addr,
            clock,
            encryption,
            direction,
            policy,
            inbox,
            inbox_notify,
            pending_acks,
            recv_task,
        })
    }

    /// Send `body` to the paired peer. When `reliable` is `true`, the
    /// packet is registered in the channel's TX window for retransmit
    /// tracking; on encryption-enabled sessions the cached `raw_bytes`
    /// for retransmit are the post-encrypt datagram so the resend is
    /// byte-identical to the original.
    ///
    /// Bodies larger than [`FRAGMENT_BODY_SIZE`] auto-fragment via
    /// [`Self::send_large_bundle`] — matches production behavior, where
    /// the chunker decides whether one packet or many goes on the wire.
    pub async fn send_bundle(&self, body: &[u8], reliable: bool) -> std::io::Result<()> {
        if body.len() > FRAGMENT_BODY_SIZE {
            return self.send_large_bundle(body, reliable).await;
        }
        let raw = self.build_and_register(body, reliable);
        self.send_with_policy(raw).await
    }

    /// Explicit fragmented send. Splits `body` into [`FRAGMENT_BODY_SIZE`]
    /// chunks, each its own Mercury packet with `FLAG_FRAGMENTED` and the
    /// frag-range footers. Each fragment gets its own TX-window entry
    /// when `reliable` (so retransmits are per-fragment, not whole-bundle).
    pub async fn send_large_bundle(&self, body: &[u8], reliable: bool) -> std::io::Result<()> {
        let base_seq = {
            let mut channel = self.channel.lock().expect("channel poisoned");
            let seq = channel.next_tx_seq & crate::packet::SEQUENCE_MASK;
            // Reserve the sequence space for all fragments up front so a
            // concurrent send can't interleave its own seq into the middle
            // of this bundle.
            let num_frags = body.len().div_ceil(FRAGMENT_BODY_SIZE) as u32;
            channel.next_tx_seq =
                channel.next_tx_seq.wrapping_add(num_frags) & crate::packet::SEQUENCE_MASK;
            seq
        };

        let acks: Vec<u32> = {
            let mut pending = self.pending_acks.lock().expect("pending_acks poisoned");
            let mut out = std::mem::take(&mut *pending);
            out.reverse();
            out
        };

        let flags = FLAG_ON_CHANNEL | if reliable { FLAG_RELIABLE } else { 0 };

        let enc = self.encryption.clone();
        let (encrypted_packets, num_frags) =
            build_fragmented_bundle(flags, body, base_seq, &acks, |plaintext| match &enc {
                Some(e) => e.encrypt(plaintext).expect("encryption failed in harness"),
                None => plaintext.to_vec(),
            });

        if reliable {
            for (i, raw) in encrypted_packets.iter().enumerate() {
                let seq = base_seq.wrapping_add(i as u32) & crate::packet::SEQUENCE_MASK;
                let packet = crate::packet::Packet {
                    flags: crate::packet::PacketFlags::from_byte(
                        flags | crate::packet::FLAG_HAS_SEQUENCE | crate::packet::FLAG_FRAGMENTED,
                    ),
                    sequence: seq,
                    body: Bytes::new(),
                };
                self.channel
                    .lock()
                    .expect("channel poisoned")
                    .register_sent_packet(packet, Bytes::from(raw.clone()))
                    .expect("register_sent_packet failed in harness");
            }
        }
        let _ = num_frags;

        for raw in encrypted_packets {
            self.send_with_policy(Bytes::from(raw)).await?;
        }
        Ok(())
    }

    /// Build the outbound datagram, optionally encrypt, and register in
    /// the channel's TX window. Returns the bytes that should go on the
    /// wire (after any policy filter is applied by `send_with_policy`).
    fn build_and_register(&self, body: &[u8], reliable: bool) -> Bytes {
        let mut channel = self.channel.lock().expect("channel poisoned");
        let mut acks = {
            let mut pending = self.pending_acks.lock().expect("pending_acks poisoned");
            std::mem::take(&mut *pending)
        };

        let mut flags = FLAG_ON_CHANNEL;
        let seq = if reliable {
            flags |= FLAG_HAS_SEQUENCE | FLAG_RELIABLE;
            let seq = channel.next_tx_seq & crate::packet::SEQUENCE_MASK;
            channel.next_tx_seq =
                channel.next_tx_seq.wrapping_add(1) & crate::packet::SEQUENCE_MASK;
            Some(seq)
        } else if !acks.is_empty() {
            // Even unreliable sends carry a piggyback seq when acking —
            // matches production cadence so the peer recognises the
            // packet shape.
            None
        } else {
            None
        };

        if !acks.is_empty() {
            flags |= FLAG_HAS_ACKS;
            // Mercury's wire format is ack-newest-first per `build_outgoing`'s
            // contract; the channel ordering produced "oldest first" so we
            // reverse here.
            acks.reverse();
        }

        let pkt = build_outgoing(flags, body, seq, &acks, None);
        let plaintext = pkt.freeze();

        let raw = match &self.encryption {
            Some(enc) => Bytes::from(
                enc.encrypt(&plaintext)
                    .expect("encryption failed in harness"),
            ),
            None => plaintext.clone(),
        };

        if reliable {
            let packet = crate::packet::Packet {
                flags: crate::packet::PacketFlags::from_byte(flags),
                sequence: seq.expect("reliable sends always have a seq"),
                body: Bytes::copy_from_slice(body),
            };
            channel
                .register_sent_packet(packet, raw.clone())
                .expect("register_sent_packet failed in harness");
        }

        raw
    }

    /// Apply the outbound [`NetworkPolicy`] and (unless dropped) call
    /// `socket.send_to`. Handles drop / latency / duplicate / reorder.
    ///
    /// Policy evaluation order per send:
    /// 1. Increment `send_count` (1-indexed).
    /// 2. Consume one slot of `drop_next` if non-zero, OR match against
    ///    `drop_at_send_count` if set, OR roll `drop_probability`. If
    ///    any drop fires, bump `drop_count` and the bytes never go on
    ///    the wire — RTO / retransmit consequences on the sender are
    ///    identical to a wire-side drop.
    /// 3. Apply `latency` via `tokio::time::sleep` before any send_to.
    /// 4. If `reorder_pairs` is set, hold the first of each pair and
    ///    send `(second, first)` when the pair completes. Otherwise
    ///    if `reorder_buffer_size > 0`, accumulate up to N and flush
    ///    reversed when full.
    /// 5. Send (one or more copies based on `duplicate_every` and
    ///    `duplicate_next_count`).
    async fn send_with_policy(&self, bytes: Bytes) -> std::io::Result<()> {
        let decision = self.decide_send_policy(bytes.clone());

        if decision.drop_this {
            return Ok(());
        }

        if !decision.latency.is_zero() {
            tokio::time::sleep(decision.latency).await;
        }

        // Current send: emit once + extra_copies times (the duplication
        // semantics apply ONLY to the current send, not to any held
        // packets being flushed alongside).
        if let Some(current) = &decision.current_packet {
            self.socket.send_to(current, self.peer_addr).await?;
            for _ in 0..decision.extra_copies {
                self.socket.send_to(current, self.peer_addr).await?;
            }
        }

        // Held packets from reorder buffer: each exactly once, in the
        // order the policy chose. Held packets had their own duplicate
        // decisions made at buffer time; re-applying here would
        // double-count the cycle.
        for held in &decision.flushed_held {
            self.socket.send_to(held, self.peer_addr).await?;
        }
        Ok(())
    }

    /// All policy decisions happen here under one lock acquisition so
    /// counters advance atomically with the checks that consume them.
    fn decide_send_policy(&self, bytes: Bytes) -> SendDecision {
        let mut policy = self.policy.lock().expect("policy poisoned");
        let dir = self.direction;

        *policy.send_count.get_mut(dir) += 1;
        let count = policy.send_count.get(dir);

        // ── Drop checks (any-of) ─────────────────────────────────
        let drop_next_slot = policy.drop_next.get_mut(dir);
        let drop_from_next = if *drop_next_slot > 0 {
            *drop_next_slot -= 1;
            true
        } else {
            false
        };

        let drop_at_slot = policy.drop_at_send_count.get_mut(dir);
        let drop_from_at = if *drop_at_slot == Some(count) {
            *drop_at_slot = None;
            true
        } else {
            false
        };

        let drop_from_prob = if let Some(prob) = policy.drop_probability.get(dir) {
            prob.roll(policy.rng_mut(dir))
        } else {
            false
        };

        let drop_this = drop_from_next || drop_from_at || drop_from_prob;
        if drop_this {
            *policy.drop_count.get_mut(dir) += 1;
        }

        let latency = policy.latency.get(dir);

        // ── Duplicate (cyclic + one-shot, additive) ───────────────
        let mut extra_copies = 0u32;

        if let Some(n) = policy.duplicate_every.get(dir) {
            if n > 0 {
                let dup_count = policy.duplicate_count.get_mut(dir);
                *dup_count += 1;
                if *dup_count >= n {
                    *dup_count = 0;
                    extra_copies += 1;
                }
            }
        }

        let one_shot = policy.duplicate_next_count.get_mut(dir);
        if *one_shot > 0 {
            extra_copies += *one_shot;
            *one_shot = 0;
        }

        // ── Reorder (pair-mode takes precedence over buffer-mode) ─
        //
        // Splits the wire emit into `current_packet` (the one passed in,
        // gets duplication applied) and `flushed_held` (previously
        // buffered packets, each emitted exactly once). When the policy
        // buffers this send, `current_packet` is None and nothing goes
        // on the wire this call.
        let (current_packet, flushed_held): (Option<Bytes>, Vec<Bytes>) = if drop_this {
            (None, Vec::new())
        } else if policy.reorder_pairs.get(dir) {
            let held_slot = policy.reorder_held.get_mut(dir);
            match held_slot.take() {
                None => {
                    *held_slot = Some(bytes);
                    (None, Vec::new())
                }
                Some(prev) => {
                    // Pair complete: current goes on the wire first
                    // (with its duplications), then the previously
                    // held packet (no duplication).
                    (Some(bytes), vec![prev])
                }
            }
        } else {
            let buf_size = policy.reorder_buffer_size.get(dir);
            if buf_size > 0 {
                let buf = policy.reorder_buffer.get_mut(dir);
                buf.push(bytes);
                if buf.len() as u32 >= buf_size {
                    // Buffer full: drain reversed. Last-pushed (the
                    // current send) is at the front after reverse, so
                    // it gets duplication; the rest of the buffer
                    // (held packets) emit once each.
                    let mut drained: Vec<Bytes> = std::mem::take(buf);
                    drained.reverse();
                    let current = drained.remove(0);
                    (Some(current), drained)
                } else {
                    (None, Vec::new())
                }
            } else {
                // No reorder: the simple case — current packet, no held.
                (Some(bytes), Vec::new())
            }
        };

        SendDecision {
            drop_this,
            latency,
            extra_copies,
            current_packet,
            flushed_held,
        }
    }

    /// Flush any pending bytes held by reorder buffers (both pair and
    /// multi-packet modes) on this peer's outbound direction. Used by
    /// tests that need to assert behavior on a partially-full reorder
    /// buffer (e.g., flushing 3 packets when `reorder_buffer_size = 4`
    /// to prove the held packets aren't lost on session teardown).
    /// Drained packets are sent in **reverse arrival order**, matching
    /// the auto-flush behavior.
    pub async fn flush_reorder_buffer(&self) -> std::io::Result<()> {
        let drained: Vec<Bytes> = {
            let mut policy = self.policy.lock().expect("policy poisoned");
            let dir = self.direction;
            let mut pair_held: Vec<Bytes> = policy
                .reorder_held
                .get_mut(dir)
                .take()
                .into_iter()
                .collect();
            let mut multi: Vec<Bytes> = std::mem::take(policy.reorder_buffer.get_mut(dir));
            multi.reverse();
            pair_held.append(&mut multi);
            pair_held
        };
        for raw in drained {
            self.socket.send_to(&raw, self.peer_addr).await?;
        }
        Ok(())
    }

    /// Drive one tick: process retransmits and emit a keepalive if due.
    /// Returns the punch list of bytes that went on the wire (subject
    /// to outbound policy).
    pub async fn tick(&self) -> std::io::Result<TickActions> {
        let mut actions = TickActions::default();

        let retransmits = {
            let mut channel = self.channel.lock().expect("channel poisoned");
            channel.check_timeouts()
        };
        for bytes in &retransmits {
            self.send_with_policy(bytes.clone()).await?;
        }
        actions.retransmits = retransmits;

        let keepalive_due = {
            let channel = self.channel.lock().expect("channel poisoned");
            channel.keepalive_due()
        };
        if keepalive_due {
            // Empty-body keepalive on the channel. Flags = ON_CHANNEL only;
            // no sequence, no reliable obligation. Production's keepalive
            // shape from `tick_sync.rs`.
            let pkt = build_outgoing(FLAG_ON_CHANNEL, &[], None, &[], None);
            let plaintext = pkt.freeze();
            let raw = match &self.encryption {
                Some(enc) => Bytes::from(
                    enc.encrypt(&plaintext)
                        .expect("encryption failed in harness"),
                ),
                None => plaintext.clone(),
            };
            self.send_with_policy(raw.clone()).await?;
            self.channel.lock().expect("channel poisoned").touch_sent();
            actions.keepalives.push(raw);
        }

        Ok(actions)
    }

    /// Wait until the inbox holds `n` reassembled bundles, or `timeout`
    /// elapses. Returns the drained bundles in arrival order. If the
    /// timeout fires first, returns however many bundles arrived.
    pub async fn recv_n_bundles(&self, n: usize, timeout: Duration) -> Vec<Bytes> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            {
                let mut inbox = self.inbox.lock().expect("inbox poisoned");
                if inbox.len() >= n {
                    return inbox.drain(..n).collect();
                }
            }
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                let mut inbox = self.inbox.lock().expect("inbox poisoned");
                return inbox.drain(..).collect();
            }
            let notified = self.inbox_notify.notified();
            tokio::select! {
                _ = notified => continue,
                _ = tokio::time::sleep(remaining) => continue,
            }
        }
    }

    /// Snapshot of the inbox without draining — useful for assertions
    /// that need to read without consuming.
    pub fn inbox_len(&self) -> usize {
        self.inbox.lock().expect("inbox poisoned").len()
    }

    /// Snapshot of pending acks count — exposed so tests can assert
    /// quiescence ("no acks owed to peer").
    pub fn pending_acks_len(&self) -> usize {
        self.pending_acks
            .lock()
            .expect("pending_acks poisoned")
            .len()
    }

    /// Snapshot of the channel's TX-window depth — exposed so tests can
    /// assert quiescence ("nothing left to retransmit").
    pub fn tx_window_len(&self) -> usize {
        self.channel
            .lock()
            .expect("channel poisoned")
            .tx_window
            .len()
    }

    /// Tear down the recv pump. Idempotent.
    pub fn shutdown(self) {
        self.recv_task.abort();
    }
}

impl Drop for LoopbackPeer {
    fn drop(&mut self) {
        self.recv_task.abort();
    }
}

/// Spawn the per-peer recv pump. Reads bytes from the socket, decrypts
/// (if encryption is set), parses, feeds packets into the channel,
/// queues acks for piggyback on the next send, and pushes reassembled
/// bundles onto the inbox.
fn spawn_recv_pump(
    socket: Arc<UdpSocket>,
    channel: Arc<Mutex<Channel>>,
    encryption: Option<Arc<MercuryEncryption>>,
    inbox: Arc<Mutex<VecDeque<Bytes>>>,
    inbox_notify: Arc<Notify>,
    pending_acks: Arc<Mutex<Vec<u32>>>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut buf = vec![0u8; crate::consts::PACKET_MAX_SIZE];
        loop {
            let (n, _src) = match socket.recv_from(&mut buf).await {
                Ok(pair) => pair,
                Err(_) => return,
            };
            let raw = &buf[..n];

            let plaintext = match &encryption {
                Some(enc) => match enc.decrypt(raw) {
                    Ok(v) => v,
                    Err(_) => continue,
                },
                None => raw.to_vec(),
            };

            let pkt = match parse_incoming(&plaintext) {
                Ok(p) => p,
                Err(_) => continue,
            };

            // Drive ack processing first — acks on inbound packets free
            // TX-window slots so a retransmit-soaked channel recovers
            // before the next tick.
            //
            // Mercury uses cumulative-ack semantics (a single ack seq
            // implicitly acks all predecessors), so we pass the highest
            // value from the ack footer to `process_acks` and let it
            // drain every TX-window entry with `seq <= highest`. The
            // `max()` is the right operation precisely because an ack
            // list with `[5, 3, 7]` semantically means "everything up
            // through 7 is acked", not "exactly these three seqs".
            if pkt.has_acks() {
                if let Some(highest) = pkt.acks.iter().copied().max() {
                    let mut ch = channel.lock().expect("channel poisoned");
                    let _ = ch.process_acks(highest);
                }
            }

            // Reliable inbound packets owe a piggyback ack to the peer
            // on the next outbound send.
            if pkt.is_reliable() {
                if let Some(seq) = pkt.seq_id {
                    let mut acks = pending_acks.lock().expect("pending_acks poisoned");
                    acks.push(seq);
                }
            }

            // Fragment reassembly or pass-through. `reassemble_parsed`
            // returns `Ok(None)` for "buffering, no complete bundle yet"
            // and `Ok(Some(bytes))` when the bundle is assembled.
            let delivered: Option<Bytes> = if pkt.is_fragmented() {
                let mut ch = channel.lock().expect("channel poisoned");
                ch.reassemble_parsed(&pkt).unwrap_or_default()
            } else {
                // Non-fragmented: the body IS the bundle. Still mark
                // receive-side activity for the keepalive / inactivity
                // timers.
                let mut ch = channel.lock().expect("channel poisoned");
                ch.touch_received();
                Some(pkt.body.clone())
            };

            if let Some(body) = delivered {
                let mut inbox = inbox.lock().expect("inbox poisoned");
                inbox.push_back(body);
                inbox_notify.notify_one();
            }
        }
    })
}
