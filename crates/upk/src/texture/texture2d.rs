//! The serial data of a cooked `Texture2D` export (Epic 486): tagged
//! properties, a 12-byte block the writer keeps verbatim, the file offset of
//! the mip array, then the mips.
//!
//! ```text
//! properties .. "None" | 12 bytes | i32 array offset | i32 mip count
//! per mip: i32 flags | i32 element bytes | i32 payload bytes | i32 payload offset
//!          payload | i32 size x | i32 size y
//! ```
//!
//! Both offsets are absolute positions in the package file, so a mip chain is
//! only valid at the offset it was laid out for.

use crate::error::{Result, UpkError};
use crate::names::NameEntry;
use crate::properties::parse_tagged_properties_with_end;

/// Mip payloads stored LZO-compressed (`BULKDATA_SerializeCompressedLZO`).
pub const FLAG_LZO: i32 = 0x10;

/// One mip as stored: its flags, the uncompressed byte count, the payload
/// exactly as it sits on disk, and its pixel size.
#[derive(Debug, Clone)]
pub struct Mip {
    pub flags: i32,
    pub elements: i32,
    pub payload: Vec<u8>,
    pub width: i32,
    pub height: i32,
}

/// A texture export split at its mip array.
#[derive(Debug, Clone)]
pub struct Texture2dData {
    /// Everything before the mip array's offset field: properties and the
    /// 12 bytes after them.
    pub head: Vec<u8>,
    pub mips: Vec<Mip>,
}

fn i32_at(data: &[u8], at: usize) -> Result<i32> {
    data.get(at..at + 4)
        .map(|b| i32::from_le_bytes(b.try_into().unwrap()))
        .ok_or_else(|| UpkError::Parse(format!("texture data is cut short at {at}")))
}

/// Split a texture export's serial data. Fails when anything is left over
/// after the last mip.
pub fn parse(data: &[u8], names: &[NameEntry]) -> Result<Texture2dData> {
    let (_props, end) = parse_tagged_properties_with_end(data, 4, names);
    let head_len = end + 12;
    let count = i32_at(data, head_len + 4)?;
    let mut at = head_len + 8;
    let mut mips = Vec::new();
    for _ in 0..count.max(0) {
        let flags = i32_at(data, at)?;
        let elements = i32_at(data, at + 4)?;
        let size = i32_at(data, at + 8)?;
        if size < 0 {
            return Err(UpkError::Parse("negative mip payload size".into()));
        }
        let end = (at + 16)
            .checked_add(size as usize)
            .ok_or_else(|| UpkError::Parse("mip payload size overflows".into()))?;
        let payload = data
            .get(at + 16..end)
            .ok_or_else(|| UpkError::Parse("mip payload runs past the export".into()))?
            .to_vec();
        at = end;
        let (width, height) = (i32_at(data, at)?, i32_at(data, at + 4)?);
        at += 8;
        mips.push(Mip {
            flags,
            elements,
            payload,
            width,
            height,
        });
    }
    if at != data.len() {
        return Err(UpkError::Parse(format!(
            "{} bytes follow the mip chain",
            data.len() - at
        )));
    }
    Ok(Texture2dData {
        head: data[..head_len].to_vec(),
        mips,
    })
}

/// Lay a texture export out again for a package file in which the export
/// starts at `export_offset`.
pub fn write(texture: &Texture2dData, export_offset: usize) -> Vec<u8> {
    let array_at = export_offset + texture.head.len() + 4;
    let mut out = texture.head.clone();
    out.extend_from_slice(&(array_at as i32).to_le_bytes());
    out.extend_from_slice(&(texture.mips.len() as i32).to_le_bytes());
    for m in &texture.mips {
        out.extend_from_slice(&m.flags.to_le_bytes());
        out.extend_from_slice(&m.elements.to_le_bytes());
        out.extend_from_slice(&(m.payload.len() as i32).to_le_bytes());
        let payload_at = export_offset + out.len() + 4;
        out.extend_from_slice(&(payload_at as i32).to_le_bytes());
        out.extend_from_slice(&m.payload);
        out.extend_from_slice(&m.width.to_le_bytes());
        out.extend_from_slice(&m.height.to_le_bytes());
    }
    out
}
