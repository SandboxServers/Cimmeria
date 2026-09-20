//! Rewrite the name indices and object refs inside export data that is being
//! moved from one package to another.
//!
//! Both are indices into per-package tables, so copying the bytes verbatim
//! would silently point at unrelated names and objects. Every field this module
//! does not positively understand is an error, not a pass-through: a missed ref
//! is corruption the client only reports as a crash.

use std::collections::HashMap;

use byteorder::{ByteOrder, LittleEndian};

use super::PatchSession;
use crate::error::{Result, UpkError};

/// Structs UE3 serializes as raw binary instead of a tagged property list.
/// They hold no names or object refs, so their bytes copy through unchanged.
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

/// Translates source-package names and refs into the target package, adding
/// names and imports to the target as needed.
pub struct Remapper<'a> {
    pub source: &'a PatchSession,
    pub target: &'a mut PatchSession,
    /// Source export ref (1-based) -> target export ref, for the objects being cloned.
    pub export_map: &'a HashMap<i32, i32>,
}

impl Remapper<'_> {
    pub fn name_index(&mut self, source_index: i32) -> Result<i32> {
        let name = self.source.name(source_index)?;
        Ok(self.target.ensure_name(name))
    }

    pub fn object(&mut self, source_ref: i32) -> Result<i32> {
        match source_ref {
            0 => Ok(0),
            r if r < 0 => self.target.ensure_import_from(self.source, r),
            r => self.export_map.get(&r).copied().ok_or_else(|| {
                UpkError::Parse(format!(
                    "data references source export {r} ({}), which is not being cloned",
                    self.source.package.resolve_object_path(r)
                ))
            }),
        }
    }

    fn name_at(&mut self, data: &mut [u8], pos: usize) -> Result<String> {
        let source_index = LittleEndian::read_i32(&data[pos..]);
        let name = self.source.name(source_index)?.to_string();
        let mapped = self.target.ensure_name(&name);
        LittleEndian::write_i32(&mut data[pos..], mapped);
        Ok(name)
    }

    fn object_at(&mut self, data: &mut [u8], pos: usize) -> Result<()> {
        let mapped = self.object(LittleEndian::read_i32(&data[pos..]))?;
        LittleEndian::write_i32(&mut data[pos..], mapped);
        Ok(())
    }
}

/// Called for every top-level property value after it has been remapped:
/// `(property name, struct name or type name, value bytes)`.
pub type ValueVisitor<'v> = &'v mut dyn FnMut(&str, &str, &mut [u8]) -> Result<()>;

/// Rewrite a tagged property list in place, starting at `start`. Field widths
/// never change, so the data keeps its length. Returns the offset just past the
/// `None` terminator.
pub fn rewrite_properties(
    data: &mut [u8],
    start: usize,
    rm: &mut Remapper,
    visit: ValueVisitor,
) -> Result<usize> {
    let mut pos = start;
    loop {
        if pos + 8 > data.len() {
            return Err(UpkError::Parse(format!(
                "property list ran off the end at {pos} (no None terminator)"
            )));
        }
        let prop_name = rm.name_at(data, pos)?;
        pos += 8;
        if prop_name == "None" {
            return Ok(pos);
        }
        if pos + 16 > data.len() {
            return Err(UpkError::Parse(format!("truncated tag for {prop_name}")));
        }
        let type_name = rm.name_at(data, pos)?;
        pos += 8;
        let size = LittleEndian::read_i32(&data[pos..]) as usize;
        pos += 8; // size + array index

        let unsupported = |why: &str| {
            Err(UpkError::Parse(format!(
                "cannot remap property {prop_name} ({type_name}, {size} bytes): {why}"
            )))
        };

        let mut value_kind = type_name.clone();
        match type_name.as_str() {
            "BoolProperty" => {
                // Epic 486: the value is a 4-byte int that `size` does not count.
                if size != 0 || pos + 4 > data.len() {
                    return unsupported("unexpected bool encoding");
                }
                pos += 4;
                continue;
            }
            "StructProperty" => {
                if pos + 8 + size > data.len() {
                    return unsupported("value runs past the data");
                }
                let struct_name = rm.name_at(data, pos)?;
                pos += 8;
                if !BINARY_STRUCTS.contains(&struct_name.as_str()) {
                    let mut ignore = |_: &str, _: &str, _: &mut [u8]| Ok(());
                    let end = rewrite_properties(&mut data[..pos + size], pos, rm, &mut ignore)?;
                    if end != pos + size {
                        return unsupported(&format!(
                            "struct {struct_name} is not a plain tagged list"
                        ));
                    }
                }
                value_kind = struct_name;
            }
            "ObjectProperty" | "ComponentProperty" | "ClassProperty" => {
                if size != 4 || pos + 4 > data.len() {
                    return unsupported("object ref is not 4 bytes");
                }
                rm.object_at(data, pos)?;
            }
            "NameProperty" => {
                if size != 8 || pos + 8 > data.len() {
                    return unsupported("name is not 8 bytes");
                }
                rm.name_at(data, pos)?;
            }
            "IntProperty" | "FloatProperty" | "StrProperty" => {
                if pos + size > data.len() {
                    return unsupported("value runs past the data");
                }
            }
            // Arrays need the inner type (which the tag does not carry) and byte
            // properties may or may not carry an enum name at this version.
            _ => return unsupported("type not supported by the remapper"),
        }
        visit(&prop_name, &value_kind, &mut data[pos..pos + size])?;
        pos += size;
    }
}
