//! Byte-exact views of the package tables.
//!
//! The parsed [`crate::ImportEntry`] / [`crate::ExportEntry`] types resolve
//! names to strings and drop FName instance numbers, which is fine for reading
//! but loses information a writer needs. These raw forms keep every field as it
//! sits on disk so an entry can be re-emitted (or cloned into another package)
//! without guessing.

use std::ops::Range;

use byteorder::{ByteOrder, LittleEndian};

use crate::error::{Result, UpkError};
use crate::header::PackageHeader;

/// An FName as stored on disk: (name-table index, instance number).
pub type RawName = (i32, i32);

/// Import table entry, 28 bytes on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawImport {
    pub class_package: RawName,
    pub class_name: RawName,
    pub outer: i32,
    pub object_name: RawName,
}

pub const IMPORT_ENTRY_SIZE: usize = 28;

/// Export table entry. Variable length on disk (ComponentMap, generation counts).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawExport {
    pub class_index: i32,
    pub super_index: i32,
    pub outer: i32,
    pub object_name: RawName,
    pub archetype: i32,
    pub object_flags: u64,
    pub serial_size: i32,
    pub serial_offset: i32,
    pub component_map: Vec<(RawName, i32)>,
    pub export_flags: u32,
    pub gen_net_obj_count: Vec<i32>,
    pub package_guid: [u32; 4],
}

/// Byte offset of `serial_size` within an export entry; `serial_offset` follows it.
pub const EXPORT_SERIAL_SIZE_AT: usize = 32;

fn need(data: &[u8], pos: usize, len: usize, what: &str) -> Result<()> {
    if pos + len > data.len() {
        return Err(UpkError::Parse(format!(
            "{what}: need {len} bytes at {pos}, image is {}",
            data.len()
        )));
    }
    Ok(())
}

fn rd_i32(data: &[u8], pos: &mut usize, what: &str) -> Result<i32> {
    need(data, *pos, 4, what)?;
    let v = LittleEndian::read_i32(&data[*pos..]);
    *pos += 4;
    Ok(v)
}

fn rd_name(data: &[u8], pos: &mut usize, what: &str) -> Result<RawName> {
    Ok((rd_i32(data, pos, what)?, rd_i32(data, pos, what)?))
}

/// Walk the name table and return the byte offset one past its last entry.
pub fn name_table_end(image: &[u8], offset: usize, count: usize) -> Result<usize> {
    let mut pos = offset;
    for _ in 0..count {
        let len = rd_i32(image, &mut pos, "name length")?;
        let bytes = if len < 0 {
            (-len) as usize * 2
        } else {
            len as usize
        };
        need(image, pos, bytes + 8, "name entry")?;
        pos += bytes + 8;
    }
    Ok(pos)
}

pub fn read_imports(image: &[u8], offset: usize, count: usize) -> Result<Vec<RawImport>> {
    let mut pos = offset;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        out.push(RawImport {
            class_package: rd_name(image, &mut pos, "import class package")?,
            class_name: rd_name(image, &mut pos, "import class name")?,
            outer: rd_i32(image, &mut pos, "import outer")?,
            object_name: rd_name(image, &mut pos, "import object name")?,
        });
    }
    Ok(out)
}

/// Walk the export table, returning each entry and the byte range it occupies.
pub fn read_exports(
    image: &[u8],
    offset: usize,
    count: usize,
) -> Result<Vec<(RawExport, Range<usize>)>> {
    let mut pos = offset;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let start = pos;
        let class_index = rd_i32(image, &mut pos, "export class")?;
        let super_index = rd_i32(image, &mut pos, "export super")?;
        let outer = rd_i32(image, &mut pos, "export outer")?;
        let object_name = rd_name(image, &mut pos, "export name")?;
        let archetype = rd_i32(image, &mut pos, "export archetype")?;
        need(image, pos, 8, "export flags")?;
        let object_flags = LittleEndian::read_u64(&image[pos..]);
        pos += 8;
        let serial_size = rd_i32(image, &mut pos, "export serial size")?;
        let serial_offset = rd_i32(image, &mut pos, "export serial offset")?;
        let comp_count = rd_i32(image, &mut pos, "export component count")?;
        if !(0..=4096).contains(&comp_count) {
            return Err(UpkError::Parse(format!(
                "export entry at {start}: implausible ComponentMap count {comp_count}"
            )));
        }
        let mut component_map = Vec::with_capacity(comp_count as usize);
        for _ in 0..comp_count {
            let name = rd_name(image, &mut pos, "component map name")?;
            component_map.push((name, rd_i32(image, &mut pos, "component map ref")?));
        }
        let export_flags = rd_i32(image, &mut pos, "export flags")? as u32;
        let gen_count = rd_i32(image, &mut pos, "export generation count")?;
        if !(0..=64).contains(&gen_count) {
            return Err(UpkError::Parse(format!(
                "export entry at {start}: implausible generation count {gen_count}"
            )));
        }
        let mut gen_net_obj_count = Vec::with_capacity(gen_count as usize);
        for _ in 0..gen_count {
            gen_net_obj_count.push(rd_i32(image, &mut pos, "export generation")?);
        }
        let mut package_guid = [0u32; 4];
        for g in &mut package_guid {
            *g = rd_i32(image, &mut pos, "export guid")? as u32;
        }
        out.push((
            RawExport {
                class_index,
                super_index,
                outer,
                object_name,
                archetype,
                object_flags,
                serial_size,
                serial_offset,
                component_map,
                export_flags,
                gen_net_obj_count,
                package_guid,
            },
            start..pos,
        ));
    }
    Ok(out)
}

fn wr_i32(out: &mut Vec<u8>, v: i32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn wr_name(out: &mut Vec<u8>, n: RawName) {
    wr_i32(out, n.0);
    wr_i32(out, n.1);
}

impl RawImport {
    pub fn write(&self, out: &mut Vec<u8>) {
        wr_name(out, self.class_package);
        wr_name(out, self.class_name);
        wr_i32(out, self.outer);
        wr_name(out, self.object_name);
    }
}

impl RawExport {
    pub fn write(&self, out: &mut Vec<u8>) {
        wr_i32(out, self.class_index);
        wr_i32(out, self.super_index);
        wr_i32(out, self.outer);
        wr_name(out, self.object_name);
        wr_i32(out, self.archetype);
        out.extend_from_slice(&self.object_flags.to_le_bytes());
        wr_i32(out, self.serial_size);
        wr_i32(out, self.serial_offset);
        wr_i32(out, self.component_map.len() as i32);
        for (name, obj) in &self.component_map {
            wr_name(out, *name);
            wr_i32(out, *obj);
        }
        wr_i32(out, self.export_flags as i32);
        wr_i32(out, self.gen_net_obj_count.len() as i32);
        for g in &self.gen_net_obj_count {
            wr_i32(out, *g);
        }
        for g in &self.package_guid {
            wr_i32(out, *g as i32);
        }
    }
}

/// Write a name-table entry: ASCII FString (length counts the NUL) + flags.
pub fn write_name_entry(out: &mut Vec<u8>, name: &str, flags: u64) {
    wr_i32(out, name.len() as i32 + 1);
    out.extend_from_slice(name.as_bytes());
    out.push(0);
    out.extend_from_slice(&flags.to_le_bytes());
}

/// Byte offsets of the summary fields a patcher has to rewrite.
#[derive(Debug, Clone, Copy)]
pub struct SummaryLayout {
    pub total_header_size_at: usize,
    pub package_flags_at: usize,
    pub name_count_at: usize,
    pub name_offset_at: usize,
    pub export_count_at: usize,
    pub export_offset_at: usize,
    pub import_count_at: usize,
    pub import_offset_at: usize,
    pub depends_offset_at: usize,
    /// Start of the generation records (12 bytes each).
    pub generations_at: usize,
    pub compression_flags_at: usize,
    /// One past the chunk count: where an uncompressed summary ends.
    pub uncompressed_summary_end: usize,
}

impl SummaryLayout {
    /// Locate the fields for an Epic-486 summary. The only variable-length
    /// parts ahead of them are the folder name and the generation list.
    pub fn locate(image: &[u8], header: &PackageHeader) -> Result<Self> {
        if header.epic_version <= 414 {
            return Err(UpkError::Parse(format!(
                "patcher supports Epic > 414 summaries, got {}",
                header.epic_version
            )));
        }
        let mut pos = 8; // tag + packed version
        let total_header_size_at = pos;
        pos += 4;
        let folder_len = rd_i32(image, &mut pos, "folder name length")?;
        pos += if folder_len < 0 {
            (-folder_len) as usize * 2
        } else {
            folder_len as usize
        };
        let package_flags_at = pos;
        let name_count_at = pos + 4;
        let name_offset_at = pos + 8;
        let export_count_at = pos + 12;
        let export_offset_at = pos + 16;
        let import_count_at = pos + 20;
        let import_offset_at = pos + 24;
        let depends_offset_at = pos + 28;
        pos += 32 + 16; // + guid
        let gen_count = rd_i32(image, &mut pos, "generation count")? as usize;
        let generations_at = pos;
        pos += gen_count * 12 + 8; // + engine version + cooker version
        let compression_flags_at = pos;
        let layout = Self {
            total_header_size_at,
            package_flags_at,
            name_count_at,
            name_offset_at,
            export_count_at,
            export_offset_at,
            import_count_at,
            import_offset_at,
            depends_offset_at,
            generations_at,
            compression_flags_at,
            uncompressed_summary_end: pos + 8,
        };
        // Cross-check the walk against the parsed header before anyone writes
        // through these offsets.
        let at = |o: usize| LittleEndian::read_i32(&image[o..]);
        if at(name_count_at) != header.name_count
            || at(export_offset_at) != header.export_offset
            || at(depends_offset_at) != header.depends_offset
            || at(compression_flags_at) as u32 != header.compression_flags
        {
            return Err(UpkError::Parse(
                "summary field walk disagrees with parsed header".into(),
            ));
        }
        Ok(layout)
    }
}
