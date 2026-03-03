//! tokio-util codec adapters for Mercury UDP packets.
//!
//! `MercuryCodec` wraps the packet encode/decode logic from [`crate::packet`]
//! with the optional encryption layer from [`crate::encryption`]. It
//! implements the `tokio_util::codec::{Decoder, Encoder}` traits so that
//! Mercury can be used with `tokio_util::udp::UdpFramed` (or manual
//! encode/decode in a recv loop).
//!
//! When encryption is enabled, the codec:
//! - **Encoding:** serialises the packet, encrypts the body, writes header + encrypted body.
//! - **Decoding:** reads header, decrypts body, passes to packet decoder.

use bytes::{Bytes, BytesMut};
use cimmeria_common::{CimmeriaError, Result};
use tokio_util::codec::{Decoder, Encoder};

use crate::encryption::MercuryEncryption;
use crate::packet::Packet;

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

        // For UDP, each datagram is a complete packet, so we consume the
        // entire buffer as one packet.
        let mut raw = src.split().freeze();

        // Parse the packet (header is always in the clear).
        let mut packet = Packet::decode(&mut raw)?;

        // If encryption is active, decrypt the body.
        if let Some(ref enc) = self.encryption {
            if !packet.body.is_empty() {
                let plaintext = enc.decrypt(&packet.body)?;
                packet.body = Bytes::from(plaintext);
            }
        }

        Ok(Some(packet))
    }
}

impl Encoder<Packet> for MercuryCodec {
    type Error = CimmeriaError;

    fn encode(&mut self, mut packet: Packet, dst: &mut BytesMut) -> Result<()> {
        // If encryption is active, encrypt the body before encoding.
        if let Some(ref enc) = self.encryption {
            if !packet.body.is_empty() {
                let ciphertext = enc.encrypt(&packet.body)?;
                packet.body = Bytes::from(ciphertext);
            }
        }

        packet.encode(dst);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::{PacketFlags, FLAG_RELIABLE};

    #[test]
    fn round_trip_plaintext() {
        let mut codec = MercuryCodec::new();
        let original = Packet::new(
            10,
            PacketFlags::default().with(FLAG_RELIABLE),
            Bytes::from_static(b"plaintext body"),
        );

        let mut buf = BytesMut::new();
        codec.encode(original.clone(), &mut buf).unwrap();

        let decoded = codec.decode(&mut buf).unwrap().unwrap();
        assert_eq!(decoded.sequence, 10);
        assert!(decoded.flags.is_reliable());
        assert_eq!(decoded.body.as_ref(), b"plaintext body");
    }

    #[test]
    fn round_trip_encrypted() {
        let enc = MercuryEncryption::new([0xAA; 32], [0xBB; 16], [0xCC; 16]);
        let mut codec = MercuryCodec::with_encryption(enc);

        let original = Packet::new(
            99,
            PacketFlags::default(),
            Bytes::from_static(b"secret message"),
        );

        let mut buf = BytesMut::new();
        codec.encode(original.clone(), &mut buf).unwrap();

        // The encoded body should NOT contain the plaintext.
        let wire_bytes = buf.clone().freeze();
        assert!(!wire_bytes.windows(14).any(|w| w == b"secret message"));

        let decoded = codec.decode(&mut buf).unwrap().unwrap();
        assert_eq!(decoded.sequence, 99);
        assert_eq!(decoded.body.as_ref(), b"secret message");
    }

    #[test]
    fn empty_body_no_encryption() {
        let enc = MercuryEncryption::new([0xAA; 32], [0xBB; 16], [0xCC; 16]);
        let mut codec = MercuryCodec::with_encryption(enc);

        let original = Packet::new(1, PacketFlags::default(), Bytes::new());

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
