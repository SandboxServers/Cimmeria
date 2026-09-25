//! The accuracy sweep: random point pairs on a `.nav`, each scored by the
//! exact tracer (truth), the occluder, and a navmesh ray.
//!
//! This recreates NA16's measurement: same-storey pairs (`|dy|` within the
//! band) 4-30 m apart horizontally, standing on the navmesh, with a 1.5 m
//! eye height on both ends. The navmesh ray is a 2D polygon walk along the
//! mesh adjacency, the same question Detour's `dtRaycast` answers: does the
//! straight line leave the walkable surface before it arrives?

use cimmeria_occluder::{Occluder, Sight};

use super::exact::ExactScene;
use crate::nav_components::NavGraph;

/// Sweep knobs.
#[derive(Debug, Clone, Copy)]
pub struct SweepParams {
    pub pairs: usize,
    pub min_dist: f32,
    pub max_dist: f32,
    /// Largest `|dy|` between the two standing points.
    pub max_dy: f32,
    pub eye: f32,
    pub seed: u64,
    /// Sample only mesh components no larger than this (XZ m^2). The
    /// interiors sit inside a map-sized terrain component that would
    /// otherwise take nearly every sample; `None` samples everything.
    pub max_component_area: Option<f64>,
}

impl Default for SweepParams {
    fn default() -> Self {
        Self {
            pairs: 4000,
            min_dist: 4.0,
            max_dist: 30.0,
            max_dy: 4.0,
            eye: 1.5,
            seed: 0x4e41_3237,
            max_component_area: None,
        }
    }
}

/// A point standing on the mesh, with the polygon it stands on.
#[derive(Debug, Clone, Copy)]
pub struct NavPoint {
    pub pos: [f32; 3],
    pub poly: u32,
}

/// Deterministic generator (PCG-style LCG), so a sweep is reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed ^ 0x9e37_79b9_7f4a_7c15)
    }
    pub fn unit(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }
}

/// `n` points spread over the mesh by XZ area (fan-triangulated polygons,
/// uniform inside each triangle).
pub fn sample_points(
    graph: &NavGraph,
    n: usize,
    rng: &mut Rng,
    keep: impl Fn(usize) -> bool,
) -> Vec<NavPoint> {
    let mut tris: Vec<(u32, [[f32; 3]; 3])> = Vec::new();
    let mut cum: Vec<f64> = Vec::new();
    let mut total = 0.0f64;
    for (pi, poly) in graph.polys.iter().enumerate() {
        if !keep(pi) {
            continue;
        }
        let v: Vec<[f32; 3]> = poly
            .verts
            .iter()
            .map(|&i| graph.verts[i as usize])
            .collect();
        for k in 1..v.len().saturating_sub(1) {
            let t = [v[0], v[k], v[k + 1]];
            let area = (((t[1][0] - t[0][0]) * (t[2][2] - t[0][2])
                - (t[2][0] - t[0][0]) * (t[1][2] - t[0][2]))
                .abs()
                * 0.5) as f64;
            if area > 0.0 {
                total += area;
                cum.push(total);
                tris.push((pi as u32, t));
            }
        }
    }
    if tris.is_empty() {
        return Vec::new();
    }
    (0..n)
        .map(|_| {
            let r = rng.unit() as f64 * total;
            let i = cum.partition_point(|&c| c < r).min(tris.len() - 1);
            let (poly, t) = tris[i];
            let (mut u, mut v) = (rng.unit(), rng.unit());
            if u + v > 1.0 {
                u = 1.0 - u;
                v = 1.0 - v;
            }
            let p = |k: usize| t[0][k] + (t[1][k] - t[0][k]) * u + (t[2][k] - t[0][k]) * v;
            NavPoint {
                pos: [p(0), p(1), p(2)],
                poly,
            }
        })
        .collect()
}

/// Up to `params.pairs` pairs meeting the distance and storey constraints,
/// drawn from a pool of sampled points.
pub fn sample_pairs(graph: &NavGraph, params: &SweepParams) -> Vec<(NavPoint, NavPoint)> {
    let mut rng = Rng::new(params.seed);
    let small: std::collections::HashSet<u32> = match params.max_component_area {
        None => (0..graph.component_count).collect(),
        Some(max) => graph
            .component_stats()
            .iter()
            .filter(|c| c.area_xz <= max)
            .map(|c| c.id)
            .collect(),
    };
    let pool = sample_points(graph, 20_000, &mut rng, |pi| {
        small.contains(&graph.component[pi])
    });
    let mut out = Vec::with_capacity(params.pairs);
    if pool.len() < 2 {
        return out;
    }
    let mut tries = 0usize;
    while out.len() < params.pairs && tries < params.pairs * 2000 {
        tries += 1;
        let a = pool[(rng.unit() * pool.len() as f32) as usize % pool.len()];
        let b = pool[(rng.unit() * pool.len() as f32) as usize % pool.len()];
        let d = ((b.pos[0] - a.pos[0]).powi(2) + (b.pos[2] - a.pos[2]).powi(2)).sqrt();
        if d < params.min_dist || d > params.max_dist {
            continue;
        }
        if (b.pos[1] - a.pos[1]).abs() > params.max_dy {
            continue;
        }
        out.push((a, b));
    }
    out
}

/// A navmesh ray's answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavRay {
    Clear,
    Blocked,
}

/// Walk the mesh from `a` (on polygon `start`) toward `b` in XZ: clear when
/// the line reaches `b` without crossing a boundary edge.
pub fn nav_ray(graph: &NavGraph, start: u32, a: [f32; 3], b: [f32; 3]) -> NavRay {
    let (dx, dz) = (b[0] - a[0], b[2] - a[2]);
    let mut poly = start as usize;
    let mut t_cur = 0.0f32;
    let mut came_from: Option<usize> = None;
    for _ in 0..4096 {
        let p = &graph.polys[poly];
        let n = p.verts.len();
        // The exit edge: the nearest crossing ahead of the current point.
        let mut best: Option<(f32, usize)> = None;
        for e in 0..n {
            if Some(e) == came_from {
                continue;
            }
            let v0 = graph.verts[p.verts[e] as usize];
            let v1 = graph.verts[p.verts[(e + 1) % n] as usize];
            let (ex, ez) = (v1[0] - v0[0], v1[2] - v0[2]);
            let denom = dx * ez - dz * ex;
            if denom.abs() < 1e-12 {
                continue;
            }
            let (wx, wz) = (v0[0] - a[0], v0[2] - a[2]);
            let s = (wx * ez - wz * ex) / denom;
            let u = (wx * dz - wz * dx) / denom;
            if s >= t_cur - 1e-5 && (-1e-4..=1.0 + 1e-4).contains(&u) {
                match best {
                    Some((bs, _)) if bs <= s => {}
                    _ => best = Some((s, e)),
                }
            }
        }
        let Some((s, e)) = best else {
            // Degenerate: nothing ahead. Treat the end as reached.
            return NavRay::Clear;
        };
        if s >= 1.0 {
            return NavRay::Clear;
        }
        let Some(next) = p.neighbours[e] else {
            return NavRay::Blocked;
        };
        let next = next as usize;
        // The edge index on the far side that leads back here.
        came_from = graph.polys[next]
            .neighbours
            .iter()
            .position(|&nb| nb == Some(poly as u32));
        poly = next;
        t_cur = s;
    }
    NavRay::Blocked
}

/// One pair's three verdicts.
#[derive(Debug, Clone, Copy)]
pub struct PairVerdict {
    pub a: [f32; 3],
    pub b: [f32; 3],
    pub truth_blocked: bool,
    pub occ: Sight,
    pub nav: NavRay,
}

/// Confusion counts for one verdict source against the truth.
#[derive(Debug, Default, Clone, Copy)]
pub struct Confusion {
    /// Said clear, truth clear.
    pub clear_ok: usize,
    /// Said clear, truth blocked.
    pub clear_wrong: usize,
    /// Said blocked, truth blocked.
    pub blocked_ok: usize,
    /// Said blocked, truth clear.
    pub blocked_wrong: usize,
    /// No answer (occluder off-grid).
    pub unknown: usize,
}

impl Confusion {
    fn add(&mut self, said_blocked: Option<bool>, truth_blocked: bool) {
        match (said_blocked, truth_blocked) {
            (None, _) => self.unknown += 1,
            (Some(false), false) => self.clear_ok += 1,
            (Some(false), true) => self.clear_wrong += 1,
            (Some(true), true) => self.blocked_ok += 1,
            (Some(true), false) => self.blocked_wrong += 1,
        }
    }

    /// Share of `Blocked` answers that are wrong.
    pub fn wrong_given_blocked(&self) -> f64 {
        ratio(self.blocked_wrong, self.blocked_ok + self.blocked_wrong)
    }

    /// Share of `Clear` answers that are wrong.
    pub fn wrong_given_clear(&self) -> f64 {
        ratio(self.clear_wrong, self.clear_ok + self.clear_wrong)
    }

    /// Share of truly-clear pairs this source calls blocked.
    pub fn false_block_rate(&self) -> f64 {
        ratio(self.blocked_wrong, self.blocked_wrong + self.clear_ok)
    }

    /// Share of truly-blocked pairs this source calls clear.
    pub fn false_clear_rate(&self) -> f64 {
        ratio(self.clear_wrong, self.clear_wrong + self.blocked_ok)
    }
}

fn ratio(a: usize, b: usize) -> f64 {
    if b == 0 {
        0.0
    } else {
        a as f64 / b as f64
    }
}

/// Score every pair.
pub fn run(
    graph: &NavGraph,
    exact: &ExactScene,
    occ: &Occluder,
    pairs: &[(NavPoint, NavPoint)],
    eye: f32,
) -> (Vec<PairVerdict>, Confusion, Confusion) {
    let mut occ_c = Confusion::default();
    let mut nav_c = Confusion::default();
    let mut rows = Vec::with_capacity(pairs.len());
    for (pa, pb) in pairs {
        let a = [pa.pos[0], pa.pos[1] + eye, pa.pos[2]];
        let b = [pb.pos[0], pb.pos[1] + eye, pb.pos[2]];
        let truth_blocked = exact.blocked(a, b);
        let o = occ.sight(a, b);
        let nav = nav_ray(graph, pa.poly, pa.pos, pb.pos);
        occ_c.add(
            match o {
                Sight::Clear => Some(false),
                Sight::Blocked { .. } => Some(true),
                Sight::OffGrid => None,
            },
            truth_blocked,
        );
        nav_c.add(Some(nav == NavRay::Blocked), truth_blocked);
        rows.push(PairVerdict {
            a,
            b,
            truth_blocked,
            occ: o,
            nav,
        });
    }
    (rows, occ_c, nav_c)
}
