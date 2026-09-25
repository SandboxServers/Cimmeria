//! Per-package decode of `SGWCoverNodeComponent` exports.

use cimmeria_upk::{Package, PropValue, TaggedProperty};

use super::{
    class_default, orient_from_transform, orient_from_ue_direction, ue_to_bw, CoverPattern,
    ExtractedCoverNode, Height, Quality,
};
use crate::transform::ActorTransform;

/// Bytes before an `AActor`'s tagged properties in a cooked export.
const ACTOR_PROPS_OFFSET: usize = 32;
/// Bytes before an `ActorComponent`'s tagged properties.
const COMPONENT_PROPS_OFFSET: usize = 8;

/// Tallies for one package (or, merged, one map). Every
/// `SGWCoverNodeComponent` export lands in exactly one bucket, so
/// `components == spec_nodes + array_nodes + skipped()`.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ChunkCoverStats {
    pub components: usize,
    pub spec_nodes: usize,
    pub array_nodes: usize,
    /// Pattern B children with `AbsoluteTranslation` false or absent,
    /// composed with the owner's transform. NA20 found none; a non-zero
    /// count is worth a look.
    pub array_nodes_composed: usize,
    /// Pattern B children not listed in any owner's `CoverNodeArray`.
    pub array_nodes_unlisted: usize,
    pub height_defaulted: usize,
    pub quality_defaulted: usize,
    pub width_defaulted: usize,
    /// `CoverQuality` bytes outside the 0..=3 enum, emitted as
    /// QUALITY_None. Castle has seven, all value 4.
    pub quality_out_of_range: usize,
    pub skipped_unreadable: usize,
    pub skipped_bad_owner: usize,
    pub skipped_bad_height: usize,
    pub skipped_no_facing: usize,
}

impl ChunkCoverStats {
    pub fn skipped(&self) -> usize {
        self.skipped_unreadable
            + self.skipped_bad_owner
            + self.skipped_bad_height
            + self.skipped_no_facing
    }

    pub fn is_balanced(&self) -> bool {
        self.components == self.spec_nodes + self.array_nodes + self.skipped()
    }

    pub fn merge(&mut self, o: &Self) {
        self.components += o.components;
        self.spec_nodes += o.spec_nodes;
        self.array_nodes += o.array_nodes;
        self.array_nodes_composed += o.array_nodes_composed;
        self.array_nodes_unlisted += o.array_nodes_unlisted;
        self.height_defaulted += o.height_defaulted;
        self.quality_defaulted += o.quality_defaulted;
        self.width_defaulted += o.width_defaulted;
        self.quality_out_of_range += o.quality_out_of_range;
        self.skipped_unreadable += o.skipped_unreadable;
        self.skipped_bad_owner += o.skipped_bad_owner;
        self.skipped_bad_height += o.skipped_bad_height;
        self.skipped_no_facing += o.skipped_no_facing;
    }
}

fn find<'a>(props: &'a [TaggedProperty], name: &str) -> Option<&'a PropValue> {
    props.iter().find(|p| p.name == name).map(|p| &p.value)
}

fn vector(props: &[TaggedProperty], name: &str) -> Option<[f32; 3]> {
    match find(props, name)? {
        PropValue::Vector { x, y, z } => Some([*x, *y, *z]),
        _ => None,
    }
}

fn rotator(props: &[TaggedProperty], name: &str) -> Option<[i32; 3]> {
    match find(props, name)? {
        PropValue::Rotator { pitch, yaw, roll } => Some([*pitch, *yaw, *roll]),
        _ => None,
    }
}

fn float(props: &[TaggedProperty], name: &str) -> Option<f32> {
    match find(props, name)? {
        PropValue::Float(f) => Some(*f),
        _ => None,
    }
}

fn byte(props: &[TaggedProperty], name: &str) -> Option<u8> {
    match find(props, name)? {
        PropValue::Byte(b) => b.first().copied(),
        _ => None,
    }
}

fn boolean(props: &[TaggedProperty], name: &str) -> Option<bool> {
    match find(props, name)? {
        PropValue::Bool(b) => Some(*b),
        _ => None,
    }
}

/// `ArrayProperty` of object references: `i32 count` then `count` i32s.
fn object_array(props: &[TaggedProperty], name: &str) -> Vec<i32> {
    let Some(PropValue::Array(bytes)) = find(props, name) else {
        return Vec::new();
    };
    if bytes.len() < 4 {
        return Vec::new();
    }
    let count = i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]).max(0) as usize;
    bytes[4..]
        .as_chunks::<4>()
        .0
        .iter()
        .take(count)
        .map(|c| i32::from_le_bytes(*c))
        .collect()
}

/// The actor placement defaults UE3 omits: origin, no rotation, unit scale.
fn actor_transform(props: &[TaggedProperty]) -> ActorTransform {
    ActorTransform {
        location: vector(props, "Location").unwrap_or([0.0; 3]),
        rotation: rotator(props, "Rotation").unwrap_or([0; 3]),
        draw_scale: float(props, "DrawScale").unwrap_or(1.0),
        draw_scale_3d: vector(props, "DrawScale3D").unwrap_or([1.0; 3]),
    }
}

/// A `USceneComponent`'s own transform (`Translation`/`Rotation`/`Scale`/
/// `Scale3D`), same defaults.
fn component_transform(props: &[TaggedProperty]) -> ActorTransform {
    ActorTransform {
        location: vector(props, "Translation").unwrap_or([0.0; 3]),
        rotation: rotator(props, "Rotation").unwrap_or([0; 3]),
        draw_scale: float(props, "Scale").unwrap_or(1.0),
        draw_scale_3d: vector(props, "Scale3D").unwrap_or([1.0; 3]),
    }
}

fn read_props(pkg: &Package, export: usize, offset: usize) -> Option<Vec<TaggedProperty>> {
    let e = pkg.exports.get(export)?;
    let data = pkg.read_export_data(e).ok()?;
    if data.len() < offset {
        return None;
    }
    Some(cimmeria_upk::parse_tagged_properties(
        &data, offset, &pkg.names,
    ))
}

/// Decode every `SGWCoverNodeComponent` in `pkg`.
///
/// `chunk` is the file stem recorded on each node for provenance and
/// ordering. Nodes come back in export order.
pub fn extract_package_cover(
    pkg: &Package,
    chunk: &str,
) -> (Vec<ExtractedCoverNode>, ChunkCoverStats) {
    let mut stats = ChunkCoverStats::default();
    let mut nodes = Vec::new();

    for (i, e) in pkg.exports.iter().enumerate() {
        if pkg.export_class_name(e) != "SGWCoverNodeComponent" {
            continue;
        }
        stats.components += 1;

        let Some(comp) = read_props(pkg, i, COMPONENT_PROPS_OFFSET) else {
            stats.skipped_unreadable += 1;
            tracing::warn!(
                chunk,
                export = i,
                "cover_extract: component body unreadable"
            );
            continue;
        };
        let owner = e.package_index;
        let owner_class = if owner > 0 {
            pkg.exports
                .get((owner - 1) as usize)
                .map(|o| pkg.export_class_name(o))
        } else {
            None
        };
        let owner_idx = (owner - 1).max(0) as usize;
        let owner_props = match owner_class {
            Some(_) => read_props(pkg, owner_idx, ACTOR_PROPS_OFFSET),
            None => None,
        };
        let Some(owner_props) = owner_props else {
            stats.skipped_bad_owner += 1;
            tracing::warn!(
                chunk,
                export = i,
                owner,
                "cover_extract: component has no readable owner actor"
            );
            continue;
        };
        let owner_xf = actor_transform(&owner_props);

        let self_ref = i as i32 + 1;
        let (pattern, location, orient) = if owner_class == Some("SGWSpecCoverNode") {
            // Pattern A: the actor is the node. A marker whose
            // `CoverNodeComponent` names a *different* component is not
            // the live one (an orphaned subobject); skip it rather than
            // emit a duplicate at the same spot.
            if let Some(PropValue::Object(r)) = find(&owner_props, "CoverNodeComponent") {
                if *r != self_ref {
                    stats.skipped_bad_owner += 1;
                    tracing::warn!(
                        chunk,
                        export = i,
                        owner_ref = *r,
                        "cover_extract: SGWSpecCoverNode points at another component"
                    );
                    continue;
                }
            }
            let loc = owner_xf.location;
            (
                CoverPattern::SpecNode,
                loc,
                orient_from_transform(&owner_xf),
            )
        } else {
            // Pattern B: the component carries its own transform.
            if !object_array(&owner_props, "CoverNodeArray").contains(&self_ref) {
                stats.array_nodes_unlisted += 1;
            }
            let cxf = component_transform(&comp);
            let absolute_t = boolean(&comp, "AbsoluteTranslation").unwrap_or(false);
            let absolute_r = boolean(&comp, "AbsoluteRotation").unwrap_or(false);
            let loc = if absolute_t {
                cxf.location
            } else {
                owner_xf.apply(cxf.location)
            };
            let orient = if absolute_r {
                orient_from_transform(&cxf)
            } else {
                // Relative rotation: take the component's local +X
                // through its own rotation/scale, then through the
                // owner's.
                let o = cxf.apply([0.0; 3]);
                let t = cxf.apply([1.0, 0.0, 0.0]);
                let local_dir = [t[0] - o[0], t[1] - o[1], t[2] - o[2]];
                let wo = owner_xf.apply([0.0; 3]);
                let wt = owner_xf.apply(local_dir);
                orient_from_ue_direction([wt[0] - wo[0], wt[1] - wo[1], wt[2] - wo[2]])
            };
            if !(absolute_t && absolute_r) {
                stats.array_nodes_composed += 1;
                tracing::warn!(
                    chunk,
                    export = i,
                    absolute_t,
                    absolute_r,
                    "cover_extract: CoverNodeArray child is not absolute; composed with owner"
                );
            }
            (CoverPattern::NodeArray, loc, orient)
        };

        let Some(orient) = orient else {
            stats.skipped_no_facing += 1;
            tracing::warn!(
                chunk,
                export = i,
                "cover_extract: marker +X axis has no horizontal extent"
            );
            continue;
        };

        let height_byte = match byte(&comp, "CoverHeight") {
            Some(b) => b,
            None => {
                stats.height_defaulted += 1;
                class_default::COVER_HEIGHT
            }
        };
        let Some(height) = Height::from_byte(height_byte) else {
            stats.skipped_bad_height += 1;
            tracing::warn!(
                chunk,
                export = i,
                height_byte,
                "cover_extract: CoverHeight outside the enum"
            );
            continue;
        };
        let quality_byte = match byte(&comp, "CoverQuality") {
            Some(b) => b,
            None => {
                stats.quality_defaulted += 1;
                class_default::COVER_QUALITY
            }
        };
        let quality = Quality::from_byte(quality_byte).unwrap_or_else(|| {
            stats.quality_out_of_range += 1;
            tracing::warn!(
                chunk,
                export = i,
                quality_byte,
                "cover_extract: CoverQuality outside the enum; emitting QUALITY_None"
            );
            Quality::None_
        });
        let width = match float(&comp, "CoverWidth") {
            Some(w) => w.abs(),
            None => {
                stats.width_defaulted += 1;
                class_default::COVER_WIDTH
            }
        };

        match pattern {
            CoverPattern::SpecNode => stats.spec_nodes += 1,
            CoverPattern::NodeArray => stats.array_nodes += 1,
        }
        nodes.push(ExtractedCoverNode {
            chunk: chunk.to_string(),
            component_export: i,
            owner_export: owner_idx,
            pattern,
            ue_location: location,
            pos: ue_to_bw(location),
            orient,
            height,
            quality,
            width,
        });
    }

    debug_assert!(stats.is_balanced(), "{stats:?}");
    (nodes, stats)
}
