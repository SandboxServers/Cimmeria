//! Extraction-coverage accounting — "how much of this map did we
//! actually recover, and what did we leave on the floor?"
//!
//! Three walkers feed the soup: `StaticMeshActor` (Phase 1.2,
//! including prefab archetypes), `Terrain` (1.3) and BSP `Model`
//! (1.4). Classes outside that set — `Polys`, `ModelComponent`,
//! `InterpActor`, `KActor`, `FracturedStaticMeshActor`,
//! `StaticMeshCollectionActor` — are still silently ignored. Before
//! deciding whether a given map's navmesh is worth building, you need
//! the numbers: how many actors resolved, why the rest didn't, how the
//! triangles split by source, and how much collision-bearing geometry
//! sits in classes nothing reads. [`decode_status`] is what keeps that
//! last question honest as phases land.
//!
//! This module is the bookkeeping half of that answer. It is pure
//! accounting — no UE3 parsing beyond a class-name census — so the
//! arithmetic can be unit tested without the cooked asset bundle.
//!
//! # Balance invariant
//!
//! For every chunk:
//!
//! ```text
//! actors_total == actors_resolved + skips.total()
//! ```
//!
//! [`ChunkCoverage::is_balanced`] checks it. A violation means the walker
//! dropped an actor on a path that forgot to tally a [`SkipReason`], which
//! would silently understate the gap — exactly the failure this module
//! exists to prevent.

use std::collections::BTreeMap;
use std::io::{BufWriter, Write};
use std::path::Path;

use cimmeria_upk::Package;

/// Why a `StaticMeshActor` produced no triangles.
///
/// Ordered from "earliest in the resolution chain" to "latest" so a TSV
/// reader can see how far each actor got before falling out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SkipReason {
    /// The actor's tagged-property block carries no `StaticMeshComponent`
    /// object reference at all.
    NoComponentRef,
    /// The `StaticMeshComponent` reference points outside the export
    /// table, at an import, or at a zero-length export.
    ComponentUnreadable,
    /// The component parsed but has **no `StaticMesh` property**, and no
    /// [`cimmeria_upk_objects::PackageIndex`] was available to follow
    /// its archetype. This is the archetype-stub shape described in the
    /// `staticmesh` module doc: a cooked component holding only
    /// per-instance overrides, with the real mesh reference living in
    /// the prefab archetype's component in another package.
    ///
    /// With an index supplied this reason no longer fires — the stub is
    /// resolved by `staticmesh::archetype`, or falls into one of the
    /// five `Archetype*` reasons below.
    ArchetypeStubComponent,
    /// The stub's `Archetype` is 0, points at a local export we could
    /// not read, or is an import whose outer chain never reaches a root
    /// package — so there is no `(package, path)` to look up.
    ArchetypeUnrooted,
    /// The archetype's owning package is not in the supplied
    /// [`cimmeria_upk_objects::PackageIndex`], or failed to open.
    ArchetypePackageNotFound,
    /// The archetype's package opened but holds no export at the
    /// template's dotted outer path.
    ArchetypeExportNotFound,
    /// The archetype chain revisited a path, or exceeded
    /// [`crate::staticmesh::archetype::MAX_ARCHETYPE_DEPTH`] hops.
    ArchetypeChainLoop,
    /// The chain terminated at a template that has neither a
    /// `StaticMesh` property nor a further archetype to climb to.
    ArchetypeNoMesh,
    /// The actor's **own** `Archetype` (the chain that supplies
    /// `bCollideActors`, distinct from the component chain above) is
    /// non-zero but could not be followed.
    ///
    /// The actor is skipped rather than emitted, because the
    /// alternative is assuming UE3's `bCollideActors = true` default
    /// for a template that may well have said `false` — and the
    /// component chain can resolve a mesh perfectly well on its own, so
    /// the extractor would happily emit geometry nothing collides with.
    /// On Castle that shape is 26 prefabs' worth of weather cards
    /// sitting in doorways; emitting them split the exterior navmesh.
    ActorArchetypeUnreadable,
    /// `CollideActors` is explicitly `false` on the component or,
    /// through UE3 property inheritance, on its archetype. The mesh is
    /// rendered but nothing collides with it, so rasterising it would
    /// put a wall or a floor in the navmesh that the player walks
    /// straight through.
    CollisionDisabled,
    /// The component has a `StaticMesh` property but it is a `None`-ref
    /// (object index 0).
    NullMeshRef,
    /// The mesh reference is an import whose outer chain never terminates
    /// in a root package, or a package-local export we can't key on.
    UnresolvableMeshRef,
    /// The `(package, object)` key is absent from the supplied
    /// [`cimmeria_upk_objects::PackageIndex`].
    MeshNotInIndex,
    /// The mesh was found in the index but the `StaticMesh` decoder
    /// errored on its bytes.
    MeshDecodeFailed,
    /// The mesh decoded cleanly but `collision_triangles()` came back
    /// empty — no kDOP tree and no LOD0 index buffer.
    MeshNoCollision,
    /// No `PackageIndex` was supplied (degraded mode). Every actor lands
    /// here; the walk still reports `actors_total`.
    NoPackageIndex,
}

impl SkipReason {
    /// Every variant, in declaration order. Used for deterministic TSV
    /// column ordering and for the `merge` / `total` loops.
    pub const ALL: [SkipReason; 16] = [
        SkipReason::NoComponentRef,
        SkipReason::ComponentUnreadable,
        SkipReason::ArchetypeStubComponent,
        SkipReason::ArchetypeUnrooted,
        SkipReason::ArchetypePackageNotFound,
        SkipReason::ArchetypeExportNotFound,
        SkipReason::ArchetypeChainLoop,
        SkipReason::ArchetypeNoMesh,
        SkipReason::ActorArchetypeUnreadable,
        SkipReason::CollisionDisabled,
        SkipReason::NullMeshRef,
        SkipReason::UnresolvableMeshRef,
        SkipReason::MeshNotInIndex,
        SkipReason::MeshDecodeFailed,
        SkipReason::MeshNoCollision,
        SkipReason::NoPackageIndex,
    ];

    /// Stable snake_case identifier — used verbatim as a TSV column head.
    pub fn column(self) -> &'static str {
        match self {
            SkipReason::NoComponentRef => "skip_no_component_ref",
            SkipReason::ComponentUnreadable => "skip_component_unreadable",
            SkipReason::ArchetypeStubComponent => "skip_archetype_stub_component",
            SkipReason::ArchetypeUnrooted => "skip_archetype_unrooted",
            SkipReason::ArchetypePackageNotFound => "skip_archetype_package_not_found",
            SkipReason::ArchetypeExportNotFound => "skip_archetype_export_not_found",
            SkipReason::ArchetypeChainLoop => "skip_archetype_chain_loop",
            SkipReason::ArchetypeNoMesh => "skip_archetype_no_mesh",
            SkipReason::ActorArchetypeUnreadable => "skip_actor_archetype_unreadable",
            SkipReason::CollisionDisabled => "skip_collision_disabled",
            SkipReason::NullMeshRef => "skip_null_mesh_ref",
            SkipReason::UnresolvableMeshRef => "skip_unresolvable_mesh_ref",
            SkipReason::MeshNotInIndex => "skip_mesh_not_in_index",
            SkipReason::MeshDecodeFailed => "skip_mesh_decode_failed",
            SkipReason::MeshNoCollision => "skip_mesh_no_collision",
            SkipReason::NoPackageIndex => "skip_no_package_index",
        }
    }

    fn slot(self) -> usize {
        match self {
            SkipReason::NoComponentRef => 0,
            SkipReason::ComponentUnreadable => 1,
            SkipReason::ArchetypeStubComponent => 2,
            SkipReason::ArchetypeUnrooted => 3,
            SkipReason::ArchetypePackageNotFound => 4,
            SkipReason::ArchetypeExportNotFound => 5,
            SkipReason::ArchetypeChainLoop => 6,
            SkipReason::ArchetypeNoMesh => 7,
            SkipReason::ActorArchetypeUnreadable => 8,
            SkipReason::CollisionDisabled => 9,
            SkipReason::NullMeshRef => 10,
            SkipReason::UnresolvableMeshRef => 11,
            SkipReason::MeshNotInIndex => 12,
            SkipReason::MeshDecodeFailed => 13,
            SkipReason::MeshNoCollision => 14,
            SkipReason::NoPackageIndex => 15,
        }
    }
}

/// Fixed-slot counter over [`SkipReason`].
///
/// A plain array rather than a `HashMap` so iteration order is the
/// declaration order of [`SkipReason::ALL`] and a row always has the same
/// columns whether or not a reason fired.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SkipTally {
    counts: [u64; SkipReason::ALL.len()],
}

impl SkipTally {
    /// Record one skipped actor.
    pub fn add(&mut self, reason: SkipReason) {
        self.add_n(reason, 1);
    }

    /// Record `n` skipped actors at once — the mesh-load stage fails a
    /// whole instance group in one go.
    pub fn add_n(&mut self, reason: SkipReason, n: u64) {
        self.counts[reason.slot()] += n;
    }

    /// Count for one reason.
    pub fn get(&self, reason: SkipReason) -> u64 {
        self.counts[reason.slot()]
    }

    /// Sum across all reasons.
    pub fn total(&self) -> u64 {
        self.counts.iter().sum()
    }

    /// Accumulate another tally into this one.
    pub fn merge(&mut self, other: &SkipTally) {
        for (dst, src) in self.counts.iter_mut().zip(other.counts.iter()) {
            *dst += *src;
        }
    }
}

/// How much of a given export class the extractor actually turns into
/// triangles.
///
/// The census used to answer this with one boolean — `class ==
/// "StaticMeshActor"` — which was true when the extractor only shipped
/// Phase 1.2. It has since grown Terrain (1.3), BSP (1.4) and prefab
/// archetypes, so a boolean now reports `Terrain` as an undecoded
/// collision risk while the same run emits 2.8 M terrain triangles.
/// Three states keep "we read it" apart from "we read it through
/// something else" apart from "this is still a hole".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeStatus {
    /// A walker enumerates exports of this class directly.
    Decoded,
    /// Its geometry reaches the soup, but through another export — the
    /// walker never looks this class up by name.
    ViaOwner,
    /// Nothing reads it.
    NotDecoded,
}

impl DecodeStatus {
    fn label(self) -> &'static str {
        match self {
            DecodeStatus::Decoded => "yes",
            DecodeStatus::ViaOwner => "via-owner",
            DecodeStatus::NotDecoded => "no",
        }
    }
}

/// What the extractor does with each export class it has an opinion
/// about. Anything absent is [`DecodeStatus::NotDecoded`].
///
/// - `StaticMeshActor` — Phase 1.2, including prefab archetypes.
/// - `StaticMeshComponent` — read through its owning actor.
/// - `Terrain` — Phase 1.3. `TerrainComponent` holds the per-patch
///   heights but is reached from the `Terrain` actor, never enumerated.
/// - `Model` — Phase 1.4. `Brush` and `BlockingVolume` are read as
///   *owners*: `bsp::classify_owner` keys the world-vs-actor-space
///   decision on the class of whatever owns the `Model`.
/// - `PrefabInstance` — the container is still ignored, but its actors
///   are separately exported as `StaticMeshActor` and now resolve
///   through `staticmesh::archetype`, so it is no longer a gap.
const DECODE_STATUS: &[(&str, DecodeStatus)] = &[
    ("StaticMeshActor", DecodeStatus::Decoded),
    ("StaticMeshComponent", DecodeStatus::ViaOwner),
    ("Terrain", DecodeStatus::Decoded),
    ("TerrainComponent", DecodeStatus::ViaOwner),
    ("Model", DecodeStatus::Decoded),
    ("Brush", DecodeStatus::ViaOwner),
    ("BlockingVolume", DecodeStatus::ViaOwner),
    ("PrefabInstance", DecodeStatus::ViaOwner),
];

/// What the extractor does with `class`.
pub fn decode_status(class: &str) -> DecodeStatus {
    DECODE_STATUS
        .iter()
        .find(|(c, _)| *c == class)
        .map(|(_, s)| *s)
        .unwrap_or(DecodeStatus::NotDecoded)
}

/// Export classes that plausibly carry collision geometry a navmesh
/// would need, decoded or not.
///
/// These get their own named TSV columns; every other class still shows
/// up in the full class census written by
/// [`MapCoverage::write_class_census_tsv`]. The `collision_risk` flag
/// there is this list **minus** whatever [`decode_status`] says we
/// already read, so the column shrinks as phases land rather than
/// staying frozen at the Phase 1.2 answer.
///
/// - `Terrain` / `TerrainComponent` — heightfield ground.
/// - `Brush` / `Model` / `Polys` — BSP. `Brush` is the actor, `Model`
///   the geometry, `Polys` the face soup (unread: BSP collision comes
///   from `UModel`'s own node tree).
/// - `BrushComponent` / `ModelComponent` — the collision and rendering
///   halves of BSP. `ModelComponent` is the discriminator worth
///   watching: every chunk carries a `Model`/`Polys` pair (often the
///   empty default builder model), but a chunk with `ModelComponent`
///   exports has BSP surfaces that were actually *built*.
/// - `BlockingVolume` — invisible collision-only brush; pure navmesh
///   input with no render mesh.
/// - `InterpActor` / `KActor` / `FracturedStaticMeshActor` — movers and
///   physics props that DO own a `StaticMeshComponent` but are not class
///   `StaticMeshActor`, so the walker's class filter drops them. Still
///   genuine gaps.
/// - `StaticMeshCollectionActor` — UE3's cooked batching actor; holds
///   an array of components rather than one. Still a genuine gap.
/// - `PrefabInstance` — the prefab container; its actors resolve
///   through the archetype chain, so it is no longer a risk.
pub const COLLISION_BEARING_CLASSES: &[&str] = &[
    "Terrain",
    "TerrainComponent",
    "Brush",
    "BrushComponent",
    "Model",
    "ModelComponent",
    "Polys",
    "BlockingVolume",
    "InterpActor",
    "KActor",
    "FracturedStaticMeshActor",
    "StaticMeshCollectionActor",
    "PrefabInstance",
];

/// Name of the per-chunk TSV's last column, whose value is `chunk` on
/// every per-chunk row and `total` on the single summary row.
pub const ROW_KIND_COLUMN: &str = "row_kind";

/// Which kind of row a per-chunk TSV line is.
///
/// The summary row shares the schema of the data rows, so nothing in
/// the bytes distinguishes them except this column and the literal
/// `TOTAL` in the `chunk` cell. A reader that misses both double-counts
/// the whole file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind {
    Chunk,
    Total,
}

impl RowKind {
    pub fn label(self) -> &'static str {
        match self {
            RowKind::Chunk => "chunk",
            RowKind::Total => "total",
        }
    }
}

/// Count exports by class name for one package.
pub fn census_export_classes(pkg: &Package) -> BTreeMap<String, u64> {
    let mut census: BTreeMap<String, u64> = BTreeMap::new();
    for export in &pkg.exports {
        *census
            .entry(pkg.export_class_name(export).to_string())
            .or_insert(0) += 1;
    }
    census
}

/// One row of the coverage report — the extraction outcome for a single
/// `.umap` chunk.
#[derive(Debug, Clone, Default)]
pub struct ChunkCoverage {
    /// Chunk filename stem, or `TOTAL` for the summary row.
    pub chunk: String,
    /// Packed chunk id from the filename (0 on the `TOTAL` row).
    pub chunk_id: u32,
    pub position_x: i16,
    pub position_z: i16,
    /// Every export in the package, all classes.
    pub exports_total: u64,
    /// Exports whose class is exactly `StaticMeshActor`.
    pub actors_total: u64,
    /// Actors that resolved to a mesh AND contributed triangles.
    pub actors_resolved: u64,
    pub skips: SkipTally,
    /// Every triangle that reached this chunk's OBJ, all sources.
    ///
    /// Invariant, asserted by `extract_map_castle_cellblock.rs`:
    /// `triangles_emitted == staticmesh_triangles + terrain_triangles +
    /// bsp_triangles`, and it equals the `f` line count of the OBJ.
    pub triangles_emitted: u64,
    /// Phase 1.2 — triangles from `StaticMeshActor` kDOP collision.
    pub staticmesh_triangles: u64,
    /// Phase 1.3 — triangles from `Terrain` heightfield patches.
    pub terrain_triangles: u64,
    /// Terrain quads skipped because `TID_Visibility_Off` marked them a
    /// hole. Not an error: holes are where the interior floors show
    /// through.
    pub terrain_quads_holed: u64,
    /// `Terrain` exports that failed to decode. Non-zero means this
    /// chunk is missing ground.
    pub terrain_parse_failures: u64,
    /// Phase 1.4 — triangles from BSP `Model` geometry, after both the
    /// `PolyFlags` filter and the hull-cap filter.
    pub bsp_triangles: u64,
    /// BSP triangles dropped as outer-hull skin
    /// (`bsp::hull_cap`). Excluded from `bsp_triangles`, so it does
    /// **not** enter the sum invariant.
    pub bsp_hull_cap_triangles: u64,
    /// `Model` exports that failed to decode. Non-zero is a decoder bug.
    pub bsp_models_failed: u64,
    /// Size of the per-chunk OBJ on disk, 0 if none was written.
    pub obj_bytes: u64,
    /// Actors whose export-table `Archetype` field is non-zero — i.e.
    /// instantiated from a prefab template rather than authored inline.
    pub archetype_actors: u64,
    /// ...of which resolved to collision triangles anyway.
    pub archetype_actors_resolved: u64,
    /// Actors whose `Outer` chain passes through a `PrefabInstance`
    /// export in this same package.
    pub prefab_outer_actors: u64,
    /// Actors whose mesh reference was recovered by walking the prefab
    /// archetype chain rather than reading the instance's own
    /// `StaticMesh` property. A subset of `actors_resolved`.
    pub actors_resolved_via_archetype: u64,
    /// Triangles those actors contributed. A subset of
    /// `staticmesh_triangles`, so it does **not** enter the source sum.
    pub triangles_via_archetype: u64,
    /// Prefab packages opened while walking this chunk's archetypes.
    pub prefab_packages_opened: u64,
    /// Full per-class export census for this chunk.
    pub class_census: BTreeMap<String, u64>,
}

impl ChunkCoverage {
    /// `actors_total == actors_resolved + skips.total()`.
    ///
    /// See the module doc — a false here means an actor fell out of the
    /// walker on an untallied path.
    pub fn is_balanced(&self) -> bool {
        self.actors_total == self.actors_resolved + self.skips.total()
    }

    /// Count for one undecoded class, 0 if absent.
    pub fn class_count(&self, class: &str) -> u64 {
        self.class_census.get(class).copied().unwrap_or(0)
    }

    /// `triangles_emitted == staticmesh + terrain + bsp`.
    ///
    /// The OBJ carries exactly one soup per chunk, so any drift here
    /// means a geometry source pushed into the soup without tallying —
    /// and every "source X contributes N%" number would be wrong.
    pub fn sources_balance(&self) -> bool {
        self.triangles_emitted
            == self.staticmesh_triangles + self.terrain_triangles + self.bsp_triangles
    }

    fn merge(&mut self, other: &ChunkCoverage) {
        self.exports_total += other.exports_total;
        self.actors_total += other.actors_total;
        self.actors_resolved += other.actors_resolved;
        self.skips.merge(&other.skips);
        self.triangles_emitted += other.triangles_emitted;
        self.staticmesh_triangles += other.staticmesh_triangles;
        self.terrain_triangles += other.terrain_triangles;
        self.terrain_quads_holed += other.terrain_quads_holed;
        self.terrain_parse_failures += other.terrain_parse_failures;
        self.bsp_triangles += other.bsp_triangles;
        self.bsp_hull_cap_triangles += other.bsp_hull_cap_triangles;
        self.bsp_models_failed += other.bsp_models_failed;
        self.obj_bytes += other.obj_bytes;
        self.archetype_actors += other.archetype_actors;
        self.archetype_actors_resolved += other.archetype_actors_resolved;
        self.prefab_outer_actors += other.prefab_outer_actors;
        self.actors_resolved_via_archetype += other.actors_resolved_via_archetype;
        self.triangles_via_archetype += other.triangles_via_archetype;
        self.prefab_packages_opened += other.prefab_packages_opened;
        for (class, n) in &other.class_census {
            *self.class_census.entry(class.clone()).or_insert(0) += n;
        }
    }
}

/// Whole-map coverage: one [`ChunkCoverage`] per chunk plus run metadata.
#[derive(Debug, Clone, Default)]
pub struct MapCoverage {
    pub map_name: String,
    pub chunks: Vec<ChunkCoverage>,
    /// Chunks that were enumerated but filtered out by `--chunk-filter`.
    pub chunks_filtered_out: usize,
    /// Wall-clock seconds for the extraction pass (excludes index build).
    pub elapsed_secs: f64,
    /// Size of the combined map-level OBJ, 0 if none was written.
    pub combined_obj_bytes: u64,
}

impl MapCoverage {
    /// Summary row across every chunk. `chunk` is the literal `TOTAL`.
    pub fn totals(&self) -> ChunkCoverage {
        let mut total = ChunkCoverage {
            chunk: "TOTAL".to_string(),
            ..Default::default()
        };
        for chunk in &self.chunks {
            total.merge(chunk);
        }
        total
    }

    /// Chunks that produced at least one triangle.
    pub fn chunks_with_geometry(&self) -> usize {
        self.chunks
            .iter()
            .filter(|c| c.triangles_emitted > 0)
            .count()
    }

    /// Chunk rows sorted by triangle count, descending.
    pub fn ranked_by_triangles(&self) -> Vec<&ChunkCoverage> {
        let mut ranked: Vec<&ChunkCoverage> = self.chunks.iter().collect();
        ranked.sort_by(|a, b| {
            b.triangles_emitted
                .cmp(&a.triangles_emitted)
                .then_with(|| a.chunk.cmp(&b.chunk))
        });
        ranked
    }

    /// Every chunk row that fails the balance invariant.
    pub fn unbalanced_chunks(&self) -> Vec<&ChunkCoverage> {
        self.chunks.iter().filter(|c| !c.is_balanced()).collect()
    }

    /// Write the per-chunk TSV: one header row, one row per chunk, then
    /// a `TOTAL` row.
    pub fn write_tsv(&self, path: &Path) -> crate::Result<()> {
        let file = std::fs::File::create(path)?;
        let mut w = BufWriter::new(file);
        self.write_tsv_into(&mut w)?;
        w.flush()?;
        Ok(())
    }

    /// TSV emitter over any `Write`, so tests can render into a `Vec<u8>`.
    ///
    /// The last column is [`ROW_KIND_COLUMN`], `chunk` on every per-chunk
    /// row and `total` on the single summary row. It exists because the
    /// summary row is *also* a data row: a reader that sums the file
    /// naively gets exactly twice the truth, which is plausible enough
    /// to go unnoticed (it happened once — 12,860 `static_mesh_actors`
    /// reported against 6,430 exports). Filter on `row_kind` rather
    /// than string-matching the `chunk` cell.
    pub fn write_tsv_into<W: Write>(&self, w: &mut W) -> crate::Result<()> {
        // Header.
        let mut header: Vec<String> = vec![
            "chunk".into(),
            "chunk_id".into(),
            "position_x".into(),
            "position_z".into(),
            "exports_total".into(),
            "static_mesh_actors".into(),
            "actors_resolved".into(),
        ];
        header.extend(SkipReason::ALL.iter().map(|r| r.column().to_string()));
        header.push("triangles".into());
        header.push("staticmesh_triangles".into());
        header.push("terrain_triangles".into());
        header.push("terrain_quads_holed".into());
        header.push("terrain_parse_failures".into());
        header.push("bsp_triangles".into());
        header.push("bsp_hull_cap_triangles".into());
        header.push("bsp_models_failed".into());
        header.push("obj_bytes".into());
        header.push("archetype_actors".into());
        header.push("archetype_actors_resolved".into());
        header.push("prefab_outer_actors".into());
        header.push("actors_resolved_via_archetype".into());
        header.push("triangles_via_archetype".into());
        header.push("prefab_packages_opened".into());
        header.extend(
            COLLISION_BEARING_CLASSES
                .iter()
                .map(|c| format!("class_{c}")),
        );
        header.push("balanced".into());
        header.push("sources_balanced".into());
        header.push(ROW_KIND_COLUMN.into());
        writeln!(w, "{}", header.join("\t"))?;

        for chunk in &self.chunks {
            write_chunk_row(w, chunk, RowKind::Chunk)?;
        }
        write_chunk_row(w, &self.totals(), RowKind::Total)?;
        Ok(())
    }

    /// Write the full class census: every export class seen anywhere in
    /// the map, its total count, how many chunks carry it, and whether
    /// the extractor decodes it today.
    pub fn write_class_census_tsv(&self, path: &Path) -> crate::Result<()> {
        let file = std::fs::File::create(path)?;
        let mut w = BufWriter::new(file);
        self.write_class_census_into(&mut w)?;
        w.flush()?;
        Ok(())
    }

    /// Class-census emitter over any `Write`.
    pub fn write_class_census_into<W: Write>(&self, w: &mut W) -> crate::Result<()> {
        let mut totals: BTreeMap<&str, (u64, usize)> = BTreeMap::new();
        for chunk in &self.chunks {
            for (class, n) in &chunk.class_census {
                let entry = totals.entry(class.as_str()).or_insert((0, 0));
                entry.0 += n;
                entry.1 += 1;
            }
        }

        let mut rows: Vec<(&str, u64, usize)> =
            totals.into_iter().map(|(c, (n, k))| (c, n, k)).collect();
        // Biggest first; ties broken by name so the file is reproducible.
        rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));

        writeln!(w, "class\texports\tchunks_present\tdecoded\tcollision_risk")?;
        for (class, exports, chunks_present) in rows {
            let status = decode_status(class);
            // A risk is a class that could carry collision AND that
            // nothing reads — not merely "is not StaticMeshActor".
            // `ViaOwner` counts as read: the geometry reaches the soup,
            // just through a different export.
            let risk =
                COLLISION_BEARING_CLASSES.contains(&class) && status == DecodeStatus::NotDecoded;
            writeln!(
                w,
                "{class}\t{exports}\t{chunks_present}\t{}\t{}",
                status.label(),
                if risk { "yes" } else { "no" },
            )?;
        }
        Ok(())
    }
}

fn write_chunk_row<W: Write>(w: &mut W, chunk: &ChunkCoverage, kind: RowKind) -> crate::Result<()> {
    let mut row: Vec<String> = vec![
        chunk.chunk.clone(),
        format!("{:08x}", chunk.chunk_id),
        chunk.position_x.to_string(),
        chunk.position_z.to_string(),
        chunk.exports_total.to_string(),
        chunk.actors_total.to_string(),
        chunk.actors_resolved.to_string(),
    ];
    row.extend(
        SkipReason::ALL
            .iter()
            .map(|r| chunk.skips.get(*r).to_string()),
    );
    row.push(chunk.triangles_emitted.to_string());
    row.push(chunk.staticmesh_triangles.to_string());
    row.push(chunk.terrain_triangles.to_string());
    row.push(chunk.terrain_quads_holed.to_string());
    row.push(chunk.terrain_parse_failures.to_string());
    row.push(chunk.bsp_triangles.to_string());
    row.push(chunk.bsp_hull_cap_triangles.to_string());
    row.push(chunk.bsp_models_failed.to_string());
    row.push(chunk.obj_bytes.to_string());
    row.push(chunk.archetype_actors.to_string());
    row.push(chunk.archetype_actors_resolved.to_string());
    row.push(chunk.prefab_outer_actors.to_string());
    row.push(chunk.actors_resolved_via_archetype.to_string());
    row.push(chunk.triangles_via_archetype.to_string());
    row.push(chunk.prefab_packages_opened.to_string());
    row.extend(
        COLLISION_BEARING_CLASSES
            .iter()
            .map(|c| chunk.class_count(c).to_string()),
    );
    row.push(if chunk.is_balanced() { "yes" } else { "NO" }.to_string());
    row.push(if chunk.sources_balance() { "yes" } else { "NO" }.to_string());
    row.push(kind.label().to_string());
    writeln!(w, "{}", row.join("\t"))?;
    Ok(())
}

#[cfg(test)]
mod tests;
