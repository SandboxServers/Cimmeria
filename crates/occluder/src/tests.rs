//! Synthetic-geometry tests for the builder, the file format and the
//! segment test. Coordinates are BigWorld metres, Y up.

use super::*;

/// The 12 triangles of an axis-aligned box.
fn cuboid(min: [f32; 3], max: [f32; 3]) -> Vec<Triangle> {
    let c = |i: usize| {
        [
            if i & 1 == 0 { min[0] } else { max[0] },
            if i & 2 == 0 { min[1] } else { max[1] },
            if i & 4 == 0 { min[2] } else { max[2] },
        ]
    };
    let quads = [
        [0, 1, 3, 2],
        [4, 5, 7, 6],
        [0, 1, 5, 4],
        [2, 3, 7, 6],
        [0, 2, 6, 4],
        [1, 3, 7, 5],
    ];
    quads
        .iter()
        .flat_map(|q| [[c(q[0]), c(q[1]), c(q[2])], [c(q[0]), c(q[2]), c(q[3])]])
        .collect()
}

/// A flat floor quad at height `y`.
fn floor(x0: f32, z0: f32, x1: f32, z1: f32, y: f32) -> Vec<Triangle> {
    vec![
        [[x0, y, z0], [x1, y, z0], [x1, y, z1]],
        [[x0, y, z0], [x1, y, z1], [x0, y, z1]],
    ]
}

fn build(tris: &[Triangle], params: BuildParams) -> Occluder {
    let mut b = OccluderBuilder::new(params, "test").unwrap();
    for t in tris {
        b.add_triangle(t, Source::Geometry);
    }
    b.finish().unwrap()
}

/// A 40 x 40 m room at y 0 with a wall across x = 20 (0.3 m thick, 4 m
/// tall), a 1 m desk at x 10, and a second storey slab at y 8 over the
/// east half.
fn room() -> Vec<Triangle> {
    let mut t = floor(0.0, 0.0, 40.0, 40.0, 0.0);
    t.extend(cuboid([19.85, 0.0, 0.0], [20.15, 4.0, 40.0]));
    t.extend(cuboid([9.5, 0.0, 15.0], [10.5, 1.0, 25.0]));
    t.extend(cuboid([25.0, 7.8, 0.0], [40.0, 8.0, 40.0]));
    t
}

const EYE: f32 = 1.5;

fn blocked(s: Sight) -> bool {
    matches!(s, Sight::Blocked { .. })
}

#[test]
fn a_wall_blocks_an_eye_height_ray() {
    let occ = build(&room(), BuildParams::default());
    let s = occ.sight([15.0, EYE, 20.0], [25.0, EYE, 20.0]);
    let Sight::Blocked { at, .. } = s else {
        panic!("{s:?}");
    };
    assert!((19.0..=20.5).contains(&at[0]), "{at:?}");
}

#[test]
fn an_eye_height_ray_sees_over_a_low_desk_but_a_knee_height_one_does_not() {
    let occ = build(&room(), BuildParams::default());
    assert_eq!(occ.sight([5.0, EYE, 20.0], [15.0, EYE, 20.0]), Sight::Clear);
    assert!(blocked(occ.sight([5.0, 0.5, 20.0], [15.0, 0.5, 20.0])));
}

#[test]
fn under_a_ceiling_is_clear_and_through_it_is_blocked() {
    let occ = build(&room(), BuildParams::default());
    assert_eq!(occ.sight([26.0, EYE, 5.0], [38.0, EYE, 30.0]), Sight::Clear);
    // Ground floor to the storey above, through the slab.
    assert!(blocked(
        occ.sight([27.0, EYE, 10.0], [35.0, 8.0 + EYE, 12.0])
    ));
    // The upper storey sees along itself.
    assert_eq!(
        occ.sight([26.0, 8.0 + EYE, 5.0], [38.0, 8.0 + EYE, 30.0]),
        Sight::Clear
    );
}

#[test]
fn a_window_in_a_wall_passes_eye_height_only() {
    let mut t = floor(0.0, 0.0, 20.0, 20.0, 0.0);
    // Wall at x 10 with a gap from 1.2 to 2.0 m.
    t.extend(cuboid([9.9, 0.0, 0.0], [10.1, 1.2, 20.0]));
    t.extend(cuboid([9.9, 2.0, 0.0], [10.1, 3.0, 20.0]));
    let occ = build(&t, BuildParams::default());
    assert_eq!(occ.sight([5.0, EYE, 10.0], [15.0, EYE, 10.0]), Sight::Clear);
    assert!(blocked(occ.sight([5.0, 0.8, 10.0], [15.0, 0.8, 10.0])));
    assert!(blocked(occ.sight([5.0, 2.5, 10.0], [15.0, 2.5, 10.0])));
}

#[test]
fn a_unit_hugging_a_wall_still_sees_along_it() {
    let occ = build(&room(), BuildParams::default());
    // 0.6 m (an agent radius) off the wall face, looking along it.
    assert_eq!(
        occ.sight([19.25, EYE, 2.0], [19.25, EYE, 30.0]),
        Sight::Clear
    );
}

#[test]
fn a_point_outside_the_coverage_is_off_grid() {
    let occ = build(&room(), BuildParams::default());
    assert_eq!(
        occ.sight([5.0, EYE, 5.0], [500.0, EYE, 5.0]),
        Sight::OffGrid
    );
    assert_eq!(
        occ.sight([5.0, f32::NAN, 5.0], [6.0, EYE, 5.0]),
        Sight::OffGrid
    );
}

#[test]
fn a_wall_far_from_any_floor_is_trimmed_and_near_one_is_kept() {
    let mut t = floor(0.0, 0.0, 10.0, 10.0, 0.0);
    t.extend(cuboid([4.0, 0.0, 0.0], [4.2, 3.0, 10.0]));
    // A free-standing sheet of wall 100 m away, no floor near it.
    t.push([
        [100.0, 50.0, 100.0],
        [100.0, 53.0, 100.0],
        [100.0, 50.0, 110.0],
    ]);
    let occ = build(&t, BuildParams::default());
    assert!(occ.covers(4.1, 5.0));
    assert!(!occ.covers(100.0, 101.0));
    let untrimmed = build(
        &t,
        BuildParams {
            margin: None,
            ..BuildParams::default()
        },
    );
    assert!(untrimmed.covers(100.0, 101.0));
}

/// Lattice terrain: 1 m patches over `[0, n)^2`, height `h(x, z)`, split
/// along the main diagonal (or the other one with `anti`), skipping the
/// patches `hole` names.
fn terrain(
    n: i32,
    h: impl Fn(f32, f32) -> f32,
    anti: bool,
    hole: impl Fn(i32, i32) -> bool,
) -> Vec<Triangle> {
    let mut out = Vec::new();
    for i in 0..n {
        for j in 0..n {
            if hole(i, j) {
                continue;
            }
            let (x0, x1, z0, z1) = (i as f32, (i + 1) as f32, j as f32, (j + 1) as f32);
            let v = |x: f32, z: f32| [x, h(x, z), z];
            if anti {
                out.push([v(x1, z0), v(x0, z1), v(x0, z0)]);
                out.push([v(x1, z0), v(x0, z1), v(x1, z1)]);
            } else {
                out.push([v(x0, z0), v(x1, z0), v(x1, z1)]);
                out.push([v(x0, z0), v(x1, z1), v(x0, z1)]);
            }
        }
    }
    out
}

fn ridge(x: f32, _z: f32) -> f32 {
    (5.0 - (x - 20.0).abs() * 0.5).max(0.0)
}

fn build_terrain(tris: &[Triangle], params: BuildParams) -> (Occluder, u64) {
    let mut b = OccluderBuilder::new(params, "terrain").unwrap();
    for t in tris {
        b.add_triangle(t, Source::Terrain);
    }
    let fallback = b.terrain_fallback_count();
    (b.finish().unwrap(), fallback)
}

#[test]
fn terrain_on_the_heightfield_blocks_a_ray_across_a_hill() {
    let (occ, fallback) = build_terrain(
        &terrain(40, ridge, false, |_, _| false),
        BuildParams::default(),
    );
    assert_eq!(fallback, 0, "every lattice triangle is taken");
    assert!(occ.layers().is_empty(), "no span layer without geometry");
    let hf = occ.heightfield().expect("a heightfield");
    assert_eq!(hf.patch_count(), 1600);
    let s = occ.sight([2.0, EYE, 20.0], [38.0, EYE, 20.0]);
    assert!(
        matches!(
            s,
            Sight::Blocked {
                layer: LayerKind::Terrain,
                ..
            }
        ),
        "{s:?}"
    );
    // Along the ridge line itself, at the foot, nothing is in the way.
    assert_eq!(occ.sight([2.0, EYE, 2.0], [2.0, EYE, 38.0]), Sight::Clear);
}

/// The heightfield is exact where a span column is not: a ray 0.2 m over
/// a 1-in-2 slope clears it, although each 1 m patch there rises 0.5 m.
#[test]
fn a_ray_skimming_a_slope_clears_the_heightfield_but_not_a_span_column() {
    let tris = terrain(40, ridge, false, |_, _| false);
    // Along z on the slope at x = 16.5 (height 3.25), 0.2 m above it.
    let (a, b) = ([16.5, 3.45, 3.0], [16.5, 3.45, 30.0]);
    let (hf, _) = build_terrain(&tris, BuildParams::default());
    assert_eq!(hf.sight(a, b), Sight::Clear);
    let (spans, _) = build_terrain(
        &tris,
        BuildParams {
            terrain_pitch: None,
            ..BuildParams::default()
        },
    );
    assert!(spans.heightfield().is_none());
    assert!(blocked(spans.sight(a, b)), "the span grid over-blocks here");
}

#[test]
fn heightfield_holes_and_either_diagonal_answer_exactly() {
    // A 3 m plateau with a hole punched through its middle.
    let plateau = |_: f32, _: f32| 3.0;
    let hole = |i: i32, j: i32| (8..12).contains(&i) && (8..12).contains(&j);
    for anti in [false, true] {
        let (occ, fallback) =
            build_terrain(&terrain(20, plateau, anti, hole), BuildParams::default());
        assert_eq!(fallback, 0);
        // Down through the plateau: blocked. Down through the hole: clear.
        assert!(blocked(occ.sight([3.0, 5.0, 3.0], [5.0, 1.0, 5.0])));
        assert_eq!(occ.sight([9.5, 5.0, 9.5], [10.5, 1.0, 10.5]), Sight::Clear);
        let back = format::decode(&format::encode(&occ)).unwrap();
        assert_eq!(back, occ);
    }
}

#[test]
fn terrain_off_the_lattice_falls_back_to_the_span_layer() {
    // Shifted by 0.3 m: no triangle is a lattice half-patch.
    let tris: Vec<Triangle> = terrain(10, ridge, false, |_, _| false)
        .into_iter()
        .map(|t| t.map(|v| [v[0] + 0.3, v[1], v[2]]))
        .collect();
    let (occ, fallback) = build_terrain(&tris, BuildParams::default());
    assert_eq!(fallback, tris.len() as u64);
    assert!(occ.heightfield().is_none());
    assert_eq!(occ.layers().len(), 1);
}

#[test]
fn encode_then_decode_is_identity() {
    let occ = build(&room(), BuildParams::default());
    let bytes = format::encode(&occ);
    let back = format::decode(&bytes).unwrap();
    assert_eq!(back, occ);
    assert_eq!(back.content_hash(), grid_hash(&bytes));
    assert_eq!(back.short_hash().len(), 8);
}

fn grid_hash(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

#[test]
fn a_build_is_deterministic() {
    let a = format::encode(&build(&room(), BuildParams::default()));
    let mut tris = room();
    tris.reverse();
    // Same triangles, other order: the grid agrees; only the source hash,
    // which is order-sensitive by design, may differ.
    let b = build(&tris, BuildParams::default());
    let a = format::decode(&a).unwrap();
    assert_eq!(a.layers(), b.layers());
    assert_eq!(
        format::encode(&build(&room(), BuildParams::default())),
        format::encode(&a)
    );
}

#[test]
fn damaged_files_are_rejected_not_misread() {
    let bytes = format::encode(&build(&room(), BuildParams::default()));
    assert!(matches!(
        format::decode(&bytes[..10]),
        Err(OccluderError::Malformed(_))
    ));
    let mut bad = bytes.clone();
    bad[0] = b'X';
    assert!(matches!(format::decode(&bad), Err(OccluderError::BadMagic)));
    let mut bad = bytes.clone();
    bad[4] = 9;
    assert!(matches!(
        format::decode(&bad),
        Err(OccluderError::Version(9))
    ));
    let mut bad = bytes.clone();
    let last = bad.len() - 3;
    bad[last] ^= 0x55;
    assert!(
        format::decode(&bad).is_err(),
        "zlib checksum catches payload damage"
    );
    let mut long = bytes;
    long.push(0);
    assert!(format::decode(&long).is_err());
}

#[test]
fn bad_parameters_are_refused() {
    let p = BuildParams {
        cell: 0.0,
        ..BuildParams::default()
    };
    assert!(OccluderBuilder::new(p, "x").is_err());
    let b = OccluderBuilder::new(BuildParams::default(), "x").unwrap();
    assert_eq!(b.finish().unwrap_err(), BuildError::Empty);
}

/// Exact segment/triangle intersection (Moller-Trumbore), both faces.
fn segment_hits_triangle(a: [f32; 3], b: [f32; 3], t: &Triangle) -> bool {
    let sub = |p: [f32; 3], q: [f32; 3]| [p[0] - q[0], p[1] - q[1], p[2] - q[2]];
    let cross = |p: [f32; 3], q: [f32; 3]| {
        [
            p[1] * q[2] - p[2] * q[1],
            p[2] * q[0] - p[0] * q[2],
            p[0] * q[1] - p[1] * q[0],
        ]
    };
    let dot = |p: [f32; 3], q: [f32; 3]| p[0] * q[0] + p[1] * q[1] + p[2] * q[2];
    let d = sub(b, a);
    let e1 = sub(t[1], t[0]);
    let e2 = sub(t[2], t[0]);
    let p = cross(d, e2);
    let det = dot(e1, p);
    if det.abs() < 1e-9 {
        return false;
    }
    let inv = 1.0 / det;
    let s = sub(a, t[0]);
    let u = dot(s, p) * inv;
    if !(0.0..=1.0).contains(&u) {
        return false;
    }
    let q = cross(s, e1);
    let v = dot(d, q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return false;
    }
    let w = dot(e2, q) * inv;
    (0.0..=1.0).contains(&w)
}

/// Tiny deterministic generator so the fuzz run is reproducible.
struct Lcg(u64);
impl Lcg {
    fn f(&mut self, lo: f32, hi: f32) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        lo + (hi - lo) * ((self.0 >> 40) as f32 / (1u64 << 24) as f32)
    }
}

/// The grid may over-block (whole cells, rounded-out spans) but must never
/// see through geometry: whenever the exact segment, minus the endpoint
/// clearance, meets a triangle, the grid says blocked.
#[test]
fn fuzz_the_grid_never_sees_through_geometry() {
    let mut rng = Lcg(0x5eed);
    for cell in [0.25f32, 0.5, 1.0] {
        let mut tris = floor(0.0, 0.0, 60.0, 60.0, 0.0);
        for _ in 0..60 {
            let (x, z) = (rng.f(2.0, 56.0), rng.f(2.0, 56.0));
            let (w, d, h) = (rng.f(0.1, 4.0), rng.f(0.1, 4.0), rng.f(0.3, 4.0));
            let y = rng.f(0.0, 3.0);
            tris.extend(cuboid([x, y, z], [x + w, y + h, z + d]));
        }
        let occ = build(
            &tris,
            BuildParams {
                cell,
                ..BuildParams::default()
            },
        );
        let mut exact_blocked = 0;
        for _ in 0..3000 {
            let a = [rng.f(1.0, 59.0), rng.f(0.2, 5.0), rng.f(1.0, 59.0)];
            let b = [rng.f(1.0, 59.0), rng.f(0.2, 5.0), rng.f(1.0, 59.0)];
            let len = ((b[0] - a[0]).powi(2) + (b[2] - a[2]).powi(2)).sqrt();
            let clear = Occluder::DEFAULT_CLEARANCE;
            if len < 2.0 * clear + 0.01 {
                continue;
            }
            let (t0, t1) = (clear / len, 1.0 - clear / len);
            let lerp = |t: f32| {
                [
                    a[0] + (b[0] - a[0]) * t,
                    a[1] + (b[1] - a[1]) * t,
                    a[2] + (b[2] - a[2]) * t,
                ]
            };
            let (ia, ib) = (lerp(t0), lerp(t1));
            if tris.iter().any(|t| segment_hits_triangle(ia, ib, t)) {
                exact_blocked += 1;
                let s = occ.sight(a, b);
                assert!(
                    blocked(s),
                    "cell {cell}: {a:?} -> {b:?} is blocked, grid says {s:?}"
                );
            }
        }
        assert!(
            exact_blocked > 300,
            "the fuzz exercised too few walls: {exact_blocked}"
        );
    }
}

#[test]
fn uniform_and_variable_tiles_round_trip_and_answer_alike() {
    // One tile of pure floor (uniform, 1 span per cell) next to one with a
    // pillar (variable).
    let mut t = floor(0.0, 0.0, 16.0, 8.0, 0.0);
    t.extend(cuboid([12.0, 0.0, 2.0], [12.4, 3.0, 2.4]));
    let occ = build(&t, BuildParams::default());
    let back = format::decode(&format::encode(&occ)).unwrap();
    assert_eq!(back, occ);
    assert!(blocked(back.sight([10.0, EYE, 2.2], [14.0, EYE, 2.2])));
    assert_eq!(back.sight([1.0, EYE, 2.2], [7.0, EYE, 2.2]), Sight::Clear);
    let col = back.column(12.2, 2.2);
    assert!(
        col.iter().any(|&(_, lo, hi)| lo <= 0.0 && hi >= 2.9),
        "{col:?}"
    );
}

/// The same guarantee with walls at arbitrary angles, where a span's
/// rectangle is loose, and single-sided sheets with no thickness.
#[test]
fn fuzz_rotated_sheets_never_leak() {
    let mut rng = Lcg(0xa11e);
    for cell in [0.25f32, 0.5, 1.0] {
        let mut tris = floor(0.0, 0.0, 60.0, 60.0, 0.0);
        for _ in 0..80 {
            let (x, z) = (rng.f(3.0, 57.0), rng.f(3.0, 57.0));
            let ang = rng.f(0.0, std::f32::consts::TAU);
            let len = rng.f(0.5, 6.0);
            let (ex, ez) = (x + ang.cos() * len, z + ang.sin() * len);
            let (y0, y1) = (rng.f(0.0, 2.0), rng.f(2.0, 4.0));
            tris.push([[x, y0, z], [ex, y0, ez], [ex, y1, ez]]);
            tris.push([[x, y0, z], [ex, y1, ez], [x, y1, z]]);
        }
        let occ = build(
            &tris,
            BuildParams {
                cell,
                ..BuildParams::default()
            },
        );
        let mut exact_blocked = 0;
        for _ in 0..3000 {
            let a = [rng.f(1.0, 59.0), rng.f(0.2, 4.5), rng.f(1.0, 59.0)];
            let b = [rng.f(1.0, 59.0), rng.f(0.2, 4.5), rng.f(1.0, 59.0)];
            let len = ((b[0] - a[0]).powi(2) + (b[2] - a[2]).powi(2)).sqrt();
            let clear = Occluder::DEFAULT_CLEARANCE;
            if len < 2.0 * clear + 0.01 {
                continue;
            }
            let (t0, t1) = (clear / len, 1.0 - clear / len);
            let lerp = |t: f32| {
                [
                    a[0] + (b[0] - a[0]) * t,
                    a[1] + (b[1] - a[1]) * t,
                    a[2] + (b[2] - a[2]) * t,
                ]
            };
            if tris
                .iter()
                .any(|t| segment_hits_triangle(lerp(t0), lerp(t1), t))
            {
                exact_blocked += 1;
                let s = occ.sight(a, b);
                assert!(
                    blocked(s),
                    "cell {cell}: {a:?} -> {b:?} is blocked, grid says {s:?}"
                );
            }
        }
        assert!(
            exact_blocked > 300,
            "too few walls exercised: {exact_blocked}"
        );
    }
}

/// The sub-cell rectangles let a ray pass close by the end of a wall: a
/// wall ending in the same cell as the ray, 0.1 m short of it, does not
/// block it.
#[test]
fn a_ray_past_the_end_of_a_wall_is_clear_within_its_cell() {
    let mut t = floor(0.0, 0.0, 20.0, 20.0, 0.0);
    // A 0.1 m thick wall at x 10, ending at z = 10.05 (mid-cell at 0.5 m).
    t.extend(cuboid([9.95, 0.0, 0.0], [10.05, 3.0, 10.05]));
    let occ = build(&t, BuildParams::default());
    assert_eq!(occ.sight([5.0, EYE, 10.2], [15.0, EYE, 10.2]), Sight::Clear);
    assert!(blocked(occ.sight([5.0, EYE, 9.9], [15.0, EYE, 9.9])));
}
