//! World-space cover-node extraction from cooked `.umap` chunks.
//!
//! Castle and Castle_CellBlock author their gameplay cover directly in the
//! level, not through the reusable `covernodes_*.pak` prefab corpus
//! (`docs/reverse-engineering/findings/cover-world-placement.md`). Two
//! shapes exist, and both are already absolute UE3 world space:
//!
//! - **Pattern A** — an `SGWSpecCoverNode` actor per cover position. Its
//!   owned `SGWCoverNodeComponent` carries no transform; the actor's
//!   `Location`/`Rotation`/`DrawScale3D` place it.
//! - **Pattern B** — a `StaticMeshActor` whose `CoverNodeArray` lists
//!   several `SGWCoverNodeComponent`s, each with its own
//!   `Translation`/`Rotation`/`Scale3D` flagged `Absolute* = true`.
//!
//! The walker keys on the component, not the actor: every
//! `SGWCoverNodeComponent` export is visited once and its *outer* export's
//! class decides the pattern. That sidesteps the `PackageIndex` /
//! `export_full_path` name-collision gap the finding documents (113
//! components in one chunk share the literal object name).
//!
//! # Output conventions
//!
//! - Positions are BigWorld metres: `bw = (ue.y, ue.z, ue.x) / 100`
//!   (`docs/engine/navmesh-build-pipeline.md` §1).
//! - `orient` is the convention `cell::cover` already consumes
//!   (`scoring.rs::is_flanked`): the node's facing is `(cos o, sin o)` in
//!   BigWorld `(x, z)`, i.e. measured from +X toward +Z. That is **not** the
//!   entity-yaw convention (`dx.atan2(dz)`, from +Z toward +X); the two are
//!   related by `orient = π/2 − yaw`. The facing is the marker's local +X
//!   axis, which points at the obstacle (the desk cluster's seven nodes all
//!   face its centre), so a threat in that half-plane is defended against.
//!
//! Submodules: [`walk`] (per-package decode), [`grouping`] (nodes → sets),
//! [`sql`] (seed rendering).

pub mod grouping;
pub mod sql;
pub mod walk;

#[cfg(test)]
mod tests;

use std::path::Path;

use crate::transform::ActorTransform;

pub use grouping::{group_into_sets, CoverSetOut, SET_ID_WORLD_STRIDE};
pub use walk::{extract_package_cover, ChunkCoverStats};

/// UE3 → BigWorld axis mapping plus the cm → m scale.
pub fn ue_to_bw(ue: [f32; 3]) -> [f32; 3] {
    [ue[1] / 100.0, ue[2] / 100.0, ue[0] / 100.0]
}

/// The cover `orient` (radians in `[0, 2π)`, facing `(cos, sin)` in BW
/// `(x, z)`) of a UE3 transform's local +X axis.
///
/// The axis goes through the full scale-then-rotate chain the navmesh
/// extractor uses for vertices, so a mirrored marker (negative
/// `DrawScale3D.x`) faces the way its rendered arrow does, and pitch/roll
/// are honoured before the result is flattened onto the ground plane.
/// Returns `None` when the axis has no horizontal extent (a zero X scale,
/// or a marker pitched straight up), which a caller must treat as a
/// malformed node rather than silently facing +X.
pub fn orient_from_transform(xf: &ActorTransform) -> Option<f32> {
    let origin = xf.apply([0.0; 3]);
    let tip = xf.apply([1.0, 0.0, 0.0]);
    orient_from_ue_direction([tip[0] - origin[0], tip[1] - origin[1], tip[2] - origin[2]])
}

/// [`orient_from_transform`] for an already world-space UE3 direction.
pub fn orient_from_ue_direction(dir: [f32; 3]) -> Option<f32> {
    // BW x = UE y, BW z = UE x; UE z (up) is dropped.
    let (bx, bz) = (dir[1], dir[0]);
    if bx.hypot(bz) < 1e-6 {
        return None;
    }
    Some(bz.atan2(bx).rem_euclid(std::f32::consts::TAU))
}

/// `resources."ECoverHeight"`, by the byte value the component stores.
///
/// The byte ordinals are the BigWorld def enum's
/// (`entities/defs/enumerations.xml` `ECoverHeight`), and NA20 confirmed
/// them from a second angle: every `CoverHeight = 1` marker has
/// `DrawScale3D.z = 1.067` and every `2` has `1.524` — the Mid/High heights
/// read out of `SGW.exe` (`findings/cover-system.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Height {
    Low,
    Mid,
    High,
    Los,
}

impl Height {
    pub fn from_byte(b: u8) -> Option<Self> {
        match b {
            0 => Some(Self::Low),
            1 => Some(Self::Mid),
            2 => Some(Self::High),
            3 => Some(Self::Los),
            _ => None,
        }
    }

    pub fn sql_name(self) -> &'static str {
        match self {
            Self::Low => "HEIGHT_Low",
            Self::Mid => "HEIGHT_Mid",
            Self::High => "HEIGHT_High",
            Self::Los => "HEIGHT_LOS",
        }
    }
}

/// `resources."ECoverQuality"`, by byte value (`enumerations.xml`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Quality {
    Good,
    Better,
    Best,
    None_,
}

impl Quality {
    pub fn from_byte(b: u8) -> Option<Self> {
        match b {
            0 => Some(Self::Good),
            1 => Some(Self::Better),
            2 => Some(Self::Best),
            3 => Some(Self::None_),
            _ => None,
        }
    }

    pub fn sql_name(self) -> &'static str {
        match self {
            Self::Good => "QUALITY_Good",
            Self::Better => "QUALITY_Better",
            Self::Best => "QUALITY_Best",
            Self::None_ => "QUALITY_None",
        }
    }
}

/// Class-default values for a property the cooker omitted.
///
/// UE3 cooks only the properties that differ from the archetype, so an
/// absent property means "the archetype's value", which is *not*
/// necessarily zero (see the `ue3-absent-property-defaults` trap). Each
/// value below is inferred from the census of both Castle maps
/// (2026-09-25, 4,009 `SGWSpecCoverNode` markers):
///
/// - `CoverHeight`: absent on 240 markers, every one of which has
///   `DrawScale3D.z = 0.71` (or an unconfigured 1.0) — the Low height. The
///   byte values present are only 1 and 2, so the archetype is 0 = Low.
/// - `CoverWidth`: absent exactly when `DrawScale3D.y = 1.0`, so 1.0.
/// - `CoverQuality`: the values present are 0, 1, 2 and 4 — **never 3**. A
///   value equal to the archetype is never written, so the archetype is 3,
///   QUALITY_None. MEDIUM confidence (no script package in the client tree
///   carries `Default__SGWCoverNodeComponent` to read it directly); the 11
///   markers it affects are also the ones with an explicit 0.0 width, i.e.
///   placeholders a designer never configured, which is what QUALITY_None
///   means in `cell::cover::types`.
pub mod class_default {
    pub const COVER_HEIGHT: u8 = 0;
    pub const COVER_QUALITY: u8 = 3;
    pub const COVER_WIDTH: f32 = 1.0;
}

/// Which authoring shape produced a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CoverPattern {
    /// `SGWSpecCoverNode` actor, one node each.
    SpecNode,
    /// `StaticMeshActor.CoverNodeArray`, several nodes per owner.
    NodeArray,
}

impl CoverPattern {
    pub fn label(self) -> &'static str {
        match self {
            Self::SpecNode => "SpecNode",
            Self::NodeArray => "NodeArray",
        }
    }
}

/// One decoded cover node, before grouping.
#[derive(Debug, Clone)]
pub struct ExtractedCoverNode {
    /// Chunk file stem, e.g. `Castle_CellBlock-fffefffd`.
    pub chunk: String,
    /// 0-based export index of the `SGWCoverNodeComponent`.
    pub component_export: usize,
    /// 0-based export index of the owning actor.
    pub owner_export: usize,
    pub pattern: CoverPattern,
    /// UE3 world position (cm), kept for provenance comments.
    pub ue_location: [f32; 3],
    /// BigWorld position (m).
    pub pos: [f32; 3],
    /// Radians, `cell::cover` convention (see the module docs).
    pub orient: f32,
    pub height: Height,
    pub quality: Quality,
    /// Metres; `CoverWidth` (which equals the marker's `DrawScale3D.y`).
    pub width: f32,
}

/// Every chunk's cover nodes for one map, in chunk-filename then
/// export-index order (the order set and node ids are assigned in).
#[derive(Debug, Default)]
pub struct MapCoverExtraction {
    pub nodes: Vec<ExtractedCoverNode>,
    pub stats: ChunkCoverStats,
    pub chunks_with_cover: usize,
    pub chunks_scanned: usize,
}

/// Walk every chunk under `map_dir`.
pub fn extract_map_cover(map_dir: &Path) -> crate::Result<MapCoverExtraction> {
    let mut out = MapCoverExtraction::default();
    for chunk_path in crate::umap::enumerate_chunks(map_dir)? {
        let stem = chunk_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("?")
            .to_string();
        let pkg = cimmeria_upk::Package::open(&chunk_path)?;
        let (nodes, stats) = extract_package_cover(&pkg, &stem);
        out.chunks_scanned += 1;
        if !nodes.is_empty() {
            out.chunks_with_cover += 1;
            tracing::info!(
                chunk = %stem,
                spec_nodes = stats.spec_nodes,
                array_nodes = stats.array_nodes,
                "cover_extract: chunk"
            );
        }
        out.stats.merge(&stats);
        out.nodes.extend(nodes);
    }
    Ok(out)
}
