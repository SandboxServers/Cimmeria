use crate::error::Result;
use crate::names::NameEntry;
use crate::reader::BinaryReader;
use std::io::{Read, Seek};

/// A single entry in the import table.
/// Disk layout: ClassPackage(FName) + ClassName(FName) + PackageIndex(i32) + ObjectName(FName) = 28 bytes.
#[derive(Debug, Clone)]
pub struct ImportEntry {
    pub class_package: String,
    pub class_name: String,
    pub package_index: i32,
    pub object_name: String,
    // Raw indices for cross-referencing
    pub class_package_idx: i32,
    pub class_name_idx: i32,
    pub object_name_idx: i32,
}

fn resolve(names: &[NameEntry], idx: i32) -> String {
    if idx >= 0 && (idx as usize) < names.len() {
        names[idx as usize].name.clone()
    } else {
        format!("<invalid_{}>", idx)
    }
}

/// Parse the entire import table.
pub fn parse_import_table<R: Read + Seek>(
    reader: &mut BinaryReader<R>,
    offset: i32,
    count: i32,
    names: &[NameEntry],
) -> Result<Vec<ImportEntry>> {
    reader.seek(offset as u64)?;
    let mut imports = Vec::with_capacity(count as usize);

    for _ in 0..count {
        let (cp_idx, _cp_num) = reader.read_fname()?;
        let (cn_idx, _cn_num) = reader.read_fname()?;
        let pkg_index = reader.read_i32()?;
        let (on_idx, _on_num) = reader.read_fname()?;

        imports.push(ImportEntry {
            class_package: resolve(names, cp_idx),
            class_name: resolve(names, cn_idx),
            package_index: pkg_index,
            object_name: resolve(names, on_idx),
            class_package_idx: cp_idx,
            class_name_idx: cn_idx,
            object_name_idx: on_idx,
        });
    }

    Ok(imports)
}
