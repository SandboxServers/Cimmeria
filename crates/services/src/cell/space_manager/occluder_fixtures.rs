//! Test-only occluders built from boxes with the shipped builder (the
//! shipped 0.5 m cell) and the paged encoding, so a line-of-sight test can
//! place a wall exactly where it needs one (NA31).

use std::sync::Arc;

use cimmeria_occluder::{
    encode_paged, BuildParams, OccluderBuilder, PagedOccluder, Source, Triangle,
};

/// The 12 triangles of an axis-aligned box.
pub(crate) fn cuboid(min: [f32; 3], max: [f32; 3]) -> Vec<Triangle> {
    let c = |i: usize| {
        [
            if i & 1 == 0 { min[0] } else { max[0] },
            if i & 2 == 0 { min[1] } else { max[1] },
            if i & 4 == 0 { min[2] } else { max[2] },
        ]
    };
    [
        [0, 1, 3, 2],
        [4, 5, 7, 6],
        [0, 1, 5, 4],
        [2, 3, 7, 6],
        [0, 2, 6, 4],
        [1, 3, 7, 5],
    ]
    .iter()
    .flat_map(|q| [[c(q[0]), c(q[1]), c(q[2])], [c(q[0]), c(q[2]), c(q[3])]])
    .collect()
}

/// A 40 x 40 m floor at y 0 (x and z from 0 to 40) plus `solids`, each a
/// `(min, max)` box.
pub(crate) fn synthetic(solids: &[([f32; 3], [f32; 3])]) -> Arc<PagedOccluder> {
    let mut t: Vec<Triangle> = vec![
        [[0.0, 0.0, 0.0], [40.0, 0.0, 0.0], [40.0, 0.0, 40.0]],
        [[0.0, 0.0, 0.0], [40.0, 0.0, 40.0], [0.0, 0.0, 40.0]],
    ];
    for (min, max) in solids {
        t.extend(cuboid(*min, *max));
    }
    let mut b = OccluderBuilder::new(BuildParams::default(), "test").unwrap();
    for tri in &t {
        b.add_triangle(tri, Source::Geometry);
    }
    let occ = b.finish().unwrap();
    Arc::new(PagedOccluder::from_bytes(encode_paged(&occ, 64.0).unwrap()).unwrap())
}

/// A 4 m wall on x 19.85-20.15 that ends at z 20. From (5, 10), a target
/// one metre behind it (x 21) is in sight above z 20.81 and hidden below.
pub(crate) fn corner() -> Arc<PagedOccluder> {
    synthetic(&[([19.85, 0.0, 0.0], [20.15, 4.0, 20.0])])
}
