//! `extract_map extract` — walk a cooked map's chunks, write OBJs, and
//! print/emit the coverage report.

use std::path::Path;
use std::time::Instant;

use cimmeria_navmesh_extractor::coverage::{MapCoverage, SkipReason, UNDECODED_COLLISION_CLASSES};
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

    print_summary(&report, &report_path, &classes_path);
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

fn print_summary(report: &MapCoverage, report_path: &Path, classes_path: &Path) {
    let t = report.totals();
    println!("== {} ==", report.map_name);
    println!(
        "chunks: {} processed ({} filtered out), {} produced geometry",
        report.chunks.len(),
        report.chunks_filtered_out,
        report.chunks_with_geometry()
    );
    println!(
        "exports: {}   StaticMeshActor: {}   resolved: {} ({:.1}%)",
        t.exports_total,
        t.actors_total,
        t.actors_resolved,
        pct(t.actors_resolved, t.actors_total)
    );
    println!("triangles: {}", t.triangles_emitted);
    println!(
        "obj bytes: {} per-chunk + {} combined",
        t.obj_bytes, report.combined_obj_bytes
    );
    println!("wall clock: {:.1}s", report.elapsed_secs);

    println!("\nskips by reason:");
    for reason in SkipReason::ALL {
        let n = t.skips.get(reason);
        if n > 0 {
            println!(
                "  {:<34} {:>8}  ({:.1}% of actors)",
                reason.column(),
                n,
                pct(n, t.actors_total)
            );
        }
    }

    println!(
        "\nprefab accounting: {} archetype-instanced actors ({:.1}%), {} of them resolved; \
         {} actors owned by a PrefabInstance outer",
        t.archetype_actors,
        pct(t.archetype_actors, t.actors_total),
        t.archetype_actors_resolved,
        t.prefab_outer_actors
    );

    println!("\nundecoded classes present (exports across the map):");
    for class in UNDECODED_COLLISION_CLASSES {
        let n = t.class_count(class);
        if n > 0 {
            println!("  {class:<34} {n:>8}");
        }
    }

    let ranked = report.ranked_by_triangles();
    println!("\ntop 5 chunks by triangles:");
    for c in ranked.iter().take(5) {
        println!(
            "  {:<24} {:>9} tris  {:>6} actors  {:>5} resolved",
            c.chunk, c.triangles_emitted, c.actors_total, c.actors_resolved
        );
    }
    println!("bottom 5 chunks by triangles:");
    for c in ranked.iter().rev().take(5) {
        println!(
            "  {:<24} {:>9} tris  {:>6} actors  {:>5} resolved",
            c.chunk, c.triangles_emitted, c.actors_total, c.actors_resolved
        );
    }

    let unbalanced = report.unbalanced_chunks();
    if !unbalanced.is_empty() {
        println!(
            "\nWARNING: {} chunk(s) failed the actors == resolved + skips invariant",
            unbalanced.len()
        );
        for c in unbalanced.iter().take(10) {
            println!(
                "  {}: {} actors, {} resolved, {} skipped",
                c.chunk,
                c.actors_total,
                c.actors_resolved,
                c.skips.total()
            );
        }
    }

    println!(
        "\nreports: {}\n         {}",
        report_path.display(),
        classes_path.display()
    );
}
