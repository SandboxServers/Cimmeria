use crate::error::Result;
use crate::reader::BinaryReader;
use std::io::{Read, Seek};

/// A single entry in the name table.
#[derive(Debug, Clone)]
pub struct NameEntry {
    pub name: String,
    pub flags: u64,
}

/// Parse the entire name table from the package.
pub fn parse_name_table<R: Read + Seek>(
    reader: &mut BinaryReader<R>,
    offset: i32,
    count: i32,
) -> Result<Vec<NameEntry>> {
    reader.seek(offset as u64)?;
    let mut names = Vec::with_capacity(count as usize);

    for _ in 0..count {
        let name = reader.read_fstring()?;
        let flags = reader.read_u64()?;
        names.push(NameEntry { name, flags });
    }

    Ok(names)
}
