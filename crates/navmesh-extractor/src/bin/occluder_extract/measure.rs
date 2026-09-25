//! `occluder_extract measure`: build a map's occluder at several cell sizes
//! and report size, RAM, build time and (with `--nav`) accuracy against the
//! exact tracer. The phase-1 numbers of NA27 come from here.

use std::io::Write;
use std::time::Instant;

use cimmeria_navmesh_extractor::nav_components::NavGraph;
use cimmeria_navmesh_extractor::occluder::exact::ExactScene;
use cimmeria_navmesh_extractor::occluder::sweep::{self, Confusion, SweepParams};
use cimmeria_navmesh_extractor::occluder::{for_each_chunk, BwTriangle};
use cimmeria_occluder::{format, BuildParams, OccluderBuilder, Sight, Source};

use crate::args::Flags;

const REPORT_HEADER: &str = "map\ttriangles\tcell\tterrain_pitch\tmargin\tgeo_dims\tgeo_stored_tiles\tgeo_spans\tterrain_dims\tterrain_stored_tiles\tterrain_patches\tterrain_fallback\tfile_bytes\tram_bytes\tbuild_secs\twalk_secs\tclearance\tpairs\ttruth_blocked\tocc_false_clear\tocc_false_block\tocc_off_grid\tocc_wrong_given_blocked\tocc_wrong_given_clear\tnav_wrong_given_blocked\tnav_wrong_given_clear";

pub(crate) fn run(rest: &[String]) -> Result<u8, String> {
    let mut allowed = vec![
        "cooked-root",
        "map",
        "index",
        "cells",
        "nav",
        "pairs",
        "seed",
        "eye",
        "max-dy",
        "max-component-area",
        "clearances",
        "pairs-out",
        "report",
        "write-dir",
        "coverage-nav",
        "include-interp-actors",
    ];
    allowed.extend(super::BUILD_KNOBS);
    let f = Flags::parse("measure", rest, &allowed)?;
    let cooked = f.path("cooked-root")?;
    let map = f.req("map")?.to_string();
    let index = super::load_index(&f.path("index")?)?;
    let include_interp_actors = f.bool_or("include-interp-actors", false)?;
    let base = f.build_params()?;
    let cells = f.f32_list_or("cells", &[0.25, 0.5, 1.0])?;
    let clearances = f.f32_list_or(
        "clearances",
        &[cimmeria_occluder::Occluder::DEFAULT_CLEARANCE],
    )?;

    let walk_started = Instant::now();
    let mut tris: Vec<(BwTriangle, Source)> = Vec::new();
    let stats = for_each_chunk(
        &cooked.join("Maps").join(&map),
        Some(&index),
        include_interp_actors,
        |c| {
            tris.extend(c.geometry.iter().map(|t| (*t, Source::Geometry)));
            tris.extend(c.terrain.iter().map(|t| (*t, Source::Terrain)));
        },
    )
    .map_err(|e| e.to_string())?;
    let walk_secs = walk_started.elapsed().as_secs_f64();
    eprintln!(
        "{map}: walked {} chunks, {} triangles (sm {} terrain {} bsp {}) in {walk_secs:.1}s",
        stats.chunks,
        tris.len(),
        stats.staticmesh_triangles,
        stats.terrain_triangles,
        stats.bsp_triangles
    );

    // Accuracy set-up: the mesh, the pairs, and the exact scene over the
    // triangles near the mesh.
    let sweep_ctx = match f.opt("nav") {
        None => None,
        Some(nav_path) => {
            let graph = super::load_nav_graph(std::path::Path::new(nav_path))?;
            let params = SweepParams {
                pairs: f.usize_or("pairs", 4000)?,
                seed: f.usize_or("seed", 0x4e41_3237)? as u64,
                eye: f.f32_or("eye", 1.5)?,
                max_dy: f.f32_or("max-dy", 4.0)?,
                max_component_area: f.opt_f32_or("max-component-area", None)?.map(f64::from),
                ..SweepParams::default()
            };
            let pairs = sweep::sample_pairs(&graph, &params);
            let (lo, hi) = (graph.bmin, graph.bmax);
            let pad = 40.0;
            let near: Vec<BwTriangle> = tris
                .iter()
                .map(|(t, _)| *t)
                .filter(|t| {
                    t.iter().any(|v| {
                        v[0] >= lo[0] - pad
                            && v[0] <= hi[0] + pad
                            && v[2] >= lo[2] - pad
                            && v[2] <= hi[2] + pad
                    })
                })
                .collect();
            eprintln!(
                "{map}: {} pairs from {nav_path} ({} polys); exact scene {} triangles",
                pairs.len(),
                graph.polys.len(),
                near.len()
            );
            Some((graph, pairs, ExactScene::new(near), params))
        }
    };

    let mut report_rows = Vec::new();
    for &cell in &cells {
        let params = BuildParams { cell, ..base };
        let label = format!(
            "{map} cell={cell} terrain_pitch={:?} margin={:?}",
            params.terrain_pitch, params.margin
        );
        let started = Instant::now();
        let mut b = OccluderBuilder::new(params, label).map_err(|e| e.to_string())?;
        if let Some(nav) = f.opt("coverage-nav") {
            super::add_nav_coverage(&mut b, std::path::Path::new(nav))?;
        }
        for (t, s) in &tris {
            b.add_triangle(t, *s);
        }
        let fallback = b.terrain_fallback_count();
        let occ = b.finish().map_err(|e| e.to_string())?;
        let build_secs = started.elapsed().as_secs_f64();
        let bytes = format::encode(&occ);
        if let Some(dir) = f.opt("write-dir") {
            let p = std::path::Path::new(dir).join(format!("{}_{cell}.occ", map.to_lowercase()));
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            std::fs::write(&p, &bytes).map_err(|e| e.to_string())?;
        }
        let g = match occ.layers().first() {
            Some(l) => {
                let (x, z) = l.dims();
                (format!("{x}x{z}"), l.stored_tiles(), l.span_count())
            }
            None => ("-".to_string(), 0, 0),
        };
        let t = match occ.heightfield() {
            Some(h) => {
                let (x, z) = h.dims();
                (format!("{x}x{z}"), h.stored_tiles(), h.patch_count())
            }
            None => ("-".to_string(), 0, 0),
        };
        let prefix = format!(
            "{map}\t{}\t{cell}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{fallback}\t{}\t{}\t{build_secs:.2}\t{walk_secs:.1}",
            tris.len(),
            params.terrain_pitch.map_or("none".into(), |c| c.to_string()),
            params.margin.map_or("none".into(), |c| c.to_string()),
            g.0,
            g.1,
            g.2,
            t.0,
            t.1,
            t.2,
            bytes.len(),
            occ.ram_bytes(),
        );
        let load_started = Instant::now();
        let loaded = format::decode(&bytes).map_err(|e| e.to_string())?;
        let load_ms = load_started.elapsed().as_secs_f64() * 1e3;
        eprintln!(
            "{map} cell {cell}: {} bytes on disk, {} bytes RAM, build {build_secs:.1}s, decode {load_ms:.1} ms",
            bytes.len(),
            occ.ram_bytes()
        );
        if let Some((_, pairs, _, sp)) = &sweep_ctx {
            // Per-query cost over the sweep's own pairs, ten passes.
            let started = Instant::now();
            let mut blocked = 0usize;
            for _ in 0..10 {
                for (a, b) in pairs {
                    let s = loaded.sight(
                        [a.pos[0], a.pos[1] + sp.eye, a.pos[2]],
                        [b.pos[0], b.pos[1] + sp.eye, b.pos[2]],
                    );
                    blocked += usize::from(matches!(s, Sight::Blocked { .. }));
                }
            }
            let n = (pairs.len() * 10).max(1);
            eprintln!(
                "  query: {:.2} us mean over {n} segments ({blocked} blocked)",
                started.elapsed().as_secs_f64() * 1e6 / n as f64
            );
        }
        match &sweep_ctx {
            None => report_rows.push(format!("{prefix}\t-\t0\t0\t0\t0\t0\t0\t0\t0\t0")),
            Some((graph, pairs, exact, sp)) => {
                for &clear in &clearances {
                    let (occ_c, nav_c, rows) = score(graph, exact, &occ, pairs, sp.eye, clear);
                    let truth_blocked = occ_c.blocked_ok + occ_c.clear_wrong;
                    eprintln!(
                        "  clearance {clear}: truth blocked {truth_blocked}/{}; occluder false clear {} ({:.2}% of truly blocked), false block {} ({:.2}% of truly clear), off grid {}; navmesh wrong|blocked {:.1}% wrong|clear {:.2}%",
                        pairs.len(),
                        occ_c.clear_wrong,
                        100.0 * occ_c.false_clear_rate(),
                        occ_c.blocked_wrong,
                        100.0 * occ_c.false_block_rate(),
                        occ_c.unknown,
                        100.0 * nav_c.wrong_given_blocked(),
                        100.0 * nav_c.wrong_given_clear(),
                    );
                    report_rows.push(format!(
                        "{prefix}\t{clear}\t{}\t{truth_blocked}\t{}\t{}\t{}\t{:.4}\t{:.4}\t{:.4}\t{:.4}",
                        pairs.len(),
                        occ_c.clear_wrong,
                        occ_c.blocked_wrong,
                        occ_c.unknown,
                        occ_c.wrong_given_blocked(),
                        occ_c.wrong_given_clear(),
                        nav_c.wrong_given_blocked(),
                        nav_c.wrong_given_clear(),
                    ));
                    if let Some(p) = f.opt("pairs-out") {
                        let p = format!("{p}.{cell}.{clear}.tsv");
                        std::fs::write(&p, rows).map_err(|e| e.to_string())?;
                    }
                }
            }
        }
    }

    let mut out: Box<dyn Write> = match f.opt("report") {
        Some(p) => {
            let exists = std::path::Path::new(p).exists();
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(p)
                .map_err(|e| format!("{p}: {e}"))?;
            if !exists {
                writeln!(file, "{REPORT_HEADER}").map_err(|e| e.to_string())?;
            }
            Box::new(file)
        }
        None => {
            println!("{REPORT_HEADER}");
            Box::new(std::io::stdout())
        }
    };
    for r in report_rows {
        writeln!(out, "{r}").map_err(|e| e.to_string())?;
    }
    Ok(0)
}

/// Score the occluder at one endpoint clearance; also returns the per-pair
/// rows as TSV text.
fn score(
    graph: &NavGraph,
    exact: &ExactScene,
    occ: &cimmeria_occluder::Occluder,
    pairs: &[(sweep::NavPoint, sweep::NavPoint)],
    eye: f32,
    clearance: f32,
) -> (Confusion, Confusion, String) {
    let (rows, _, nav_c) = sweep::run(graph, exact, occ, pairs, eye);
    let mut occ_c = Confusion::default();
    let mut text = String::from("ax\tay\taz\tbx\tby\tbz\ttruth\tocc\tnav\tnear\n");
    for r in &rows {
        let s = occ.sight_with_clearance(r.a, r.b, clearance);
        let said = match s {
            Sight::Clear => Some(false),
            Sight::Blocked { .. } => Some(true),
            Sight::OffGrid => None,
        };
        match (said, r.truth_blocked) {
            (None, _) => occ_c.unknown += 1,
            (Some(false), false) => occ_c.clear_ok += 1,
            (Some(false), true) => occ_c.clear_wrong += 1,
            (Some(true), true) => occ_c.blocked_ok += 1,
            (Some(true), false) => occ_c.blocked_wrong += 1,
        }
        // For a disagreement, how far the exact segment is from geometry.
        let near = if said == Some(!r.truth_blocked) {
            near_miss(exact, r.a, r.b)
        } else {
            None
        };
        text.push_str(&format!(
            "{:.2}\t{:.2}\t{:.2}\t{:.2}\t{:.2}\t{:.2}\t{}\t{}\t{:?}\t{}\n",
            r.a[0],
            r.a[1],
            r.a[2],
            r.b[0],
            r.b[1],
            r.b[2],
            if r.truth_blocked { "blocked" } else { "clear" },
            s.label(),
            r.nav,
            near.map_or("-".to_string(), |d| d.to_string())
        ));
    }
    (occ_c, nav_c, text)
}

/// The smallest offset (metres) at which a parallel copy of the segment,
/// shifted sideways or vertically, is blocked: how close the clear segment
/// passes to geometry. `None` beyond 0.5 m.
fn near_miss(exact: &ExactScene, a: [f32; 3], b: [f32; 3]) -> Option<f32> {
    let (dx, dz) = (b[0] - a[0], b[2] - a[2]);
    let len = (dx * dx + dz * dz).sqrt().max(1e-6);
    let perp = [-dz / len, 0.0, dx / len];
    for d in [0.02f32, 0.05, 0.1, 0.2, 0.35, 0.5] {
        for off in [
            [perp[0] * d, 0.0, perp[2] * d],
            [-perp[0] * d, 0.0, -perp[2] * d],
            [0.0, d, 0.0],
            [0.0, -d, 0.0],
        ] {
            let sa = [a[0] + off[0], a[1] + off[1], a[2] + off[2]];
            let sb = [b[0] + off[0], b[1] + off[1], b[2] + off[2]];
            if exact.blocked(sa, sb) {
                return Some(d);
            }
        }
    }
    None
}
