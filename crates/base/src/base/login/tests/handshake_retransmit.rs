//! #842: the login reply (seq 1) and the time-sync bundle (seq 2) are
//! covered by the session channel's retransmit scan.
//!
//! Both go out with `FLAG_RELIABLE` before the client's channel exists.
//! They used to be raw sends only, never entered in the channel's TX
//! window, so a lost one was never resent: a lost reply hung the login,
//! a lost time-sync left a permanent gap at the head of the client's
//! reliable stream.
//!
//! The chaos guards run the real receive loop (`run_connect_loop`) on a
//! real loopback socket behind a transport that drops one reliable
//! sequence once (TESTING.md type 10). A plain UDP socket plays the
//! client: it sends the plaintext `baseAppLogin`, waits for both handshake
//! packets, then acks them the way the SGW client does in the captured
//! logins (an ack-only packet, `acks [2, 1]`). With the raw sends put back,
//! the dropped packet never arrives and the guards time out.

use std::time::Duration;

use async_trait::async_trait;
use cimmeria_mercury::encryption::MercuryEncryption;
use cimmeria_mercury::packet::{
    build_outgoing, parse_incoming, FLAG_HAS_ACKS, FLAG_HAS_SEQUENCE, FLAG_ON_CHANNEL,
    FLAG_RELIABLE,
};
use cimmeria_mercury::transport::{BidirectionalTransport, UdpTransport};
use tokio::net::UdpSocket;

use super::*;
use crate::base::connect_loop::run_connect_loop;
use crate::base::login::{CONNECT_REPLY_SEQ, TIME_SYNC_SEQ};

const ACCOUNT_ID: u32 = 0x0842_0001;
const KEY_BYTE: u8 = 0x84;
const REQUEST_ID: u32 = 0x0000_8421;

/// Longest a guard waits for the dropped packet. The first resend comes
/// one initial RTO (1.5 s) after login, on the next 100 ms tick.
const RESEND_DEADLINE: Duration = Duration::from_secs(8);

/// Sequences still outstanding in the session channel (TX window plus
/// deferred queue), in window order.
fn outstanding_seqs(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
) -> Vec<u32> {
    let map = connected.lock().unwrap();
    let channel = map[&addr].channel.lock().unwrap();
    channel
        .tx_window
        .iter()
        .chain(channel.unsent_packets.iter())
        .map(|e| e.packet.sequence)
        .collect()
}

/// `handle_login` puts both handshake datagrams, byte for byte as sent,
/// into the new channel's TX window at seqs 1 and 2. This is the
/// bookkeeping the retransmit scan reads; the byte-exact wire test in
/// `mod.rs` pins that the datagrams themselves did not change.
#[tokio::test]
async fn login_puts_connect_reply_and_time_sync_in_the_tx_window() {
    let transport = Arc::new(TestTransport::new());
    let dyn_transport: Arc<dyn Transport> = transport.clone();
    let addr: SocketAddr = "127.0.0.1:58421".parse().unwrap();
    let pending_logins = Arc::new(Mutex::new(HashMap::new()));
    let connected = Arc::new(Mutex::new(HashMap::new()));
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));

    let pending = make_pending_login(ACCOUNT_ID, KEY_BYTE);
    let ticket = pending.ticket.clone();
    pending_logins
        .lock()
        .unwrap()
        .insert(ticket.clone(), pending);

    handle_login(
        &dyn_transport,
        addr,
        REQUEST_ID,
        &ticket,
        &pending_logins,
        &connected,
        &entity_manager,
        &None,
        &entity_to_addr,
        &None,
        cimmeria_mercury::encryption::EncryptionVersion::V1,
    )
    .await
    .expect("Phase 3 handoff");
    cancel_session(&connected, addr);

    let sent = transport.drain();
    assert_eq!(sent.len(), 2, "connect_reply + time_sync");

    let map = connected.lock().unwrap();
    let channel = map[&addr].channel.lock().unwrap();
    let window: Vec<(u32, Vec<u8>)> = channel
        .tx_window
        .iter()
        .map(|e| (e.packet.sequence, e.raw_bytes.to_vec()))
        .collect();
    assert_eq!(
        window,
        vec![
            (CONNECT_REPLY_SEQ, sent[0].1.clone()),
            (TIME_SYNC_SEQ, sent[1].1.clone()),
        ],
        "both handshake datagrams must be tracked for retransmit, with the exact bytes sent"
    );
    assert!(
        channel
            .tx_window
            .iter()
            .all(|e| e.packet.flags.is_reliable()),
        "the tracked entries carry the reliable flag the datagrams went out with"
    );
}

/// Server transport that drops the first datagram to `client` whose
/// reliable sequence is `drop_seq`, and passes everything else through.
struct DropReliableSeqOnce {
    inner: UdpTransport,
    enc: MercuryEncryption,
    client: SocketAddr,
    drop_seq: u32,
    dropped: Mutex<Option<Vec<u8>>>,
}

impl DropReliableSeqOnce {
    fn is_target(&self, bytes: &[u8], addr: SocketAddr) -> bool {
        if addr != self.client || self.dropped.lock().unwrap().is_some() {
            return false;
        }
        let Ok(plain) = self.enc.decrypt(bytes) else {
            return false;
        };
        let Ok(pkt) = parse_incoming(&plain) else {
            return false;
        };
        pkt.flags & FLAG_RELIABLE != 0 && pkt.seq_id == Some(self.drop_seq)
    }
}

#[async_trait]
impl Transport for DropReliableSeqOnce {
    async fn send_to(&self, bytes: &[u8], addr: SocketAddr) -> std::io::Result<usize> {
        if self.is_target(bytes, addr) {
            *self.dropped.lock().unwrap() = Some(bytes.to_vec());
            return Ok(bytes.len());
        }
        self.inner.send_to(bytes, addr).await
    }

    fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.inner.local_addr()
    }
}

#[async_trait]
impl BidirectionalTransport for DropReliableSeqOnce {
    async fn recv_from(&self, buf: &mut [u8]) -> std::io::Result<(usize, SocketAddr)> {
        self.inner.recv_from(buf).await
    }
}

/// The client's plaintext `baseAppLogin`: flags `0x41`, msg `0x00` with a
/// u16 length of 25, the request header (reply id + next-request offset),
/// account id, ticket length, the 20-character ticket, then the footers
/// (first-request offset, sequence 1).
fn plaintext_base_app_login(ticket: &str) -> Vec<u8> {
    let mut raw = vec![0x41, 0x00];
    raw.extend_from_slice(&25u16.to_le_bytes());
    raw.extend_from_slice(&REQUEST_ID.to_le_bytes());
    raw.extend_from_slice(&0u16.to_le_bytes());
    raw.extend_from_slice(&ACCOUNT_ID.to_le_bytes());
    raw.push(ticket.len() as u8);
    raw.extend_from_slice(ticket.as_bytes());
    raw.extend_from_slice(&1u16.to_le_bytes());
    raw.extend_from_slice(&1u32.to_le_bytes());
    raw
}

/// Log in over real UDP with `drop_seq` lost once on the way to the
/// client, and check the client still ends up with both handshake packets,
/// the lost one as a byte-exact resend, and that its ACK retires them.
async fn lost_handshake_packet_is_resent(drop_seq: u32) {
    let key = [KEY_BYTE; 32];
    let enc = MercuryEncryption::from_session_key(key);

    let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let client_addr = client.local_addr().unwrap();
    let server_socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
    let server_addr = server_socket.local_addr().unwrap();
    let transport = Arc::new(DropReliableSeqOnce {
        inner: UdpTransport::new(server_socket),
        enc: enc.clone(),
        client: client_addr,
        drop_seq,
        dropped: Mutex::new(None),
    });

    let pending_logins = Arc::new(Mutex::new(HashMap::new()));
    let connected = Arc::new(Mutex::new(HashMap::new()));
    let pending = make_pending_login(ACCOUNT_ID, KEY_BYTE);
    let ticket = pending.ticket.clone();
    pending_logins
        .lock()
        .unwrap()
        .insert(ticket.clone(), pending);

    let server = tokio::spawn(run_connect_loop(
        transport.clone(),
        pending_logins,
        None,
        None,
        None,
        Arc::clone(&connected),
        Arc::new(Mutex::new(EntityManager::new())),
        Arc::new(Mutex::new(HashMap::new())),
        cimmeria_mercury::encryption::EncryptionVersion::V1,
    ));

    client
        .send_to(&plaintext_base_app_login(&ticket), server_addr)
        .await
        .unwrap();

    // Reliable arrivals in order: (seq, datagram). Tick-sync heartbeats
    // are unreliable and skipped.
    let mut arrivals: Vec<(u32, Vec<u8>)> = Vec::new();
    let mut buf = [0u8; 2048];
    let deadline = tokio::time::Instant::now() + RESEND_DEADLINE;
    let have = |a: &Vec<(u32, Vec<u8>)>, s: u32| a.iter().any(|(seq, _)| *seq == s);
    while !(have(&arrivals, CONNECT_REPLY_SEQ) && have(&arrivals, TIME_SYNC_SEQ)) {
        let Ok(Ok((n, _))) = tokio::time::timeout_at(deadline, client.recv_from(&mut buf)).await
        else {
            break;
        };
        let Ok(plain) = enc.decrypt(&buf[..n]) else {
            continue;
        };
        let Ok(pkt) = parse_incoming(&plain) else {
            continue;
        };
        if pkt.flags & FLAG_RELIABLE != 0 {
            if let Some(seq) = pkt.seq_id {
                arrivals.push((seq, buf[..n].to_vec()));
            }
        }
    }

    let dropped = transport
        .dropped
        .lock()
        .unwrap()
        .clone()
        .expect("the transport must have dropped the target packet");
    let seqs: Vec<u32> = arrivals.iter().map(|(s, _)| *s).collect();
    let resent = arrivals
        .iter()
        .find(|(s, _)| *s == drop_seq)
        .unwrap_or_else(|| {
            panic!(
                "seq {drop_seq} was lost once and never resent within {RESEND_DEADLINE:?}; \
                 the client saw reliable seqs {seqs:?}"
            )
        });
    assert_eq!(
        resent.1, dropped,
        "the resend is the lost datagram, byte for byte"
    );
    let other = if drop_seq == CONNECT_REPLY_SEQ {
        TIME_SYNC_SEQ
    } else {
        CONNECT_REPLY_SEQ
    };
    let pos = |s: u32| seqs.iter().position(|x| *x == s).unwrap();
    assert!(
        pos(other) < pos(drop_seq),
        "seq {drop_seq} can only have arrived as a resend, after seq {other}: {seqs:?}"
    );

    // Nothing has acked them yet, so the retire check below cannot pass
    // vacuously.
    let before = outstanding_seqs(&connected, client_addr);
    assert!(
        before.contains(&CONNECT_REPLY_SEQ) && before.contains(&TIME_SYNC_SEQ),
        "both handshake packets are outstanding before the client acks: {before:?}"
    );

    // The client's ACK, in the shape the captured logins show: an
    // ack-only, unreliable packet on the nub counter (seq 2), `acks [2, 1]`.
    let ack = build_outgoing(
        FLAG_HAS_SEQUENCE | FLAG_ON_CHANNEL | FLAG_HAS_ACKS,
        &[],
        Some(2),
        &[TIME_SYNC_SEQ, CONNECT_REPLY_SEQ],
        None,
    );
    client
        .send_to(&enc.encrypt(&ack).unwrap(), server_addr)
        .await
        .unwrap();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    loop {
        let outstanding = outstanding_seqs(&connected, client_addr);
        if !outstanding.contains(&CONNECT_REPLY_SEQ) && !outstanding.contains(&TIME_SYNC_SEQ) {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "the client's ACK must retire both handshake packets; still outstanding: {outstanding:?}"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    cancel_session(&connected, client_addr);
    server.abort();
}

/// A lost login reply (seq 1) is resent, so the client can finish login.
#[tokio::test]
async fn lost_connect_reply_is_resent_until_the_client_has_it() {
    lost_handshake_packet_is_resent(CONNECT_REPLY_SEQ).await;
}

/// A lost time-sync bundle (seq 2) is resent, so the client's reliable
/// stream has no gap at its head.
#[tokio::test]
async fn lost_time_sync_is_resent_until_the_client_has_it() {
    lost_handshake_packet_is_resent(TIME_SYNC_SEQ).await;
}
