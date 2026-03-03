//! Unified TCP inter-service framing.
//!
//! The "Unified" protocol is used for TCP connections between Cimmeria
//! services (AuthenticationServer <-> BaseApp <-> CellApp). It uses a
//! simple length-prefixed framing scheme:
//!
//! ```text
//! [u32 LE length] [u8 message_id] [payload ...]
//! ```
//!
//! Where `length` covers `message_id + payload` (i.e., total frame size
//! minus the 4-byte length prefix). This is distinct from the UDP Mercury
//! protocol used for client communication.

use bytes::{Buf, BufMut, Bytes, BytesMut};
use cimmeria_common::{CimmeriaError, Result};
use tokio_util::codec::{Decoder, Encoder};

/// Size of the length prefix in bytes.
const LENGTH_PREFIX_SIZE: usize = 4;

/// Maximum frame size (64 KiB) to prevent memory exhaustion from malformed
/// length prefixes. Inter-service messages should never approach this limit.
const MAX_FRAME_SIZE: usize = 65536;

// ── UnifiedFrame ────────────────────────────────────────────────────────────

/// A single framed message on a TCP inter-service connection.
#[derive(Debug, Clone)]
pub struct UnifiedFrame {
    /// Total length of `message_id + payload` (the wire `length` field).
    pub length: u32,
    /// Application message type ID.
    pub message_id: u8,
    /// Message payload (after the message_id byte).
    pub payload: Bytes,
}

impl UnifiedFrame {
    /// Create a new frame from a message ID and payload.
    ///
    /// The `length` field is computed automatically.
    pub fn new(message_id: u8, payload: Bytes) -> Self {
        let length = 1 + payload.len() as u32; // 1 for message_id
        Self {
            length,
            message_id,
            payload,
        }
    }
}

// ── UnifiedCodec ────────────────────────────────────────────────────────────

/// tokio-util codec for the Unified TCP framing protocol.
///
/// Implements `Decoder` to read length-prefixed frames from a TCP byte
/// stream, and `Encoder` to write them.
#[derive(Debug, Default)]
pub struct UnifiedCodec;

impl UnifiedCodec {
    pub fn new() -> Self {
        Self
    }
}

impl Decoder for UnifiedCodec {
    type Item = UnifiedFrame;
    type Error = CimmeriaError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>> {
        // Need at least the length prefix to proceed.
        if src.len() < LENGTH_PREFIX_SIZE {
            return Ok(None);
        }

        // Peek at the length prefix without consuming it yet.
        let length = u32::from_le_bytes([src[0], src[1], src[2], src[3]]) as usize;

        if length == 0 {
            return Err(CimmeriaError::Protocol(
                "Unified frame with zero length".into(),
            ));
        }

        if length > MAX_FRAME_SIZE {
            return Err(CimmeriaError::Protocol(format!(
                "Unified frame length {} exceeds maximum {}",
                length, MAX_FRAME_SIZE
            )));
        }

        // Check if the full frame has arrived.
        let total_needed = LENGTH_PREFIX_SIZE + length;
        if src.len() < total_needed {
            // Reserve space for the rest of the frame to reduce reallocation.
            src.reserve(total_needed - src.len());
            return Ok(None);
        }

        // Consume the length prefix.
        src.advance(LENGTH_PREFIX_SIZE);

        // Read message_id (1 byte) + payload (length - 1 bytes).
        let message_id = src[0];
        src.advance(1);

        let payload_len = length - 1;
        let payload = src.split_to(payload_len).freeze();

        Ok(Some(UnifiedFrame {
            length: length as u32,
            message_id,
            payload,
        }))
    }
}

impl Encoder<UnifiedFrame> for UnifiedCodec {
    type Error = CimmeriaError;

    fn encode(&mut self, frame: UnifiedFrame, dst: &mut BytesMut) -> Result<()> {
        let length = 1u32 + frame.payload.len() as u32;

        if length as usize > MAX_FRAME_SIZE {
            return Err(CimmeriaError::Protocol(format!(
                "Unified frame length {} exceeds maximum {}",
                length, MAX_FRAME_SIZE
            )));
        }

        dst.reserve(LENGTH_PREFIX_SIZE + length as usize);
        dst.put_u32_le(length);
        dst.put_u8(frame.message_id);
        dst.put_slice(&frame.payload);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_unified_frame() {
        let frame = UnifiedFrame::new(7, Bytes::from_static(b"test payload"));

        let mut codec = UnifiedCodec::new();
        let mut buf = BytesMut::new();
        codec.encode(frame.clone(), &mut buf).unwrap();

        let decoded = codec.decode(&mut buf).unwrap().unwrap();
        assert_eq!(decoded.message_id, 7);
        assert_eq!(decoded.payload.as_ref(), b"test payload");
    }

    #[test]
    fn partial_frame_returns_none() {
        let frame = UnifiedFrame::new(1, Bytes::from_static(b"hello"));

        let mut codec = UnifiedCodec::new();
        let mut buf = BytesMut::new();
        codec.encode(frame, &mut buf).unwrap();

        // Truncate to simulate partial read.
        buf.truncate(3);
        assert!(codec.decode(&mut buf).unwrap().is_none());
    }

    #[test]
    fn empty_payload() {
        let frame = UnifiedFrame::new(42, Bytes::new());

        let mut codec = UnifiedCodec::new();
        let mut buf = BytesMut::new();
        codec.encode(frame, &mut buf).unwrap();

        let decoded = codec.decode(&mut buf).unwrap().unwrap();
        assert_eq!(decoded.message_id, 42);
        assert!(decoded.payload.is_empty());
    }

    #[test]
    fn zero_length_frame_errors() {
        let mut codec = UnifiedCodec::new();
        let mut buf = BytesMut::new();
        buf.put_u32_le(0); // zero-length frame

        let err = codec.decode(&mut buf).unwrap_err();
        assert!(matches!(err, CimmeriaError::Protocol(_)));
    }

    #[test]
    fn oversized_frame_errors() {
        let mut codec = UnifiedCodec::new();
        let mut buf = BytesMut::new();
        buf.put_u32_le((MAX_FRAME_SIZE + 1) as u32);

        let err = codec.decode(&mut buf).unwrap_err();
        assert!(matches!(err, CimmeriaError::Protocol(_)));
    }
}
