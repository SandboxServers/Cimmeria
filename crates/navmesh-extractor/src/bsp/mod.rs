//! Phase 1.4 — BSP (`UModel`) collision geometry extraction.
//!
//! A cooked `.umap` carries BSP geometry in two shapes, distinguished
//! purely by which export *owns* the `Model`:
//!
//! 1. **`Level`-owned** (`PersistentLevel.Model`) — the compiled CSG
//!    world geometry. Already in world space; no actor transform.
//! 2. **Actor-owned** — one standalone `Model` per placed `Brush` or
//!    `*Volume` actor, in actor-local space. Placed via the owning
//!    actor's `Location` / `Rotation` / `DrawScale` / `DrawScale3D`
//!    (and `PrePivot`, which UE3 subtracts *before* scale/rotate).
//!
//! Ownership is the classifier rather than the actor's `Brush`
//! ObjectProperty because it can't dangle: every `Model` export's
//! `package_index` points at its owner directly.
//!
//! # Coordinates and winding
//!
//! Triangles are pushed into the [`TriangleSoup`] in **raw UE3-cm world
//! space**, exactly as [`crate::staticmesh`] does. The BW axis swap
//! (`BW = (ue.y/100, ue.z/100, ue.x/100)`) belongs to the OBJ writer,
//! not here.
//!
//! Winding matters: NavBuilder treats a triangle as walkable when the
//! UE3 right-hand-rule normal of the *emitted* index order points down
//! (`n_ue3.z < 0`). BSP node vertex pools are wound so that the
//! right-hand-rule normal of the stored order is the **inward**-facing
//! direction relative to the surface plane normal — see
//! [`EMIT_REVERSED`] for the measurement that settled this.
//!
//! # Volume classes
//!
//! Enumerated across all 144 `Maps/Castle/Castle-*.umap` chunks, the
//! only `*Volume` classes that own a `Model` are `TriggerVolume` (62)
//! and `DynamicTriggerVolume` (15). Both are excluded: they are query
//! volumes, not blocking geometry, and their convex hulls would seal
//! off doorways. `BlockingVolume` is included by name even though the
//! Castle set contains none, because it *is* blocking geometry
//! wherever it appears. Any other class ending in `Volume` is excluded
//! conservatively and reported by name via
//! [`BspStats::unclassified_volume_classes`] so a new one can't slip in
//! silently.
//!
//! # Outer-hull skin
//!
//! One more class of face is dropped, by geometry rather than by owner
//! class: the top and bottom skin of the enclosing additive CSG block
//! the Castle interior is carved out of, where the chunk's terrain
//! proves it is buried. It is 68 % of the map's near-horizontal BSP
//! area and its upward-facing half rasterises into 87,709 m² of
//! unreachable walkable surface. See [`hull_cap`] for the rule and the
//! measurements behind it; the filter is off unless
//! [`BspOptions::terrain_ceiling`] is supplied.

pub mod hull_cap;

use cimmeria_upk::{Package, PropValue, TaggedProperty};
use cimmeria_upk_objects::model::{deserialize_model, CollisionFilter, Model};

use crate::geometry::TriangleSoup;
use crate::staticmesh::transform_from_actor_props;
use crate::transform::ActorTransform;

pub use hull_cap::{HullCap, TerrainCeiling};

/// Tagged-property stream offset for an `AActor` export. Actors carry a
/// 32-byte binary prefix ahead of their property block (components use
/// 8, plain `UObject`s use 4).
const ACTOR_PROPS_OFFSET: usize = 32;

/// The `Level` class owns the persistent level's world-space `Model`.
const LEVEL_CLASS: &str = "Level";

/// Actor classes whose owned `Model` is treated as blocking geometry.
///
/// `BlockingVolume` is listed although no Castle tile contains one —
/// it is, by definition, collision geometry, and the coordinator's
/// brief calls it out explicitly as INCLUDED.
pub const INCLUDED_ACTOR_CLASSES: &[&str] = &["Brush", "BlockingVolume"];

/// Volume classes explicitly excluded, with the reason.
///
/// Observed counts are across the 144 `Maps/Castle` chunks.
pub const EXCLUDED_VOLUME_CLASSES: &[(&str, &str)] = &[
    (
        "TriggerVolume",
        "query volume for Kismet/script events; its hull spans doorways and rooms",
    ),
    (
        "DynamicTriggerVolume",
        "movable query volume; same non-blocking semantics as TriggerVolume",
    ),
];

/// Whether the fan emitted from a node's vertex pool is reversed before
/// being pushed into the soup. **It is** — see below.
///
/// **Measured, not assumed.** BSP node vertex pools are wound so that
/// the right-hand-rule normal of the stored order *agrees* with the
/// authored surface normal (`Vectors[vNormal]`): on
/// `Castle-000a0002.umap`'s persistent-level `Model`, 400 of 411
/// near-horizontal faces agree and 11 disagree. That is the opposite
/// of UE3's render/collision convention, where triangles are wound
/// clockwise in UE3's left-handed basis so their right-hand-rule
/// normal is the *negation* of the surface normal — which is what
/// `StaticMesh` kDOP triangles give `staticmesh.rs`, and what
/// NavBuilder relies on (a triangle is walkable when the UE3
/// right-hand-rule normal of the emitted order points down,
/// `n_ue3.z < 0`).
///
/// Emitting BSP fans unreversed put every floor at the known-walkable
/// height (BW y ~66.79) at `n_ue3.z > 0` — 111 triangles that
/// NavBuilder would have read as ceilings, i.e. an entirely
/// unwalkable interior. Reversing brings BSP into the same convention
/// as the StaticMesh geometry it shares an OBJ with.
///
/// `bsp_castle_model_decode.rs`'s
/// `bsp_floor_winding_matches_navbuilder_walkable_convention` pins
/// this with the full bucket table; flip the constant only with a
/// re-measured table.
pub const EMIT_REVERSED: bool = true;

/// What [`collect_bsp_triangles`] did with a chunk.
#[derive(Debug, Default)]
pub struct BspStats {
    /// `Model` exports found in the chunk, of any ownership.
    pub models_total: usize,
    /// `Model` exports that decoded successfully.
    pub models_parsed: usize,
    /// `Model` exports that did not reach the soup because something
    /// about them failed to read: the export body, the `UModel`
    /// payload, or the owning actor's placement properties. Any
    /// non-zero value here is a decoder bug or corrupt input, not a
    /// data-shape we tolerate — the deserializer enforces exact
    /// consumption and a model with unreadable placement would land at
    /// the wrong world position.
    ///
    /// A placement failure is counted here *and* in
    /// [`models_parsed`](Self::models_parsed): the payload did decode,
    /// it just cannot be placed. The two counters answer different
    /// questions and deliberately overlap in that one case.
    pub models_failed: usize,
    /// One `"<export_name>#<idx>: <error>"` line per failed decode.
    pub parse_errors: Vec<String>,
    /// `Model`s owned by `PersistentLevel` (world space, no transform).
    pub level_models: usize,
    /// `Model`s owned by an included actor class, placed by transform.
    pub actor_models_included: usize,
    /// `Model`s skipped because their owner is an excluded volume.
    pub actor_models_excluded: usize,
    /// `Model`s owned by the package root — the editor builder brush,
    /// always an empty 108-byte stub. Skipped.
    pub builder_brush_models: usize,
    /// `Model`s that decoded but held zero BSP nodes.
    pub models_empty: usize,
    /// `(class name, count)` for owner classes that are neither
    /// `Level`, an included class, nor a listed excluded volume. Names
    /// ending in `Volume` were excluded; the rest were included.
    pub unclassified_owner_classes: Vec<(String, usize)>,
    /// Subset of the above that ended in `Volume` and were therefore
    /// excluded. Empty on every Castle tile.
    pub unclassified_volume_classes: Vec<(String, usize)>,
    /// BSP nodes walked across every decoded model.
    pub nodes_total: usize,
    /// Nodes with `NumVertices == 0` (pure splitters).
    pub nodes_without_vertices: usize,
    /// Nodes whose indices fell outside their arrays. Non-zero means
    /// the `FBspNode` field offsets are wrong.
    pub nodes_out_of_range: usize,
    /// Triangles pushed into the soup.
    pub triangles_emitted: usize,
    /// Triangles the PolyFlags/NodeFlags filter removed.
    pub triangles_excluded: usize,
    /// Per-flag triangle exclusion counts, summed over every model.
    /// Reported for *every* entry in the flag table regardless of
    /// whether the active filter uses that bit, so a wrong PolyFlags
    /// assumption shows up as an implausible count.
    pub excluded_by_flag: Vec<(&'static str, u32, usize)>,
    /// `(PolyFlags value, face-carrying node count)`, descending.
    pub poly_flag_histogram: Vec<(u32, usize)>,
    /// `Model`s in which [`hull_cap::HullCap::detect`] found an
    /// enclosing hull, i.e. where the cap filter could fire at all.
    pub models_with_hull: usize,
    /// Triangles dropped as outer-hull skin. Zero when
    /// [`BspOptions::terrain_ceiling`] is absent.
    pub hull_cap_triangles_excluded: usize,
    /// Surface area of those triangles, m². Reported because the cap is
    /// judged by how much unreachable *sheet* it removes, not by
    /// triangle count — 962 Castle triangles carry 122,801 m².
    pub hull_cap_area_m2: f64,
}

/// Knobs for [`collect_bsp_triangles`].
#[derive(Debug, Clone, Copy, Default)]
pub struct BspOptions<'a> {
    /// The chunk's terrain surface, used to decide whether an outer
    /// hull plane is buried. **Without it nothing is dropped** — the
    /// rule needs both halves and the geometric half alone deletes real
    /// floors (see [`hull_cap`]). `extract_map` supplies it from the
    /// terrain it has already decoded, unless `skip_terrain` is set.
    pub terrain_ceiling: Option<&'a TerrainCeiling>,
}

/// A decoded BSP `Model` plus everything needed to place and attribute
/// it.
///
/// Exposed so analysis code (the floor probe, the winding bucket
/// table) can reach the authored surface normals without re-walking
/// the package.
#[derive(Debug)]
pub struct BspModelInstance {
    /// 1-based export index of the `Model`, matching `inspect-export`.
    pub export_index: usize,
    /// Owning export's class name (`Level`, `Brush`, …).
    pub owner_class: String,
    /// Owning export's object name.
    pub owner_name: String,
    /// `true` when the model is the persistent level's world geometry.
    pub is_level_model: bool,
    /// Placement transform. Identity for a level model.
    pub transform: ActorTransform,
    /// UE3 `PrePivot`, subtracted from each model-local vertex before
    /// scale/rotate/translate. Zero for a level model.
    pub pre_pivot: [f32; 3],
    pub model: Model,
}

impl BspModelInstance {
    /// Rotate a model-local *direction* (an authored surface normal)
    /// into world space.
    ///
    /// A level model is already in world space. For an actor-placed
    /// model the direction is carried through the transform as the
    /// difference of two transformed points, which drops the
    /// translation while keeping scale and rotation — the cap
    /// classifier only looks at the sign of `z`, so a non-uniform
    /// scale would not change its answer either.
    pub fn normal_to_world(&self, n: [f32; 3]) -> [f32; 3] {
        if self.is_level_model {
            return n;
        }
        let o = self.transform.apply([0.0; 3]);
        let p = self.transform.apply(n);
        [p[0] - o[0], p[1] - o[1], p[2] - o[2]]
    }

    /// Place a model-local vertex into world space.
    pub fn to_world(&self, v: [f32; 3]) -> [f32; 3] {
        if self.is_level_model {
            // Level CSG is baked in world space already; applying the
            // (identity) transform would still be a no-op, but taking
            // the short path documents the intent.
            return v;
        }
        self.transform.apply([
            v[0] - self.pre_pivot[0],
            v[1] - self.pre_pivot[1],
            v[2] - self.pre_pivot[2],
        ])
    }
}

/// How a `Model`'s owner class is treated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnerKind {
    /// `PersistentLevel` — world-space CSG geometry.
    Level,
    /// The package root's editor builder brush; never has geometry.
    BuilderBrush,
    /// An actor class we treat as blocking geometry.
    IncludedActor,
    /// A listed non-blocking volume class.
    ExcludedVolume,
    /// Unrecognised class ending in `Volume` — excluded conservatively.
    UnknownVolume,
    /// Unrecognised non-volume actor class — included, since a
    /// `Model`-owning actor that isn't a volume is brush geometry.
    UnknownActor,
}

/// Decide what to do with a `Model` based on its owner's class name.
///
/// `None` means the model has no owner export (`package_index == 0`),
/// i.e. the package-root builder brush.
pub fn classify_owner(owner_class: Option<&str>) -> OwnerKind {
    let Some(class) = owner_class else {
        return OwnerKind::BuilderBrush;
    };
    if class == LEVEL_CLASS {
        return OwnerKind::Level;
    }
    if INCLUDED_ACTOR_CLASSES.contains(&class) {
        return OwnerKind::IncludedActor;
    }
    if EXCLUDED_VOLUME_CLASSES.iter().any(|(c, _)| *c == class) {
        return OwnerKind::ExcludedVolume;
    }
    if class.ends_with("Volume") {
        return OwnerKind::UnknownVolume;
    }
    OwnerKind::UnknownActor
}

impl OwnerKind {
    /// Whether geometry from a model with this owner reaches the soup.
    pub fn emits_geometry(self) -> bool {
        matches!(
            self,
            OwnerKind::Level | OwnerKind::IncludedActor | OwnerKind::UnknownActor
        )
    }
}

/// Walk every `Model` export in `pkg`, decode it, and push the
/// collidable triangles of the ones we keep into `soup` in world space.
///
/// Decode failures are counted and reported rather than aborting the
/// chunk — one bad `Model` shouldn't cost the whole tile's geometry —
/// but [`BspStats::models_failed`] is expected to be zero on real data
/// and the integration test asserts that.
///
/// `opts` is a required argument rather than a defaulted one because
/// its only field — the terrain ceiling — silently disables the
/// hull-cap filter when absent. A caller that forgets it would get a
/// navmesh with the buried skin back in, and nothing would say so.
pub fn collect_bsp_triangles(pkg: &Package, soup: &mut TriangleSoup, opts: BspOptions) -> BspStats {
    let (instances, mut stats) = collect_bsp_models(pkg);

    let filter = CollisionFilter::default();
    let mut flag_totals: Vec<(&'static str, u32, usize)> = Vec::new();
    let mut poly_hist: Vec<(u32, usize)> = Vec::new();

    for inst in &instances {
        let t = inst.model.triangulate(filter);
        stats.nodes_total += t.nodes_total;
        stats.nodes_without_vertices += t.nodes_without_vertices;
        stats.nodes_out_of_range += t.nodes_out_of_range;
        stats.triangles_excluded += t.triangles_excluded;

        for (name, bit, n) in &t.excluded_by_flag {
            match flag_totals.iter_mut().find(|e| e.0 == *name) {
                Some(e) => e.2 += n,
                None => flag_totals.push((name, *bit, *n)),
            }
        }
        for (value, n) in &t.poly_flag_histogram {
            match poly_hist.iter_mut().find(|e| e.0 == *value) {
                Some(e) => e.1 += n,
                None => poly_hist.push((*value, *n)),
            }
        }

        // World-space first: the cap planes are a property of the placed
        // model, and for an actor-placed model the local Z extent is not
        // the world Z extent.
        // A mirrored brush (negative scale determinant) reverses winding
        // just as a mirrored StaticMesh does; keep each face's facing.
        let mirrored = !inst.is_level_model && inst.transform.is_mirrored();
        let world_tris: Vec<[[f32; 3]; 3]> = t
            .triangles
            .iter()
            .map(|tri| {
                let (a, b, c) = (
                    inst.to_world(tri[0]),
                    inst.to_world(tri[1]),
                    inst.to_world(tri[2]),
                );
                if mirrored {
                    [a, c, b]
                } else {
                    [a, b, c]
                }
            })
            .collect();
        let cap = opts
            .terrain_ceiling
            .and_then(|ceiling| HullCap::detect(&world_tris).map(|cap| (cap, ceiling)));
        if cap.is_some() {
            stats.models_with_hull += 1;
        }

        for (i, world_tri) in world_tris.iter().enumerate() {
            if let Some((cap, ceiling)) = cap {
                let surf_index = t.triangle_surf[i] as usize;
                let n =
                    inst.normal_to_world(inst.model.surf_normal(surf_index).unwrap_or([0.0; 3]));
                if cap.is_buried_cap(world_tri, n, ceiling) {
                    stats.hull_cap_triangles_excluded += 1;
                    stats.hull_cap_area_m2 += triangle_area_m2(world_tri);
                    continue;
                }
            }
            let mut world = *world_tri;
            if EMIT_REVERSED {
                world.swap(1, 2);
            }
            soup.push(world);
            stats.triangles_emitted += 1;
        }
    }

    poly_hist.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    stats.poly_flag_histogram = poly_hist;
    stats.excluded_by_flag = flag_totals;
    stats
}

/// Decode every `Model` export that should contribute geometry and
/// return it with its placement, alongside the classification stats.
///
/// Split out from [`collect_bsp_triangles`] so analysis tooling (the
/// floor probe and the winding measurement) can read authored surface
/// normals, which a flat triangle soup has thrown away.
pub fn collect_bsp_models(pkg: &Package) -> (Vec<BspModelInstance>, BspStats) {
    let mut stats = BspStats::default();
    let mut out = Vec::new();

    for (idx, export) in pkg.exports.iter().enumerate() {
        if pkg.export_class_name(export) != "Model" {
            continue;
        }
        stats.models_total += 1;

        let owner = owner_export(pkg, export.package_index);
        let owner_class = owner.map(|o| pkg.export_class_name(o).to_string());
        let kind = classify_owner(owner_class.as_deref());

        match kind {
            OwnerKind::BuilderBrush => stats.builder_brush_models += 1,
            OwnerKind::Level => stats.level_models += 1,
            OwnerKind::IncludedActor => stats.actor_models_included += 1,
            OwnerKind::ExcludedVolume => stats.actor_models_excluded += 1,
            OwnerKind::UnknownVolume => {
                stats.actor_models_excluded += 1;
                if let Some(c) = &owner_class {
                    bump_class(&mut stats.unclassified_volume_classes, c);
                    bump_class(&mut stats.unclassified_owner_classes, c);
                }
            }
            OwnerKind::UnknownActor => {
                stats.actor_models_included += 1;
                if let Some(c) = &owner_class {
                    bump_class(&mut stats.unclassified_owner_classes, c);
                }
            }
        }

        if !kind.emits_geometry() {
            continue;
        }

        let data = match pkg.read_export_data(export) {
            Ok(d) => d,
            Err(e) => {
                stats.models_failed += 1;
                stats
                    .parse_errors
                    .push(format!("{}#{}: read: {e}", export.object_name, idx + 1));
                continue;
            }
        };
        let model = match deserialize_model(&data, &pkg.names) {
            Ok(m) => m,
            Err(e) => {
                stats.models_failed += 1;
                stats
                    .parse_errors
                    .push(format!("{}#{}: {e}", export.object_name, idx + 1));
                continue;
            }
        };
        stats.models_parsed += 1;
        if model.nodes.is_empty() {
            stats.models_empty += 1;
            continue;
        }

        let is_level_model = kind == OwnerKind::Level;
        let (transform, pre_pivot) = if is_level_model {
            (ActorTransform::default(), [0.0; 3])
        } else {
            // A model we cannot place is worse than a model we drop:
            // identity defaults put real geometry at the world origin,
            // and the navmesh then has a floor where there is none and
            // a hole where the floor should be. Skip it and say so.
            match actor_placement(pkg, export.package_index) {
                Ok(placement) => placement,
                Err(e) => {
                    stats.models_failed += 1;
                    stats.parse_errors.push(format!(
                        "{}#{}: owner placement: {e}",
                        export.object_name,
                        idx + 1
                    ));
                    continue;
                }
            }
        };

        out.push(BspModelInstance {
            export_index: idx + 1,
            owner_class: owner_class.unwrap_or_default(),
            owner_name: owner.map(|o| o.object_name.clone()).unwrap_or_default(),
            is_level_model,
            transform,
            pre_pivot,
            model,
        });
    }

    (out, stats)
}

/// Read the owning actor's placement properties.
///
/// `PrePivot` is read here rather than folded into [`ActorTransform`]
/// because UE3 subtracts it *before* scale and rotation
/// (`FTranslationMatrix(-PrePivot) * Scale * Rotation * Translation`),
/// which the shared `ActorTransform::apply` has no slot for. Every
/// Castle brush `Model` is an empty stub, so this path emits nothing on
/// the Castle data set — it is here for maps whose brushes do carry
/// geometry, and is untested against real non-empty data.
///
/// **Fallible on purpose.** Missing placement properties take UE3's
/// cooked defaults, which is correct — a cooked actor at the origin
/// with no rotation writes no `Location` tag. A property stream that
/// *stopped early* is a different thing: the tags after the break are
/// not absent, they are unread, and defaulting them silently moves the
/// model. Callers must count the failure and drop the model rather
/// than emit it somewhere plausible-looking.
fn actor_placement(
    pkg: &Package,
    owner_index: i32,
) -> std::result::Result<(ActorTransform, [f32; 3]), String> {
    let Some(owner) = owner_export(pkg, owner_index) else {
        return Err(format!(
            "owner index {owner_index} is not an export in this package"
        ));
    };
    let data = pkg
        .read_export_data(owner)
        .map_err(|e| format!("could not read owner {}: {e}", owner.object_name))?;
    if data.len() <= ACTOR_PROPS_OFFSET {
        return Err(format!(
            "owner {} body is {} bytes, too short for the {ACTOR_PROPS_OFFSET}-byte actor header \
             plus a property stream",
            owner.object_name,
            data.len()
        ));
    }
    let (props, end) =
        cimmeria_upk::parse_tagged_properties_with_end(&data, ACTOR_PROPS_OFFSET, &pkg.names);
    if !ended_on_none_terminator(&data, end, &pkg.names) {
        return Err(format!(
            "owner {} property stream stopped at byte {end} of {} without reaching the `None` \
             terminator; {} properties were read",
            owner.object_name,
            data.len(),
            props.len()
        ));
    }
    let pre_pivot = find_vector(&props, "PrePivot").unwrap_or([0.0; 3]);
    Ok((transform_from_actor_props(&props), pre_pivot))
}

/// Whether a tagged-property walk stopped on the `None` FName rather
/// than on a malformed or truncated tag.
///
/// `parse_tagged_properties_with_end` reports where it stopped but not
/// why. It always advances past the 8-byte FName it last read, so a
/// clean walk leaves exactly the `None` FName in the eight bytes before
/// `end`, and every early `break` leaves something else there (a
/// garbage name index, a type FName, or a size/array-index pair).
fn ended_on_none_terminator(data: &[u8], end: usize, names: &[cimmeria_upk::NameEntry]) -> bool {
    let Some(start) = end.checked_sub(8) else {
        return false;
    };
    if end > data.len() {
        return false;
    }
    let idx = i32::from_le_bytes([
        data[start],
        data[start + 1],
        data[start + 2],
        data[start + 3],
    ]);
    let num = i32::from_le_bytes([
        data[start + 4],
        data[start + 5],
        data[start + 6],
        data[start + 7],
    ]);
    num == 0
        && usize::try_from(idx)
            .ok()
            .and_then(|i| names.get(i))
            .is_some_and(|n| n.name == "None")
}

/// Resolve a `package_index` to its owning export, if any.
fn owner_export(pkg: &Package, owner_index: i32) -> Option<&cimmeria_upk::ExportEntry> {
    if owner_index <= 0 {
        return None;
    }
    pkg.exports.get((owner_index - 1) as usize)
}

fn find_vector(props: &[TaggedProperty], name: &str) -> Option<[f32; 3]> {
    props.iter().find(|p| p.name == name).and_then(|p| {
        if let PropValue::Vector { x, y, z } = &p.value {
            Some([*x, *y, *z])
        } else {
            None
        }
    })
}

/// Area of a UE3-cm triangle in square metres.
fn triangle_area_m2(t: &[[f32; 3]; 3]) -> f64 {
    let u = [t[1][0] - t[0][0], t[1][1] - t[0][1], t[1][2] - t[0][2]];
    let v = [t[2][0] - t[0][0], t[2][1] - t[0][1], t[2][2] - t[0][2]];
    let n = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    0.5 * (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt() as f64 / 10_000.0
}

fn bump_class(hist: &mut Vec<(String, usize)>, class: &str) {
    match hist.iter_mut().find(|e| e.0 == class) {
        Some(e) => e.1 += 1,
        None => hist.push((class.to_string(), 1)),
    }
}

#[cfg(test)]
mod tests;
