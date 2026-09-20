//! Clone placed actors (and the components outered to them) from one cooked
//! map chunk into another, then register them in the target level's actor list.
//!
//! Scope is deliberately narrow: simple placed actors such as `StaticMeshActor`
//! and `InterpActor` whose components carry no baked lighting. Anything else
//! trips one of the shape checks below rather than producing a package the
//! client might half-load.

use std::collections::HashMap;

use byteorder::{ByteOrder, LittleEndian};

use super::property_remap::{rewrite_properties, Remapper};
use super::raw_tables::RawExport;
use super::PatchSession;
use crate::error::{Result, UpkError};

/// Actor serial data: FStateFrame (Node, StateNode, ProbeMask, LatentAction,
/// Offset) then NetIndex, then tagged properties.
const ACTOR_NET_INDEX_AT: usize = 28;
const ACTOR_PROPS_AT: usize = 32;
/// Component serial data: TemplateOwnerClass, NetIndex, then tagged properties.
const COMPONENT_NET_INDEX_AT: usize = 4;
const COMPONENT_PROPS_AT: usize = 8;
/// Level serial data: NetIndex, then tagged properties, then the actor TTransArray
/// (owner ref, count, refs).
const LEVEL_PROPS_AT: usize = 4;

/// Where the cloned group goes.
#[derive(Debug, Clone, Copy)]
pub enum Placement {
    /// Translate every actor by this UE-space delta.
    Offset([f32; 3]),
    /// Translate the group so the first listed actor lands here (UE space).
    FirstActorAt([f32; 3]),
}

#[derive(Debug)]
pub struct ClonedActor {
    pub source_index: usize,
    pub class: String,
    pub target_ref: i32,
    pub location: [f32; 3],
    pub components: usize,
}

#[derive(Debug)]
pub struct CloneReport {
    pub actors: Vec<ClonedActor>,
    pub names_added: usize,
    pub imports_added: usize,
    pub exports_added: usize,
    pub level_actor_count: (i32, i32),
}

fn err<T>(msg: String) -> Result<T> {
    Err(UpkError::Parse(msg))
}

fn read_vec3(v: &[u8]) -> [f32; 3] {
    [
        LittleEndian::read_f32(&v[0..]),
        LittleEndian::read_f32(&v[4..]),
        LittleEndian::read_f32(&v[8..]),
    ]
}

/// `Location` of a source actor, read without modifying anything.
fn source_location(source: &PatchSession, index: usize) -> Result<[f32; 3]> {
    let data = source.export_data(index)?;
    let props = crate::parse_tagged_properties(data, ACTOR_PROPS_AT, &source.package.names);
    for p in props {
        if let (true, crate::PropValue::Vector { x, y, z }) = (p.name == "Location", &p.value) {
            return Ok([*x, *y, *z]);
        }
    }
    err(format!(
        "source export {index} has no Location property (actor at origin is not supported)"
    ))
}

/// Clone `actor_indices` (0-based source export indices) and their components.
pub fn clone_actors(
    target: &mut PatchSession,
    source: &PatchSession,
    actor_indices: &[usize],
    placement: Placement,
) -> Result<CloneReport> {
    if actor_indices.is_empty() {
        return err("no actors to clone".into());
    }
    let source_level_ref = source.level_export_index()? as i32 + 1;
    let target_level_index = target.level_export_index()?;
    let target_level_ref = target_level_index as i32 + 1;
    let before = target.additions();

    // Plan: each actor followed by the exports outered to it.
    let mut plan: Vec<(usize, Vec<usize>)> = Vec::new();
    for &actor in actor_indices {
        let entry = source.raw_export(actor)?;
        if entry.outer != source_level_ref {
            return err(format!(
                "source export {actor} is not outered to the level; not a placed actor"
            ));
        }
        let actor_ref = actor as i32 + 1;
        let components: Vec<usize> = (0..source.package.exports.len())
            .filter(|&i| source.package.exports[i].package_index == actor_ref)
            .collect();
        for &c in &components {
            let c_ref = c as i32 + 1;
            if source
                .package
                .exports
                .iter()
                .any(|e| e.package_index == c_ref)
            {
                return err(format!(
                    "component {c} of actor {actor} has sub-objects; not supported"
                ));
            }
        }
        plan.push((actor, components));
    }

    let mut export_map: HashMap<i32, i32> = HashMap::new();
    export_map.insert(source_level_ref, target_level_ref);
    let mut next = target.next_export_ref();
    for (actor, components) in &plan {
        for &i in std::iter::once(actor).chain(components) {
            export_map.insert(i as i32 + 1, next);
            next += 1;
        }
    }

    let delta = match placement {
        Placement::Offset(d) => d,
        Placement::FirstActorAt(p) => {
            let from = source_location(source, plan[0].0)?;
            [p[0] - from[0], p[1] - from[1], p[2] - from[2]]
        }
    };

    let mut actors = Vec::new();
    let mut new_actor_refs = Vec::new();
    for (actor, components) in &plan {
        let mut location = None;
        let target_ref = clone_one(target, source, &export_map, *actor, true, &mut |v| {
            let from = read_vec3(v);
            let to = [from[0] + delta[0], from[1] + delta[1], from[2] + delta[2]];
            for (i, c) in to.iter().enumerate() {
                LittleEndian::write_f32(&mut v[i * 4..], *c);
            }
            location = Some(to);
        })?;
        let location = location.ok_or_else(|| {
            UpkError::Parse(format!("source export {actor} has no Location property"))
        })?;
        for &c in components {
            clone_one(target, source, &export_map, c, false, &mut |_| {})?;
        }
        new_actor_refs.push(target_ref);
        actors.push(ClonedActor {
            source_index: *actor,
            class: source
                .package
                .export_class_name(&source.package.exports[*actor])
                .to_string(),
            target_ref,
            location,
            components: components.len(),
        });
    }

    let level_actor_count = splice_level_actors(target, target_level_index, &new_actor_refs)?;

    let after = target.additions();
    Ok(CloneReport {
        actors,
        names_added: after.0 - before.0,
        imports_added: after.1 - before.1,
        exports_added: after.2 - before.2,
        level_actor_count,
    })
}

fn clone_one(
    target: &mut PatchSession,
    source: &PatchSession,
    export_map: &HashMap<i32, i32>,
    index: usize,
    is_actor: bool,
    on_location: &mut dyn FnMut(&mut [u8]),
) -> Result<i32> {
    let src = source.raw_export(index)?.clone();
    let mut data = source.export_data(index)?.to_vec();
    let what = if is_actor { "actor" } else { "component" };
    let (net_index_at, props_at) = if is_actor {
        (ACTOR_NET_INDEX_AT, ACTOR_PROPS_AT)
    } else {
        (COMPONENT_NET_INDEX_AT, COMPONENT_PROPS_AT)
    };
    if data.len() < props_at + 8 {
        return err(format!(
            "{what} export {index} is too short ({} bytes)",
            data.len()
        ));
    }
    if is_actor {
        // The state frame's Node and StateNode both name the actor's class. If
        // they do not, this is not the layout the offsets above assume.
        let node = LittleEndian::read_i32(&data[0..]);
        let state_node = LittleEndian::read_i32(&data[4..]);
        if node != src.class_index || state_node != src.class_index {
            return err(format!(
                "actor export {index}: state frame ({node}, {state_node}) does not name class {}",
                src.class_index
            ));
        }
    }

    let mut rm = Remapper {
        source,
        target,
        export_map,
    };
    let entry = RawExport {
        class_index: rm.object(src.class_index)?,
        super_index: rm.object(src.super_index)?,
        outer: rm.object(src.outer)?,
        object_name: (rm.name_index(src.object_name.0)?, src.object_name.1),
        archetype: rm.object(src.archetype)?,
        object_flags: src.object_flags,
        serial_size: 0,
        serial_offset: 0,
        component_map: src
            .component_map
            .iter()
            // ComponentMap values are 0-based export indices, not object refs.
            .map(|(n, i)| Ok(((rm.name_index(n.0)?, n.1), rm.object(*i + 1)? - 1)))
            .collect::<Result<_>>()?,
        export_flags: src.export_flags,
        gen_net_obj_count: src.gen_net_obj_count.clone(),
        package_guid: src.package_guid,
    };

    // Prefix refs.
    let prefix_refs: &[usize] = if is_actor { &[0, 4] } else { &[0] };
    for &at in prefix_refs {
        let mapped = rm.object(LittleEndian::read_i32(&data[at..]))?;
        LittleEndian::write_i32(&mut data[at..], mapped);
    }
    // The source NetIndex belongs to the source package's net-object numbering.
    // INDEX_NONE keeps the clone out of the target's numbering entirely.
    LittleEndian::write_i32(&mut data[net_index_at..], -1);

    let end = rewrite_properties(&mut data, props_at, &mut rm, &mut |name, kind, value| {
        if name == "Location" && kind == "Vector" && value.len() == 12 {
            on_location(value);
        }
        Ok(())
    })?;

    let tail = &data[end..];
    let tail_ok = if is_actor {
        tail.is_empty()
    } else {
        // An empty LODData array: no lightmaps or shadow maps to carry over.
        tail == [0, 0, 0, 0]
    };
    if !tail_ok {
        return err(format!(
            "{what} export {index} has {} bytes of post-property data the cloner does not understand",
            tail.len()
        ));
    }

    Ok(rm.target.add_export(entry, data))
}

/// Append `new_refs` to the level's actor array. Returns (old count, new count).
fn splice_level_actors(
    target: &mut PatchSession,
    level_index: usize,
    new_refs: &[i32],
) -> Result<(i32, i32)> {
    let level_ref = level_index as i32 + 1;
    let serial_offset = target.raw_export(level_index)?.serial_offset as usize;
    let table_len = target.export_count() as i32;
    let data = target.export_data(level_index)?.to_vec();

    let (_, props_end) =
        crate::parse_tagged_properties_with_end(&data, LEVEL_PROPS_AT, &target.package.names);
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

    // Moving the level's data would break any inline bulk-data header inside it,
    // since those store their own absolute file offset. Detect, do not guess.
    let self_offsets = (0..data.len().saturating_sub(3))
        .filter(|&p| LittleEndian::read_i32(&data[p..]) as usize == serial_offset + p + 4)
        .count();
    if self_offsets > 0 {
        return err(format!(
            "Level export holds {self_offsets} self-referential file offset(s); relocating it needs an offset fix-up"
        ));
    }

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
