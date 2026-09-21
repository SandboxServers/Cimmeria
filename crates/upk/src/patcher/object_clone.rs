//! Clone object graphs between cooked packages: placed actors with their
//! components, and Kismet sequences with everything outered to them.
//!
//! Each root brings every export outered to it, recursively. References must
//! resolve to a cloned object, an explicitly mapped existing object, the level,
//! or an import; anything else is an error rather than a dangling ref.

use std::collections::HashMap;

use byteorder::{ByteOrder, LittleEndian};

use super::host_splice::{append_object_array_ref, splice_level_actors};
use super::property_remap::{rewrite_properties, Remapper};
use super::raw_tables::RawExport;
use super::PatchSession;
use crate::error::{Result, UpkError};

/// `RF_HasStack`: the object serializes an `FStateFrame`. Set on every actor.
const RF_HAS_STACK: u64 = 0x0200_0000_0000_0000;
/// UE3 rotator units per full turn.
const ROTATOR_TURN: f32 = 65536.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// FStateFrame (Node, StateNode, ProbeMask, LatentAction, Offset), NetIndex, properties.
    Actor,
    /// TemplateOwnerClass, NetIndex, properties.
    Component,
    /// NetIndex, properties.
    Object,
}

impl Kind {
    fn of(session: &PatchSession, index: usize) -> Result<Self> {
        let export = &session.package.exports[index];
        Ok(
            if session.raw_export(index)?.object_flags & RF_HAS_STACK != 0 {
                Kind::Actor
            } else if session
                .package
                .export_class_name(export)
                .ends_with("Component")
            {
                Kind::Component
            } else {
                Kind::Object
            },
        )
    }

    /// (offsets of object refs in the prefix, NetIndex offset, property list offset)
    fn layout(self) -> (&'static [usize], usize, usize) {
        match self {
            Kind::Actor => (&[0, 4], 28, 32),
            Kind::Component => (&[0], 4, 8),
            Kind::Object => (&[], 0, 4),
        }
    }
}

/// Where cloned actors go. Coordinates are UE units.
#[derive(Debug, Clone, Copy)]
pub enum Placement {
    /// Translate every actor by this delta.
    Offset([f32; 3]),
    /// Translate the group so the first root that is an actor lands here.
    FirstActorAt([f32; 3]),
    /// Carry the group from one reference actor's frame to another's: the clone
    /// sits relative to `target` (an export in the target package) as the
    /// originals sit relative to `source`, including the yaw difference.
    Anchor { source: usize, target: usize },
}

pub struct CloneRequest<'a> {
    /// Source export indices (0-based). Descendants by outer come along.
    pub roots: &'a [usize],
    /// (source export index, existing target export index): refs to the former
    /// are redirected to the latter and the source object is not cloned.
    pub mapped: &'a [(usize, usize)],
    pub placement: Placement,
}

#[derive(Debug)]
pub struct ClonedObject {
    pub source_index: usize,
    pub class: String,
    pub target_ref: i32,
    /// Object name with its instance suffix, as it appears in an object path.
    pub name: String,
    pub location: Option<[f32; 3]>,
    pub is_root: bool,
}

#[derive(Debug)]
pub struct CloneReport {
    pub objects: Vec<ClonedObject>,
    pub names_added: usize,
    pub imports_added: usize,
    pub exports_added: usize,
    /// Level actor count before and after, when actors were cloned.
    pub level_actor_count: Option<(i32, i32)>,
    /// (parent sequence export index, new sequence ref) for each attached sequence.
    pub sequences_attached: Vec<(usize, i32)>,
}

fn err<T>(msg: String) -> Result<T> {
    Err(UpkError::Parse(msg))
}

/// Rigid transform applied to cloned actors.
struct Transform {
    from: [f32; 3],
    to: [f32; 3],
    yaw_delta: i32,
}

impl Transform {
    fn location(&self, p: [f32; 3]) -> [f32; 3] {
        let (dx, dy) = (p[0] - self.from[0], p[1] - self.from[1]);
        let a = self.yaw_delta as f32 / ROTATOR_TURN * std::f32::consts::TAU;
        let (sin, cos) = a.sin_cos();
        [
            self.to[0] + dx * cos - dy * sin,
            self.to[1] + dx * sin + dy * cos,
            self.to[2] + p[2] - self.from[2],
        ]
    }
}

/// `Location` and yaw of a placed actor.
fn actor_frame(session: &PatchSession, index: usize) -> Result<([f32; 3], i32)> {
    let data = session.export_data(index)?;
    let (mut location, mut yaw) = (None, 0);
    for p in crate::parse_tagged_properties(data, 32, &session.package.names) {
        match (p.name.as_str(), &p.value) {
            ("Location", crate::PropValue::Vector { x, y, z }) => location = Some([*x, *y, *z]),
            ("Rotation", crate::PropValue::Rotator { yaw: y, .. }) => yaw = *y,
            _ => {}
        }
    }
    match location {
        Some(l) => Ok((l, yaw)),
        None => err(format!(
            "export {index} has no Location property (actor at origin is not supported)"
        )),
    }
}

fn children_of(source: &PatchSession, index: usize) -> Vec<usize> {
    let parent_ref = index as i32 + 1;
    (0..source.package.exports.len())
        .filter(|&i| source.package.exports[i].package_index == parent_ref)
        .collect()
}

pub fn clone_objects(
    target: &mut PatchSession,
    source: &PatchSession,
    request: &CloneRequest,
) -> Result<CloneReport> {
    if request.roots.is_empty() {
        return err("nothing to clone".into());
    }
    let source_level = source.level_export_index()?;
    let target_level = target.level_export_index()?;
    let before = target.additions();

    // Depth-first order: every parent is numbered before its children.
    let mut order: Vec<(usize, bool)> = Vec::new();
    let mut stack: Vec<(usize, bool)> = request.roots.iter().rev().map(|&r| (r, true)).collect();
    while let Some((index, is_root)) = stack.pop() {
        source.raw_export(index)?;
        if order.iter().any(|&(i, _)| i == index) {
            return err(format!("source export {index} is listed or reached twice"));
        }
        order.push((index, is_root));
        stack.extend(
            children_of(source, index)
                .into_iter()
                .rev()
                .map(|c| (c, false)),
        );
    }

    let mut export_map: HashMap<i32, i32> = HashMap::new();
    export_map.insert(source_level as i32 + 1, target_level as i32 + 1);
    for &(s, t) in request.mapped {
        source.raw_export(s)?;
        target.raw_export(t)?;
        export_map.insert(s as i32 + 1, t as i32 + 1);
    }
    let first_ref = target.next_export_ref();
    for (n, &(index, _)) in order.iter().enumerate() {
        if export_map
            .insert(index as i32 + 1, first_ref + n as i32)
            .is_some()
        {
            return err(format!("source export {index} is both cloned and mapped"));
        }
    }

    let first_actor = order
        .iter()
        .find(|&&(i, root)| root && Kind::of(source, i).is_ok_and(|k| k == Kind::Actor));
    let transform = match request.placement {
        Placement::Offset(d) => Transform {
            from: [0.0; 3],
            to: d,
            yaw_delta: 0,
        },
        Placement::FirstActorAt(to) => {
            let Some(&(index, _)) = first_actor else {
                return err("FirstActorAt placement needs an actor among the roots".into());
            };
            Transform {
                from: actor_frame(source, index)?.0,
                to,
                yaw_delta: 0,
            }
        }
        Placement::Anchor {
            source: s,
            target: t,
        } => {
            let (from, from_yaw) = actor_frame(source, s)?;
            let (to, to_yaw) = actor_frame(target, t)?;
            Transform {
                from,
                to,
                yaw_delta: to_yaw.wrapping_sub(from_yaw),
            }
        }
    };

    let mut objects = Vec::new();
    let mut new_actor_refs = Vec::new();
    let mut new_sequences = Vec::new();
    for &(index, is_root) in &order {
        let kind = Kind::of(source, index)?;
        if kind == Kind::Actor && source.raw_export(index)?.outer != source_level as i32 + 1 {
            return err(format!("actor export {index} is not outered to the level"));
        }
        let cloned = clone_one(
            target,
            source,
            &export_map,
            index,
            kind,
            is_root,
            &transform,
        )?;
        if kind == Kind::Actor {
            new_actor_refs.push(cloned.target_ref);
        }
        if is_root && cloned.class == "Sequence" {
            new_sequences.push(cloned.target_ref);
        }
        objects.push(cloned);
    }

    let level_actor_count = if new_actor_refs.is_empty() {
        None
    } else {
        Some(splice_level_actors(target, target_level, &new_actor_refs)?)
    };

    // A sequence the engine cannot reach from its parent's SequenceObjects never
    // gets its events registered, so attach each cloned root sequence.
    let mut sequences_attached = Vec::new();
    for seq_ref in new_sequences {
        let parent_ref = target.new_export_outer(seq_ref)?;
        if parent_ref <= 0 {
            return err(format!("cloned sequence {seq_ref} has no parent sequence"));
        }
        let parent = parent_ref as usize - 1;
        append_object_array_ref(target, parent, "SequenceObjects", seq_ref)?;
        sequences_attached.push((parent, seq_ref));
    }

    let after = target.additions();
    Ok(CloneReport {
        objects,
        names_added: after.0 - before.0,
        imports_added: after.1 - before.1,
        exports_added: after.2 - before.2,
        level_actor_count,
        sequences_attached,
    })
}

fn clone_one(
    target: &mut PatchSession,
    source: &PatchSession,
    export_map: &HashMap<i32, i32>,
    index: usize,
    kind: Kind,
    is_root: bool,
    transform: &Transform,
) -> Result<ClonedObject> {
    let src = source.raw_export(index)?.clone();
    let mut data = source.export_data(index)?.to_vec();
    let (prefix_refs, net_index_at, props_at) = kind.layout();
    if data.len() < props_at + 8 {
        return err(format!(
            "export {index} is too short ({} bytes)",
            data.len()
        ));
    }
    if kind == Kind::Actor {
        // The state frame's Node and StateNode both name the actor's class. If
        // they do not, this is not the layout `Kind::layout` assumes.
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
    let outer = rm.object(src.outer)?;
    let name_index = rm.name_index(src.object_name.0)?;
    // A root lands under an object that already has children, so its name must
    // not repeat one of theirs: UE3 would construct it over the existing object.
    let name_number = if is_root {
        rm.target
            .free_name_number(outer, name_index, src.object_name.1)?
    } else {
        src.object_name.1
    };
    let entry = RawExport {
        class_index: rm.object(src.class_index)?,
        super_index: rm.object(src.super_index)?,
        outer,
        object_name: (name_index, name_number),
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

    for &at in prefix_refs {
        let mapped = rm.object(LittleEndian::read_i32(&data[at..]))?;
        LittleEndian::write_i32(&mut data[at..], mapped);
    }
    // The source NetIndex belongs to the source package's net-object numbering.
    // INDEX_NONE keeps the clone out of the target's numbering entirely.
    LittleEndian::write_i32(&mut data[net_index_at..], -1);

    let mut location = None;
    let end = rewrite_properties(
        &mut data,
        props_at,
        &mut rm,
        &mut |name, value_kind, value| {
            if kind != Kind::Actor {
                return Ok(());
            }
            if name == "Location" && value_kind == "Vector" && value.len() == 12 {
                let from = [
                    LittleEndian::read_f32(&value[0..]),
                    LittleEndian::read_f32(&value[4..]),
                    LittleEndian::read_f32(&value[8..]),
                ];
                let to = transform.location(from);
                for (i, c) in to.iter().enumerate() {
                    LittleEndian::write_f32(&mut value[i * 4..], *c);
                }
                location = Some(to);
            } else if name == "Rotation" && value_kind == "Rotator" && value.len() == 12 {
                let yaw = LittleEndian::read_i32(&value[4..]).wrapping_add(transform.yaw_delta);
                LittleEndian::write_i32(&mut value[4..], yaw);
            }
            Ok(())
        },
    )?;
    if kind == Kind::Actor && location.is_none() {
        return err(format!("actor export {index} has no Location property"));
    }

    // Native data after the property list. An empty array (LODData on a mesh
    // component, SavedActorTransforms on SeqAct_Interp) is a zero count; anything
    // more is baked data this cloner does not understand.
    let tail = &data[end..];
    if tail.len() > 8 || tail.iter().any(|&b| b != 0) {
        return err(format!(
            "export {index} has {} bytes of post-property data the cloner does not understand",
            tail.len()
        ));
    }

    let class = source
        .package
        .export_class_name(&source.package.exports[index])
        .to_string();
    let base_name = rm.target.name(name_index)?.to_string();
    let name = if name_number > 0 {
        format!("{base_name}_{}", name_number - 1)
    } else {
        base_name
    };
    let target_ref = rm.target.add_export(entry, data);
    Ok(ClonedObject {
        source_index: index,
        class,
        target_ref,
        name,
        location,
        is_root,
    })
}
