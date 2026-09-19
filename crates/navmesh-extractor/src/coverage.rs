//! Extraction-coverage accounting — "how much of this map did the
//! StaticMesh path actually recover, and what did it leave on the floor?"
//!
//! The extractor's Phase 1.2 walker only understands `StaticMeshActor`
//! exports. Everything else in a `.umap` — `Terrain`, BSP (`Model` /
//! `Polys` / `Brush`), `BlockingVolume`, `InterpActor`,
//! `StaticMeshCollectionActor` — is silently ignored. Before deciding
//! whether a StaticMesh-only navmesh is worth building for a given map,
//! you need the numbers: how many actors resolved, why the rest didn't,
//! and how much collision-bearing geometry sits in classes we don't
//! decode.
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
    /// The component parsed but has **no `StaticMesh` property**. This is
    /// the archetype-stub shape described in the `staticmesh` module doc:
    /// a ~76-byte cooked component holding only a `CullDistance`
    /// override, with the real mesh reference living in the prefab
    /// archetype's component in another package.
    ArchetypeStubComponent,
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
    pub const ALL: [SkipReason; 9] = [
        SkipReason::NoComponentRef,
        SkipReason::ComponentUnreadable,
        SkipReason::ArchetypeStubComponent,
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
            SkipReason::NullMeshRef => 3,
            SkipReason::UnresolvableMeshRef => 4,
            SkipReason::MeshNotInIndex => 5,
            SkipReason::MeshDecodeFailed => 6,
            SkipReason::MeshNoCollision => 7,
            SkipReason::NoPackageIndex => 8,
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

/// Export classes that are NOT decoded by the Phase 1.2 StaticMesh path
/// but plausibly carry collision geometry a navmesh would need.
///
/// These get their own named TSV columns; every other class still shows
/// up in the full class census written by
/// [`MapCoverage::write_class_census_tsv`].
///
/// - `Terrain` / `TerrainComponent` — Phase 1.3, heightfield ground.
///   `Terrain` is the actor; the components hold the per-patch data.
/// - `Brush` / `Model` / `Polys` — Phase 1.4 BSP. `Brush` is the actor,
///   `Model` the geometry, `Polys` the face soup.
/// - `BrushComponent` / `ModelComponent` — the collision and rendering
///   halves of BSP. `ModelComponent` is the discriminator worth
///   watching: every chunk carries a `Model`/`Polys` pair (often the
///   empty default builder model), but a chunk with `ModelComponent`
///   exports has BSP surfaces that were actually *built*.
/// - `BlockingVolume` — invisible collision-only brush; pure navmesh
///   input with no render mesh.
/// - `InterpActor` / `KActor` / `FracturedStaticMeshActor` — movers and
///   physics props that DO own a `StaticMeshComponent` but are not class
///   `StaticMeshActor`, so the walker's class filter drops them.
/// - `StaticMeshCollectionActor` — UE3's cooked batching actor; holds
///   an array of components rather than one.
/// - `PrefabInstance` — the prefab container; see the archetype columns.
pub const UNDECODED_COLLISION_CLASSES: &[&str] = &[
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
    pub triangles_emitted: u64,
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

    fn merge(&mut self, other: &ChunkCoverage) {
        self.exports_total += other.exports_total;
        self.actors_total += other.actors_total;
        self.actors_resolved += other.actors_resolved;
        self.skips.merge(&other.skips);
        self.triangles_emitted += other.triangles_emitted;
        self.obj_bytes += other.obj_bytes;
        self.archetype_actors += other.archetype_actors;
        self.archetype_actors_resolved += other.archetype_actors_resolved;
        self.prefab_outer_actors += other.prefab_outer_actors;
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
        header.push("obj_bytes".into());
        header.push("archetype_actors".into());
        header.push("archetype_actors_resolved".into());
        header.push("prefab_outer_actors".into());
        header.extend(
            UNDECODED_COLLISION_CLASSES
                .iter()
                .map(|c| format!("undecoded_{c}")),
        );
        header.push("balanced".into());
        writeln!(w, "{}", header.join("\t"))?;

        for chunk in &self.chunks {
            write_chunk_row(w, chunk)?;
        }
        write_chunk_row(w, &self.totals())?;
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
            let decoded = class == "StaticMeshActor";
            let risk = UNDECODED_COLLISION_CLASSES.contains(&class) && !decoded;
            writeln!(
                w,
                "{class}\t{exports}\t{chunks_present}\t{}\t{}",
                if decoded { "yes" } else { "no" },
                if risk { "yes" } else { "no" },
            )?;
        }
        Ok(())
    }
}

fn write_chunk_row<W: Write>(w: &mut W, chunk: &ChunkCoverage) -> crate::Result<()> {
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
    row.push(chunk.obj_bytes.to_string());
    row.push(chunk.archetype_actors.to_string());
    row.push(chunk.archetype_actors_resolved.to_string());
    row.push(chunk.prefab_outer_actors.to_string());
    row.extend(
        UNDECODED_COLLISION_CLASSES
            .iter()
            .map(|c| chunk.class_count(c).to_string()),
    );
    row.push(if chunk.is_balanced() { "yes" } else { "NO" }.to_string());
    writeln!(w, "{}", row.join("\t"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(name: &str, actors: u64, resolved: u64, tris: u64) -> ChunkCoverage {
        let mut skips = SkipTally::default();
        skips.add_n(SkipReason::ArchetypeStubComponent, actors - resolved);
        ChunkCoverage {
            chunk: name.to_string(),
            chunk_id: 0x0001_0002,
            position_x: 2,
            position_z: 1,
            exports_total: actors * 3,
            actors_total: actors,
            actors_resolved: resolved,
            skips,
            triangles_emitted: tris,
            obj_bytes: tris * 30,
            archetype_actors: actors - resolved,
            archetype_actors_resolved: 0,
            prefab_outer_actors: 0,
            class_census: BTreeMap::from([
                ("StaticMeshActor".to_string(), actors),
                ("Terrain".to_string(), 3),
            ]),
        }
    }

    #[test]
    fn skip_tally_add_and_total() {
        let mut t = SkipTally::default();
        t.add(SkipReason::NoComponentRef);
        t.add(SkipReason::NoComponentRef);
        t.add_n(SkipReason::MeshNotInIndex, 7);
        assert_eq!(t.get(SkipReason::NoComponentRef), 2);
        assert_eq!(t.get(SkipReason::MeshNotInIndex), 7);
        assert_eq!(t.get(SkipReason::MeshNoCollision), 0);
        assert_eq!(t.total(), 9);
    }

    #[test]
    fn skip_tally_merge_sums_every_slot() {
        let mut a = SkipTally::default();
        a.add_n(SkipReason::NullMeshRef, 3);
        a.add_n(SkipReason::MeshNoCollision, 1);
        let mut b = SkipTally::default();
        b.add_n(SkipReason::NullMeshRef, 4);
        b.add_n(SkipReason::NoComponentRef, 2);
        a.merge(&b);
        assert_eq!(a.get(SkipReason::NullMeshRef), 7);
        assert_eq!(a.get(SkipReason::MeshNoCollision), 1);
        assert_eq!(a.get(SkipReason::NoComponentRef), 2);
        assert_eq!(a.total(), 10);
    }

    /// Every variant must own a distinct slot — a duplicated `slot()`
    /// arm would silently merge two reasons into one counter and make
    /// the report lie about *why* actors were skipped.
    #[test]
    fn every_skip_reason_has_a_distinct_slot_and_column() {
        let mut slots: Vec<usize> = SkipReason::ALL.iter().map(|r| r.slot()).collect();
        slots.sort_unstable();
        slots.dedup();
        assert_eq!(slots.len(), SkipReason::ALL.len());

        let mut cols: Vec<&str> = SkipReason::ALL.iter().map(|r| r.column()).collect();
        cols.sort_unstable();
        cols.dedup();
        assert_eq!(cols.len(), SkipReason::ALL.len());
    }

    #[test]
    fn balance_invariant_detects_an_untallied_drop() {
        let mut c = chunk("a", 10, 4, 100);
        assert!(c.is_balanced(), "6 skipped + 4 resolved == 10");
        // Simulate the bug: an actor falls out without a tally.
        c.actors_total += 1;
        assert!(!c.is_balanced());
    }

    #[test]
    fn totals_row_sums_every_chunk() {
        let cov = MapCoverage {
            map_name: "Castle".into(),
            chunks: vec![chunk("a", 10, 4, 100), chunk("b", 20, 15, 900)],
            ..Default::default()
        };
        let total = cov.totals();
        assert_eq!(total.chunk, "TOTAL");
        assert_eq!(total.actors_total, 30);
        assert_eq!(total.actors_resolved, 19);
        assert_eq!(total.triangles_emitted, 1000);
        assert_eq!(total.skips.get(SkipReason::ArchetypeStubComponent), 11);
        assert_eq!(total.class_count("Terrain"), 6);
        assert_eq!(total.class_count("StaticMeshActor"), 30);
        assert!(total.is_balanced());
    }

    #[test]
    fn ranked_by_triangles_is_descending_and_tie_broken_by_name() {
        let cov = MapCoverage {
            chunks: vec![
                chunk("b", 1, 1, 50),
                chunk("a", 1, 1, 50),
                chunk("c", 1, 1, 900),
            ],
            ..Default::default()
        };
        let ranked: Vec<&str> = cov
            .ranked_by_triangles()
            .iter()
            .map(|c| c.chunk.as_str())
            .collect();
        assert_eq!(ranked, vec!["c", "a", "b"]);
    }

    #[test]
    fn unbalanced_chunks_are_surfaced() {
        let mut bad = chunk("bad", 10, 4, 100);
        bad.actors_resolved = 99;
        let cov = MapCoverage {
            chunks: vec![chunk("ok", 10, 4, 100), bad],
            ..Default::default()
        };
        let flagged: Vec<&str> = cov
            .unbalanced_chunks()
            .iter()
            .map(|c| c.chunk.as_str())
            .collect();
        assert_eq!(flagged, vec!["bad"]);
    }

    #[test]
    fn tsv_header_matches_row_width_and_ends_with_a_total_row() {
        let cov = MapCoverage {
            chunks: vec![chunk("a", 10, 4, 100), chunk("b", 20, 15, 900)],
            ..Default::default()
        };
        let mut buf = Vec::new();
        cov.write_tsv_into(&mut buf).unwrap();
        let text = String::from_utf8(buf).unwrap();
        let lines: Vec<&str> = text.lines().collect();

        // header + 2 chunks + TOTAL
        assert_eq!(lines.len(), 4);
        let width = lines[0].split('\t').count();
        for line in &lines {
            assert_eq!(
                line.split('\t').count(),
                width,
                "ragged TSV row: {line:?} (header width {width})"
            );
        }
        assert!(lines[0].starts_with("chunk\tchunk_id\t"));
        assert!(lines[0].contains("skip_archetype_stub_component"));
        assert!(lines[0].contains("undecoded_Terrain"));
        assert!(lines[3].starts_with("TOTAL\t"));
        // 19 resolved across the two chunks shows up on the TOTAL row.
        assert!(lines[3].contains("\t19\t"));
    }

    #[test]
    fn tsv_flags_an_unbalanced_row() {
        let mut bad = chunk("bad", 10, 4, 100);
        bad.actors_total = 11;
        let cov = MapCoverage {
            chunks: vec![bad],
            ..Default::default()
        };
        let mut buf = Vec::new();
        cov.write_tsv_into(&mut buf).unwrap();
        let text = String::from_utf8(buf).unwrap();
        assert!(
            text.lines().nth(1).unwrap().ends_with("\tNO"),
            "unbalanced chunk row must end in NO: {text}"
        );
    }

    #[test]
    fn class_census_sorts_by_count_and_marks_collision_risk() {
        let cov = MapCoverage {
            chunks: vec![chunk("a", 10, 4, 100), chunk("b", 20, 15, 900)],
            ..Default::default()
        };
        let mut buf = Vec::new();
        cov.write_class_census_into(&mut buf).unwrap();
        let text = String::from_utf8(buf).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(
            lines[0],
            "class\texports\tchunks_present\tdecoded\tcollision_risk"
        );
        // StaticMeshActor (30) outranks Terrain (6).
        assert_eq!(lines[1], "StaticMeshActor\t30\t2\tyes\tno");
        assert_eq!(lines[2], "Terrain\t6\t2\tno\tyes");
    }
}
