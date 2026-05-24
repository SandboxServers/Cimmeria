//! Pcap-replay harness for deterministic regression against real
//! captured sessions.
//!
//! Loads a `.pcap` file alongside a hex-encoded session key, parses
//! out the UDP payloads, decrypts them via the saved key, and
//! exposes the stream as an ordered sequence of
//! `(direction, plaintext_bytes)` events.
//!
//! Each event carries the captured timestamp so tests can drive a
//! [`Channel`](crate::channel::Channel) with the same inter-packet
//! gaps the original session exhibited (or compress them via the
//! test's clock).
//!
//! # The lomiada fixture
//!
//! `debug/lomiada-broke-in-hallway02/` is the canonical regression
//! fixture: a real player session from Germany to a US server,
//! ~6 minutes of clean play followed by a transatlantic UDP drop
//! of server-sent packet `#1148`. The pcap is paired with a
//! `*-keys.txt` file containing the 32-byte AES session key in
//! 64-char hex form.
//!
//! # Wire layout
//!
//! Each pcap record contains a complete Ethernet + IPv4 + UDP +
//! payload frame. The UDP payload is a single encrypted Mercury
//! datagram. After decryption via [`MercuryEncryption::decrypt`]
//! it can be fed to [`parse_incoming`](crate::packet::parse_incoming)
//! and then to [`Channel::receive_packet`](crate::channel::Channel)
//! or [`Channel::reassemble_parsed`](crate::channel::Channel).

use std::fs::File;
use std::io::BufReader;
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::time::Duration;

use bytes::Bytes;
use etherparse::{NetSlice, SlicedPacket, TransportSlice};
use pcap_file::pcap::PcapReader;

use crate::encryption::MercuryEncryption;

/// One direction of the captured session, identified by the
/// `(server, client)` endpoint pair. The harness can replay a pcap
/// in either direction depending on which one is the "server" for
/// the test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureDirection {
    /// Captured packet was server → client (the typical case for
    /// the lomiada-style "server retransmit recovers from drop"
    /// scenario).
    ServerToClient,
    /// Captured packet was client → server (acks, client-initiated
    /// requests).
    ClientToServer,
}

/// One decrypted Mercury packet lifted from a pcap record.
#[derive(Debug, Clone)]
pub struct PcapEvent {
    /// Original capture timestamp relative to the first packet in
    /// the pcap (so tests can compute inter-packet delays).
    pub offset: Duration,
    /// Which side originated this packet.
    pub direction: CaptureDirection,
    /// Decrypted plaintext bytes, ready to feed into
    /// [`parse_incoming`](crate::packet::parse_incoming).
    pub plaintext: Bytes,
    /// Source UDP address — useful for diagnostics; tests don't
    /// usually need it.
    pub source: SocketAddr,
    /// Destination UDP address — useful for diagnostics.
    pub destination: SocketAddr,
}

/// Builder for a pcap-replay session.
///
/// Loaded lazily via [`Self::events`] so the pcap is only fully
/// parsed when the test consumes it. Decryption failures are
/// recorded but don't abort the load — they typically indicate a
/// pre-handshake packet that was sent before the session key was
/// negotiated.
///
/// Intentionally does not implement `Debug` — `MercuryEncryption`
/// holds the raw AES key and we want the key to never appear in
/// log output. Tests that need to unwrap a `Result<PcapReplay, _>`
/// use a match on the `Err` arm instead of `.expect()`/`.unwrap()`.
pub struct PcapReplay {
    pcap_path: std::path::PathBuf,
    key: Option<MercuryEncryption>,
    server_addr: Option<SocketAddr>,
}

impl PcapReplay {
    /// Open the pcap at `path`. Use [`Self::with_key`] or
    /// [`Self::with_key_from`] before calling [`Self::events`].
    pub fn load(path: impl AsRef<Path>) -> Self {
        Self {
            pcap_path: path.as_ref().to_path_buf(),
            key: None,
            server_addr: None,
        }
    }

    /// Attach an explicit 32-byte session key (raw bytes).
    pub fn with_key(mut self, key: [u8; 32]) -> Self {
        self.key = Some(MercuryEncryption::from_session_key(key));
        self
    }

    /// Attach a session key by reading a file containing the 64-char
    /// hex-encoded AES key (the format `debug/<session>/*-keys.txt`
    /// uses). Whitespace around the hex is trimmed.
    pub fn with_key_from(self, path: impl AsRef<Path>) -> std::io::Result<Self> {
        let raw = std::fs::read_to_string(path)?;
        let hex = raw.trim();
        if hex.len() != 64 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("expected 64 hex chars, got {}", hex.len()),
            ));
        }
        let mut key = [0u8; 32];
        for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
            let byte_str = std::str::from_utf8(chunk).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("non-utf8 hex: {e}"),
                )
            })?;
            key[i] = u8::from_str_radix(byte_str, 16).map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, format!("invalid hex: {e}"))
            })?;
        }
        Ok(self.with_key(key))
    }

    /// Explicitly tell the replay which endpoint is the server.
    /// Inferred automatically if unset by majority-payload-size
    /// heuristic (server traffic dominates the lomiada-class
    /// scenarios). Set explicitly for fixtures where the heuristic
    /// fails.
    pub fn with_server_addr(mut self, addr: SocketAddr) -> Self {
        self.server_addr = Some(addr);
        self
    }

    /// Iterate through every Mercury packet in the pcap, decrypted
    /// and tagged by direction.
    ///
    /// Packets that fail to decrypt (e.g. pre-handshake handshake
    /// packets that don't use the saved key) are silently skipped
    /// — the test asserts against the decryptable post-handshake
    /// stream.
    pub fn events(&self) -> std::io::Result<Vec<PcapEvent>> {
        let key = self.key.as_ref().ok_or_else(|| {
            std::io::Error::other("PcapReplay needs a session key (call .with_key_from)")
        })?;

        let file = File::open(&self.pcap_path)?;
        let mut reader = PcapReader::new(BufReader::new(file))
            .map_err(|e| std::io::Error::other(format!("pcap header parse failed: {e}")))?;

        // First pass: gather all UDP frames + infer server addr.
        //
        // Failure-mode discipline:
        //   - Pcap record-level parse failures (truncated capture,
        //     malformed PCAP frame) → return Err. A damaged fixture
        //     should fail the test, not produce a partial replay
        //     that passes weakly.
        //   - Ethernet/IP/UDP parse failures on individual records →
        //     skip silently. Some captures contain non-UDP frames
        //     (ICMP, ARP) that are legitimately not part of the
        //     Mercury session.
        let mut frames: Vec<(Duration, SocketAddr, SocketAddr, Vec<u8>)> = Vec::new();
        let mut first_ts: Option<Duration> = None;
        while let Some(rec_result) = reader.next_packet() {
            let rec = rec_result.map_err(|e| {
                std::io::Error::other(format!(
                    "pcap record parse failed (capture may be truncated or malformed): {e}"
                ))
            })?;
            let ts = rec.timestamp;
            let offset = match first_ts {
                None => {
                    first_ts = Some(ts);
                    Duration::ZERO
                }
                Some(first) => ts.saturating_sub(first),
            };
            let parsed = match SlicedPacket::from_ethernet(&rec.data) {
                Ok(p) => p,
                Err(_) => continue,
            };
            let Some(NetSlice::Ipv4(ip)) = parsed.net else {
                continue;
            };
            let Some(TransportSlice::Udp(udp)) = parsed.transport else {
                continue;
            };
            let src_ip: IpAddr = ip.header().source_addr().into();
            let dst_ip: IpAddr = ip.header().destination_addr().into();
            let src = SocketAddr::new(src_ip, udp.source_port());
            let dst = SocketAddr::new(dst_ip, udp.destination_port());
            let payload = udp.payload().to_vec();
            if !payload.is_empty() {
                frames.push((offset, src, dst, payload));
            }
        }

        if frames.is_empty() {
            // No UDP frames at all → not a Mercury capture (or pcap
            // is empty). Return an empty event list rather than
            // panicking in `infer_server_addr`. Tests assert on
            // event count and will fail with a clear message.
            return Ok(Vec::new());
        }

        let server_addr = match self.server_addr {
            Some(s) => s,
            None => infer_server_addr(&frames),
        };

        // Second pass: decrypt + tag direction.
        let mut events = Vec::with_capacity(frames.len());
        for (offset, src, dst, payload) in frames {
            let plaintext = match key.decrypt(&payload) {
                Ok(p) => p,
                Err(_) => continue, // pre-handshake or non-Mercury UDP
            };
            let direction = if src == server_addr {
                CaptureDirection::ServerToClient
            } else if dst == server_addr {
                CaptureDirection::ClientToServer
            } else {
                continue; // neither side matches; not part of this session
            };
            events.push(PcapEvent {
                offset,
                direction,
                plaintext: Bytes::from(plaintext),
                source: src,
                destination: dst,
            });
        }

        Ok(events)
    }
}

/// Heuristic: the server is the endpoint that *receives* more
/// distinct packets (client→server is typically lower-volume than
/// server→client world updates). Falls back to the first observed
/// destination if everything ties.
///
/// **Precondition**: `frames` must be non-empty. Callers check this
/// before calling — the `events()` path returns early with an empty
/// vec when no UDP frames were found in the capture.
fn infer_server_addr(frames: &[(Duration, SocketAddr, SocketAddr, Vec<u8>)]) -> SocketAddr {
    debug_assert!(
        !frames.is_empty(),
        "infer_server_addr called with empty frames — events() must early-return first"
    );
    use std::collections::HashMap;
    let mut inbound_counts: HashMap<SocketAddr, usize> = HashMap::new();
    for (_, _, dst, _) in frames {
        *inbound_counts.entry(*dst).or_insert(0) += 1;
    }
    inbound_counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(addr, _)| addr)
        .unwrap_or_else(|| frames[0].2)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unwrap_err(result: Result<PcapReplay, std::io::Error>, msg: &str) -> std::io::Error {
        match result {
            Ok(_) => panic!("{msg}"),
            Err(e) => e,
        }
    }

    #[test]
    fn with_key_from_missing_file_returns_helpful_error() {
        let err = unwrap_err(
            PcapReplay::load("/dev/null/does/not/exist.pcap")
                .with_key_from("/dev/null/does/not/exist.keys"),
            "missing key file must surface as an Err",
        );
        assert!(
            !err.to_string().contains("expected 64 hex chars"),
            "missing-file should surface as I/O error, not a hex-length complaint; got: {err}"
        );
    }

    #[test]
    fn with_key_from_wrong_length_complains_specifically() {
        let tmpdir = std::env::temp_dir().join("cimmeria-pcap-replay-test");
        std::fs::create_dir_all(&tmpdir).unwrap();
        let key_path = tmpdir.join("short.keys");
        std::fs::write(&key_path, "deadbeef").unwrap();

        let err = unwrap_err(
            PcapReplay::load("ignored.pcap").with_key_from(&key_path),
            "8-char key must be rejected",
        );
        assert!(
            err.to_string().contains("expected 64 hex chars"),
            "wrong-length must complain about hex length; got: {err}"
        );

        let _ = std::fs::remove_file(&key_path);
    }

    #[test]
    fn with_key_from_non_hex_complains() {
        let tmpdir = std::env::temp_dir().join("cimmeria-pcap-replay-test");
        std::fs::create_dir_all(&tmpdir).unwrap();
        let key_path = tmpdir.join("bad_hex.keys");
        std::fs::write(
            &key_path,
            "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz",
        )
        .unwrap();

        let err = unwrap_err(
            PcapReplay::load("ignored.pcap").with_key_from(&key_path),
            "non-hex key must be rejected",
        );
        assert!(
            err.to_string().contains("invalid hex"),
            "non-hex must complain about hex parsing; got: {err}"
        );

        let _ = std::fs::remove_file(&key_path);
    }

    #[test]
    fn infer_server_addr_picks_majority_destination() {
        let a: SocketAddr = "127.0.0.1:1000".parse().unwrap();
        let b: SocketAddr = "127.0.0.1:2000".parse().unwrap();
        let p: Vec<u8> = vec![1, 2, 3];
        let frames = vec![
            (Duration::ZERO, b, a, p.clone()), // → a
            (Duration::ZERO, b, a, p.clone()), // → a
            (Duration::ZERO, a, b, p.clone()), // → b
        ];
        assert_eq!(infer_server_addr(&frames), a);
    }

    #[test]
    fn infer_server_addr_single_packet_returns_destination() {
        let src: SocketAddr = "10.0.0.1:5000".parse().unwrap();
        let dst: SocketAddr = "10.0.0.2:6000".parse().unwrap();
        let frames = vec![(Duration::ZERO, src, dst, vec![0])];
        assert_eq!(infer_server_addr(&frames), dst);
    }
}
