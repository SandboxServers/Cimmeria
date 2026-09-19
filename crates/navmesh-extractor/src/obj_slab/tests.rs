use std::io::Write;

use super::*;

/// Write a chunk OBJ in the extractor's exact format (CRLF, UE3 cm with Y/Z
/// swapped) so the parser is tested against the real shape, not a
/// convenience format.
fn write_chunk(dir: &Path, stem: &str, tris_ue_cm: &[[[f32; 3]; 3]]) {
    let mut f = std::fs::File::create(dir.join(format!("{stem}.obj"))).unwrap();
    write!(f, "# test fixture\r\no Chunk_{stem}\r\n").unwrap();
    for t in tris_ue_cm {
        for v in t {
            // v <ue.x> <ue.z> <ue.y>
            write!(f, "v {} {} {}\r\n", v[0], v[2], v[1]).unwrap();
        }
    }
    for i in 0..tris_ue_cm.len() {
        let b = i * 3 + 1;
        write!(f, "f {} {} {}\r\n", b, b + 1, b + 2).unwrap();
    }
}

/// BigWorld metres → the UE3 centimetres a chunk OBJ would carry.
/// `bw = (ue.y, ue.z, ue.x) / 100`, so `ue = (bw.z, bw.x, bw.y) * 100`.
fn ue_cm(bw: [f32; 3]) -> [f32; 3] {
    [bw[2] * 100.0, bw[0] * 100.0, bw[1] * 100.0]
}

/// A flat quad at BigWorld height `y` spanning `[x0,x1] x [z0,z1]`.
fn floor_quad(x0: f32, z0: f32, x1: f32, z1: f32, y: f32) -> [[[f32; 3]; 3]; 2] {
    [
        [ue_cm([x0, y, z0]), ue_cm([x1, y, z0]), ue_cm([x1, y, z1])],
        [ue_cm([x0, y, z0]), ue_cm([x1, y, z1]), ue_cm([x0, y, z1])],
    ]
}

/// A vertical wall panel along constant x, spanning `[z0,z1]` and
/// `[y0,y1]`.
fn wall_x(x: f32, z0: f32, z1: f32, y0: f32, y1: f32) -> [[[f32; 3]; 3]; 2] {
    [
        [ue_cm([x, y0, z0]), ue_cm([x, y0, z1]), ue_cm([x, y1, z1])],
        [ue_cm([x, y0, z0]), ue_cm([x, y1, z1]), ue_cm([x, y1, z0])],
    ]
}

fn tmpdir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("cimmeria-obj-slab-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// The axis mapping is the whole point of this module: a vertex written as
/// UE3 `(x, z, y)` centimetres must come back as BigWorld metres
/// `(ue.y, ue.z, ue.x) / 100`. Getting it wrong would put every measurement
/// in the wrong place while every unit still looked plausible.
#[test]
fn vertices_come_back_in_bigworld_metres() {
    // BW (250.0, 66.0, 1040.0) ⇒ ue = (104000, 25000, 6600) cm, written as
    // `v ue.x ue.z ue.y` = `v 104000 6600 25000`.
    let v = parse_vertex("104000 6600 25000").expect("three columns");
    assert!((v[0] - 250.0).abs() < 1e-3, "bw.x = {}", v[0]);
    assert!((v[1] - 66.0).abs() < 1e-3, "bw.y = {}", v[1]);
    assert!((v[2] - 1040.0).abs() < 1e-3, "bw.z = {}", v[2]);
}

/// A face line whose indices run past the vertex list is dropped, not
/// panicked on. NavBuilder's own parser silently drops short lines
/// (`mesh.cpp:115`), so truncated files are a real input.
#[test]
fn out_of_range_and_short_faces_are_dropped() {
    assert_eq!(parse_face("1 2 3", 3), Some([0, 1, 2]));
    assert_eq!(parse_face("1 2 3", 2), None);
    assert_eq!(parse_face("1 2", 3), None);
    assert_eq!(parse_face("0 1 2", 3), None, "OBJ indices are 1-based");
    assert_eq!(parse_face("1/1 2/2 3/3", 3), Some([0, 1, 2]));
}

/// Two floors stacked 12 m apart with nothing between them — the shape the
/// Castle 405↔754 gap turned out to be. `column` must report both, in
/// height order, and the step between them.
#[test]
fn column_reports_stacked_floors_in_height_order() {
    let dir = tmpdir("column");
    let mut tris: Vec<[[f32; 3]; 3]> = Vec::new();
    tris.extend(floor_quad(275.0, 865.0, 290.0, 885.0, 43.4));
    tris.extend(floor_quad(275.0, 865.0, 290.0, 885.0, 55.4));
    write_chunk(&dir, "00080002o", &tris);

    let mut set = SlabSet::new(vec![Slab::new(
        "stack",
        [270.0, 30.0, 860.0],
        [295.0, 70.0, 890.0],
    )]);
    set.load(&dir).unwrap();
    let slab = &set.slabs[0];
    assert_eq!(slab.tris.len(), 4);

    let col = slab.column(282.0, 875.0, 60.0);
    assert_eq!(col.len(), 2, "one surface per storey, got {col:?}");
    assert!((col[0].y - 43.4).abs() < 1e-3);
    assert!((col[1].y - 55.4).abs() < 1e-3);
    assert!((col[1].y - col[0].y - 12.0).abs() < 1e-3, "the 12 m step");
    assert!(col[0].tilt_degrees < 1e-3, "flat floor");

    let head = slab.headroom(282.0, 875.0, 43.4, 60.0).unwrap();
    assert!((head - 12.0).abs() < 1e-3, "headroom = {head}");
    assert!(
        slab.headroom(282.0, 875.0, 55.4, 60.0).is_none(),
        "nothing above the top storey"
    );
}

/// A wall with a gap in it: the clear width across the opening is what
/// decides whether Recast's `agentRadius` erosion closes the doorway.
#[test]
fn free_runs_measure_the_clear_width_of_an_opening() {
    let dir = tmpdir("door");
    let mut tris: Vec<[[f32; 3]; 3]> = Vec::new();
    tris.extend(floor_quad(0.0, 0.0, 20.0, 20.0, 0.0));
    // Wall along x = 10, z ∈ [0, 9] and z ∈ [10.2, 20]: a 1.2 m opening.
    tris.extend(wall_x(10.0, 0.0, 9.0, 0.0, 3.0));
    tris.extend(wall_x(10.0, 10.2, 20.0, 0.0, 3.0));
    write_chunk(&dir, "00000000o", &tris);

    let mut set = SlabSet::new(vec![Slab::new("door", [0.0, -1.0, 0.0], [20.0, 5.0, 20.0])]);
    set.load(&dir).unwrap();
    let slab = &set.slabs[0];

    // Band from knee to head height; only near-vertical geometry counts.
    let occ = slab.occupancy(0.2, 1.8, 0.1, 45.0);
    let widest = occ.widest_free_run([10.0, 0.0], [10.0, 20.0]);
    assert!(
        (widest - 1.2).abs() < 0.25,
        "expected a ~1.2 m opening, measured {widest:.2} m"
    );
    // Recast erodes by agentRadius on both sides: 1.2 m survives 0.6 m of
    // total erosion but not the 1.2 m that agentRadius = 0.6 at cs = 0.3
    // actually costs (2 cells each side).
    assert!(widest < 1.5, "this is the doorway-too-narrow shape");

    // A line that misses the wall entirely is clear end to end.
    let clear = occ.widest_free_run([2.0, 1.0], [2.0, 19.0]);
    assert!(clear > 17.0, "no wall at x = 2, measured {clear:.2} m");
}

/// A 60-degree ramp is a surface, not a wall — but it is over Recast's 45
/// degree limit, so it will not be walkable. `slope_profile` has to separate
/// the two.
#[test]
fn slope_profile_separates_ramps_from_walls_and_floors() {
    let dir = tmpdir("slope");
    let mut tris: Vec<[[f32; 3]; 3]> = Vec::new();
    tris.extend(floor_quad(0.0, 0.0, 10.0, 10.0, 0.0));
    // Ramp rising 10 m over 10 m in x ⇒ 45 degrees exactly; nudge to 60 by
    // rising 17.32 m over 10 m.
    tris.push([
        ue_cm([10.0, 0.0, 0.0]),
        ue_cm([20.0, 17.32, 0.0]),
        ue_cm([20.0, 17.32, 10.0]),
    ]);
    tris.extend(wall_x(25.0, 0.0, 10.0, 0.0, 5.0));
    write_chunk(&dir, "00000000o", &tris);

    let mut set = SlabSet::new(vec![Slab::new(
        "slope",
        [-1.0, -1.0, -1.0],
        [30.0, 20.0, 12.0],
    )]);
    set.load(&dir).unwrap();
    let profile = set.slabs[0].slope_profile(&[45.0, 75.0, 90.1]);

    assert!(
        (profile[0].1 - 100.0).abs() < 1.0,
        "the 10x10 floor is the only walkable area, got {} m^2",
        profile[0].1
    );
    assert!(profile[1].1 > 10.0, "the 60-degree ramp lands in 45..75");
    assert!(
        profile[2].1 < 1.0,
        "a vertical wall has no XZ footprint, got {} m^2",
        profile[2].1
    );
}

/// The chunk pre-filter must skip chunks that cannot touch the box and must
/// never skip one that can. A chunk stem that does not decode is read.
#[test]
fn the_chunk_prefilter_skips_only_out_of_range_chunks() {
    let dir = tmpdir("prefilter");
    // 00000000 → BW x,z ∈ [0,100]; 000a0002 → x ∈ [200,300], z ∈ [1000,1100].
    write_chunk(&dir, "00000000o", &floor_quad(10.0, 10.0, 20.0, 20.0, 5.0));
    write_chunk(
        &dir,
        "000a0002o",
        &floor_quad(260.0, 1035.0, 270.0, 1045.0, 66.0),
    );

    let mut set = SlabSet::new(vec![Slab::new(
        "zuritska",
        [265.0, 60.0, 1040.0],
        [272.0, 70.0, 1046.0],
    )]);
    set.margin = 10.0;
    set.load(&dir).unwrap();

    assert_eq!(set.chunks_read, vec!["000a0002o".to_string()]);
    assert_eq!(set.chunks_skipped, 1);
    assert!(!set.slabs[0].tris.is_empty());
    assert_eq!(
        set.slabs[0].by_chunk.get("000a0002o"),
        Some(&2),
        "attribution back to the source chunk"
    );
}

/// Triangles outside the box are not kept — the box is a filter, not a hint.
#[test]
fn triangles_outside_the_box_are_not_collected() {
    let dir = tmpdir("filter");
    let mut tris: Vec<[[f32; 3]; 3]> = Vec::new();
    tris.extend(floor_quad(0.0, 0.0, 5.0, 5.0, 0.0));
    tris.extend(floor_quad(50.0, 50.0, 55.0, 55.0, 0.0));
    write_chunk(&dir, "00000000o", &tris);

    let mut set = SlabSet::new(vec![Slab::new("near", [-1.0, -1.0, -1.0], [6.0, 1.0, 6.0])]);
    set.load(&dir).unwrap();
    assert_eq!(set.slabs[0].tris.len(), 2, "only the first quad");
}
