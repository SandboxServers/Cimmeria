//! tokio-util codec adapters for Mercury UDP packets.
//!
//! `MercuryCodec` wraps the packet encode/decode logic from [`crate::packet`]
//! with the optional encryption layer from [`crate::encryption`]. It
//! implements the `tokio_util::codec::{Decoder, Encoder}` traits so that
//! Mercury can be used with `tokio_util::udp::UdpFramed` (or manual
//! encode/decode in a recv loop).
//!
//! When encryption is enabled, the codec:
//! - **Encoding:** serialises the packet bytes, encrypts everything, appends HMAC.
//! - **Decoding:** verifies HMAC, decrypts, passes to packet parser.

use bytes::{Bytes, BytesMut};
use cimmeria_common::{CimmeriaError, Result};
use tokio_util::codec::{Decoder, Encoder};

use crate::encryption::MercuryEncryption;
use crate::packet::{parse_incoming, Packet, PacketFlags};

// ── MercuryCodec ────────────────────────────────────────────────────────────

/// Codec that encodes/decodes Mercury UDP packets, with optional encryption.
pub struct MercuryCodec {
    /// Optional encryption context. `None` for unencrypted connections
    /// (e.g., during the initial handshake before key exchange).
    encryption: Option<MercuryEncryption>,
}

impl MercuryCodec {
    /// Create a codec with no encryption (plaintext mode).
    pub fn new() -> Self {
        Self { encryption: None }
    }

    /// Create a codec with encryption enabled.
    pub fn with_encryption(encryption: MercuryEncryption) -> Self {
        Self {
            encryption: Some(encryption),
        }
    }

    /// Enable or replace the encryption context.
    ///
    /// This is called after the login handshake completes and key material
    /// has been exchanged.
    pub fn set_encryption(&mut self, encryption: MercuryEncryption) {
        self.encryption = Some(encryption);
    }

    /// Disable encryption (revert to plaintext).
    pub fn clear_encryption(&mut self) {
        self.encryption = None;
    }

    /// Returns `true` if encryption is currently active.
    pub fn is_encrypted(&self) -> bool {
        self.encryption.is_some()
    }
}

impl Default for MercuryCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl Decoder for MercuryCodec {
    type Item = Packet;
    type Error = CimmeriaError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>> {
        if src.is_empty() {
            return Ok(None);
        }

        // For UDP, each datagram is a complete packet.
        let raw_bytes = src.split().freeze();

        // If encryption is active, decrypt first.
        let plaintext: Bytes = if let Some(ref enc) = self.encryption {
            Bytes::from(enc.decrypt(&raw_bytes)?)
        } else {
            raw_bytes
        };

        // Parse the decrypted bytes into a Packet.
        let parsed = parse_incoming(&plaintext)?;
        let pkt = Packet::new(
            PacketFlags::from_byte(parsed.flags),
            parsed.seq_id.unwrap_or(0),
            parsed.body,
        );

        Ok(Some(pkt))
    }
}

impl Encoder<Packet> for MercuryCodec {
    type Error = CimmeriaError;

    fn encode(&mut self, packet: Packet, dst: &mut BytesMut) -> Result<()> {
        let wire = packet.encode();

        if let Some(ref enc) = self.encryption {
            let ciphertext = enc.encrypt(&wire)?;
            dst.extend_from_slice(&ciphertext);
        } else {
            dst.extend_from_slice(&wire);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::FLAG_RELIABLE;

    #[test]
    fn round_trip_plaintext() {
        use crate::packet::FLAG_HAS_SEQUENCE;

        let mut codec = MercuryCodec::new();
        // Include FLAG_HAS_SEQUENCE so encode() writes the seq_id footer and
        // decode() (via parse_incoming) correctly strips it.
        let flags = PacketFlags::default()
            .with(FLAG_RELIABLE)
            .with(FLAG_HAS_SEQUENCE);
        let original = Packet::new(flags, 10, Bytes::from_static(b"plaintext body"));

        let mut buf = BytesMut::new();
        codec.encode(original.clone(), &mut buf).unwrap();

        let decoded = codec.decode(&mut buf).unwrap().unwrap();
        assert_eq!(decoded.sequence, 10);
        assert!(decoded.flags.is_reliable());
        assert_eq!(decoded.body.as_ref(), b"plaintext body");
    }

    #[test]
    fn round_trip_encrypted() {
        use crate::packet::FLAG_HAS_SEQUENCE;

        let enc = MercuryEncryption::new([0xAA; 32], [0xBB; 16], [0xCC; 32]);
        let mut codec = MercuryCodec::with_encryption(enc);

        let flags = PacketFlags::default().with(FLAG_HAS_SEQUENCE);
        let original = Packet::new(flags, 99, Bytes::from_static(b"secret message"));

        let mut buf = BytesMut::new();
        codec.encode(original.clone(), &mut buf).unwrap();

        // The encoded bytes should NOT contain the plaintext.
        let wire_bytes = buf.clone().freeze();
        assert!(!wire_bytes.windows(14).any(|w| w == b"secret message"));

        let decoded = codec.decode(&mut buf).unwrap().unwrap();
        assert_eq!(decoded.sequence, 99);
        assert_eq!(decoded.body.as_ref(), b"secret message");
    }

    #[test]
    fn empty_body_round_trip() {
        let enc = MercuryEncryption::new([0xAA; 32], [0xBB; 16], [0xCC; 32]);
        let mut codec = MercuryCodec::with_encryption(enc);

        let original = Packet::new(PacketFlags::default(), 1, Bytes::new());

        let mut buf = BytesMut::new();
        codec.encode(original, &mut buf).unwrap();

        let decoded = codec.decode(&mut buf).unwrap().unwrap();
        assert!(decoded.body.is_empty());
    }

    #[test]
    fn empty_buffer_returns_none() {
        let mut codec = MercuryCodec::new();
        let mut buf = BytesMut::new();
        assert!(codec.decode(&mut buf).unwrap().is_none());
    }
}
