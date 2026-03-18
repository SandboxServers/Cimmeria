//! FUntypedBulkData / TLazyArray parser for UE3 export data.
//!
//! UE3 uses bulk data containers for large binary payloads (mesh vertices, texture
//! mip data, etc.). SGW (Epic version 486) sits in a transitional zone:
//!
//! - The modern `FUntypedBulkData` format (flags + element_count + size_on_disk + offset)
//!   appears in version >= 491 (Build 10897 reference source).
//! - The legacy `TLazyArray` format (skip_offset + count + element_size + inline data)
//!   appears in earlier builds.
//!
//! We implement both and let callers try modern first, falling back to legacy.

use byteorder::{ByteOrder, LittleEndian};
use crate::error::{ObjectError, Result};

// --- FUntypedBulkData flags ---

pub const BULKDATA_STORE_IN_SEPARATE_FILE: u32 = 1 << 0;
pub const BULKDATA_COMPRESSED_ZLIB: u32 = 1 << 1;
pub const BULKDATA_FORCE_SINGLE_ELEMENT: u32 = 1 << 2;
pub const BULKDATA_SINGLE_USE: u32 = 1 << 3;
pub const BULKDATA_COMPRESSED_LZO: u32 = 1 << 4;
pub const BULKDATA_UNUSED: u32 = 1 << 5;
pub const BULKDATA_STORE_ONLY_PAYLOAD: u32 = 1 << 6;
pub const BULKDATA_COMPRESSED_LZX: u32 = 1 << 7;

/// Parsed bulk data container.
#[derive(Debug, Clone)]
pub struct BulkData {
    /// Raw flags from the header.
    pub flags: u32,
    /// Number of elements in the data.
    pub element_count: i32,
    /// Size of the data on disk (compressed size if compressed).
    pub size_on_disk: i32,
    /// The actual data bytes (decompressed if applicable).
    /// Empty if data is stored in a separate file.
    pub data: Vec<u8>,
    /// How many bytes this bulk data consumed from the input buffer
    /// (header + inline data, if any).
    pub bytes_consumed: usize,
}

/// Parse `FUntypedBulkData` from a byte buffer at the given offset.
///
/// Modern UE3 format (version >= 491):
/// ```text
/// flags:          u32
/// element_count:  i32
/// size_on_disk:   i32   (compressed size if compressed, else uncompressed)
/// offset_in_file: i64   (absolute offset, or 0xFFFFFFFFFFFFFFFF for inline)
/// [inline data bytes if present]
/// ```
///
/// Returns the parsed `BulkData` including how many bytes were consumed.
pub fn parse_bulk_data(data: &[u8], offset: usize) -> Result<BulkData> {
    parse_bulk_data_inner(data, offset, 8)
}

/// Parse `FUntypedBulkData` for SGW v486 packages.
///
/// SGW v486 uses 32-bit offsets and sizes (pre-VER_BULKDATA_OFFSET_64BIT):
/// ```text
/// flags:          u32
/// element_count:  i32
/// size_on_disk:   i32
/// offset_in_file: i32   (32-bit absolute offset)
/// [inline data bytes if present]
/// ```
pub fn parse_bulk_data_v486(data: &[u8], offset: usize) -> Result<BulkData> {
    parse_bulk_data_inner(data, offset, 4)
}

fn parse_bulk_data_inner(data: &[u8], offset: usize, offset_width: usize) -> Result<BulkData> {
    let header_size = 4 + 4 + 4 + offset_width; // flags + count + size + offset
    if offset + header_size > data.len() {
        return Err(ObjectError::InvalidData(format!(
            "Bulk data header requires {} bytes at offset {}, but only {} available",
            header_size, offset, data.len().saturating_sub(offset)
        )));
    }

    let mut pos = offset;

    let flags = LittleEndian::read_u32(&data[pos..]);
    pos += 4;

    let element_count = LittleEndian::read_i32(&data[pos..]);
    pos += 4;

    let size_on_disk = LittleEndian::read_i32(&data[pos..]);
    pos += 4;

    // Read offset_in_file (32-bit for v486, 64-bit for modern)
    let _offset_in_file = if offset_width == 8 {
        let v = LittleEndian::read_i64(&data[pos..]);
        pos += 8;
        v
    } else {
        let v = LittleEndian::read_i32(&data[pos..]) as i64;
        pos += 4;
        v
    };

    // Unused bulk data — no payload
    if flags & BULKDATA_UNUSED != 0 {
        return Ok(BulkData {
            flags,
            element_count,
            size_on_disk,
            data: Vec::new(),
            bytes_consumed: pos - offset,
        });
    }

    // Data stored in a separate file (e.g., .tfc texture file cache)
    if flags & BULKDATA_STORE_IN_SEPARATE_FILE != 0 {
        return Ok(BulkData {
            flags,
            element_count,
            size_on_disk,
            data: Vec::new(),
            bytes_consumed: pos - offset,
        });
    }

    let data_size = size_on_disk as usize;
    if data_size == 0 {
        return Ok(BulkData {
            flags,
            element_count,
            size_on_disk,
            data: Vec::new(),
            bytes_consumed: pos - offset,
        });
    }

    // Inline data follows immediately after the header
    if pos + data_size > data.len() {
        return Err(ObjectError::InvalidData(format!(
            "Bulk data payload requires {} bytes at offset {}, but only {} available",
            data_size, pos, data.len().saturating_sub(pos)
        )));
    }

    let payload = data[pos..pos + data_size].to_vec();
    pos += data_size;

    Ok(BulkData {
        flags,
        element_count,
        size_on_disk,
        data: payload,
        bytes_consumed: pos - offset,
    })
}

/// Parse legacy `TLazyArray` from a byte buffer at the given offset.
///
/// Legacy format (pre-491):
/// ```text
/// skip_offset:  i32   // absolute file offset to skip past this data
/// count:        i32   // element count
/// element_size: i32   // size of each element
/// [count * element_size bytes of inline data]
/// ```
pub fn parse_lazy_array(data: &[u8], offset: usize) -> Result<BulkData> {
    // Minimum header: 4 + 4 + 4 = 12 bytes
    if offset + 12 > data.len() {
        return Err(ObjectError::InvalidData(format!(
            "TLazyArray header requires 12 bytes at offset {}, but only {} available",
            offset,
            data.len().saturating_sub(offset)
        )));
    }

    let mut pos = offset;

    let _skip_offset = LittleEndian::read_i32(&data[pos..]);
    pos += 4;

    let count = LittleEndian::read_i32(&data[pos..]);
    pos += 4;

    let element_size = LittleEndian::read_i32(&data[pos..]);
    pos += 4;

    if count < 0 || element_size < 0 {
        return Err(ObjectError::InvalidData(format!(
            "TLazyArray negative count ({}) or element_size ({})",
            count, element_size
        )));
    }

    let total_size = (count as usize) * (element_size as usize);

    if total_size == 0 {
        return Ok(BulkData {
            flags: 0,
            element_count: count,
            size_on_disk: 0,
            data: Vec::new(),
            bytes_consumed: pos - offset,
        });
    }

    if pos + total_size > data.len() {
        return Err(ObjectError::InvalidData(format!(
            "TLazyArray payload requires {} bytes at offset {}, but only {} available",
            total_size,
            pos,
            data.len().saturating_sub(pos)
        )));
    }

    let payload = data[pos..pos + total_size].to_vec();
    pos += total_size;

    Ok(BulkData {
        flags: 0,
        element_count: count,
        size_on_disk: total_size as i32,
        data: payload,
        bytes_consumed: pos - offset,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_unused_bulk_data() {
        let mut buf = vec![0u8; 20];
        // flags = BULKDATA_UNUSED
        LittleEndian::write_u32(&mut buf[0..], BULKDATA_UNUSED);
        // element_count = 10
        LittleEndian::write_i32(&mut buf[4..], 10);
        // size_on_disk = 100
        LittleEndian::write_i32(&mut buf[8..], 100);
        // offset_in_file = -1
        LittleEndian::write_i64(&mut buf[12..], -1);

        let result = parse_bulk_data(&buf, 0).unwrap();
        assert_eq!(result.flags, BULKDATA_UNUSED);
        assert_eq!(result.element_count, 10);
        assert!(result.data.is_empty());
        assert_eq!(result.bytes_consumed, 20);
    }

    #[test]
    fn parse_inline_bulk_data() {
        let payload = vec![0xAA, 0xBB, 0xCC, 0xDD];
        let mut buf = vec![0u8; 20 + payload.len()];
        // flags = 0 (no special flags)
        LittleEndian::write_u32(&mut buf[0..], 0);
        // element_count = 4
        LittleEndian::write_i32(&mut buf[4..], 4);
        // size_on_disk = 4
        LittleEndian::write_i32(&mut buf[8..], 4);
        // offset_in_file = -1 (inline)
        LittleEndian::write_i64(&mut buf[12..], -1);
        buf[20..24].copy_from_slice(&payload);

        let result = parse_bulk_data(&buf, 0).unwrap();
        assert_eq!(result.data, payload);
        assert_eq!(result.bytes_consumed, 24);
    }

    #[test]
    fn parse_empty_lazy_array() {
        let mut buf = vec![0u8; 12];
        LittleEndian::write_i32(&mut buf[0..], 100); // skip_offset
        LittleEndian::write_i32(&mut buf[4..], 0);   // count
        LittleEndian::write_i32(&mut buf[8..], 4);   // element_size

        let result = parse_lazy_array(&buf, 0).unwrap();
        assert_eq!(result.element_count, 0);
        assert!(result.data.is_empty());
        assert_eq!(result.bytes_consumed, 12);
    }

    #[test]
    fn parse_lazy_array_with_data() {
        let mut buf = vec![0u8; 12 + 8];
        LittleEndian::write_i32(&mut buf[0..], 20);  // skip_offset
        LittleEndian::write_i32(&mut buf[4..], 2);   // count
        LittleEndian::write_i32(&mut buf[8..], 4);   // element_size
        buf[12..20].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);

        let result = parse_lazy_array(&buf, 0).unwrap();
        assert_eq!(result.element_count, 2);
        assert_eq!(result.data.len(), 8);
        assert_eq!(result.bytes_consumed, 20);
    }
}
