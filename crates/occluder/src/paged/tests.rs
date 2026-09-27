//! The paged occluder answers exactly what the unpaged one does, and its
//! residency follows the players.

use super::*;
use crate::{BuildParams, OccluderBuilder, Source, Triangle};

fn cuboid(min: [f32; 3], max: [f32; 3]) -> Vec<Triangle> {
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

/// A 300 x 300 m world straddling the origin (negative page coordinates
/// too): rolling lattice terrain, walls and boxes scattered over it.
fn world(rng: &mut Lcg) -> Occluder {
    let mut b = OccluderBuilder::new(BuildParams::default(), "paged-test").unwrap();
    let h = |x: f32, z: f32| 2.0 * (x * 0.05).sin() + 1.5 * (z * 0.07).cos();
    for i in -150..150 {
        for j in -150..150 {
            let (x0, x1, z0, z1) = (i as f32, (i + 1) as f32, j as f32, (j + 1) as f32);
            let v = |x: f32, z: f32| [x, h(x, z), z];
            b.add_triangle(&[v(x0, z0), v(x1, z0), v(x1, z1)], Source::Terrain);
            b.add_triangle(&[v(x0, z0), v(x1, z1), v(x0, z1)], Source::Terrain);
        }
    }
    for _ in 0..400 {
        let (x, z) = (rng.f(-148.0, 146.0), rng.f(-148.0, 146.0));
        let (w, d, hh) = (rng.f(0.1, 6.0), rng.f(0.1, 6.0), rng.f(0.5, 5.0));
        let y = h(x, z) - 0.5;
        for t in cuboid([x, y, z], [x + w, y + hh, z + d]) {
            b.add_triangle(&t, Source::Geometry);
        }
    }
    b.finish().unwrap()
}

#[test]
fn a_paged_occluder_answers_exactly_what_the_unpaged_one_does() {
    let mut rng = Lcg(0x9a9e);
    let occ = world(&mut rng);
    let paged = PagedOccluder::from_bytes(encode_paged(&occ, DEFAULT_PAGE_SIZE).unwrap()).unwrap();
    assert!(
        paged.grid().px0 < 0 && paged.grid().nx >= 5,
        "{:?}",
        paged.grid()
    );
    let (mut blocked, mut clear) = (0, 0);
    for _ in 0..4000 {
        let a = [rng.f(-149.0, 149.0), 0.0, rng.f(-149.0, 149.0)];
        let ang = rng.f(0.0, std::f32::consts::TAU);
        let len = rng.f(1.0, 90.0);
        let b = [a[0] + ang.cos() * len, 0.0, a[2] + ang.sin() * len];
        let a = [a[0], rng.f(-1.0, 6.0), a[2]];
        let b = [b[0], rng.f(-1.0, 6.0), b[2]];
        let (u, p) = (occ.sight(a, b), paged.sight(a, b));
        match (u, p) {
            // Both blocked. The layer may differ: the unpaged test runs the
            // geometry layer over the whole segment before the terrain,
            // the paged one runs both per page, nearest page first.
            (Sight::Blocked { .. }, Sight::Blocked { .. }) => blocked += 1,
            _ => {
                assert_eq!(u, p, "{a:?} -> {b:?}");
                clear += usize::from(u == Sight::Clear);
            }
        }
    }
    assert!(
        blocked > 500 && clear > 500,
        "blocked {blocked} clear {clear}"
    );
}

#[test]
fn residency_follows_the_players_and_a_query_unpacks_what_it_needs() {
    let mut rng = Lcg(7);
    let occ = world(&mut rng);
    let full: usize = occ.ram_bytes();
    let paged = PagedOccluder::from_bytes(encode_paged(&occ, DEFAULT_PAGE_SIZE).unwrap()).unwrap();
    let st = paged.stats();
    assert_eq!(st.resident_pages, 0, "loading unpacks nothing");
    assert!(st.pages >= 25);
    // Within a page's rounding of the unpaged size (per-page headers).
    let sum = paged.full_ram_bytes();
    assert!(
        sum >= full / 2 && sum <= full * 2,
        "pages {sum} vs whole {full}"
    );

    let r = paged.retain_near(&[[0.0, 0.0]], 40.0);
    assert_eq!(r.evicted, Vec::<u32>::new());
    assert!(!r.unpacked.is_empty() && r.unpacked.len() <= 9, "{r:?}");
    assert_eq!(r.resident_pages, r.unpacked.len());

    // A query far away unpacks the pages it crosses, counted as query unpacks.
    let before = paged.stats().query_unpacks;
    let _ = paged.sight([120.0, 3.0, 120.0], [140.0, 3.0, 130.0]);
    assert!(paged.stats().query_unpacks > before);

    // The player moves: the old pages and the query's pages go.
    let r2 = paged.retain_near(&[[-120.0, -120.0]], 40.0);
    assert!(r2.evicted.len() >= r.unpacked.len(), "{r2:?}");
    assert!(r2.unpacked.iter().all(|i| !r.unpacked.contains(i)));
    let st = paged.stats();
    assert_eq!(st.resident_pages, r2.resident_pages);
    assert!(st.resident_bytes < sum);

    // Nobody left: everything packed again.
    let r3 = paged.retain_near(&[], 40.0);
    assert_eq!(r3.resident_pages, 0);
    assert_eq!(paged.stats().resident_bytes, 0);
}

#[test]
fn damaged_paged_files_are_rejected() {
    let mut rng = Lcg(3);
    let bytes = encode_paged(&world(&mut rng), DEFAULT_PAGE_SIZE).unwrap();
    assert!(PagedOccluder::from_bytes(bytes[..20].to_vec()).is_err());
    let mut bad = bytes.clone();
    bad[0] = b'X';
    assert!(matches!(
        PagedOccluder::from_bytes(bad),
        Err(OccluderError::BadMagic)
    ));
    let mut long = bytes.clone();
    long.push(1);
    assert!(PagedOccluder::from_bytes(long).is_err());
    // A damaged blob fails when its page is unpacked, not at load: the page
    // then reads as absent (off the grid), never as open space.
    let mut bad_blob = bytes;
    let n = bad_blob.len();
    bad_blob[n - 5] ^= 0xFF;
    let paged = PagedOccluder::from_bytes(bad_blob).unwrap();
    paged.unpack_all();
    assert!(paged.stats().resident_pages < paged.stats().pages);
}

#[test]
fn a_page_size_that_does_not_tile_the_layers_is_refused() {
    let mut rng = Lcg(5);
    let occ = world(&mut rng);
    assert!(
        encode_paged(&occ, 20.0).is_err(),
        "20 m is not a multiple of 16 m terrain tiles"
    );
    assert!(encode_paged(&occ, 32.0).is_ok());
}
