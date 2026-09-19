//! Report formatting for the census.
//!
//! Every function here writes into an `impl Write` rather than calling
//! `println!`, so a test can assert on the exact text without a process
//! boundary.

use std::io::{self, Write};

use super::census::{Census, MeshStats};

/// Meshes ranked the way both the human report and the TSV want them:
/// most-instanced first, name as the tie-break.
pub fn ranked(census: &Census) -> Vec<(&String, &MeshStats)> {
    let mut v: Vec<(&String, &MeshStats)> = census.stats.iter().collect();
    v.sort_by_key(|(name, s)| (std::cmp::Reverse(s.instances), (*name).clone()));
    v
}

/// The whole human-readable report.
pub fn write_report(
    out: &mut impl Write,
    map: &str,
    chunks: usize,
    census: &Census,
    cache_stats: (u64, u64),
    distinct_paths: usize,
) -> io::Result<()> {
    let (hits, misses) = cache_stats;
    writeln!(out, "== summary ==")?;
    writeln!(out, "map\t{map}")?;
    writeln!(out, "chunks\t{chunks}")?;
    writeln!(out, "direct_components\t{}", census.direct_total)?;
    writeln!(out, "archetype_stub_components\t{}", census.stub_total)?;
    writeln!(out, "resolved\t{}", census.placements.len())?;
    writeln!(out, "distinct_archetype_paths\t{distinct_paths}")?;
    writeln!(out, "cache_hits\t{hits}\tcache_misses\t{misses}")?;
    writeln!(out, "distinct_meshes\t{}", census.stats.len())?;
    let total_tris: u64 = census
        .stats
        .values()
        .map(|s| s.instances * s.tris_per_instance as u64)
        .sum();
    writeln!(out, "triangles_added\t{total_tris}")?;
    let read_failures: u64 = census.read_failures.values().sum();
    writeln!(out, "unreadable_exports\t{read_failures}")?;

    writeln!(out, "\n== unreadable exports (actor skipped) ==")?;
    if census.read_failures.is_empty() {
        writeln!(out, "(none)")?;
    }
    for (k, n) in &census.read_failures {
        writeln!(out, "{n}\t{k}")?;
    }

    writeln!(out, "\n== actors suppressed by bCollideActors = false ==")?;
    if census.collision_disabled.is_empty() {
        writeln!(out, "(none)")?;
    }
    let mut cd: Vec<(&String, &u64)> = census.collision_disabled.iter().collect();
    cd.sort_by_key(|(k, n)| (std::cmp::Reverse(**n), (*k).clone()));
    let cd_total: u64 = census.collision_disabled.values().sum();
    for (k, n) in cd {
        writeln!(out, "{n}\t{k}")?;
    }
    writeln!(out, "TOTAL\t{cd_total}")?;

    writeln!(
        out,
        "\n== traversal-keyword actors (elevator/lift/stair/ramp/door/...) =="
    )?;
    if census.traversal.is_empty() {
        writeln!(out, "(none)")?;
    }
    writeln!(
        out,
        "bw_x\tbw_y\tbw_z\tchunk\tsource\tcollides\tarchetype\tmesh"
    )?;
    let mut traversal: Vec<&super::census::Traversal> = census.traversal.iter().collect();
    traversal.sort_by(|a, b| {
        a.mesh
            .cmp(&b.mesh)
            .then(a.bw[0].total_cmp(&b.bw[0]))
            .then(a.bw[2].total_cmp(&b.bw[2]))
    });
    for t in &traversal {
        writeln!(
            out,
            "{:.2}\t{:.2}\t{:.2}\t{}\t{}\t{}\t{}\t{}",
            t.bw[0],
            t.bw[1],
            t.bw[2],
            t.chunk,
            t.kind,
            if t.collides { "yes" } else { "NO" },
            if t.archetype_instanced { "yes" } else { "no" },
            t.mesh
        )?;
    }

    writeln!(out, "\n== resolution failures ==")?;
    if census.failures.is_empty() {
        writeln!(out, "(none)")?;
    }
    for (k, v) in &census.failures {
        writeln!(out, "{v}\t{k}")?;
    }

    writeln!(out, "\n== component-local transform properties ==")?;
    if census.comp_transform_props.is_empty() {
        writeln!(out, "(none — actor transform is the whole story)")?;
    }
    for (k, v) in &census.comp_transform_props {
        writeln!(out, "{k}\t{v}")?;
    }

    writeln!(
        out,
        "\n== actor transform properties inherited from the archetype =="
    )?;
    if census.actor_inherited_transform.is_empty() {
        writeln!(out, "(none — every stub actor carries its own placement)")?;
    }
    for (k, v) in &census.actor_inherited_transform {
        writeln!(out, "{k}\t{v}")?;
    }

    writeln!(out, "\n== per-mesh ==")?;
    writeln!(
        out,
        "instances\ttris_each\ttris_total\tfootprint_m2\twalkable_m2\twalkable_pct\t\
         bw_y_min\tbw_y_max\tchunks\tmesh"
    )?;
    for (name, s) in ranked(census) {
        let pct = if s.footprint_m2 > 0.0 {
            100.0 * s.walkable_m2 / s.footprint_m2
        } else {
            0.0
        };
        writeln!(
            out,
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
        )?;
    }

    writeln!(out, "\n== per-chunk ==")?;
    let mut chunk_rows: Vec<(&String, &u64)> = census.per_chunk.iter().collect();
    chunk_rows.sort_by_key(|(name, n)| (std::cmp::Reverse(**n), (*name).clone()));
    for (chunk, n) in chunk_rows {
        writeln!(out, "{n}\t{chunk}")?;
    }
    Ok(())
}

/// One row per distinct mesh.
pub fn write_meshes_tsv(out: &mut impl Write, census: &Census) -> io::Result<()> {
    writeln!(
        out,
        "mesh\tinstances\ttris_each\ttris_total\tfootprint_m2\twalkable_m2\tbw_y_min\tbw_y_max\tsample_bw_x\tsample_bw_y\tsample_bw_z\tarchetype_paths"
    )?;
    for (name, s) in ranked(census) {
        writeln!(
            out,
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
        )?;
    }
    Ok(())
}

/// One row per resolved instance.
pub fn write_positions_tsv(out: &mut impl Write, census: &Census) -> io::Result<()> {
    writeln!(out, "chunk\tactor\tmesh\tbw_x\tbw_y\tbw_z\tarchetype_path")?;
    for p in &census.placements {
        writeln!(
            out,
            "{}\t{}\t{}\t{:.3}\t{:.3}\t{:.3}\t{}",
            p.chunk, p.actor, p.mesh, p.bw[0], p.bw[1], p.bw[2], p.arch_path
        )?;
    }
    Ok(())
}
