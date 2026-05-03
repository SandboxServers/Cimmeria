//! Texture2D deserializer for UE3 export data.
//!
//! SGW v486 UTexture2D binary layout (after tagged properties):
//!   1. SourceArt: FByteBulkData (16-byte header, usually empty in cooked)
//!   2. NumMips: i32
//!   3. Per-mip: FUntypedBulkData (16-byte header) + SizeX (i32) + SizeY (i32)
//!
//! Tagged properties provide: SizeX, SizeY, Format (ByteProperty), MipTailBaseIdx.

use crate::bulk_data::parse_bulk_data_v486;
use crate::error::{ObjectError, Result};
use byteorder::{ByteOrder, LittleEndian};

/// A decoded UE3 Texture2D object.
#[derive(Debug)]
pub struct Texture2D {
    pub size_x: u32,
    pub size_y: u32,
    pub format: PixelFormat,
    pub mips: Vec<MipLevel>,
    pub num_mips: u32,
}

/// Pixel format enum matching UE3's EPixelFormat for SGW v486.
///
/// Derived from Ghidra analysis of the SGW client's texture loading code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    A8R8G8B8,
    G8,
    DXT1,
    DXT3,
    DXT5,
    /// A byte property value we haven't mapped yet.
    Unknown(u8),
}

impl PixelFormat {
    fn from_byte(b: u8) -> Self {
        // UE3 v486 EPixelFormat enum ordering (from engine headers):
        //   0 = PF_A8R8G8B8
        //   1 = PF_G8
        //   2 = PF_G16
        //   3 = PF_DXT1
        //   4 = PF_DXT3
        //   5 = PF_DXT5
        match b {
            0 => PixelFormat::A8R8G8B8,
            1 => PixelFormat::G8,
            3 => PixelFormat::DXT1,
            4 => PixelFormat::DXT3,
            5 => PixelFormat::DXT5,
            _ => PixelFormat::Unknown(b),
        }
    }

    /// Bytes per 4x4 block for block-compressed formats, or bytes per pixel.
    pub fn block_size(&self) -> usize {
        match self {
            PixelFormat::DXT1 => 8,                      // 8 bytes per 4x4 block
            PixelFormat::DXT3 | PixelFormat::DXT5 => 16, // 16 bytes per 4x4 block
            PixelFormat::A8R8G8B8 => 4,                  // 4 bytes per pixel
            PixelFormat::G8 => 1,                        // 1 byte per pixel
            PixelFormat::Unknown(_) => 4,                // assume 4 bpp
        }
    }

    /// Whether this format uses block compression (DXT).
    pub fn is_block_compressed(&self) -> bool {
        matches!(
            self,
            PixelFormat::DXT1 | PixelFormat::DXT3 | PixelFormat::DXT5
        )
    }
}

/// A single mip level of a texture.
#[derive(Debug)]
pub struct MipLevel {
    /// Raw pixel data (compressed on disk — DXT and/or LZO).
    /// Empty if data is stored externally (e.g., in .tfc) or was not loaded.
    pub data: Vec<u8>,
    /// Width of this mip level.
    pub size_x: u32,
    /// Height of this mip level.
    pub size_y: u32,
    /// Whether the data is LZO/ZLIB compressed (needs decompression before GPU use).
    pub is_bulk_compressed: bool,
}

/// Deserialize a Texture2D from export serial data.
///
/// `data` is the raw bytes from `pkg.read_export_data(export)`.
/// `names` is the package name table for property parsing.
pub fn deserialize_texture2d(data: &[u8], names: &[cimmeria_upk::NameEntry]) -> Result<Texture2D> {
    // 1. Parse tagged properties starting after NetIndex (offset 4)
    let (props, bin_offset) = cimmeria_upk::parse_tagged_properties_with_end(data, 4, names);

    // Extract key properties
    let size_x = find_int(&props, "SizeX").unwrap_or(0) as u32;
    let size_y = find_int(&props, "SizeY").unwrap_or(0) as u32;
    let format_byte = find_byte_value(&props, "Format").unwrap_or(0);
    let format = PixelFormat::from_byte(format_byte);

    // 2. After properties: SourceArt FByteBulkData (16-byte v486 header)
    let mut pos = bin_offset;
    if pos + 16 > data.len() {
        return Err(ObjectError::InvalidData(
            "Not enough data for SourceArt bulk header".into(),
        ));
    }
    let source_art = parse_bulk_data_v486(data, pos)?;
    pos += source_art.bytes_consumed;

    // 3. NumMips
    if pos + 4 > data.len() {
        return Err(ObjectError::InvalidData(
            "Not enough data for NumMips".into(),
        ));
    }
    let num_mips = LittleEndian::read_i32(&data[pos..]) as u32;
    pos += 4;

    // 4. Per-mip: FUntypedBulkData + SizeX + SizeY
    let mut mips = Vec::with_capacity(num_mips as usize);
    for i in 0..num_mips {
        if pos + 16 > data.len() {
            tracing::warn!("Truncated at mip {}/{}", i, num_mips);
            break;
        }

        let bulk = parse_bulk_data_v486(data, pos)?;
        pos += bulk.bytes_consumed;

        if pos + 8 > data.len() {
            tracing::warn!("Truncated after mip {} bulk data", i);
            break;
        }
        let mip_sx = LittleEndian::read_i32(&data[pos..]) as u32;
        pos += 4;
        let mip_sy = LittleEndian::read_i32(&data[pos..]) as u32;
        pos += 4;

        let is_compressed = bulk.flags
            & (crate::bulk_data::BULKDATA_COMPRESSED_ZLIB
                | crate::bulk_data::BULKDATA_COMPRESSED_LZO
                | crate::bulk_data::BULKDATA_COMPRESSED_LZX)
            != 0;

        mips.push(MipLevel {
            data: bulk.data,
            size_x: mip_sx,
            size_y: mip_sy,
            is_bulk_compressed: is_compressed,
        });
    }

    Ok(Texture2D {
        size_x,
        size_y,
        format,
        mips,
        num_mips,
    })
}

fn find_int(props: &[cimmeria_upk::TaggedProperty], name: &str) -> Option<i32> {
    props.iter().find(|p| p.name == name).and_then(|p| {
        if let cimmeria_upk::PropValue::Int(v) = &p.value {
            Some(*v)
        } else {
            None
        }
    })
}

fn find_byte_value(props: &[cimmeria_upk::TaggedProperty], name: &str) -> Option<u8> {
    props.iter().find(|p| p.name == name).and_then(|p| {
        if let cimmeria_upk::PropValue::Byte(bytes) = &p.value {
            bytes.first().copied()
        } else {
            None
        }
    })
}
