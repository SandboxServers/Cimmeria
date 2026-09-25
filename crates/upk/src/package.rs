use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek, SeekFrom};
use std::path::Path;

use crate::error::{Result, UpkError};
use crate::exports::{self, ExportEntry};
use crate::header::{PackageHeader, COMPRESS_LZO};
use crate::imports::{self, ImportEntry};
use crate::names::{self, NameEntry};
use crate::reader::BinaryReader;

/// A parsed UE3 package (.upk or .umap file).
pub struct Package {
    pub header: PackageHeader,
    pub names: Vec<NameEntry>,
    pub imports: Vec<ImportEntry>,
    pub exports: Vec<ExportEntry>,
    /// Decompressed data (if package was compressed), or None (read from file).
    decompressed: Option<Vec<u8>>,
    /// Path to the original file (for re-reading serial data).
    filepath: String,
}

impl Package {
    /// Parse a package from a file path.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let filepath = path.as_ref().to_string_lossy().to_string();
        let file = File::open(path)?;
        let mut reader = BinaryReader::new(BufReader::new(file));

        // Parse header first
        let header = PackageHeader::parse(&mut reader)?;

        // If compressed, decompress the full package into memory
        let decompressed = if header.is_compressed() {
            Some(decompress_package(&mut reader, &header)?)
        } else {
            None
        };

        // Parse tables from either decompressed buffer or original file
        let (names, imports, exports) = if let Some(ref data) = decompressed {
            let mut mem_reader = BinaryReader::new(Cursor::new(data.as_slice()));
            let names =
                names::parse_name_table(&mut mem_reader, header.name_offset, header.name_count)?;
            let imports = imports::parse_import_table(
                &mut mem_reader,
                header.import_offset,
                header.import_count,
                &names,
            )?;
            let exports = exports::parse_export_table(
                &mut mem_reader,
                header.export_offset,
                header.export_count,
                header.epic_version,
                &names,
            )?;
            (names, imports, exports)
        } else {
            let names =
                names::parse_name_table(&mut reader, header.name_offset, header.name_count)?;
            let imports = imports::parse_import_table(
                &mut reader,
                header.import_offset,
                header.import_count,
                &names,
            )?;
            let exports = exports::parse_export_table(
                &mut reader,
                header.export_offset,
                header.export_count,
                header.epic_version,
                &names,
            )?;
            (names, imports, exports)
        };

        Ok(Self {
            header,
            names,
            imports,
            exports,
            decompressed,
            filepath,
        })
    }

    /// Resolve a name table index to a string.
    pub fn resolve_name(&self, index: i32) -> &str {
        if index >= 0 && (index as usize) < self.names.len() {
            &self.names[index as usize].name
        } else {
            "<invalid>"
        }
    }

    /// Resolve an object index to a name.
    /// Positive = export (1-based), negative = import (negated 1-based), 0 = None.
    pub fn resolve_object_name(&self, obj_index: i32) -> &str {
        if obj_index == 0 {
            "None"
        } else if obj_index < 0 {
            let idx = (-obj_index - 1) as usize;
            if idx < self.imports.len() {
                &self.imports[idx].object_name
            } else {
                "<import_oob>"
            }
        } else {
            let idx = (obj_index - 1) as usize;
            if idx < self.exports.len() {
                &self.exports[idx].object_name
            } else {
                "<export_oob>"
            }
        }
    }

    /// Get the class name for an export entry.
    pub fn export_class_name(&self, export: &ExportEntry) -> &str {
        self.resolve_object_name(export.class_index)
    }

    /// Build the full object path for an export (Package.Group.Name).
    pub fn export_full_path(&self, export: &ExportEntry) -> String {
        let mut parts = vec![export.object_name.clone()];
        let mut current = export.package_index;
        let mut depth = 0;

        while current != 0 && depth < 20 {
            if current > 0 {
                let idx = (current - 1) as usize;
                if idx < self.exports.len() {
                    parts.insert(0, self.exports[idx].object_name.clone());
                    current = self.exports[idx].package_index;
                } else {
                    break;
                }
            } else {
                let idx = (-current - 1) as usize;
                if idx < self.imports.len() {
                    parts.insert(0, self.imports[idx].object_name.clone());
                    current = self.imports[idx].package_index;
                } else {
                    break;
                }
            }
            depth += 1;
        }

        parts.join(".")
    }

    /// Build the full object path for an import (Package.Group.Name).
    ///
    /// Same walk as [`Self::export_full_path`], but seeded from an import entry.
    /// Import outer chains normally terminate in a package import (PackageIndex 0),
    /// so the result is the cross-package path the cooker recorded.
    pub fn import_full_path(&self, import: &ImportEntry) -> String {
        let mut parts = vec![import.object_name.clone()];
        let mut current = import.package_index;
        let mut depth = 0;

        while current != 0 && depth < 20 {
            if current > 0 {
                let idx = (current - 1) as usize;
                if idx < self.exports.len() {
                    parts.insert(0, self.exports[idx].object_name.clone());
                    current = self.exports[idx].package_index;
                } else {
                    break;
                }
            } else {
                let idx = (-current - 1) as usize;
                if idx < self.imports.len() {
                    parts.insert(0, self.imports[idx].object_name.clone());
                    current = self.imports[idx].package_index;
                } else {
                    break;
                }
            }
            depth += 1;
        }

        parts.join(".")
    }

    /// Resolve an object index to a full path (export or import).
    /// Positive = export (1-based), negative = import (negated 1-based), 0 = None.
    pub fn resolve_object_path(&self, obj_index: i32) -> String {
        if obj_index == 0 {
            "None".to_string()
        } else if obj_index < 0 {
            let idx = (-obj_index - 1) as usize;
            match self.imports.get(idx) {
                Some(imp) => self.import_full_path(imp),
                None => format!("<import_oob:{}>", obj_index),
            }
        } else {
            let idx = (obj_index - 1) as usize;
            match self.exports.get(idx) {
                Some(exp) => self.export_full_path(exp),
                None => format!("<export_oob:{}>", obj_index),
            }
        }
    }

    /// The full uncompressed package image: every summary offset and every
    /// export `serial_offset` indexes straight into it. For an LZO package this
    /// is the decompressed buffer; for an uncompressed one, the file itself.
    pub fn image(&self) -> Result<Vec<u8>> {
        match &self.decompressed {
            Some(data) => Ok(data.clone()),
            None => Ok(std::fs::read(&self.filepath)?),
        }
    }

    /// Read the serial data for an export entry.
    pub fn read_export_data(&self, export: &ExportEntry) -> Result<Vec<u8>> {
        if export.serial_size <= 0 {
            return Ok(Vec::new());
        }

        if let Some(ref data) = self.decompressed {
            let start = export.serial_offset as usize;
            let end = start + export.serial_size as usize;
            if end > data.len() {
                return Err(UpkError::DataTooShort {
                    need: end,
                    have: data.len(),
                });
            }
            Ok(data[start..end].to_vec())
        } else {
            let mut file = File::open(&self.filepath)?;
            file.seek(SeekFrom::Start(export.serial_offset as u64))?;
            let mut buf = vec![0u8; export.serial_size as usize];
            file.read_exact(&mut buf)?;
            Ok(buf)
        }
    }
}

/// Decompress a compressed UE3 package into a contiguous byte buffer.
fn decompress_package<R: Read + Seek>(
    reader: &mut BinaryReader<R>,
    header: &PackageHeader,
) -> Result<Vec<u8>> {
    // Calculate total uncompressed size
    let last = header
        .compressed_chunks
        .last()
        .ok_or_else(|| UpkError::Parse("No compressed chunks".into()))?;
    let total_size = (last.uncompressed_offset + last.uncompressed_size) as usize;

    // Read header portion (before first compressed chunk)
    let header_end = header.compressed_chunks[0].compressed_offset as usize;
    reader.seek(0)?;
    let header_data = reader.read_bytes(header_end)?;

    let mut output = vec![0u8; total_size];
    output[..header_end].copy_from_slice(&header_data);

    for chunk in &header.compressed_chunks {
        reader.seek(chunk.compressed_offset as u64)?;
        let chunk_data = reader.read_bytes(chunk.compressed_size as usize)?;
        decompress_chunk(
            &chunk_data,
            &mut output,
            chunk.uncompressed_offset as usize,
            header.compression_flags,
        )?;
    }

    Ok(output)
}

/// Decompress a single chunk with sub-block headers.
fn decompress_chunk(
    chunk_data: &[u8],
    output: &mut [u8],
    write_offset: usize,
    compression_flags: u32,
) -> Result<()> {
    let mut pos = 0;

    // Sub-block header
    let _tag = u32::from_le_bytes(chunk_data[pos..pos + 4].try_into().unwrap());
    pos += 4;
    let block_size = u32::from_le_bytes(chunk_data[pos..pos + 4].try_into().unwrap()) as usize;
    pos += 4;
    let _total_comp = i32::from_le_bytes(chunk_data[pos..pos + 4].try_into().unwrap());
    pos += 4;
    let total_uncomp = i32::from_le_bytes(chunk_data[pos..pos + 4].try_into().unwrap()) as usize;
    pos += 4;

    let num_blocks = total_uncomp.div_ceil(block_size);

    // Read sub-block sizes
    let mut sub_blocks = Vec::with_capacity(num_blocks);
    for _ in 0..num_blocks {
        let comp_sz = i32::from_le_bytes(chunk_data[pos..pos + 4].try_into().unwrap()) as usize;
        pos += 4;
        let uncomp_sz = i32::from_le_bytes(chunk_data[pos..pos + 4].try_into().unwrap()) as usize;
        pos += 4;
        sub_blocks.push((comp_sz, uncomp_sz));
    }

    // Decompress each sub-block
    let mut write_pos = write_offset;
    for (comp_sz, uncomp_sz) in sub_blocks {
        let compressed = &chunk_data[pos..pos + comp_sz];
        pos += comp_sz;

        if compression_flags & COMPRESS_LZO != 0 {
            let decompressed =
                lzokay_native::decompress(&mut std::io::Cursor::new(compressed), Some(uncomp_sz))
                    .map_err(|e| UpkError::LzoError(format!("{:?}", e)))?;
            output[write_pos..write_pos + decompressed.len()].copy_from_slice(&decompressed);
            write_pos += decompressed.len();
        } else {
            return Err(UpkError::UnsupportedCompression(compression_flags));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn import(object_name: &str, package_index: i32) -> ImportEntry {
        ImportEntry {
            class_package: "Core".to_string(),
            class_name: "Package".to_string(),
            package_index,
            object_name: object_name.to_string(),
            class_package_idx: 0,
            class_name_idx: 0,
            object_name_idx: 0,
        }
    }

    fn export(object_name: &str, package_index: i32) -> ExportEntry {
        ExportEntry {
            class_index: 0,
            super_index: 0,
            package_index,
            object_name: object_name.to_string(),
            object_name_idx: 0,
            object_name_num: 0,
            archetype: 0,
            object_flags: 0,
            serial_size: 0,
            serial_offset: 0,
            component_map: Vec::new(),
            export_flags: 0,
            gen_net_obj_count: Vec::new(),
            package_guid: [0; 4],
        }
    }

    /// Imports: [0] GLB-Global (package), [1] Meshes (group in -1),
    /// [2] GLB-RingTransporter00 (mesh in -2).
    /// Exports: [0] TheWorld, [1] PersistentLevel (in export 1).
    fn package() -> Package {
        Package {
            header: PackageHeader::default(),
            names: Vec::new(),
            imports: vec![
                import("GLB-Global", 0),
                import("Meshes", -1),
                import("GLB-RingTransporter00", -2),
            ],
            exports: vec![export("TheWorld", 0), export("PersistentLevel", 1)],
            decompressed: None,
            filepath: String::new(),
        }
    }

    #[test]
    fn import_full_path_walks_the_outer_chain_to_the_package() {
        let pkg = package();
        assert_eq!(
            pkg.import_full_path(&pkg.imports[2]),
            "GLB-Global.Meshes.GLB-RingTransporter00"
        );
        assert_eq!(pkg.import_full_path(&pkg.imports[0]), "GLB-Global");
    }

    #[test]
    fn import_full_path_stops_on_a_self_referencing_outer() {
        let mut pkg = package();
        pkg.imports[0].package_index = -1;
        // Depth cap, not a hang: 20 repeats of the cycle plus the leaf.
        let path = pkg.import_full_path(&pkg.imports[0]);
        assert_eq!(path.matches("GLB-Global").count(), 21);
    }

    #[test]
    fn resolve_object_path_maps_sign_to_table() {
        let pkg = package();
        assert_eq!(pkg.resolve_object_path(0), "None");
        // Negative refs are negated 1-based import indices.
        assert_eq!(
            pkg.resolve_object_path(-3),
            "GLB-Global.Meshes.GLB-RingTransporter00"
        );
        // Positive refs are 1-based export indices.
        assert_eq!(pkg.resolve_object_path(2), "TheWorld.PersistentLevel");
    }

    #[test]
    fn resolve_object_path_reports_out_of_range_refs() {
        let pkg = package();
        assert_eq!(pkg.resolve_object_path(-4), "<import_oob:-4>");
        assert_eq!(pkg.resolve_object_path(3), "<export_oob:3>");
    }
}
