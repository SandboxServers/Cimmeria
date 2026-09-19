//! The census walk: one chunk in, tallies out.
//!
//! Everything here is a pure function of an already-open
//! [`Package`] plus a [`PackageIndex`], so the whole thing can be
//! driven from synthetic fixtures — which is the only way any of it
//! runs in CI, where the cooked client tree does not exist.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use cimmeria_navmesh_extractor::staticmesh::archetype::{
    import_chain, ActorArchetypeProps, ArchetypeCache, OpenPrefabs,
};
use cimmeria_navmesh_extractor::staticmesh::{archetype, mesh_ref};
use cimmeria_navmesh_extractor::transform::ActorTransform;
use cimmeria_upk::Package;
use cimmeria_upk_objects::{deserialize_static_mesh, PackageIndex, StaticMesh};

use super::geometry::{areas, ue3_to_bw};

/// Lower-case substrings that would make a mesh vertical circulation
/// or a doorway — the shapes that decide whether a navmesh gap is
/// missing geometry or a scripted link. Matched against the resolved
/// `package:object` key, so a prefab's own name cannot hide the mesh it
/// actually places.
pub const TRAVERSAL_KEYWORDS: &[&str] = &[
    "elevator",
    "lift",
    "platform",
    "stair",
    "ramp",
    "ladder",
    "step",
    "escalator",
    "catwalk",
    "bridge",
    "walkway",
    "door",
    "gate",
    "hatch",
];

/// One actor whose resolved mesh name matched [`TRAVERSAL_KEYWORDS`].
pub struct Traversal {
    pub chunk: String,
    pub mesh: String,
    pub bw: [f32; 3],
    /// `direct` or `stub` — whether the mesh came off the instance's
    /// own component or off the prefab archetype.
    pub kind: &'static str,
    /// Merged `bCollideActors`. `false` means it is NOT in the navmesh.
    pub collides: bool,
    /// Export-table `Archetype` is non-zero.
    pub archetype_instanced: bool,
}

#[derive(Default)]
pub struct MeshStats {
    pub instances: u64,
    pub chunks: BTreeSet<String>,
    /// Triangles in one instance of this mesh.
    pub tris_per_instance: usize,
    /// Summed |XZ| area across instances, BigWorld m².
    pub footprint_m2: f64,
    /// ...of which is near-horizontal and faces up for Recast.
    pub walkable_m2: f64,
    /// Archetype paths that resolve to this mesh.
    pub via_paths: BTreeSet<String>,
    /// One representative BigWorld position.
    pub sample_bw: [f32; 3],
    /// BigWorld y range across instances.
    pub y_min: f32,
    pub y_max: f32,
}

pub struct Placement {
    pub chunk: String,
    pub actor: String,
    pub mesh: String,
    pub arch_path: String,
    pub bw: [f32; 3],
}

/// Memo of `(package, object)` → decoded mesh, shared across chunks.
pub type MeshCache = HashMap<(String, String), Option<StaticMesh>>;

/// Everything the walk accumulates.
#[derive(Default)]
pub struct Census {
    pub stats: BTreeMap<String, MeshStats>,
    pub placements: Vec<Placement>,
    pub per_chunk: BTreeMap<String, u64>,
    pub failures: BTreeMap<String, u64>,
    /// Instance components carrying their own transform properties —
    /// which the actor-transform-only extractor would ignore.
    pub comp_transform_props: BTreeMap<String, u64>,
    /// Transform properties the instance omits and the archetype
    /// supplies.
    pub actor_inherited_transform: BTreeMap<String, u64>,
    pub collision_disabled: BTreeMap<String, u64>,
    pub traversal: Vec<Traversal>,
    pub stub_total: u64,
    pub direct_total: u64,
    /// Actors dropped because a body could not be read at all. Their
    /// own bucket rather than a default-empty property list: an
    /// unreadable component that is *treated* as an empty one looks
    /// exactly like an archetype stub, and the census then resolves an
    /// archetype and reports collision and placement for an actor whose
    /// real instance overrides were never seen.
    pub read_failures: BTreeMap<String, u64>,
}

impl Census {
    /// Walk every `StaticMeshActor` in one chunk.
    pub fn add_chunk(
        &mut self,
        chunk_name: &str,
        pkg: &Package,
        index: &PackageIndex,
        cache: &mut ArchetypeCache,
        mesh_cache: &mut MeshCache,
    ) {
        let mut open = OpenPrefabs::default();

        for export in &pkg.exports {
            if pkg.export_class_name(export) != "StaticMeshActor" || export.serial_size <= 32 {
                continue;
            }
            let Ok(data) = pkg.read_export_data(export) else {
                *self
                    .read_failures
                    .entry("actor body unreadable".to_string())
                    .or_default() += 1;
                continue;
            };
            let props = cimmeria_upk::parse_tagged_properties(&data, 32, &pkg.names);
            let Some(comp_ref) = mesh_ref::find_object(&props, "StaticMeshComponent") else {
                continue;
            };
            if comp_ref <= 0 {
                continue;
            }
            let Some(component) = pkg.exports.get((comp_ref - 1) as usize) else {
                continue;
            };
            let cdata = match pkg.read_export_data(component) {
                Ok(d) => d,
                Err(e) => {
                    // NOT `unwrap_or_default()`. An empty property list
                    // is indistinguishable from an archetype stub, so
                    // swallowing the error turns "we could not read
                    // this" into a confident, wrong census row.
                    *self
                        .read_failures
                        .entry(format!("component body unreadable: {e}"))
                        .or_default() += 1;
                    continue;
                }
            };
            let cprops = cimmeria_upk::parse_tagged_properties(&cdata, 8, &pkg.names);

            let is_stub = !cprops.iter().any(|p| p.name == "StaticMesh");
            if is_stub {
                self.stub_total += 1;
            } else {
                self.direct_total += 1;
            }

            // Inheritance probe 1: component-local transforms. The
            // extractor ignores these; if a map ever sets one, every
            // instance of it is placed wrong and nothing says so.
            for p in &cprops {
                if matches!(
                    p.name.as_str(),
                    "Translation" | "Rotation" | "Scale" | "Scale3D"
                ) {
                    *self
                        .comp_transform_props
                        .entry(format!(
                            "{}:{}",
                            if is_stub { "stub" } else { "direct" },
                            p.name
                        ))
                        .or_default() += 1;
                }
            }

            // Inheritance probe 2: what does the actor archetype supply
            // that the instance omits? Run for direct actors too — a
            // non-stub component says nothing about the actor's own
            // collision flag.
            let resolution =
                archetype::resolve_actor_archetype(pkg, export.archetype, index, cache, &mut open);
            let kind = if is_stub { "stub" } else { "direct" };
            let Some(arch) = resolution.props() else {
                *self
                    .failures
                    .entry(format!(
                        "{:?} actor-archetype {}",
                        resolution
                            .skip_reason()
                            .expect("unresolved carries a reason"),
                        mesh_ref::archetype_label(pkg, export.archetype)
                            .unwrap_or_else(|| "<none>".to_string())
                    ))
                    .or_default() += 1;
                continue;
            };
            let collides = arch.collides(&props);

            // Resolve the mesh for EVERY actor, emitted or not: the
            // traversal scan below has to see suppressed actors too
            // (an elevator with collision off is exactly the case that
            // would otherwise look like "no elevator in the map").
            let mesh_name = if is_stub {
                archetype::resolve_via_archetype(pkg, component, index, cache, &mut open)
            } else {
                mesh_ref::resolve_mesh_ref_from_component(pkg, comp_ref)
            }
            .map(|(p, o)| format!("{p}:{o}"))
            .unwrap_or_else(|r| format!("<{r:?}>"));

            if TRAVERSAL_KEYWORDS
                .iter()
                .any(|k| mesh_name.to_lowercase().contains(k))
            {
                let xf = arch.merge_transform(&props);
                self.traversal.push(Traversal {
                    chunk: chunk_name.to_string(),
                    mesh: mesh_name.clone(),
                    bw: ue3_to_bw(xf.location),
                    kind,
                    collides,
                    archetype_instanced: export.archetype != 0,
                });
            }

            if !collides {
                let path = import_chain(pkg, export.archetype)
                    .map(|c| c.join("."))
                    .unwrap_or_else(|| "<instance>".to_string());
                // Naming the mesh it *would* have emitted is the only
                // way to see what the pre-existing over-emission on the
                // direct half consists of.
                *self
                    .collision_disabled
                    .entry(format!("{kind}\t{mesh_name}\t{path}"))
                    .or_default() += 1;
                continue;
            }
            self.note_inherited_transforms(kind, &arch, &props);

            if !is_stub {
                continue;
            }
            self.add_stub_instance(
                chunk_name, pkg, export, component, &props, &arch, index, cache, &mut open,
                mesh_cache,
            );
        }
    }

    fn note_inherited_transforms(
        &mut self,
        kind: &str,
        arch: &ActorArchetypeProps,
        props: &[cimmeria_upk::TaggedProperty],
    ) {
        let mut note = |name: &str, value: Option<String>| {
            // Only interesting when the archetype supplies it AND
            // the instance is silent — that is the case where the
            // pre-archetype extractor placed the mesh wrong.
            if let Some(v) = value {
                if !props.iter().any(|p| p.name == name) {
                    *self
                        .actor_inherited_transform
                        .entry(format!("{kind}:{name} {v}"))
                        .or_default() += 1;
                }
            }
        };
        note("Rotation", arch.rotation.map(|v| format!("{v:?}")));
        note("DrawScale3D", arch.draw_scale_3d.map(|v| format!("{v:?}")));
        note("DrawScale", arch.draw_scale.map(|v| format!("{v}")));
        if !props.iter().any(|p| p.name == "Location") {
            *self
                .actor_inherited_transform
                .entry(format!("{kind}:NO Location on the instance"))
                .or_default() += 1;
        }
    }

    /// Resolve and measure one archetype-stub actor.
    #[allow(clippy::too_many_arguments)]
    fn add_stub_instance(
        &mut self,
        chunk_name: &str,
        pkg: &Package,
        export: &cimmeria_upk::ExportEntry,
        component: &cimmeria_upk::ExportEntry,
        props: &[cimmeria_upk::TaggedProperty],
        arch: &ActorArchetypeProps,
        index: &PackageIndex,
        cache: &mut ArchetypeCache,
        open: &mut OpenPrefabs,
        mesh_cache: &mut MeshCache,
    ) {
        let arch_path = import_chain(pkg, component.archetype)
            .map(|c| c.join("."))
            .unwrap_or_else(|| format!("<local:{}>", component.archetype));

        let key = match archetype::resolve_via_archetype(pkg, component, index, cache, open) {
            Ok(k) => k,
            Err(reason) => {
                *self
                    .failures
                    .entry(format!("{reason:?} {arch_path}"))
                    .or_default() += 1;
                return;
            }
        };

        let mesh = mesh_cache
            .entry(key.clone())
            .or_insert_with(|| load_mesh(index, &key));
        let Some(mesh) = mesh.as_ref() else {
            *self
                .failures
                .entry(format!("MeshLoadFailed {}.{}", key.0, key.1))
                .or_default() += 1;
            return;
        };
        let tris = mesh.collision_triangles();
        if tris.is_empty() {
            *self
                .failures
                .entry(format!("MeshNoCollision {}.{}", key.0, key.1))
                .or_default() += 1;
            return;
        }

        let xf: ActorTransform = arch.merge_transform(props);
        let bw = ue3_to_bw(xf.location);
        let mesh_name = format!("{}:{}", key.0, key.1);

        let (footprint, walkable) = areas(&tris, &xf);
        let entry = self
            .stats
            .entry(mesh_name.clone())
            .or_insert_with(|| MeshStats {
                sample_bw: bw,
                y_min: f32::MAX,
                y_max: f32::MIN,
                ..Default::default()
            });
        entry.instances += 1;
        entry.chunks.insert(chunk_name.to_string());
        entry.tris_per_instance = tris.len();
        entry.footprint_m2 += footprint;
        entry.walkable_m2 += walkable;
        entry.via_paths.insert(arch_path.clone());
        entry.y_min = entry.y_min.min(bw[1]);
        entry.y_max = entry.y_max.max(bw[1]);
        *self.per_chunk.entry(chunk_name.to_string()).or_default() += 1;

        self.placements.push(Placement {
            chunk: chunk_name.to_string(),
            actor: export.object_name.clone(),
            mesh: mesh_name,
            arch_path,
            bw,
        });
    }
}

/// Open a mesh's home package through the index and decode it.
pub fn load_mesh(index: &PackageIndex, key: &(String, String)) -> Option<StaticMesh> {
    let loc = index.find(&key.0, &key.1)?;
    let pkg = Package::open(&loc.file_path).ok()?;
    let export = pkg.exports.get(loc.export_index)?;
    let data = pkg.read_export_data(export).ok()?;
    deserialize_static_mesh(&data, &pkg.names).ok()
}
