//! `occluder_extract build`: one world's shipped, paged `.occ`, trimmed to
//! the explorable area.

use std::io::Write;
use std::path::Path;
use std::time::Instant;

use cimmeria_navmesh_extractor::nav_components::NavGraph;
use cimmeria_navmesh_extractor::nav_roundtrip::XrcNav;
use cimmeria_navmesh_extractor::occluder::explorable::{
    component_triangles, explorable_components, grow_components, map_entry_actors,
    read_entry_points,
};
use cimmeria_navmesh_extractor::occluder::for_each_chunk;
use cimmeria_occluder::{encode_paged, OccluderBuilder, PagedOccluder, Source};

use crate::args::Flags;

/// Radius around a player whose pages stay unpacked, metres: the default
/// AoI radius (100) plus a margin. The cell uses the same number.
pub(crate) const RESIDENCY_RADIUS: f32 = 132.0;

const REPORT_HEADER: &str = "world\tmap\ttriangles\ttrimmed_triangles\tentry_points\tentry_located\tcomponents_kept\tcomponents_total\tfallback_all_components\tpages\tfile_bytes\tfull_ram_bytes\tone_player_pages\tone_player_ram_bytes\tone_player_at\tbuild_secs\tunpack_us_mean\tunpack_us_max\tquery_us_mean";

pub(crate) fn run(rest: &[String]) -> Result<u8, String> {
    let mut allowed = vec![
        "cooked-root",
        "map",
        "index",
        "out",
        "nav",
        "entry-points",
        "page",
        "report",
    ];
    allowed.extend(super::BUILD_KNOBS);
    let f = Flags::parse("build", rest, &allowed)?;
    let cooked = f.path("cooked-root")?;
    let map = f.req("map")?.to_string();
    let out = f.path("out")?;
    let index = super::load_index(&f.path("index")?)?;
    let page = f.f32_or("page", cimmeria_occluder::DEFAULT_PAGE_SIZE)?;
    let world_key = out
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&map)
        .to_string();
    let map_dir = cooked.join("Maps").join(&map);
    let started = Instant::now();

    // Coverage: the explorable navmesh components, when a mesh is given.
    let mut params = f.build_params()?;
    let mut entry = Vec::new();
    let mut coverage = None;
    let (mut kept, mut total, mut located, mut fallback) = (0usize, 0usize, 0usize, false);
    if let Some(nav) = f.opt("nav") {
        // Explorable trimming wants a wider margin than the floor-like
        // default: 15 m past the last walkable polygon.
        if f.opt("margin").is_none() {
            params.margin = Some(15.0);
        }
        let mut file = std::fs::File::open(nav).map_err(|e| format!("{nav}: {e}"))?;
        let graph = NavGraph::from_nav(&XrcNav::read(&mut file).map_err(|e| e.to_string())?);
        if let Some(tsv) = f.opt("entry-points") {
            entry.extend(read_entry_points(Path::new(tsv), &world_key).map_err(|e| e.to_string())?);
        }
        entry.extend(map_entry_actors(&map_dir).map_err(|e| e.to_string())?);
        let ex = explorable_components(&graph, &entry);
        total = graph.component_count as usize;
        located = ex.located;
        let comps = if ex.components.is_empty() {
            // No entry point lands on the mesh: keep every component rather
            // than ship an occluder that covers nothing.
            fallback = true;
            eprintln!(
                "warning: {map}: none of {} entry points is on the mesh; keeping all {total} components",
                entry.len()
            );
            for p in &ex.unlocated {
                let near = graph
                    .locate(p.pos)
                    .map(|h| (h.horizontal_distance, h.vertical_distance));
                eprintln!(
                    "  {} at {:?}: nearest polygon (h, dy) {near:?}",
                    p.source, p.pos
                );
            }
            (0..graph.component_count).collect()
        } else {
            let mut comps = ex.components;
            grow_components(&graph, &mut comps);
            comps
        };
        kept = comps.len();
        coverage = Some(component_triangles(&graph, &comps));
    }

    let label = format!(
        "{map} cell={} terrain_pitch={:?} margin={:?} merge_gap={} y_step={} page={page} explorable={}/{}",
        params.cell, params.terrain_pitch, params.margin, params.merge_gap, params.y_step, kept, total
    );
    let mut builder = OccluderBuilder::new(params, label).map_err(|e| e.to_string())?;
    if let Some(tris) = &coverage {
        for t in tris {
            builder.add_coverage_triangle(t);
        }
    }
    let stats = for_each_chunk(&map_dir, Some(&index), |c| {
        for t in &c.geometry {
            builder.add_triangle(t, Source::Geometry);
        }
        for t in &c.terrain {
            builder.add_triangle(t, Source::Terrain);
        }
    })
    .map_err(|e| e.to_string())?;
    let trimmed = builder.trimmed_count();
    let occ = match builder.finish() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("occluder_extract: {map}: {e}");
            return Ok(2);
        }
    };
    let bytes = encode_paged(&occ, page).map_err(|e| e.to_string())?;
    drop(occ);
    if let Some(parent) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&out, &bytes).map_err(|e| e.to_string())?;
    let build_secs = started.elapsed().as_secs_f64();

    // Residency with one player at the first located entry point.
    let paged = PagedOccluder::from_bytes(bytes).map_err(|e| e.to_string())?;
    let at = entry
        .iter()
        .find(|p| paged.covers(p.pos[0], p.pos[2]))
        .map(|p| p.pos);
    let one = at.map(|p| paged.retain_near(&[[p[0], p[2]]], RESIDENCY_RADIUS));
    // Query cost near the player, pages warm: 2,000 eye-height segments up
    // to 30 m long around the entry point.
    let query_us = at.map_or(0.0, |p| {
        let mut seed = 0x51u64;
        let mut rnd = || {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (seed >> 40) as f32 / (1u64 << 24) as f32
        };
        let segs: Vec<([f32; 3], [f32; 3])> = (0..2000)
            .map(|_| {
                let a = [
                    p[0] + rnd() * 60.0 - 30.0,
                    p[1] + 1.5,
                    p[2] + rnd() * 60.0 - 30.0,
                ];
                let b = [
                    p[0] + rnd() * 60.0 - 30.0,
                    p[1] + 1.5,
                    p[2] + rnd() * 60.0 - 30.0,
                ];
                (a, b)
            })
            .collect();
        let t = Instant::now();
        let mut n = 0usize;
        for (a, b) in &segs {
            n += usize::from(matches!(
                paged.sight(*a, *b),
                cimmeria_occluder::Sight::Blocked { .. }
            ));
        }
        let us = t.elapsed().as_secs_f64() * 1e6 / segs.len() as f64;
        eprintln!(
            "{map}: query {us:.2} us mean over {} segments near the player ({n} blocked)",
            segs.len()
        );
        us
    });
    let full = paged.full_ram_bytes();
    let st = paged.stats();
    let unpack_mean = if st.unpacks > 0 {
        st.unpack_us_total as f64 / st.unpacks as f64
    } else {
        0.0
    };
    let row = format!(
        "{world_key}\t{map}\t{}\t{trimmed}\t{}\t{located}\t{kept}\t{total}\t{fallback}\t{}\t{}\t{full}\t{}\t{}\t{}\t{build_secs:.1}\t{unpack_mean:.0}\t{}\t{query_us:.2}",
        stats.triangles(),
        entry.len(),
        st.pages,
        st.packed_bytes,
        one.as_ref().map_or(0, |r| r.resident_pages),
        one.as_ref().map_or(0, |r| r.resident_bytes),
        at.map_or("-".to_string(), |p| format!("{:.1},{:.1},{:.1}", p[0], p[1], p[2])),
        st.unpack_us_max,
    );
    println!(
        "{map}: {} pages, {} bytes on disk, {full} bytes if all unpacked, {} bytes with one player; {trimmed} of {} triangles trimmed; {located}/{} entry points on {kept}/{total} components; {build_secs:.1}s",
        st.pages,
        st.packed_bytes,
        one.as_ref().map_or(0, |r| r.resident_bytes),
        stats.triangles(),
        entry.len(),
    );
    if let Some(p) = f.opt("report") {
        let exists = Path::new(p).exists();
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(p)
            .map_err(|e| format!("{p}: {e}"))?;
        if !exists {
            writeln!(file, "{REPORT_HEADER}").map_err(|e| e.to_string())?;
        }
        writeln!(file, "{row}").map_err(|e| e.to_string())?;
    }
    Ok(0)
}
