//! Message aggregation ("Bundles").
//!
//! A Bundle groups one or more application-level messages into a single
//! logical unit for transmission. Each message in the bundle carries its
//! own message ID and payload. The bundle is serialised into one or more
//! Mercury packets (fragmenting if necessary).
//!
//! Wire format for a bundle body (packed into packet payload):
//! ```text
//! [u8 message_count]
//! For each message:
//!   [u8  message_id]
//!   [u16 LE payload_length]
//!   [payload_length bytes of payload]
//! ```

use bytes::{Buf, BufMut, Bytes, BytesMut};
use cimmeria_common::{CimmeriaError, Result};

// ── BundleMessage ───────────────────────────────────────────────────────────

/// A single application-level message within a Bundle.
#[derive(Debug, Clone)]
pub struct BundleMessage {
    /// Mercury message type identifier (see [`crate::messages`]).
    pub message_id: u8,
    /// Serialised message payload.
    pub payload: Bytes,
}

impl BundleMessage {
    /// Create a new bundle message.
    pub fn new(message_id: u8, payload: Bytes) -> Self {
        Self {
            message_id,
            payload,
        }
    }

    /// Wire size of this message: 1 (id) + 2 (length) + payload.
    pub fn wire_size(&self) -> usize {
        1 + 2 + self.payload.len()
    }
}

// ── Bundle ──────────────────────────────────────────────────────────────────

/// A collection of messages destined for a single peer.
///
/// Bundles are the primary unit of application-level I/O in Mercury.
/// Game code adds messages to a bundle, then the bundle is sent over
/// a channel (possibly spanning multiple packets if fragmentation is needed).
#[derive(Debug, Clone)]
pub struct Bundle {
    /// The messages aggregated in this bundle.
    pub messages: Vec<BundleMessage>,
    /// Whether this bundle requires reliable delivery.
    pub reliable: bool,
}

impl Bundle {
    /// Create a new empty bundle.
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            reliable: false,
        }
    }

    /// Create a new reliable bundle.
    pub fn new_reliable() -> Self {
        Self {
            messages: Vec::new(),
            reliable: true,
        }
    }

    /// Append a message to this bundle.
    pub fn add_message(&mut self, message_id: u8, payload: Bytes) {
        self.messages.push(BundleMessage::new(message_id, payload));
    }

    /// Total wire size of the bundle body (header + all messages).
    pub fn wire_size(&self) -> usize {
        // 1 byte for message count + each message's wire size.
        1 + self.messages.iter().map(|m| m.wire_size()).sum::<usize>()
    }

    /// Encode the entire bundle into `buf`.
    ///
    /// Format: `[u8 count] [message_id, u16 len, payload]...`
    pub fn encode(&self, buf: &mut BytesMut) {
        if self.messages.len() > 255 {
            tracing::warn!(
                count = self.messages.len(),
                "bundle exceeds 255 messages — truncating"
            );
        }

        let count = self.messages.len().min(255) as u8;
        buf.reserve(self.wire_size());
        buf.put_u8(count);

        for msg in self.messages.iter().take(count as usize) {
            buf.put_u8(msg.message_id);
            // Payload length as u16 LE — payloads > 65535 bytes must be fragmented.
            let len = msg.payload.len().min(u16::MAX as usize) as u16;
            buf.put_u16_le(len);
            buf.put_slice(&msg.payload[..len as usize]);
        }
    }

    /// Decode a bundle from `buf`.
    pub fn decode(buf: &Bytes) -> Result<Bundle> {
        if buf.is_empty() {
            return Err(CimmeriaError::PacketDecode(
                "empty buffer for bundle decode".into(),
            ));
        }

        let mut cursor = buf.clone();
        let count = cursor.get_u8();
        let mut messages = Vec::with_capacity(count as usize);

        for i in 0..count {
            if cursor.remaining() < 3 {
                return Err(CimmeriaError::BufferUnderflow {
                    needed: 3,
                    available: cursor.remaining(),
                });
            }

            let message_id = cursor.get_u8();
            let payload_len = cursor.get_u16_le() as usize;

            if cursor.remaining() < payload_len {
                return Err(CimmeriaError::PacketDecode(format!(
                    "bundle message {i}: declared length {payload_len} but only {} bytes remain",
                    cursor.remaining()
                )));
            }

            let payload = cursor.split_to(payload_len);
            messages.push(BundleMessage::new(message_id, payload));
        }

        Ok(Bundle {
            messages,
            reliable: false, // Reliable flag is a channel-level concern, not on the wire.
        })
    }
}

impl Default for Bundle {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_bundle() {
        let mut bundle = Bundle::new();
        bundle.add_message(1, Bytes::from_static(b"hello"));
        bundle.add_message(2, Bytes::from_static(b"world"));

        let mut buf = BytesMut::new();
        bundle.encode(&mut buf);

        let decoded = Bundle::decode(&buf.freeze()).unwrap();
        assert_eq!(decoded.messages.len(), 2);
        assert_eq!(decoded.messages[0].message_id, 1);
        assert_eq!(decoded.messages[0].payload.as_ref(), b"hello");
        assert_eq!(decoded.messages[1].message_id, 2);
        assert_eq!(decoded.messages[1].payload.as_ref(), b"world");
    }

    #[test]
    fn empty_bundle() {
        let bundle = Bundle::new();
        let mut buf = BytesMut::new();
        bundle.encode(&mut buf);

        let decoded = Bundle::decode(&buf.freeze()).unwrap();
        assert_eq!(decoded.messages.len(), 0);
    }

    #[test]
    fn decode_empty_buf_errors() {
        let buf = Bytes::new();
        assert!(Bundle::decode(&buf).is_err());
    }
}
