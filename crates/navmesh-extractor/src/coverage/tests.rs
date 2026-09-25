//! Unit tests for the coverage accounting.
//!
//! Pure arithmetic over hand-built [`ChunkCoverage`] rows — no UE3
//! parsing and no filesystem — so the invariants the whole report
//! rests on (`is_balanced`, `sources_balance`, TSV column/row
//! alignment) are checked in CI whether or not the cooked asset
//! bundle is present.

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
        // The fixture is a StaticMesh-only chunk, so the whole soup
        // came from that source and `sources_balance()` holds.
        staticmesh_triangles: tris,
        obj_bytes: tris * 30,
        archetype_actors: actors - resolved,
        archetype_actors_resolved: 0,
        prefab_outer_actors: 0,
        class_census: BTreeMap::from([
            ("StaticMeshActor".to_string(), actors),
            ("Terrain".to_string(), 3),
        ]),
        ..Default::default()
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
    assert!(lines[0].contains("class_Terrain"));
    assert!(lines[3].starts_with("TOTAL\t"));
    // 19 resolved across the two chunks shows up on the TOTAL row.
    assert!(lines[3].contains("\t19\t"));
}

/// The summary row shares the data rows' schema, so a reader that sums
/// the whole file gets exactly twice the truth — plausible enough to be
/// believed. It was, once: 12,860 `static_mesh_actors` reported against
/// a map with 6,430 `StaticMeshActor` exports.
///
/// `row_kind` is the machine-readable discriminator. This pins that the
/// per-chunk rows sum to the `total` row and that summing everything
/// doubles it, so the trap is documented by a failing arithmetic rather
/// than by a comment nobody reads.
#[test]
fn the_total_row_is_marked_and_is_not_itself_a_chunk() {
    let cov = MapCoverage {
        chunks: vec![chunk("a", 10, 4, 100), chunk("b", 20, 15, 900)],
        ..Default::default()
    };
    let mut buf = Vec::new();
    cov.write_tsv_into(&mut buf).unwrap();
    let text = String::from_utf8(buf).unwrap();
    let header: Vec<&str> = text.lines().next().unwrap().split('\t').collect();
    let kind_idx = header.iter().position(|h| *h == ROW_KIND_COLUMN).unwrap();
    let actors_idx = header
        .iter()
        .position(|h| *h == "static_mesh_actors")
        .unwrap();

    let rows: Vec<Vec<&str>> = text
        .lines()
        .skip(1)
        .map(|l| l.split('\t').collect())
        .collect();
    let kinds: Vec<&str> = rows.iter().map(|r| r[kind_idx]).collect();
    assert_eq!(kinds, vec!["chunk", "chunk", "total"]);

    let sum = |kind: &str| -> u64 {
        rows.iter()
            .filter(|r| r[kind_idx] == kind)
            .map(|r| r[actors_idx].parse::<u64>().unwrap())
            .sum()
    };
    assert_eq!(sum("chunk"), 30, "per-chunk rows");
    assert_eq!(sum("total"), 30, "the summary row restates the same 30");
    assert_eq!(
        sum("chunk") + sum("total"),
        60,
        "summing the file without filtering doubles every counter"
    );
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
    assert_eq!(
        column(&text, "balanced", 1),
        "NO",
        "unbalanced chunk row must say NO: {text}"
    );
    assert_eq!(
        column(&text, "sources_balanced", 1),
        "yes",
        "the actor invariant and the per-source invariant are different \
         claims and must be reported in different columns: {text}"
    );
}

#[test]
fn tsv_flags_a_row_whose_sources_do_not_sum() {
    // A chunk whose OBJ holds more triangles than the three source
    // tallies account for — the shape a new geometry source that
    // forgot its tally would produce.
    let mut bad = chunk("bad", 10, 10, 100);
    bad.terrain_triangles = 50;
    bad.triangles_emitted = 200;
    assert!(!bad.sources_balance());
    let cov = MapCoverage {
        chunks: vec![bad],
        ..Default::default()
    };
    let mut buf = Vec::new();
    cov.write_tsv_into(&mut buf).unwrap();
    let text = String::from_utf8(buf).unwrap();
    assert_eq!(column(&text, "sources_balanced", 1), "NO", "{text}");
    assert_eq!(column(&text, "balanced", 1), "yes", "{text}");
}

#[test]
fn tsv_carries_one_column_per_geometry_source() {
    let mut c = chunk("c", 4, 4, 30);
    c.staticmesh_triangles = 10;
    c.terrain_triangles = 12;
    c.terrain_quads_holed = 7;
    c.terrain_parse_failures = 1;
    c.bsp_triangles = 8;
    c.bsp_hull_cap_triangles = 3;
    c.bsp_models_failed = 2;
    let cov = MapCoverage {
        chunks: vec![c],
        ..Default::default()
    };
    let mut buf = Vec::new();
    cov.write_tsv_into(&mut buf).unwrap();
    let text = String::from_utf8(buf).unwrap();
    for (name, want) in [
        ("triangles", "30"),
        ("staticmesh_triangles", "10"),
        ("terrain_triangles", "12"),
        ("terrain_quads_holed", "7"),
        ("terrain_parse_failures", "1"),
        ("bsp_triangles", "8"),
        ("bsp_hull_cap_triangles", "3"),
        ("bsp_models_failed", "2"),
    ] {
        assert_eq!(column(&text, name, 1), want, "column {name} in {text}");
    }
    // 10 + 12 + 8 == 30; the dropped hull-cap triangles are not part
    // of the sum because they never reached the OBJ.
    assert_eq!(column(&text, "sources_balanced", 1), "yes");
}

/// Value of the named column on data row `row` (1-based, so row 1 is
/// the first chunk).
fn column<'a>(tsv: &'a str, name: &str, row: usize) -> &'a str {
    let mut lines = tsv.lines();
    let header: Vec<&str> = lines.next().expect("header").split('\t').collect();
    let idx = header
        .iter()
        .position(|h| *h == name)
        .unwrap_or_else(|| panic!("no column {name} in {header:?}"));
    tsv.lines()
        .nth(row)
        .expect("data row")
        .split('\t')
        .nth(idx)
        .expect("cell")
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
    // StaticMeshActor (30) outranks Terrain (6). Terrain is decoded
    // since Phase 1.3, so it is emphatically NOT a collision risk —
    // the same run that writes this row emits 2.8 M terrain triangles.
    assert_eq!(lines[1], "StaticMeshActor\t30\t2\tyes\tno");
    assert_eq!(lines[2], "Terrain\t6\t2\tyes\tno");
}

/// The `decoded` column has to track the phases that have landed.
/// Reporting `Terrain` or `Model` as undecoded — which a
/// `class == "StaticMeshActor"` test does — tells a reader the map is
/// missing its ground and its interior floors when it is not.
#[test]
fn decode_status_tracks_the_phases_that_have_landed() {
    for class in [
        "StaticMeshActor",
        "Terrain",
        "Model",
        // NA36: KActor / FracturedStaticMeshActor are StaticMeshActor-
        // shaped and now walked the same way, unconditionally — see
        // `staticmesh::MESH_ACTOR_CLASSES`. InterpActor is opt-in;
        // covered separately below.
        "KActor",
        "FracturedStaticMeshActor",
    ] {
        assert_eq!(
            decode_status(class, false),
            DecodeStatus::Decoded,
            "{class}"
        );
        assert_eq!(
            decode_status(class, true),
            DecodeStatus::Decoded,
            "{class} (unaffected by include_interp_actors)"
        );
    }
    for class in [
        "StaticMeshComponent",
        "TerrainComponent",
        "Brush",
        "BlockingVolume",
    ] {
        assert_eq!(
            decode_status(class, false),
            DecodeStatus::ViaOwner,
            "{class}"
        );
    }
    for class in [
        "Polys",
        "ModelComponent",
        // Still a genuine gap post-NA36: owns an array of components,
        // not one, so it needs its own walk.
        "StaticMeshCollectionActor",
        "SomethingNew",
    ] {
        assert_eq!(
            decode_status(class, false),
            DecodeStatus::NotDecoded,
            "{class}"
        );
    }
}

/// InterpActor's decode status tracks the run, not a static table —
/// this is the whole point of making it opt-in.
#[test]
fn interp_actor_decode_status_follows_the_run_flag_not_a_static_table() {
    assert_eq!(
        decode_status("InterpActor", false),
        DecodeStatus::NotDecoded
    );
    assert_eq!(decode_status("InterpActor", true), DecodeStatus::Decoded);
}

/// Only classes that are BOTH collision-bearing AND unread are risks,
/// with the run's `include_interp_actors` flag off (the default).
#[test]
fn collision_risk_is_the_intersection_not_the_whole_list() {
    let risky: Vec<&str> = COLLISION_BEARING_CLASSES
        .iter()
        .copied()
        .filter(|c| decode_status(c, false) == DecodeStatus::NotDecoded)
        .collect();
    assert_eq!(
        risky,
        vec![
            "BrushComponent",
            "ModelComponent",
            "Polys",
            // NA36 moved KActor / FracturedStaticMeshActor out of this
            // list unconditionally: they are always Decoded, so the
            // intersection with COLLISION_BEARING_CLASSES no longer
            // includes them. InterpActor is back in this list by
            // default (opt-in, off) — a future map's rebuild that
            // forgets the flag still sees it flagged as a risk here.
            // StaticMeshCollectionActor remains the one class with no
            // implementation at all.
            "InterpActor",
            "StaticMeshCollectionActor",
        ],
        "the risk set drifted; update the phase table in the README too"
    );
}
