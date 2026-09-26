//! The BigWorld `WSTRING` encoder.
//!
//! One definition for every serializer that writes a `WSTRING`: the mail and
//! contact-list payloads here, and the base's packet builders through the
//! `cimmeria_services::mercury::write_wstring` re-export. The matching reader
//! is still `cimmeria_services::mercury::read_wstring`.

/// Write a BigWorld `WSTRING` to a buffer.
///
/// Wire format: `[char_count: u32 LE][UTF-16LE data: char_count × 2 bytes]`.
pub fn write_wstring(buf: &mut Vec<u8>, s: &str) {
    let chars: Vec<u16> = s.encode_utf16().collect();
    buf.extend_from_slice(&(chars.len() as u32).to_le_bytes());
    for &ch in &chars {
        buf.extend_from_slice(&ch.to_le_bytes());
    }
}
