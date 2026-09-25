//! Edits to exports that already exist in the target package: the two places a
//! clone has to be registered for the engine to see it.
//!
//! Both grow the export, so its data is replaced (appended at the end of the
//! file) rather than edited in place.

use byteorder::{ByteOrder, LittleEndian};

use super::property_remap::find_property;
use super::PatchSession;
use crate::error::{Result, UpkError};

fn err<T>(msg: String) -> Result<T> {
    Err(UpkError::Parse(msg))
}

/// Append `new_refs` to the level's actor array. Returns (old count, new count).
pub(super) fn splice_level_actors(
    target: &mut PatchSession,
    level_index: usize,
    new_refs: &[i32],
) -> Result<(i32, i32)> {
    let level_ref = level_index as i32 + 1;
    let serial_offset = target.raw_export(level_index)?.serial_offset as usize;
    let table_len = target.export_count() as i32;
    let data = target.export_data(level_index)?.to_vec();

    // Level serial data: NetIndex, tagged properties, then the actor TTransArray
    // (owner ref, count, refs).
    let (_, props_end) = crate::parse_tagged_properties_with_end(&data, 4, &target.package.names);
    if props_end + 8 > data.len() {
        return err("Level export too short for an actor array".into());
    }
    let owner = LittleEndian::read_i32(&data[props_end..]);
    if owner != level_ref {
        return err(format!(
            "Level actor array owner is {owner}, expected the level itself ({level_ref})"
        ));
    }
    let count = LittleEndian::read_i32(&data[props_end + 4..]);
    let refs_at = props_end + 8;
    let refs_end = refs_at + count.max(0) as usize * 4;
    if count < 0 || refs_end > data.len() {
        return err(format!("Level actor count {count} is implausible"));
    }
    for i in 0..count as usize {
        let r = LittleEndian::read_i32(&data[refs_at + i * 4..]);
        if r < 0 || r > table_len {
            return err(format!("Level actor slot {i} holds {r}, not an export ref"));
        }
    }
    refuse_self_offsets(&data, serial_offset, "Level")?;

    let mut out = Vec::with_capacity(data.len() + new_refs.len() * 4);
    out.extend_from_slice(&data[..refs_end]);
    for r in new_refs {
        out.extend_from_slice(&r.to_le_bytes());
    }
    out.extend_from_slice(&data[refs_end..]);
    let new_count = count + new_refs.len() as i32;
    LittleEndian::write_i32(&mut out[props_end + 4..], new_count);
    target.replace_export_data(level_index, out)?;
    Ok((count, new_count))
}

/// Moving an export breaks any inline bulk-data header inside it, since those
/// store their own absolute file offset. Detect, do not guess.
fn refuse_self_offsets(data: &[u8], serial_offset: usize, what: &str) -> Result<()> {
    let hits = (0..data.len().saturating_sub(3))
        .filter(|&p| LittleEndian::read_i32(&data[p..]) as usize == serial_offset + p + 4)
        .count();
    if hits > 0 {
        return err(format!(
            "{what} export holds {hits} self-referential file offset(s); relocating it needs an offset fix-up"
        ));
    }
    Ok(())
}

/// Append an object ref to a top-level object-array property of an existing
/// export (for example a sequence's `SequenceObjects`).
pub(super) fn append_object_array_ref(
    target: &mut PatchSession,
    index: usize,
    property: &str,
    new_ref: i32,
) -> Result<()> {
    let serial_offset = target.raw_export(index)?.serial_offset as usize;
    let data = target.export_data(index)?.to_vec();
    let tag = {
        let name_of = |i: i32| target.name(i).map(str::to_string);
        find_property(&data, 4, &name_of, property)?
    };
    let Some(tag) = tag else {
        return err(format!("export {index} has no {property} property"));
    };
    let count = LittleEndian::read_i32(&data[tag.value_at..]);
    if count < 0 || tag.size != 4 + count as usize * 4 {
        return err(format!(
            "{property} on export {index} is not an object array"
        ));
    }
    refuse_self_offsets(&data, serial_offset, property)?;

    let insert_at = tag.value_at + tag.size;
    let mut out = Vec::with_capacity(data.len() + 4);
    out.extend_from_slice(&data[..insert_at]);
    out.extend_from_slice(&new_ref.to_le_bytes());
    out.extend_from_slice(&data[insert_at..]);
    LittleEndian::write_i32(&mut out[tag.size_at..], tag.size as i32 + 4);
    LittleEndian::write_i32(&mut out[tag.value_at..], count + 1);
    target.replace_export_data(index, out)
}
