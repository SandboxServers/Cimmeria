//! `archetype_census` — what is in the prefab-archetype gap?
//!
//! `extract_map` reports how many `StaticMeshActor`s resolved and why
//! the rest didn't. That answers "how big is the gap"; it does not
//! answer "does the gap matter", which for a navmesh means: is the
//! missing geometry stairs, ramps and floor plates, or is it wall
//! lights and signage?
//!
//! This binary walks a map's chunks, resolves every archetype-stub
//! component through [`staticmesh::archetype`] — the same code path
//! `extract_map` uses — and reports, per resolved mesh:
//!
//! - instance count, and which chunks they sit in;
//! - collision triangles per instance and in total;
//! - world footprint (XZ projected area, BigWorld m²) and how much of
//!   it is near-horizontal **walkable-facing** under NavBuilder's
//!   convention (`floor_probe::recast_up` ≥ cos 45°, i.e. the UE3
//!   right-hand normal of the emitted winding points *down*);
//! - the BigWorld position of every instance, with `--positions`.
//!
//! It also re-measures two inheritance questions the extractor's
//! correctness rests on, so a future map that behaves differently
//! surfaces here rather than silently shifting geometry:
//!
//! - do instance components carry `Translation` / `Rotation` / `Scale`
//!   / `Scale3D` (which the actor-transform-only path would ignore)?
//! - do instance actors *omit* `Location` / `Rotation` / `DrawScale` /
//!   `DrawScale3D` while their actor archetype supplies one?
//!
//! ```bash
//! CIMMERIA_PACKAGE_INDEX=/path/package_index.bin \
//! cargo run -p cimmeria-navmesh-extractor --release --bin archetype_census -- \
//!   <cooked-root> <MapName> [--positions <TSV>] [--meshes <TSV>]
//! ```

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::Write;
use std::path::{Path, PathBuf};

use cimmeria_navmesh_extractor::floor_probe::recast_up;
use cimmeria_navmesh_extractor::staticmesh::archetype::{
    import_chain, ArchetypeCache, OpenPrefabs,
};
use cimmeria_navmesh_extractor::staticmesh::{archetype, mesh_ref};
use cimmeria_navmesh_extractor::transform::ActorTransform;
use cimmeria_navmesh_extractor::umap::enumerate_chunks;
use cimmeria_upk::Package;
use cimmeria_upk_objects::{deserialize_static_mesh, PackageIndex, StaticMesh};

/// Recast's walkable-slope cut-off at the 45° NavBuilder is configured
/// with: cos 45° == 1/sqrt(2). Same constant `floor_probe` defaults to.
const MIN_UP: f32 = std::f32::consts::FRAC_1_SQRT_2;

/// Centimetres per BigWorld unit.
const CM_PER_BW: f32 = 100.0;

/// `bw = (ue.Y, ue.Z, ue.X) / 100` — the calibrated Castle mapping.
fn ue3_to_bw(v: [f32; 3]) -> [f32; 3] {
    [v[1] / CM_PER_BW, v[2] / CM_PER_BW, v[0] / CM_PER_BW]
}

#[derive(Default)]
struct MeshStats {
    instances: u64,
    chunks: BTreeSet<String>,
    /// Triangles in one instance of this mesh.
    tris_per_instance: usize,
    /// Summed |XZ| area across instances, BigWorld m².
    footprint_m2: f64,
    /// ...of which is near-horizontal and faces up for Recast.
    walkable_m2: f64,
    /// Archetype paths that resolve to this mesh.
    via_paths: BTreeSet<String>,
    /// One representative BigWorld position.
    sample_bw: [f32; 3],
    /// BigWorld y range across instances.
    y_min: f32,
    y_max: f32,
}

struct Placement {
    chunk: String,
    actor: String,
    mesh: String,
    arch_path: String,
    bw: [f32; 3],
}

fn main() {
    let mut args = std::env::args().skip(1);
    let cooked = PathBuf::from(args.next().expect("usage: archetype_census <cooked-root> <map>"));
    let map = args.next().expect("map name");
    let mut positions_out: Option<PathBuf> = None;
    let mut meshes_out: Option<PathBuf> = None;
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--positions" => positions_out = args.next().map(PathBuf::from),
            "--meshes" => meshes_out = args.next().map(PathBuf::from),
            other => panic!("unknown flag {other}"),
        }
    }

    let index_path = std::env::var("CIMMERIA_PACKAGE_INDEX")
        .expect("set CIMMERIA_PACKAGE_INDEX to a package_index.bin");
    let index = PackageIndex::load(Path::new(&index_path)).expect("load PackageIndex");
    eprintln!(
        "index: {} packages, {} exports",
        index.package_count, index.export_count
    );

    let map_dir = cooked.join("Maps").join(&map);
    let chunks = enumerate_chunks(&map_dir).expect("enumerate_chunks");
    eprintln!("{}: {} chunks", map, chunks.len());

    let mut cache = ArchetypeCache::default();
    let mut mesh_cache: HashMap<(String, String), Option<StaticMesh>> = HashMap::new();
    let mut stats: BTreeMap<String, MeshStats> = BTreeMap::new();
    let mut placements: Vec<Placement> = Vec::new();
    let mut per_chunk: BTreeMap<String, u64> = BTreeMap::new();
    let mut failures: BTreeMap<String, u64> = BTreeMap::new();

    // Inheritance probes.
    let mut comp_transform_props: BTreeMap<String, u64> = BTreeMap::new();
    let mut actor_inherited_transform: BTreeMap<String, u64> = BTreeMap::new();
    let mut collision_disabled: BTreeMap<String, u64> = BTreeMap::new();
    let mut stub_total = 0u64;
    let mut direct_total = 0u64;

    for chunk_path in &chunks {
        let chunk_name = chunk_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("?")
            .to_string();
        let pkg = match Package::open(chunk_path) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("open {}: {e}", chunk_path.display());
                continue;
            }
        };
        let mut open = OpenPrefabs::default();

        for export in &pkg.exports {
            if pkg.export_class_name(export) != "StaticMeshActor" || export.serial_size <= 32 {
                continue;
            }
            let Ok(data) = pkg.read_export_data(export) else {
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
            let cdata = pkg.read_export_data(component).unwrap_or_default();
            let cprops = cimmeria_upk::parse_tagged_properties(&cdata, 8, &pkg.names);

            let is_stub = !cprops.iter().any(|p| p.name == "StaticMesh");
            if is_stub {
                stub_total += 1;
            } else {
                direct_total += 1;
            }

            // Inheritance probe 1: component-local transforms. The
            // extractor ignores these; if a map ever sets one, every
            // instance of it is placed wrong and nothing says so.
            for p in &cprops {
                if matches!(
                    p.name.as_str(),
                    "Translation" | "Rotation" | "Scale" | "Scale3D"
                ) {
                    *comp_transform_props
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
            let arch = if export.archetype != 0 {
                archetype::resolve_actor_archetype(
                    &pkg,
                    export.archetype,
                    &index,
                    &mut cache,
                    &mut open,
                )
            } else {
                archetype::ActorArchetypeProps::default()
            };
            let kind = if is_stub { "stub" } else { "direct" };
            if !arch.collides(&props) {
                let path = import_chain(&pkg, export.archetype)
                    .map(|c| c.join("."))
                    .unwrap_or_else(|| "<instance>".to_string());
                // Name the mesh it *would* have emitted — for the
                // direct half this is the only way to see what the
                // pre-existing over-emission consists of.
                let mesh = if is_stub {
                    archetype::resolve_via_archetype(
                        &pkg, component, &index, &mut cache, &mut open,
                    )
                } else {
                    mesh_ref::resolve_mesh_ref_from_component(&pkg, comp_ref)
                }
                .map(|(p, o)| format!("{p}:{o}"))
                .unwrap_or_else(|r| format!("<{r:?}>"));
                *collision_disabled
                    .entry(format!("{kind}\t{mesh}\t{path}"))
                    .or_default() += 1;
                continue;
            }
            if arch.rotation.is_some() && !props.iter().any(|p| p.name == "Rotation") {
                *actor_inherited_transform
                    .entry(format!("{kind}:Rotation {:?}", arch.rotation.unwrap()))
                    .or_default() += 1;
            }
            if arch.draw_scale_3d.is_some() && !props.iter().any(|p| p.name == "DrawScale3D") {
                *actor_inherited_transform
                    .entry(format!("{kind}:DrawScale3D {:?}", arch.draw_scale_3d.unwrap()))
                    .or_default() += 1;
            }
            if arch.draw_scale.is_some() && !props.iter().any(|p| p.name == "DrawScale") {
                *actor_inherited_transform
                    .entry(format!("{kind}:DrawScale {:?}", arch.draw_scale.unwrap()))
                    .or_default() += 1;
            }
            if !props.iter().any(|p| p.name == "Location") {
                *actor_inherited_transform
                    .entry(format!("{kind}:NO Location on the instance"))
                    .or_default() += 1;
            }

            if !is_stub {
                continue;
            }

            let arch_path = import_chain(&pkg, component.archetype)
                .map(|c| c.join("."))
                .unwrap_or_else(|| format!("<local:{}>", component.archetype));

            let resolved =
                archetype::resolve_via_archetype(&pkg, component, &index, &mut cache, &mut open);
            let key = match resolved {
                Ok(k) => k,
                Err(reason) => {
                    *failures.entry(format!("{reason:?} {arch_path}")).or_default() += 1;
                    continue;
                }
            };

            let mesh = mesh_cache
                .entry(key.clone())
                .or_insert_with(|| load_mesh(&index, &key));
            let Some(mesh) = mesh.as_ref() else {
                *failures
                    .entry(format!("MeshLoadFailed {}.{}", key.0, key.1))
                    .or_default() += 1;
                continue;
            };
            let tris = mesh.collision_triangles();
            if tris.is_empty() {
                *failures
                    .entry(format!("MeshNoCollision {}.{}", key.0, key.1))
                    .or_default() += 1;
                continue;
            }

            let xf: ActorTransform = arch.merge_transform(&props);
            let bw = ue3_to_bw(xf.location);
            let mesh_name = format!("{}:{}", key.0, key.1);

            let (footprint, walkable) = areas(&tris, &xf);
            let entry = stats.entry(mesh_name.clone()).or_insert_with(|| MeshStats {
                sample_bw: bw,
                y_min: f32::MAX,
                y_max: f32::MIN,
                ..Default::default()
            });
            entry.instances += 1;
            entry.chunks.insert(chunk_name.clone());
            entry.tris_per_instance = tris.len();
            entry.footprint_m2 += footprint;
            entry.walkable_m2 += walkable;
            entry.via_paths.insert(arch_path.clone());
            entry.y_min = entry.y_min.min(bw[1]);
            entry.y_max = entry.y_max.max(bw[1]);
            *per_chunk.entry(chunk_name.clone()).or_default() += 1;

            placements.push(Placement {
                chunk: chunk_name.clone(),
                actor: export.object_name.clone(),
                mesh: mesh_name,
                arch_path,
                bw,
            });
        }
    }

    let (hits, misses) = cache.stats();
    println!("== summary ==");
    println!("map\t{map}");
    println!("chunks\t{}", chunks.len());
    println!("direct_components\t{direct_total}");
    println!("archetype_stub_components\t{stub_total}");
    println!("resolved\t{}", placements.len());
    println!("distinct_archetype_paths\t{}", cache.len());
    println!("cache_hits\t{hits}\tcache_misses\t{misses}");
    println!("distinct_meshes\t{}", stats.len());
    let total_tris: u64 = stats
        .values()
        .map(|s| s.instances * s.tris_per_instance as u64)
        .sum();
    println!("triangles_added\t{total_tris}");

    println!("\n== actors suppressed by bCollideActors = false ==");
    if collision_disabled.is_empty() {
        println!("(none)");
    }
    let mut cd: Vec<(&String, &u64)> = collision_disabled.iter().collect();
    cd.sort_by_key(|(k, n)| (std::cmp::Reverse(**n), (*k).clone()));
    let cd_total: u64 = collision_disabled.values().sum();
    for (k, n) in cd {
        println!("{n}\t{k}");
    }
    println!("TOTAL\t{cd_total}");

    println!("\n== resolution failures ==");
    if failures.is_empty() {
        println!("(none)");
    }
    for (k, v) in &failures {
        println!("{v}\t{k}");
    }

    println!("\n== component-local transform properties ==");
    if comp_transform_props.is_empty() {
        println!("(none — actor transform is the whole story)");
    }
    for (k, v) in &comp_transform_props {
        println!("{k}\t{v}");
    }

    println!("\n== actor transform properties inherited from the archetype ==");
    if actor_inherited_transform.is_empty() {
        println!("(none — every stub actor carries its own placement)");
    }
    for (k, v) in &actor_inherited_transform {
        println!("{k}\t{v}");
    }

    println!("\n== per-mesh ==");
    println!(
        "instances\ttris_each\ttris_total\tfootprint_m2\twalkable_m2\twalkable_pct\t\
         bw_y_min\tbw_y_max\tchunks\tmesh"
    );
    let mut ranked: Vec<(&String, &MeshStats)> = stats.iter().collect();
    ranked.sort_by_key(|(name, s)| (std::cmp::Reverse(s.instances), (*name).clone()));
    for (name, s) in &ranked {
        let pct = if s.footprint_m2 > 0.0 {
            100.0 * s.walkable_m2 / s.footprint_m2
        } else {
            0.0
        };
        println!(
            "{}\t{}\t{}\t{:.1}\t{:.1}\t{:.1}\t{:.2}\t{:.2}\t{}\t{name}",
            s.instances,
            s.tris_per_instance,
            s.instances * s.tris_per_instance as u64,
            s.footprint_m2,
            s.walkable_m2,
            pct,
            s.y_min,
            s.y_max,
            s.chunks.len()
        );
    }

    println!("\n== per-chunk ==");
    let mut chunk_rows: Vec<(&String, &u64)> = per_chunk.iter().collect();
    chunk_rows.sort_by_key(|(name, n)| (std::cmp::Reverse(**n), (*name).clone()));
    for (chunk, n) in chunk_rows {
        println!("{n}\t{chunk}");
    }

    if let Some(path) = meshes_out {
        let mut w = std::fs::File::create(&path).expect("create meshes TSV");
        writeln!(
            w,
            "mesh\tinstances\ttris_each\ttris_total\tfootprint_m2\twalkable_m2\tbw_y_min\tbw_y_max\tsample_bw_x\tsample_bw_y\tsample_bw_z\tarchetype_paths"
        )
        .unwrap();
        for (name, s) in &ranked {
            writeln!(
                w,
                "{name}\t{}\t{}\t{}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{}",
                s.instances,
                s.tris_per_instance,
                s.instances * s.tris_per_instance as u64,
                s.footprint_m2,
                s.walkable_m2,
                s.y_min,
                s.y_max,
                s.sample_bw[0],
                s.sample_bw[1],
                s.sample_bw[2],
                s.via_paths.iter().cloned().collect::<Vec<_>>().join(";")
            )
            .unwrap();
        }
        eprintln!("wrote {}", path.display());
    }

    if let Some(path) = positions_out {
        let mut w = std::fs::File::create(&path).expect("create positions TSV");
        writeln!(w, "chunk\tactor\tmesh\tbw_x\tbw_y\tbw_z\tarchetype_path").unwrap();
        for p in &placements {
            writeln!(
                w,
                "{}\t{}\t{}\t{:.3}\t{:.3}\t{:.3}\t{}",
                p.chunk, p.actor, p.mesh, p.bw[0], p.bw[1], p.bw[2], p.arch_path
            )
            .unwrap();
        }
        eprintln!("wrote {}", path.display());
    }
}

/// Footprint and walkable-facing area of one instance, in BigWorld m².
///
/// Footprint is the XZ-projected triangle area — the shadow the mesh
/// casts on the ground plane, which is what "how much of the map does
/// this thing cover" means for a navmesh. Walkable is the *true* area
/// of the triangles Recast would accept as floor, which is the quantity
/// that decides whether the missing geometry is a surface you can stand
/// on.
fn areas(local_tris: &[[[f32; 3]; 3]], xf: &ActorTransform) -> (f64, f64) {
    let mut footprint = 0.0f64;
    let mut walkable = 0.0f64;
    for t in local_tris {
        let bw = [
            ue3_to_bw(xf.apply(t[0])),
            ue3_to_bw(xf.apply(t[1])),
            ue3_to_bw(xf.apply(t[2])),
        ];
        footprint += xz_area(&bw) as f64;
        if recast_up(&bw) >= MIN_UP {
            walkable += area3(&bw) as f64;
        }
    }
    (footprint, walkable)
}

fn xz_area(t: &[[f32; 3]; 3]) -> f32 {
    let (ax, az) = (t[1][0] - t[0][0], t[1][2] - t[0][2]);
    let (bx, bz) = (t[2][0] - t[0][0], t[2][2] - t[0][2]);
    0.5 * (ax * bz - az * bx).abs()
}

fn area3(t: &[[f32; 3]; 3]) -> f32 {
    let e1 = [t[1][0] - t[0][0], t[1][1] - t[0][1], t[1][2] - t[0][2]];
    let e2 = [t[2][0] - t[0][0], t[2][1] - t[0][1], t[2][2] - t[0][2]];
    let n = [
        e1[1] * e2[2] - e1[2] * e2[1],
        e1[2] * e2[0] - e1[0] * e2[2],
        e1[0] * e2[1] - e1[1] * e2[0],
    ];
    0.5 * (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt()
}

fn load_mesh(index: &PackageIndex, key: &(String, String)) -> Option<StaticMesh> {
    let loc = index.find(&key.0, &key.1)?;
    let pkg = Package::open(&loc.file_path).ok()?;
    let export = pkg.exports.get(loc.export_index)?;
    let data = pkg.read_export_data(export).ok()?;
    deserialize_static_mesh(&data, &pkg.names).ok()
}
