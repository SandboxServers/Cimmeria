//! What a chunk's Kismet does to each actor in it.
//!
//! UE3 moves an `InterpActor` from Kismet: a `SeqAct_Interp` (a Matinee
//! player) names each of its `InterpGroup`s in a variable link whose
//! `LinkDesc` is the group name, and that link points at a
//! `SeqVar_Object` whose `ObjValue` is the actor. The group's tracks
//! live on the `InterpData` wired to the action's `Data` link. Kismet
//! can only reference actors in its own level, and every SGW map chunk
//! is its own level, so one chunk's exports are the whole story for the
//! actors in it.
//!
//! [`MotionEvidence::collect`] follows that chain for every Matinee in
//! the chunk and records, per actor export, each group that drives it
//! and every *other* Kismet reference to it. It does not judge; that is
//! [`super::classify`].

use std::collections::HashMap;

use cimmeria_upk::objects::kismet::is_kismet_class;
use cimmeria_upk::{parse_tagged_properties, Package, PropValue, TaggedProperty};

use super::move_track::{struct_array_elements, MoveTrack};

/// Kismet and Matinee objects carry a 4-byte prefix before their tagged
/// properties (actors carry 32).
const OBJECT_PROPS_OFFSET: usize = 4;

/// One Matinee group that names an actor.
#[derive(Debug, Clone, PartialEq)]
pub struct MatineeGroup {
    /// 1-based export index of the `SeqAct_Interp`.
    pub seq_act: i32,
    /// The variable link's `LinkDesc`, which UE3 matches to
    /// `InterpGroup.GroupName`.
    pub group: String,
    /// The group's tracks, or `None` when the action has no `Data`
    /// link, the `InterpData` could not be read, or it holds no group
    /// of that name.
    pub tracks: Option<GroupTracks>,
}

/// The tracks of one `InterpGroup`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GroupTracks {
    pub moves: Vec<MoveTrack>,
    /// Class names of every track that is not an `InterpTrackMove`.
    pub other: Vec<String>,
}

/// Everything the chunk's Kismet says about one actor.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ActorMotion {
    /// Matinee groups that drive the actor.
    pub matinee: Vec<MatineeGroup>,
    /// Every other Kismet reference, as `Class[LinkDesc]` for a
    /// variable link from an op other than `SeqAct_Interp`, or
    /// `Class.Property` for an object property that names the actor
    /// directly.
    pub other_refs: Vec<String>,
}

impl ActorMotion {
    /// No Kismet in the chunk references the actor at all.
    pub fn is_unreferenced(&self) -> bool {
        self.matinee.is_empty() && self.other_refs.is_empty()
    }
}

/// Per-actor Kismet evidence for one chunk package.
#[derive(Debug, Default)]
pub struct MotionEvidence {
    /// Keyed by 1-based export index of the referenced object.
    by_object: HashMap<i32, ActorMotion>,
}

/// One `SequenceOp` variable link.
struct VarLink {
    desc: String,
    vars: Vec<i32>,
}

impl MotionEvidence {
    /// Walk every Kismet export in `pkg`.
    pub fn collect(pkg: &Package) -> Self {
        let mut evidence = Self::default();
        let mut seqvar_object: HashMap<i32, i32> = HashMap::new();
        let mut ops: Vec<(i32, String, Vec<VarLink>)> = Vec::new();

        for (i, export) in pkg.exports.iter().enumerate() {
            let class = pkg.export_class_name(export);
            if !is_kismet_class(class) || export.serial_size <= OBJECT_PROPS_OFFSET as i32 {
                continue;
            }
            let props = read_props(pkg, i);
            let key = i as i32 + 1;
            for p in &props {
                let PropValue::Object(target) = p.value else {
                    continue;
                };
                if target <= 0 || p.name == "ParentSequence" {
                    continue;
                }
                if class == "SeqVar_Object" && p.name == "ObjValue" {
                    seqvar_object.insert(key, target);
                } else {
                    evidence
                        .entry(target)
                        .other_refs
                        .push(format!("{class}.{}", p.name));
                }
            }
            let links = variable_links(&props, pkg);
            if !links.is_empty() {
                ops.push((key, class.to_string(), links));
            }
        }

        for (key, class, links) in &ops {
            let groups = if class == "SeqAct_Interp" {
                Some(matinee_groups(pkg, links))
            } else {
                None
            };
            for link in links {
                if groups.is_some() && link.desc == "Data" {
                    continue;
                }
                for var in &link.vars {
                    let Some(&actor) = seqvar_object.get(var) else {
                        continue;
                    };
                    let entry = evidence.entry(actor);
                    match &groups {
                        Some(groups) => entry.matinee.push(MatineeGroup {
                            seq_act: *key,
                            group: link.desc.clone(),
                            tracks: groups.get(&link.desc).cloned(),
                        }),
                        None => entry.other_refs.push(format!("{class}[{}]", link.desc)),
                    }
                }
            }
        }
        evidence
    }

    /// Evidence for the actor at 0-based export index `export_index`.
    /// An actor nothing references gets an empty [`ActorMotion`].
    pub fn for_export(&self, export_index: usize) -> ActorMotion {
        self.by_object
            .get(&(export_index as i32 + 1))
            .cloned()
            .unwrap_or_default()
    }

    fn entry(&mut self, object: i32) -> &mut ActorMotion {
        self.by_object.entry(object).or_default()
    }
}

fn read_props(pkg: &Package, export_index: usize) -> Vec<TaggedProperty> {
    match pkg.read_export_data(&pkg.exports[export_index]) {
        Ok(data) => parse_tagged_properties(&data, OBJECT_PROPS_OFFSET, &pkg.names),
        Err(_) => Vec::new(),
    }
}

fn read_export_props(pkg: &Package, object: i32) -> Option<(String, Vec<TaggedProperty>)> {
    let index = usize::try_from(object).ok()?.checked_sub(1)?;
    let export = pkg.exports.get(index)?;
    Some((
        pkg.export_class_name(export).to_string(),
        read_props(pkg, index),
    ))
}

/// `VariableLinks`: an array of `SeqVarLink` structs, each with a
/// `LinkDesc` string and a `LinkedVariables` object array.
fn variable_links(props: &[TaggedProperty], pkg: &Package) -> Vec<VarLink> {
    let Some(PropValue::Array(bytes)) = props
        .iter()
        .find(|p| p.name == "VariableLinks")
        .map(|p| &p.value)
    else {
        return Vec::new();
    };
    struct_array_elements(bytes, &pkg.names)
        .into_iter()
        .map(|link| VarLink {
            desc: link
                .iter()
                .find_map(|p| match (&p.name[..], &p.value) {
                    ("LinkDesc", PropValue::Str(s)) => Some(s.clone()),
                    ("LinkDesc", PropValue::Name(s)) => Some(s.clone()),
                    _ => None,
                })
                .unwrap_or_default(),
            vars: link
                .iter()
                .find_map(|p| match (&p.name[..], &p.value) {
                    ("LinkedVariables", PropValue::Array(b)) => Some(object_array(b)),
                    _ => None,
                })
                .unwrap_or_default(),
        })
        .collect()
}

fn object_array(bytes: &[u8]) -> Vec<i32> {
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .skip(1)
        .map(|c| i32::from_le_bytes(*c))
        .collect()
}

/// Group name → tracks, from the `InterpData` on the action's `Data`
/// link.
fn matinee_groups(pkg: &Package, links: &[VarLink]) -> HashMap<String, GroupTracks> {
    let mut out = HashMap::new();
    let Some(data) = links
        .iter()
        .find(|l| l.desc == "Data")
        .and_then(|l| l.vars.first())
    else {
        return out;
    };
    let Some((_, data_props)) = read_export_props(pkg, *data) else {
        return out;
    };
    for group in object_array_prop(&data_props, "InterpGroups") {
        let Some((_, group_props)) = read_export_props(pkg, group) else {
            continue;
        };
        let Some(name) = group_props
            .iter()
            .find_map(|p| match (&p.name[..], &p.value) {
                ("GroupName", PropValue::Name(n)) => Some(n.clone()),
                _ => None,
            })
        else {
            continue;
        };
        let mut tracks = GroupTracks::default();
        for track in object_array_prop(&group_props, "InterpTracks") {
            let Some((class, track_props)) = read_export_props(pkg, track) else {
                tracks.other.push(format!("<unreadable #{track}>"));
                continue;
            };
            if class == "InterpTrackMove" {
                tracks
                    .moves
                    .push(MoveTrack::from_props(&track_props, &pkg.names));
            } else {
                tracks.other.push(class);
            }
        }
        out.insert(name, tracks);
    }
    out
}

fn object_array_prop(props: &[TaggedProperty], name: &str) -> Vec<i32> {
    props
        .iter()
        .find_map(|p| match (&p.name[..], &p.value) {
            (n, PropValue::Array(b)) if n == name => Some(object_array(b)),
            _ => None,
        })
        .unwrap_or_default()
}
