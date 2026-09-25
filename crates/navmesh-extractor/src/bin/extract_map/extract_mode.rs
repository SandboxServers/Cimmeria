//! `extract_map extract` — walk a cooked map's chunks, write OBJs, and
//! print/emit the coverage report.

use std::fmt::Write as _;
use std::path::Path;
use std::time::Instant;

use cimmeria_navmesh_extractor::coverage::{
    decode_status, DecodeStatus, MapCoverage, SkipReason, COLLISION_BEARING_CLASSES,
};
use cimmeria_navmesh_extractor::{extract_map_with_report, ExtractOptions};
use cimmeria_upk_objects::PackageIndex;

use crate::args::ExtractArgs;
use crate::pct;

pub(crate) fn run(args: ExtractArgs) -> Result<(), Box<dyn std::error::Error>> {
    let map_dir = args.map_dir();
    if !map_dir.is_dir() {
        return Err(format!("map directory not found: {}", map_dir.display()).into());
    }

    let index = load_or_build_index(&args.index, &args.cooked_root)?;

    let report = extract_map_with_report(
        &map_dir,
        &args.out,
        ExtractOptions {
            index: Some(&index),
            chunk_filter: args.chunk_filter.as_deref(),
            combined_obj: args.combined.as_deref(),
            ..Default::default()
        },
    )?;

    let report_path = args.report_path();
    let classes_path = args.classes_path();
    report.write_tsv(&report_path)?;
    report.write_class_census_tsv(&classes_path)?;

    print!("{}", summary(&report, &report_path, &classes_path));
    Ok(())
}

/// Load the cached index, or build it from `cooked_root` and save it.
///
/// Building walks ~5000 packages and takes about 45 seconds, so the
/// cache is the difference between a 3-second run and a 50-second one.
fn load_or_build_index(
    cache: &Path,
    cooked_root: &Path,
) -> Result<PackageIndex, Box<dyn std::error::Error>> {
    if cache.exists() {
        let t = Instant::now();
        let index = PackageIndex::load(cache)?;
        eprintln!(
            "loaded package index: {} exports / {} packages from {} in {:.1}s",
            index.export_count,
            index.package_count,
            cache.display(),
            t.elapsed().as_secs_f32()
        );
        return Ok(index);
    }
    eprintln!(
        "no index cache at {} — building from {} (about 45s)",
        cache.display(),
        cooked_root.display()
    );
    let t = Instant::now();
    let index = PackageIndex::build(cooked_root)?;
    if let Some(parent) = cache.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    index.save(cache)?;
    eprintln!(
        "built package index: {} exports / {} packages in {:.1}s -> {}",
        index.export_count,
        index.package_count,
        t.elapsed().as_secs_f32(),
        cache.display()
    );
    Ok(index)
}

/// Render the whole human-readable summary.
///
/// Returns a `String` rather than printing. The two warning branches
/// below — per-source tallies that do not sum, and chunks that fail the
/// balance invariant — cannot be produced by a valid extraction, so the
/// only way to cover them is to hand this function a report that has
/// them. `std::fmt::Write` into a `String` never fails, which is why
/// the `let _ =` here reads past nothing.
fn summary(report: &MapCoverage, report_path: &Path, classes_path: &Path) -> String {
    let mut o = String::new();
    let t = report.totals();
    let _ = writeln!(o, "== {} ==", report.map_name);
    let _ = writeln!(
        o,
        "chunks: {} processed ({} filtered out), {} produced geometry",
        report.chunks.len(),
        report.chunks_filtered_out,
        report.chunks_with_geometry()
    );
    let _ = writeln!(
        o,
        // "mesh actors" = staticmesh::MESH_ACTOR_CLASSES (StaticMeshActor,
        // InterpActor, KActor, FracturedStaticMeshActor as of NA36), not
        // just the literal StaticMeshActor class.
        "exports: {}   mesh actors: {}   resolved: {} ({:.1}%)",
        t.exports_total,
        t.actors_total,
        t.actors_resolved,
        pct(t.actors_resolved, t.actors_total)
    );
    let _ = writeln!(
        o,
        "triangles: {} total = {} StaticMesh + {} Terrain + {} BSP{}",
        t.triangles_emitted,
        t.staticmesh_triangles,
        t.terrain_triangles,
        t.bsp_triangles,
        if t.sources_balance() {
            String::new()
        } else {
            "   *** SOURCES DO NOT SUM ***".to_string()
        }
    );
    let _ = writeln!(
        o,
        "  terrain: {} quads holed, {} parse failures | BSP: {} hull-cap triangles dropped, \
         {} Model decode failures",
        t.terrain_quads_holed,
        t.terrain_parse_failures,
        t.bsp_hull_cap_triangles,
        t.bsp_models_failed
    );
    let _ = writeln!(
        o,
        "obj bytes: {} per-chunk + {} combined",
        t.obj_bytes, report.combined_obj_bytes
    );
    let _ = writeln!(o, "wall clock: {:.1}s", report.elapsed_secs);

    let _ = writeln!(o, "\nskips by reason:");
    for reason in SkipReason::ALL {
        let n = t.skips.get(reason);
        if n > 0 {
            let _ = writeln!(
                o,
                "  {:<34} {:>8}  ({:.1}% of actors)",
                reason.column(),
                n,
                pct(n, t.actors_total)
            );
        }
    }

    let _ = writeln!(
        o,
        "\nprefab accounting: {} archetype-instanced actors ({:.1}%), {} of them resolved; \
         {} actors owned by a PrefabInstance outer",
        t.archetype_actors,
        pct(t.archetype_actors, t.actors_total),
        t.archetype_actors_resolved,
        t.prefab_outer_actors
    );

    // NA36: this used to print every COLLISION_BEARING_CLASSES entry
    // with a nonzero count, regardless of decode status — which made
    // Terrain/Model-style "read as owner" classes and (post-NA36)
    // InterpActor/KActor/FracturedStaticMeshActor read as gaps even
    // though decode_status() already says they are not. Filter to the
    // classes the header actually claims: undecoded ones.
    let _ = writeln!(o, "\nundecoded classes present (exports across the map):");
    for class in COLLISION_BEARING_CLASSES {
        if decode_status(class) != DecodeStatus::NotDecoded {
            continue;
        }
        let n = t.class_count(class);
        if n > 0 {
            let _ = writeln!(o, "  {class:<34} {n:>8}");
        }
    }

    let ranked = report.ranked_by_triangles();
    let _ = writeln!(o, "\ntop 5 chunks by triangles:");
    for c in ranked.iter().take(5) {
        let _ = writeln!(
            o,
            "  {:<24} {:>9} tris  {:>6} actors  {:>5} resolved",
            c.chunk, c.triangles_emitted, c.actors_total, c.actors_resolved
        );
    }
    let _ = writeln!(o, "bottom 5 chunks by triangles:");
    for c in ranked.iter().rev().take(5) {
        let _ = writeln!(
            o,
            "  {:<24} {:>9} tris  {:>6} actors  {:>5} resolved",
            c.chunk, c.triangles_emitted, c.actors_total, c.actors_resolved
        );
    }

    let unbalanced = report.unbalanced_chunks();
    if !unbalanced.is_empty() {
        let _ = writeln!(
            o,
            "\nWARNING: {} chunk(s) failed the actors == resolved + skips invariant",
            unbalanced.len()
        );
        for c in unbalanced.iter().take(10) {
            let _ = writeln!(
                o,
                "  {}: {} actors, {} resolved, {} skipped",
                c.chunk,
                c.actors_total,
                c.actors_resolved,
                c.skips.total()
            );
        }
    }

    let _ = writeln!(
        o,
        "\nreports: {}\n         {}",
        report_path.display(),
        classes_path.display()
    );
    o
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_navmesh_extractor::coverage::ChunkCoverage;

    /// A report whose per-source tallies do not sum and whose one chunk
    /// fails the balance invariant. Both are impossible out of a real
    /// extraction and both have their own warning branch here.
    fn broken_report() -> MapCoverage {
        MapCoverage {
            map_name: "Broken".to_string(),
            chunks: vec![ChunkCoverage {
                chunk: "Broken-00000001".to_string(),
                chunk_id: 1,
                exports_total: 10,
                actors_total: 7,
                actors_resolved: 2,
                // 2 resolved + 0 skipped != 7 total.
                triangles_emitted: 99,
                // 99 != 1 + 2 + 3.
                staticmesh_triangles: 1,
                terrain_triangles: 2,
                bsp_triangles: 3,
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    #[test]
    fn the_summary_shouts_when_the_per_source_tallies_do_not_sum() {
        // Silence here means every "source X contributes N%" figure
        // downstream is wrong with nothing saying so.
        let text = summary(&broken_report(), Path::new("r.tsv"), Path::new("c.tsv"));
        assert!(text.contains("*** SOURCES DO NOT SUM ***"), "{text}");
    }

    #[test]
    fn the_summary_names_every_chunk_that_fails_the_balance_invariant() {
        let text = summary(&broken_report(), Path::new("r.tsv"), Path::new("c.tsv"));
        assert!(
            text.contains("1 chunk(s) failed the actors == resolved + skips invariant"),
            "{text}"
        );
        assert!(
            text.contains("Broken-00000001: 7 actors, 2 resolved, 0 skipped"),
            "{text}"
        );
    }

    #[test]
    fn a_balanced_report_carries_neither_warning() {
        // The near-miss: the warnings must be conditional, not printed
        // unconditionally and ignored.
        let ok = MapCoverage {
            map_name: "Fine".to_string(),
            chunks: vec![ChunkCoverage {
                chunk: "Fine-00000001".to_string(),
                chunk_id: 1,
                actors_total: 2,
                actors_resolved: 2,
                triangles_emitted: 6,
                staticmesh_triangles: 1,
                terrain_triangles: 2,
                bsp_triangles: 3,
                ..Default::default()
            }],
            ..Default::default()
        };
        let text = summary(&ok, Path::new("r.tsv"), Path::new("c.tsv"));
        assert!(!text.contains("SOURCES DO NOT SUM"), "{text}");
        assert!(!text.contains("failed the actors"), "{text}");
        assert!(text.contains("== Fine =="), "{text}");
        assert!(text.contains("reports: r.tsv"), "{text}");
    }
}
