use crate::error::Result;
use crate::names::NameEntry;
use crate::reader::BinaryReader;
use std::io::{Read, Seek};

/// A single entry in the export table.
/// Variable-length on disk due to ComponentMap (TMap<FName,i32>) and GenNetObjCount (TArray<i32>).
/// Reverse-engineered from FUN_004bc9b0 in SGW.exe via Ghidra.
#[derive(Debug, Clone)]
pub struct ExportEntry {
    pub class_index: i32,
    pub super_index: i32,
    pub package_index: i32,
    pub object_name: String,
    pub object_name_idx: i32,
    pub object_name_num: i32,
    pub archetype: i32,
    pub object_flags: u64,
    pub serial_size: i32,
    pub serial_offset: i32,
    pub component_map: Vec<(String, i32)>,
    pub export_flags: u32,
    pub gen_net_obj_count: Vec<i32>,
    pub package_guid: [u32; 4],
}

fn resolve(names: &[NameEntry], idx: i32) -> String {
    if idx >= 0 && (idx as usize) < names.len() {
        names[idx as usize].name.clone()
    } else {
        format!("<invalid_{}>", idx)
    }
}

/// Parse the entire export table.
///
/// Disk order per record (SGW Epic version 486):
/// 1. ClassIndex: i32
/// 2. SuperIndex: i32
/// 3. PackageIndex: i32 (OuterIndex)
/// 4. ObjectName: FName (i32 + i32)
/// 5. Archetype: i32
/// 6. ObjectFlags: u64
/// 7. SerialSize: i32
/// 8. SerialOffset: i32 (version > 248)
/// 9. ComponentMap: TMap<FName, i32>
/// 10. ExportFlags: u32 (version >= 247)
/// 11. GenerationNetObjectCount: TArray<i32> (version > 321)
/// 12. PackageGuid: FGuid (version > 321)
pub fn parse_export_table<R: Read + Seek>(
    reader: &mut BinaryReader<R>,
    offset: i32,
    count: i32,
    epic_version: u16,
    names: &[NameEntry],
) -> Result<Vec<ExportEntry>> {
    reader.seek(offset as u64)?;
    let mut exports = Vec::with_capacity(count as usize);

    for _ in 0..count {
        let class_index = reader.read_i32()?;
        let super_index = reader.read_i32()?;
        let package_index = reader.read_i32()?;

        let (name_idx, name_num) = reader.read_fname()?;
        let object_name = resolve(names, name_idx);

        let archetype = reader.read_i32()?;
        let object_flags = reader.read_u64()?;
        let serial_size = reader.read_i32()?;

        let serial_offset = if serial_size > 0 || epic_version > 248 {
            reader.read_i32()?
        } else {
            0
        };

        // ComponentMap: TMap<FName, i32>
        let comp_count = reader.read_i32()?;
        let mut component_map = Vec::with_capacity(comp_count as usize);
        for _ in 0..comp_count {
            let (cm_idx, _cm_num) = reader.read_fname()?;
            let cm_value = reader.read_i32()?;
            component_map.push((resolve(names, cm_idx), cm_value));
        }

        // ExportFlags (version >= 247)
        let export_flags = if epic_version >= 247 {
            reader.read_u32()?
        } else {
            0
        };

        // GenerationNetObjectCount + PackageGuid (version > 321)
        let mut gen_net_obj_count = Vec::new();
        let mut package_guid = [0u32; 4];
        if epic_version > 321 {
            let gen_count = reader.read_i32()?;
            gen_net_obj_count = Vec::with_capacity(gen_count as usize);
            for _ in 0..gen_count {
                gen_net_obj_count.push(reader.read_i32()?);
            }
            for g in &mut package_guid {
                *g = reader.read_u32()?;
            }
        }

        exports.push(ExportEntry {
            class_index,
            super_index,
            package_index,
            object_name,
            object_name_idx: name_idx,
            object_name_num: name_num,
            archetype,
            object_flags,
            serial_size,
            serial_offset,
            component_map,
            export_flags,
            gen_net_obj_count,
            package_guid,
        });
    }

    Ok(exports)
}
