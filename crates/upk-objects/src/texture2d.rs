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

/// Pixel format enum matching the SGW client's `EPixelFormat`.
///
/// The byte values index the client's `GPixelFormats` table at `0x01dd6a08`
/// in SGW.exe (stride `0x24`; UTF-16 names from `0x018f7960`). The D3D init
/// code that fills each entry's `PlatformFormat` field (`0x01dd6a20 + n *
/// 0x24`, next to the `supportFloatingPointRenderTargets` probe in
/// `docs/reverse-engineering/decompiled/14_standalone_named.c`) pins the order:
///
/// | Byte | Format | PlatformFormat |
/// |---|---|---|
/// | 0 | Unknown | 0 |
/// | 1 | A32B32G32R32F | `0x74` (D3DFMT_A32B32G32R32F) |
/// | 2 | A8R8G8B8 | `0x15` (D3DFMT_A8R8G8B8) |
/// | 3 | G8 | `0x32` (D3DFMT_L8) |
/// | 4 | G16 | 0 |
/// | 5 | DXT1 | `'DXT1'` FourCC |
/// | 6 | DXT3 | `'DXT3'` FourCC |
/// | 7 | DXT5 | `'DXT5'` FourCC |
/// | 8 | UYVY | `'UYVY'` FourCC |
///
/// Stock cooked data agrees: `BS_HF_Torso00_D` (512x256, 10 mips) carries
/// byte 5, and its mips are 8 bytes per 4x4 block, i.e. DXT1 (#839).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    A32B32G32R32F,
    A8R8G8B8,
    G8,
    G16,
    DXT1,
    DXT3,
    DXT5,
    UYVY,
    /// `PF_Unknown` (0), or a byte past the end of the client's table.
    Unknown(u8),
}

impl PixelFormat {
    /// Map a Texture2D `Format` byte to its `EPixelFormat` variant.
    pub fn from_byte(b: u8) -> Self {
        match b {
            1 => PixelFormat::A32B32G32R32F,
            2 => PixelFormat::A8R8G8B8,
            3 => PixelFormat::G8,
            4 => PixelFormat::G16,
            5 => PixelFormat::DXT1,
            6 => PixelFormat::DXT3,
            7 => PixelFormat::DXT5,
            8 => PixelFormat::UYVY,
            _ => PixelFormat::Unknown(b),
        }
    }

    /// Bytes per 4x4 block for block-compressed formats, or bytes per pixel.
    pub fn block_size(&self) -> usize {
        match self {
            PixelFormat::DXT1 => 8,                      // 8 bytes per 4x4 block
            PixelFormat::DXT3 | PixelFormat::DXT5 => 16, // 16 bytes per 4x4 block
            PixelFormat::A32B32G32R32F => 16,            // 4 x f32 per pixel
            PixelFormat::A8R8G8B8 => 4,                  // 4 bytes per pixel
            PixelFormat::G16 | PixelFormat::UYVY => 2,   // 2 bytes per pixel
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

    /// Uncompressed size in bytes of one `width` x `height` mip.
    ///
    /// Block-compressed formats round each dimension up to whole 4x4 blocks.
    pub fn mip_bytes(&self, width: u32, height: u32) -> usize {
        let (w, h) = (width as usize, height as usize);
        if self.is_block_compressed() {
            w.div_ceil(4).max(1) * h.div_ceil(4).max(1) * self.block_size()
        } else {
            w * h * self.block_size()
        }
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
    // A tagged property equal to its class default is not serialized, and
    // Texture2D's default Format is 0 (`PF_Unknown`). A missing tag therefore
    // decodes as Unknown(0), never as a concrete format.
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

#[cfg(test)]
mod tests {
    use super::PixelFormat;

    /// #839: pins the `GPixelFormats` order at SGW.exe `0x01dd6a08`. The old
    /// map (0 = A8R8G8B8, 3 = DXT1, 5 = DXT5) was off by two and fails here.
    #[test]
    fn from_byte_follows_client_gpixelformats_table() {
        let expected = [
            (0, PixelFormat::Unknown(0)),
            (1, PixelFormat::A32B32G32R32F),
            (2, PixelFormat::A8R8G8B8),
            (3, PixelFormat::G8),
            (4, PixelFormat::G16),
            (5, PixelFormat::DXT1),
            (6, PixelFormat::DXT3),
            (7, PixelFormat::DXT5),
            (8, PixelFormat::UYVY),
            (9, PixelFormat::Unknown(9)),
        ];
        for (byte, format) in expected {
            assert_eq!(PixelFormat::from_byte(byte), format, "Format byte {byte}");
        }
    }

    /// Stock `BS_HF_Torso00_D` is 512x256 with Format byte 5, and its mip 0
    /// holds 65,536 bytes, which only DXT1's 8-byte 4x4 blocks produce.
    #[test]
    fn dxt1_512x256_mip0_is_65536_bytes() {
        assert_eq!(PixelFormat::from_byte(5).mip_bytes(512, 256), 65_536);
        assert_eq!(PixelFormat::from_byte(7).mip_bytes(512, 256), 131_072);
        // A 1x1 tail mip still occupies one whole block.
        assert_eq!(PixelFormat::DXT1.mip_bytes(1, 1), 8);
        assert_eq!(PixelFormat::A8R8G8B8.mip_bytes(4, 2), 32);
    }
}
