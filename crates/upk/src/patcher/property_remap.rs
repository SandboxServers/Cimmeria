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

/// Array properties whose elements are object refs. The tag does not carry the
/// element type, so a bare `count + 4 * count` array is only remapped when its
/// property name is on this list; any other non-struct array is an error.
const OBJECT_ARRAYS: [&str; 7] = [
    "SequenceObjects",
    "LinkedVariables",
    "InterpGroups",
    "InterpTracks",
    "SMComponents",
    "Materials",
    "Targets",
];

/// Rewrite one array value (`count` then elements). Struct arrays are
/// self-describing: each element is a tagged list ending in `None`.
fn rewrite_array(value: &mut [u8], prop_name: &str, rm: &mut Remapper) -> Result<()> {
    let count = LittleEndian::read_i32(value);
    if count < 0 {
        return Err(UpkError::Parse(format!("array count {count}")));
    }
    let count = count as usize;
    if count == 0 {
        return if value.len() == 4 {
            Ok(())
        } else {
            Err(UpkError::Parse("empty array with trailing bytes".into()))
        };
    }
    if OBJECT_ARRAYS.contains(&prop_name) {
        if value.len() != 4 + count * 4 {
            return Err(UpkError::Parse(
                "object array is not 4 bytes per element".into(),
            ));
        }
        for i in 0..count {
            rm.object_at(value, 4 + i * 4)?;
        }
        return Ok(());
    }
    let mut pos = 4;
    for i in 0..count {
        let mut ignore = |_: &str, _: &str, _: &mut [u8]| Ok(());
        pos = rewrite_properties(value, pos, rm, &mut ignore).map_err(|e| {
            UpkError::Parse(format!(
                "element {i} is not a tagged struct and {prop_name} is not a known object array ({e})"
            ))
        })?;
    }
    if pos != value.len() {
        return Err(UpkError::Parse(format!(
            "struct array ended at {pos} of {} bytes",
            value.len()
        )));
    }
    Ok(())
}

/// Position of a top-level property's value, found without modifying anything.
pub struct TagPos {
    /// Offset of the tag's i32 size field.
    pub size_at: usize,
    pub value_at: usize,
    pub size: usize,
}

/// Find top-level property `wanted` in a tagged list starting at `start`.
pub fn find_property(
    data: &[u8],
    start: usize,
    name_of: &dyn Fn(i32) -> Result<String>,
    wanted: &str,
) -> Result<Option<TagPos>> {
    let mut pos = start;
    loop {
        if pos + 8 > data.len() {
            return Err(UpkError::Parse(
                "property list has no None terminator".into(),
            ));
        }
        let name = name_of(LittleEndian::read_i32(&data[pos..]))?;
        pos += 8;
        if name == "None" {
            return Ok(None);
        }
        if pos + 16 > data.len() {
            return Err(UpkError::Parse(format!("truncated tag for {name}")));
        }
        let type_name = name_of(LittleEndian::read_i32(&data[pos..]))?;
        let size_at = pos + 8;
        let size = LittleEndian::read_i32(&data[size_at..]) as usize;
        pos += 16;
        match type_name.as_str() {
            "BoolProperty" => {
                pos += 4;
                continue;
            }
            "StructProperty" => pos += 8,
            _ => {}
        }
        if pos + size > data.len() {
            return Err(UpkError::Parse(format!("{name} runs past the data")));
        }
        if name == wanted {
            return Ok(Some(TagPos {
                size_at,
                value_at: pos,
                size,
            }));
        }
        pos += size;
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
            "ByteProperty" => {
                // Epic 486 predates enum names in byte tags: the value is one raw byte.
                if size != 1 || pos + 1 > data.len() {
                    return unsupported("byte property is not a single byte");
                }
            }
            "ArrayProperty" => {
                if size < 4 || pos + size > data.len() {
                    return unsupported("array runs past the data");
                }
                if let Err(e) = rewrite_array(&mut data[pos..pos + size], &prop_name, rm) {
                    return unsupported(&e.to_string());
                }
            }
            _ => return unsupported("type not supported by the remapper"),
        }
        visit(&prop_name, &value_kind, &mut data[pos..pos + size])?;
        pos += size;
    }
}
