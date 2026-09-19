//! `extract_map probe` — read back the emitted OBJs and ask, for each
//! known-walkable world point, whether the geometry puts a floor under
//! it, under each candidate UE3 → BigWorld axis mapping.

use std::path::{Path, PathBuf};
use std::time::Instant;

use cimmeria_navmesh_extractor::floor_probe::{
    self, report as probe_report, AxisMapping, ProbeRun,
};
use cimmeria_navmesh_extractor::obj;

use crate::args::ProbeArgs;

/// The two mappings whose per-point detail is always written, even when
/// they score zero: CA05's calibrated hypothesis, and what NavBuilder's
/// `loadOBJ` swizzle produces from a raw-UE3-cm OBJ. A row of zeroes
/// for the latter is itself the evidence.
const ALWAYS_DETAIL: [AxisMapping; 2] = [AxisMapping::CA05, AxisMapping::NAVBUILDER_ON_RAW_UE3];

pub(crate) fn run(args: ProbeArgs) -> Result<(), Box<dyn std::error::Error>> {
    let points = match &args.points {
        Some(p) => probe_report::load_points_tsv(p)?,
        None => floor_probe::castle_probe_points(),
    };
    eprintln!(
        "probing {} point(s) under {} mapping(s)",
        points.len(),
        args.mappings.len()
    );

    let mut runs: Vec<ProbeRun> = args
        .mappings
        .iter()
        .map(|m| ProbeRun::new(*m, args.config, points.clone()))
        .collect();

    // Per-chunk OBJs only — the combined map OBJ is the same triangles
    // again and would double-count every hit.
    let mut obj_files: Vec<PathBuf> = std::fs::read_dir(&args.obj_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| is_chunk_obj(p))
        .collect();
    obj_files.sort();
    if obj_files.is_empty() {
        return Err(format!(
            "no <chunkid>o.obj files in {} — run `extract` first",
            args.obj_dir.display()
        )
        .into());
    }

    // One chunk in memory at a time, fed to every mapping. 48 candidate
    // mappings over a whole map would be ~14 GB if we materialised a
    // transformed copy per mapping.
    let t = Instant::now();
    let mut triangles = 0u64;
    for path in &obj_files {
        // Back to UE3 cm — the OBJ on disk is Y/Z-swapped for
        // NavBuilder, and the probe's mappings are UE3 -> BigWorld.
        let soup = obj::read_obj_as_ue3(path)?;
        triangles += soup.triangle_count() as u64;
        for run in &mut runs {
            run.add_soup(&soup);
        }
    }
    eprintln!(
        "read {} OBJ files, {} triangles, in {:.1}s",
        obj_files.len(),
        triangles,
        t.elapsed().as_secs_f32()
    );

    let scores = probe_report::rank(&runs);

    let report_path = args.report_path();
    let mut buf = Vec::new();
    probe_report::write_mapping_summary_into(&mut buf, &scores)?;
    std::fs::write(&report_path, &buf)?;

    let detail_path = args.detail_path();
    let detail = render_detail(&runs, &scores)?;
    std::fs::write(&detail_path, detail)?;

    print_summary(&scores, &runs, triangles, &report_path, &detail_path);
    Ok(())
}

/// Concatenate the per-point detail of every interesting mapping into
/// one TSV with a single header row, best-ranked first.
fn render_detail(
    runs: &[ProbeRun],
    scores: &[probe_report::MappingScore],
) -> Result<String, Box<dyn std::error::Error>> {
    let mut out = String::new();
    for run in order_runs(runs, scores) {
        if run.points_with_floor() == 0 && !ALWAYS_DETAIL.contains(&run.mapping) {
            continue;
        }
        let mut one = Vec::new();
        probe_report::write_point_detail_into(&mut one, run)?;
        let text = String::from_utf8(one)?;
        let mut lines = text.lines();
        let header = lines.next().unwrap_or_default();
        if out.is_empty() {
            out.push_str(header);
            out.push('\n');
        }
        for line in lines {
            out.push_str(line);
            out.push('\n');
        }
    }
    Ok(out)
}

/// The runs, in the order `scores` ranked them.
fn order_runs<'a>(
    runs: &'a [ProbeRun],
    scores: &[probe_report::MappingScore],
) -> Vec<&'a ProbeRun> {
    scores
        .iter()
        .filter_map(|s| runs.iter().find(|r| r.mapping.label() == s.label))
        .collect()
}

fn print_summary(
    scores: &[probe_report::MappingScore],
    runs: &[ProbeRun],
    triangles: u64,
    report_path: &Path,
    detail_path: &Path,
) {
    println!("== floor probe over {triangles} triangles ==");
    println!(
        "{:<8} {:>10} {:>10} {:>10}",
        "mapping", "HIGH hits", "all hits", "near geom"
    );
    for s in scores.iter().take(8) {
        println!(
            "{:<8} {:>5}/{:<4} {:>5}/{:<4} {:>10}",
            s.label,
            s.high_hits,
            s.high_total,
            s.all_hits,
            s.all_total,
            s.points_with_nearby_geometry
        );
    }

    match scores.first() {
        Some(best) if best.high_hits > 0 => println!(
            "\nbest mapping: {} ({}/{} HIGH-confidence points have a floor)",
            best.label, best.high_hits, best.high_total
        ),
        _ => println!(
            "\nNO mapping put a floor under any HIGH-confidence point. \
             Check the per-point detail for `near_vertical` (walls present, floors missing) \
             versus `nearest_dist` (nothing anywhere near — transform is wrong)."
        ),
    }

    for named in ALWAYS_DETAIL {
        let Some(run) = runs.iter().find(|r| r.mapping == named) else {
            continue;
        };
        println!("\n-- {} --", named.label());
        for (p, r) in run.points.iter().zip(run.results().iter()) {
            println!(
                "  {:<28} {:<7} floor={:<4} col_tris={:<7} col_walk={:<7} gap={:<9} \
                 near={:<7} near_vert={:<7} nearest={}",
                p.label,
                p.confidence.as_str(),
                if r.has_floor() { "yes" } else { "no" },
                r.column_tris,
                r.column_walkable,
                opt(r.column_gap(p.bw[1])),
                r.near_tris,
                r.near_vertical,
                opt(r.nearest_dist),
            );
        }
    }

    println!(
        "\nreports: {}\n         {}",
        report_path.display(),
        detail_path.display()
    );
}

fn opt(v: Option<f32>) -> String {
    v.map(|x| format!("{x:.2}")).unwrap_or_else(|| "-".into())
}

/// `<8 hex digits>o.obj`, matching `ChunkId::obj_filename`.
fn is_chunk_obj(path: &Path) -> bool {
    if path.extension().and_then(|e| e.to_str()) != Some("obj") {
        return false;
    }
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.len() == 9 && s.ends_with('o') && s[..8].chars().all(|c| c.is_ascii_hexdigit()))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_obj_filter_accepts_only_the_navbuilder_naming() {
        assert!(is_chunk_obj(Path::new("000a0002o.obj")));
        assert!(is_chunk_obj(Path::new("/x/y/fffefffdo.obj")));
        // The combined map OBJ must not be re-probed — it holds the same
        // triangles as the per-chunk files and would double every count.
        assert!(!is_chunk_obj(Path::new("castle.obj")));
        assert!(!is_chunk_obj(Path::new("000a0002o.txt")));
        assert!(!is_chunk_obj(Path::new("000a0002.obj")));
        assert!(!is_chunk_obj(Path::new("zzzzzzzzo.obj")));
    }

    /// The two named mappings must always make it into the detail file,
    /// even at zero hits — the `+Z+Y+X` zero row is the evidence about
    /// NavBuilder's current swizzle.
    #[test]
    fn detail_always_includes_the_two_named_mappings() {
        use cimmeria_navmesh_extractor::floor_probe::{Confidence, ProbeConfig, ProbePoint};

        let points = vec![ProbePoint::new(
            "p",
            Confidence::High,
            [0.0, 0.0, 0.0],
            "synthetic",
        )];
        // Three mappings, no geometry fed: nobody scores a floor.
        let runs: Vec<ProbeRun> = [
            AxisMapping::CA05,
            AxisMapping::NAVBUILDER_ON_RAW_UE3,
            AxisMapping::from_label("+X+Y+Z").unwrap(),
        ]
        .into_iter()
        .map(|m| ProbeRun::new(m, ProbeConfig::default(), points.clone()))
        .collect();

        let scores = probe_report::rank(&runs);
        let detail = render_detail(&runs, &scores).unwrap();
        let lines: Vec<&str> = detail.lines().collect();
        assert_eq!(lines.len(), 3, "header + the two named mappings: {detail}");
        assert!(detail.contains("+Y+Z+X\tp\t"));
        assert!(detail.contains("+Z+Y+X\tp\t"));
        assert!(
            !detail.contains("+X+Y+Z\tp\t"),
            "a zero-hit unnamed mapping should be omitted"
        );
    }
}
