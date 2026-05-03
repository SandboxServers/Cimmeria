use crate::error::Result;
use crate::reader::BinaryReader;
use std::io::{Read, Seek};

pub const PACKAGE_FILE_TAG: u32 = 0x9E2A83C1;

/// Compression flags (early UE3 enum values used by SGW).
pub const COMPRESS_ZLIB: u32 = 0x01;
pub const COMPRESS_LZO: u32 = 0x02;

/// Generation info within the package header.
#[derive(Debug, Clone)]
pub struct GenerationInfo {
    pub export_count: i32,
    pub name_count: i32,
    pub net_object_count: i32,
}

/// Compressed chunk descriptor.
#[derive(Debug, Clone)]
pub struct CompressedChunk {
    pub uncompressed_offset: i32,
    pub uncompressed_size: i32,
    pub compressed_offset: i32,
    pub compressed_size: i32,
}

/// FPackageFileSummary — the package header.
/// Parsed following FUN_004bcab0 in SGW.exe.
#[derive(Debug, Clone, Default)]
pub struct PackageHeader {
    pub tag: u32,
    pub epic_version: u16,
    pub licensee_version: u16,
    pub total_header_size: i32,
    pub folder_name: String,
    pub package_flags: u32,
    pub name_count: i32,
    pub name_offset: i32,
    pub export_count: i32,
    pub export_offset: i32,
    pub import_count: i32,
    pub import_offset: i32,
    pub depends_offset: i32,
    pub guid: [u32; 4],
    pub generations: Vec<GenerationInfo>,
    pub engine_version: i32,
    pub cooker_version: i32,
    pub compression_flags: u32,
    pub compressed_chunks: Vec<CompressedChunk>,
}

impl PackageHeader {
    /// Parse the package header from a binary stream.
    pub fn parse<R: Read + Seek>(reader: &mut BinaryReader<R>) -> Result<Self> {
        use crate::error::UpkError;

        reader.seek(0)?;
        let tag = reader.read_u32()?;
        if tag != PACKAGE_FILE_TAG {
            return Err(UpkError::InvalidTag(tag));
        }
        let mut h = Self {
            tag,
            ..Self::default()
        };

        let packed_version = reader.read_u32()?;
        h.epic_version = (packed_version & 0xFFFF) as u16;
        h.licensee_version = ((packed_version >> 16) & 0xFFFF) as u16;

        // TotalHeaderSize (Epic >= 249)
        if h.epic_version >= 249 {
            h.total_header_size = reader.read_i32()?;
        }

        // FolderName (Epic >= 269)
        if h.epic_version >= 269 {
            h.folder_name = reader.read_fstring()?;
        }

        h.package_flags = reader.read_u32()?;
        h.name_count = reader.read_i32()?;
        h.name_offset = reader.read_i32()?;
        h.export_count = reader.read_i32()?;
        h.export_offset = reader.read_i32()?;
        h.import_count = reader.read_i32()?;
        h.import_offset = reader.read_i32()?;

        // DependsOffset (Epic > 414)
        if h.epic_version > 414 {
            h.depends_offset = reader.read_i32()?;
        }

        // GUID
        for g in &mut h.guid {
            *g = reader.read_u32()?;
        }

        // Generations
        let gen_count = reader.read_i32()?;
        h.generations = Vec::with_capacity(gen_count as usize);
        for _ in 0..gen_count {
            h.generations.push(GenerationInfo {
                export_count: reader.read_i32()?,
                name_count: reader.read_i32()?,
                net_object_count: reader.read_i32()?,
            });
        }

        // EngineVersion (Epic >= 245)
        if h.epic_version >= 245 {
            h.engine_version = reader.read_i32()?;
        }

        // CookerVersion (Epic >= 277)
        if h.epic_version >= 277 {
            h.cooker_version = reader.read_i32()?;
        }

        // Compression (Epic > 333)
        if h.epic_version > 333 {
            h.compression_flags = reader.read_u32()?;
            let chunk_count = reader.read_i32()?;
            h.compressed_chunks = Vec::with_capacity(chunk_count as usize);
            for _ in 0..chunk_count {
                h.compressed_chunks.push(CompressedChunk {
                    uncompressed_offset: reader.read_i32()?,
                    uncompressed_size: reader.read_i32()?,
                    compressed_offset: reader.read_i32()?,
                    compressed_size: reader.read_i32()?,
                });
            }
        }

        Ok(h)
    }

    /// Returns true if the package is compressed.
    pub fn is_compressed(&self) -> bool {
        self.compression_flags != 0 && !self.compressed_chunks.is_empty()
    }
}
