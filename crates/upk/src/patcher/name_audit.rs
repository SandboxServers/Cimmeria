//! Audit of the names a patched package's objects read through.
//!
//! The client's name-table loader (SGW.exe `0x4bad20`, the "serializing name
//! map" step of the linker tick) ANDs every entry's flags with its load
//! context; an entry that does not load on the client is stored as FName
//! `(0, 0)`, which is `None`. A property tag named by such an entry reads as
//! `None`, which ends the tagged list, and everything after it in the object
//! is read from the wrong offset (client patch 010 froze every client that
//! loaded its map this way; see `data/client-patches/README.md`, 011).
//! Nothing in the package itself is malformed, so a parse that only checks
//! structure passes. This walks the tagged property lists of the given
//! exports and reports every tag, type or struct name whose entry the client
//! would not load.
//!
//! Only exports the client itself loads are audited (`RF_LoadForClient` set
//! in the object's flags). Stock packages name editor-only names on editor
//! objects (a `Brush`, a `DrawLightConeComponent`, an `InterpCurveEdSetup`)
//! that the client never serializes: 90 such tags in stock Castle, 3 in stock
//! Ihpet, and none on any client-loaded object, so a whole-package audit of an
//! unmodified package is empty.

use std::ops::Range;

use byteorder::{ByteOrder, LittleEndian};

use crate::error::{Result, UpkError};
use crate::package::Package;

/// `RF_LoadForClient`: in a name entry's flags, and in an object's flags.
const LOAD_FOR_CLIENT: u64 = 0x0001_0000_0000_0000;
/// `RF_HasStack`: the object serializes an `FStateFrame`. Set on every actor.
const RF_HAS_STACK: u64 = 0x0200_0000_0000_0000;

/// Structs UE3 serializes as raw binary, not as a tagged list.
const BINARY_STRUCTS: [&str; 12] = [
    "Vector",
    "Rotator",
    "Color",
    "LinearColor",
    "Guid",
    "Box",
    "Plane",
    "Matrix",
    "Quat",
    "Vector2D",
    "IntPoint",
    "TwoVectors",
];

/// One name the client would read as `None`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnloadableName {
    /// 0-based export index.
    pub export: usize,
    pub name: String,
    /// Index of the entry in the name table (the table can hold the same
    /// string twice, with different flags).
    pub name_index: usize,
}

/// What an audit saw.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NameAudit {
    /// Names in client-loaded objects that the client would read as `None`.
    pub unloadable: Vec<UnloadableName>,
    /// Client-loaded exports whose property list the walk could not follow
    /// (native data in the middle, an index out of range). What it saw
    /// before is in `unloadable`; the rest of the object is not audited.
    pub not_audited: Vec<usize>,
    /// Client-loaded exports walked to their end.
    pub audited: usize,
}

/// Audit exports `range` (0-based). `unloadable` empty and `not_audited`
/// empty means the lists will be read as written.
pub fn audit_client_names(package: &Package, range: Range<usize>) -> Result<NameAudit> {
    let mut out = NameAudit::default();
    for index in range {
        let export = package
            .exports
            .get(index)
            .ok_or_else(|| UpkError::Parse(format!("export {index} out of range")))?;
        if export.object_flags & LOAD_FOR_CLIENT == 0 {
            continue;
        }
        let data = package.read_export_data(export)?;
        let start = if export.object_flags & RF_HAS_STACK != 0 {
            32
        } else if package.export_class_name(export).ends_with("Component") {
            8
        } else {
            4
        };
        if data.len() < start + 8 {
            out.not_audited.push(index);
            continue;
        }
        let mut found = Vec::new();
        match walk(package, &data, start, &mut found) {
            Ok(_) => out.audited += 1,
            Err(_) => out.not_audited.push(index),
        }
        out.unloadable
            .extend(found.into_iter().map(|name_index| UnloadableName {
                export: index,
                name: package.names[name_index].name.clone(),
                name_index,
            }));
    }
    Ok(out)
}

fn loadable(package: &Package, index: i32) -> Option<bool> {
    usize::try_from(index)
        .ok()
        .and_then(|i| package.names.get(i))
        .map(|n| n.flags & LOAD_FOR_CLIENT != 0)
}

/// Check one name field; push it when unloadable. `Err` when out of range.
fn check(package: &Package, index: i32, found: &mut Vec<usize>) -> Result<()> {
    match loadable(package, index) {
        Some(true) => Ok(()),
        Some(false) => {
            found.push(index as usize);
            Ok(())
        }
        None => Err(UpkError::Parse("name index out of range".into())),
    }
}

/// Walk one tagged list from `pos`; push every unloadable name index. Returns
/// the offset just past the terminator.
fn walk(package: &Package, data: &[u8], mut pos: usize, found: &mut Vec<usize>) -> Result<usize> {
    let bad = || UpkError::Parse("tagged list the audit cannot follow".into());
    loop {
        if pos + 8 > data.len() {
            return Err(bad());
        }
        let name = LittleEndian::read_i32(&data[pos..]);
        check(package, name, found)?;
        pos += 8;
        if package.names[name as usize].name == "None" {
            return Ok(pos);
        }
        if pos + 16 > data.len() {
            return Err(bad());
        }
        let ty = LittleEndian::read_i32(&data[pos..]);
        check(package, ty, found)?;
        let size = LittleEndian::read_i32(&data[pos + 8..]).max(0) as usize;
        pos += 16;
        match package.names[ty as usize].name.as_str() {
            // Epic 486: a 4-byte value the tag's size does not count.
            "BoolProperty" => {
                pos += 4;
                continue;
            }
            "StructProperty" => {
                if pos + 8 > data.len() {
                    return Err(bad());
                }
                let st = LittleEndian::read_i32(&data[pos..]);
                check(package, st, found)?;
                pos += 8;
                let binary = BINARY_STRUCTS.contains(&package.names[st as usize].name.as_str());
                if !binary && pos + size <= data.len() {
                    walk(package, &data[..pos + size], pos, found)?;
                }
            }
            "ArrayProperty" if pos + size <= data.len() && size > 4 => {
                audit_struct_array(package, &data[pos..pos + size], found);
            }
            _ => {}
        }
        pos += size;
    }
}

/// An array property whose elements are tagged structs (what the remapper
/// also walks): `count`, then `count` tagged lists back to back. A bare array
/// of refs or ints does not parse that way, so it is skipped without a
/// verdict; findings from a parse that does not end exactly at the end of the
/// value are discarded for the same reason.
fn audit_struct_array(package: &Package, value: &[u8], found: &mut Vec<usize>) {
    let count = LittleEndian::read_i32(value);
    if count <= 0 || count as usize > value.len() {
        return;
    }
    let mut tentative = Vec::new();
    let mut pos = 4;
    for _ in 0..count {
        match walk(package, value, pos, &mut tentative) {
            Ok(end) => pos = end,
            Err(_) => return,
        }
    }
    if pos == value.len() {
        found.extend(tentative);
    }
}
