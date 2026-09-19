//! TSV rendering and probe-point loading for the floor probe.
//!
//! Kept apart from the probe math in [`super`] so the geometry can be
//! unit tested without dragging in file formats, and so the report
//! columns have one definition a reader can diff against.

use std::io::Write;
use std::path::Path;

use super::{AxisMapping, Confidence, ProbePoint, ProbeRun};
use crate::ExtractError;

/// One row of the mapping-ranking table.
#[derive(Debug, Clone)]
pub struct MappingScore {
    pub label: String,
    pub high_hits: usize,
    pub high_total: usize,
    pub all_hits: usize,
    pub all_total: usize,
    pub points_with_nearby_geometry: usize,
    /// Sum of nearest-vertex distances across every point — a tie-break
    /// that prefers the mapping which lands geometry *closest* to the
    /// probe set, not merely somewhere in the world.
    pub nearest_dist_sum: f32,
}

impl MappingScore {
    pub fn from_run(run: &ProbeRun) -> Self {
        let high_total = run
            .points
            .iter()
            .filter(|p| p.confidence == Confidence::High)
            .count();
        let nearest_dist_sum = run
            .results()
            .iter()
            .map(|r| r.nearest_dist.unwrap_or(f32::MAX / 64.0))
            .sum();
        Self {
            label: run.mapping.label(),
            high_hits: run.high_confidence_hits(),
            high_total,
            all_hits: run.points_with_floor(),
            all_total: run.points.len(),
            points_with_nearby_geometry: run.points_with_nearby_geometry(),
            nearest_dist_sum,
        }
    }
}

/// Rank runs best-first: most HIGH-confidence floors, then most floors
/// overall, then the geometry that lands nearest the probe set.
pub fn rank(runs: &[ProbeRun]) -> Vec<MappingScore> {
    let mut scores: Vec<MappingScore> = runs.iter().map(MappingScore::from_run).collect();
    scores.sort_by(|a, b| {
        b.high_hits
            .cmp(&a.high_hits)
            .then_with(|| b.all_hits.cmp(&a.all_hits))
            .then_with(|| {
                b.points_with_nearby_geometry
                    .cmp(&a.points_with_nearby_geometry)
            })
            .then_with(|| {
                a.nearest_dist_sum
                    .partial_cmp(&b.nearest_dist_sum)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| a.label.cmp(&b.label))
    });
    scores
}

/// Write the mapping-ranking TSV.
pub fn write_mapping_summary_into<W: Write>(
    w: &mut W,
    scores: &[MappingScore],
) -> crate::Result<()> {
    writeln!(
        w,
        "mapping\thigh_floor_hits\thigh_points\tall_floor_hits\tall_points\tpoints_with_nearby_geometry\tnearest_dist_sum"
    )?;
    for s in scores {
        writeln!(
            w,
            "{}\t{}\t{}\t{}\t{}\t{}\t{:.2}",
            s.label,
            s.high_hits,
            s.high_total,
            s.all_hits,
            s.all_total,
            s.points_with_nearby_geometry,
            s.nearest_dist_sum
        )?;
    }
    Ok(())
}

/// Write the per-point detail TSV for one mapping.
pub fn write_point_detail_into<W: Write>(w: &mut W, run: &ProbeRun) -> crate::Result<()> {
    writeln!(
        w,
        "mapping\tpoint\tconfidence\tbw_x\tbw_y\tbw_z\thas_floor\tfloor_hits\tbest_floor_y\t\
         column_tris\tcolumn_walkable\tcolumn_best_below_y\tcolumn_gap\t\
         near_tris\tnear_walkable\tnear_vertical\tnearest_dist\tsource"
    )?;
    let label = run.mapping.label();
    for (p, r) in run.points.iter().zip(run.results().iter()) {
        writeln!(
            w,
            "{label}\t{}\t{}\t{:.2}\t{:.2}\t{:.2}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            p.label,
            p.confidence.as_str(),
            p.bw[0],
            p.bw[1],
            p.bw[2],
            if r.has_floor() { "yes" } else { "no" },
            r.floor_hits,
            fmt_opt(r.best_floor_y),
            r.column_tris,
            r.column_walkable,
            fmt_opt(r.column_walkable_best_below_y),
            fmt_opt(r.column_gap(p.bw[1])),
            r.near_tris,
            r.near_walkable,
            r.near_vertical,
            fmt_opt(r.nearest_dist),
            p.source,
        )?;
    }
    Ok(())
}

fn fmt_opt(v: Option<f32>) -> String {
    match v {
        Some(v) => format!("{v:.3}"),
        None => "-".to_string(),
    }
}

/// Load probe points from a TSV: `label<TAB>HIGH|MEDIUM<TAB>x<TAB>y<TAB>z[<TAB>source]`.
///
/// Blank lines and `#` comments are skipped, as is a header row whose
/// first field is literally `label`.
pub fn load_points_tsv(path: &Path) -> crate::Result<Vec<ProbePoint>> {
    let text = std::fs::read_to_string(path)?;
    let mut points = Vec::new();
    for (lineno, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let f: Vec<&str> = line.split('\t').map(|s| s.trim()).collect();
        if f[0].eq_ignore_ascii_case("label") {
            continue;
        }
        if f.len() < 5 {
            return Err(ExtractError::Other(format!(
                "{}:{}: expected at least 5 tab-separated fields, got {}",
                path.display(),
                lineno + 1,
                f.len()
            )));
        }
        let confidence = match f[1].to_ascii_uppercase().as_str() {
            "HIGH" => Confidence::High,
            "MEDIUM" | "MED" => Confidence::Medium,
            other => {
                return Err(ExtractError::Other(format!(
                    "{}:{}: unknown confidence {other:?} (want HIGH or MEDIUM)",
                    path.display(),
                    lineno + 1
                )))
            }
        };
        let mut bw = [0.0f32; 3];
        for (i, slot) in bw.iter_mut().enumerate() {
            *slot = f[2 + i].parse::<f32>().map_err(|_| {
                ExtractError::Other(format!(
                    "{}:{}: field {} is not a number: {:?}",
                    path.display(),
                    lineno + 1,
                    i + 3,
                    f[2 + i]
                ))
            })?;
        }
        points.push(ProbePoint::new(
            f[0],
            confidence,
            bw,
            f.get(5).copied().unwrap_or("points file"),
        ));
    }
    Ok(points)
}

/// Parse a `--mapping` argument: either `all`, or one or more
/// comma-separated labels such as `+Y+Z+X,+Z+Y+X`.
pub fn parse_mapping_selector(arg: &str) -> crate::Result<Vec<AxisMapping>> {
    if arg.eq_ignore_ascii_case("all") {
        return Ok(AxisMapping::all());
    }
    let mut out = Vec::new();
    for token in arg.split(',') {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        let m = AxisMapping::from_label(token).ok_or_else(|| {
            ExtractError::Other(format!(
                "not an axis-mapping label: {token:?} (want e.g. +Y+Z+X, or `all`)"
            ))
        })?;
        out.push(m);
    }
    if out.is_empty() {
        return Err(ExtractError::Other(
            "--mapping listed no mappings".to_string(),
        ));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::floor_probe::{ProbeConfig, ProbeRun};

    fn run_with(mapping: AxisMapping, points: Vec<ProbePoint>) -> ProbeRun {
        ProbeRun::new(mapping, ProbeConfig::default(), points)
    }

    fn pts() -> Vec<ProbePoint> {
        vec![
            ProbePoint::new("hi", Confidence::High, [100.0, 50.0, 200.0], "s"),
            ProbePoint::new("med", Confidence::Medium, [900.0, 50.0, 900.0], "s"),
        ]
    }

    /// A single floor quad placed for the CA05 mapping must make CA05
    /// rank first out of all 48 candidates. This is the whole selection
    /// mechanism in miniature.
    #[test]
    fn the_mapping_that_finds_the_floor_ranks_first() {
        let mut soup = crate::geometry::TriangleSoup::new(None);
        // BW (100, 50, 200) under CA05 == UE3 (20000, 10000, 5000).
        soup.push([
            [19_500.0, 9_500.0, 5_000.0],
            [20_500.0, 9_500.0, 5_000.0],
            [20_500.0, 10_500.0, 5_000.0],
        ]);
        soup.push([
            [19_500.0, 9_500.0, 5_000.0],
            [20_500.0, 10_500.0, 5_000.0],
            [19_500.0, 10_500.0, 5_000.0],
        ]);

        let mut runs: Vec<ProbeRun> = AxisMapping::all()
            .into_iter()
            .map(|m| run_with(m, pts()))
            .collect();
        for r in &mut runs {
            r.add_soup(&soup);
        }

        let scores = rank(&runs);
        assert_eq!(scores[0].label, AxisMapping::CA05.label());
        assert_eq!(scores[0].high_hits, 1);
        assert_eq!(scores[0].high_total, 1);
        // Nothing else should claim the HIGH point.
        assert_eq!(
            scores.iter().filter(|s| s.high_hits > 0).count(),
            1,
            "exactly one mapping should find the floor"
        );
    }

    #[test]
    fn mapping_summary_tsv_has_one_row_per_mapping_plus_a_header() {
        let runs: Vec<ProbeRun> = vec![
            run_with(AxisMapping::CA05, pts()),
            run_with(AxisMapping::NAVBUILDER_ON_RAW_UE3, pts()),
        ];
        let scores = rank(&runs);
        let mut buf = Vec::new();
        write_mapping_summary_into(&mut buf, &scores).unwrap();
        let text = String::from_utf8(buf).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("mapping\thigh_floor_hits\t"));
        let width = lines[0].split('\t').count();
        assert!(lines.iter().all(|l| l.split('\t').count() == width));
    }

    #[test]
    fn point_detail_tsv_is_rectangular_and_names_the_mapping() {
        let run = run_with(AxisMapping::CA05, pts());
        let mut buf = Vec::new();
        write_point_detail_into(&mut buf, &run).unwrap();
        let text = String::from_utf8(buf).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 3, "header + 2 points");
        let width = lines[0].split('\t').count();
        assert!(
            lines.iter().all(|l| l.split('\t').count() == width),
            "{text}"
        );
        assert!(lines[1].starts_with("+Y+Z+X\thi\tHIGH\t"));
        // No geometry was fed, so every optional column is a dash.
        assert!(lines[1].contains("\t-\t"));
    }

    #[test]
    fn parse_mapping_selector_accepts_all_and_label_lists() {
        assert_eq!(parse_mapping_selector("all").unwrap().len(), 48);
        assert_eq!(parse_mapping_selector("ALL").unwrap().len(), 48);
        let two = parse_mapping_selector("+Y+Z+X, +Z+Y+X").unwrap();
        assert_eq!(
            two,
            vec![AxisMapping::CA05, AxisMapping::NAVBUILDER_ON_RAW_UE3]
        );
    }

    #[test]
    fn parse_mapping_selector_rejects_garbage() {
        assert!(parse_mapping_selector("+Y+Z").is_err());
        assert!(parse_mapping_selector("").is_err());
        assert!(parse_mapping_selector("+X+X+X").is_err());
    }

    #[test]
    fn load_points_tsv_reads_a_file_and_skips_comments_and_header() {
        let dir = std::env::temp_dir().join(format!(
            "cimmeria-probe-points-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("points.tsv");
        std::fs::write(
            &path,
            "# a comment\nlabel\tconfidence\tx\ty\tz\tsource\n\
             cell\tHIGH\t268.0\t66.79\t1042.59\ttelemetry\n\
             \n\
             comms\tmedium\t271.7\t55.2\t858.0\n",
        )
        .unwrap();

        let points = load_points_tsv(&path).unwrap();
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].label, "cell");
        assert_eq!(points[0].confidence, Confidence::High);
        assert_eq!(points[0].bw, [268.0, 66.79, 1042.59]);
        assert_eq!(points[0].source, "telemetry");
        assert_eq!(points[1].confidence, Confidence::Medium);
        assert_eq!(points[1].source, "points file");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_points_tsv_rejects_a_bad_confidence_and_a_short_row() {
        let dir = std::env::temp_dir().join(format!(
            "cimmeria-probe-points-bad-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();

        let bad_conf = dir.join("bad_conf.tsv");
        std::fs::write(&bad_conf, "p\tMAYBE\t1\t2\t3\n").unwrap();
        assert!(load_points_tsv(&bad_conf).is_err());

        let short = dir.join("short.tsv");
        std::fs::write(&short, "p\tHIGH\t1\t2\n").unwrap();
        assert!(load_points_tsv(&short).is_err());

        let nan = dir.join("nan.tsv");
        std::fs::write(&nan, "p\tHIGH\t1\tabc\t3\n").unwrap();
        assert!(load_points_tsv(&nan).is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
